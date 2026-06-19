//! Binding-pattern destructuring compilation shared by declarations,
//! function parameters, loop heads, and catch parameters.

use qjs_ast::{BinaryOp, BindingPattern, ObjectBindingPropertyKey, VarKind};

use crate::{RuntimeError, Value};

use super::compiler::Compiler;
use super::ir::{ObjectRestExclusion, Op};

/// Compiler state for one protected array-destructuring region driven by a
/// lazily stepped iterator.
pub(super) struct ArrayDestructuring {
    iterator_slot: usize,
    next_slot: usize,
    done_slot: usize,
    enter: usize,
}

impl Compiler {
    /// Compiles a declaration initializer, applying NamedEvaluation when the
    /// binding is a single identifier and the initializer is an anonymous
    /// function or class. Destructuring patterns never name their values, so
    /// they fall back to the ordinary expression path.
    pub(super) fn compile_declaration_init(
        &mut self,
        pattern: &BindingPattern,
        init: &qjs_ast::Expr,
    ) -> Result<(), RuntimeError> {
        if let BindingPattern::Identifier { name, .. } = pattern {
            self.compile_named_expr(init, name)
        } else {
            self.compile_expr(init)
        }
    }

    pub(super) fn compile_binding_initializer(
        &mut self,
        pattern: &BindingPattern,
        kind: VarKind,
    ) -> Result<(), RuntimeError> {
        match pattern {
            BindingPattern::Identifier { name, .. } => {
                let slot = self.declare_var_kind_slot(name, kind);
                self.emit_store_var_initializer(slot, name, kind);
            }
            BindingPattern::Array { elements, rest, .. } => {
                let destructuring = self.begin_array_destructuring();
                for element in elements {
                    self.emit_iterator_step(&destructuring);
                    let Some(element) = element else {
                        self.emit(Op::Pop);
                        continue;
                    };
                    self.compile_binding_default(
                        element.default.as_ref(),
                        binding_inferred_name(&element.binding),
                    )?;
                    self.compile_binding_initializer(&element.binding, kind)?;
                }
                if let Some(rest) = rest {
                    self.emit_iterator_rest(&destructuring);
                    self.compile_binding_initializer(rest, kind)?;
                }
                self.end_array_destructuring(&destructuring);
            }
            BindingPattern::Object {
                properties, rest, ..
            } => {
                self.emit(Op::RequireObjectCoercible);
                let source_slot = self.temp_local("object_binding_source");
                self.emit(Op::StoreLocal(source_slot));
                let mut excluded = Vec::with_capacity(properties.len());
                for property in properties {
                    let exclusion = self.compile_object_binding_key(&property.key)?;
                    excluded.push(exclusion);
                    self.emit(Op::LoadLocal(source_slot));
                    self.load_object_binding_key(&property.key, &excluded);
                    self.emit(Op::GetProp);
                    self.compile_binding_default(
                        property.default.as_ref(),
                        binding_inferred_name(&property.binding),
                    )?;
                    self.compile_binding_initializer(&property.binding, kind)?;
                }
                if let Some(rest) = rest {
                    self.emit(Op::LoadLocal(source_slot));
                    self.emit(Op::ObjectRestExcluding { excluded });
                    self.compile_binding_initializer(rest, kind)?;
                }
            }
        }
        Ok(())
    }

    fn compile_object_binding_key(
        &mut self,
        key: &ObjectBindingPropertyKey,
    ) -> Result<ObjectRestExclusion, RuntimeError> {
        match key {
            ObjectBindingPropertyKey::Literal(key) => Ok(ObjectRestExclusion::Literal(key.clone())),
            ObjectBindingPropertyKey::Computed(expr) => {
                self.compile_expr(expr)?;
                self.emit(Op::ToPropertyKey);
                let slot = self.temp_local("object_binding_key");
                self.emit(Op::StoreLocal(slot));
                Ok(ObjectRestExclusion::Local(slot))
            }
        }
    }

    fn load_object_binding_key(
        &mut self,
        key: &ObjectBindingPropertyKey,
        excluded: &[ObjectRestExclusion],
    ) {
        match key {
            ObjectBindingPropertyKey::Literal(key) => {
                let key = self.const_slot(Value::String(key.clone().into()));
                self.emit(Op::LoadConst(key));
            }
            ObjectBindingPropertyKey::Computed(_) => {
                let Some(ObjectRestExclusion::Local(slot)) = excluded.last() else {
                    unreachable!("computed binding key should record a local exclusion");
                };
                self.emit(Op::LoadLocal(*slot));
            }
        }
    }

    pub(super) fn compile_binding_uninitialized(
        &mut self,
        pattern: &BindingPattern,
        kind: VarKind,
    ) -> Result<(), RuntimeError> {
        if kind == VarKind::Var {
            return Ok(());
        }
        for name in pattern.names() {
            let slot = self.declare_var_kind_slot(&name, kind);
            self.emit_load_undefined();
            self.emit_store_var_binding(slot, &name, kind);
        }
        Ok(())
    }

    pub(super) fn compile_binding_default(
        &mut self,
        default: Option<&qjs_ast::Expr>,
        inferred_name: Option<&str>,
    ) -> Result<(), RuntimeError> {
        let Some(default) = default else {
            return Ok(());
        };

        let value_slot = self.temp_local("binding_value");
        self.emit(Op::StoreLocal(value_slot));
        self.emit(Op::LoadLocal(value_slot));
        self.emit_load_undefined();
        self.emit(Op::Binary(BinaryOp::StrictEq));
        let keep_existing = self.emit(Op::JumpIfFalse(usize::MAX));
        self.emit(Op::Pop);
        // A default value bound to a single identifier (`{ f = function(){} }`,
        // `[f = () => {}]`, or a parameter default `g(f = class {})`) gets the
        // binding name via NamedEvaluation.
        if let Some(name) = inferred_name {
            self.compile_named_expr(default, name)?;
        } else {
            self.compile_expr(default)?;
        }
        let done = self.emit(Op::Jump(usize::MAX));
        let keep_existing_target = self.code.len();
        self.patch_jump(keep_existing, keep_existing_target);
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(value_slot));
        let done_target = self.code.len();
        self.patch_jump(done, done_target);
        Ok(())
    }

    /// Consumes the iterable on top of the stack, opens its iterator, caches
    /// the `next` method, and enters a protected region that closes the
    /// iterator on abrupt completion.
    pub(super) fn begin_array_destructuring(&mut self) -> ArrayDestructuring {
        let iterator_slot = self.temp_local("array_pattern_iterator");
        let next_slot = self.temp_local("array_pattern_next");
        let done_slot = self.temp_local("array_pattern_done");
        self.emit(Op::GetIterator);
        self.emit(Op::StoreLocal(iterator_slot));
        self.emit(Op::LoadLocal(iterator_slot));
        let next_key = self.const_slot(Value::String("next".to_owned().into()));
        self.emit(Op::LoadConst(next_key));
        self.emit(Op::GetProp);
        self.emit(Op::StoreLocal(next_slot));
        let false_slot = self.const_slot(Value::Boolean(false));
        self.emit(Op::LoadConst(false_slot));
        self.emit(Op::StoreLocal(done_slot));
        let enter = self.emit(Op::EnterTry {
            catch: None,
            finally: None,
            catch_scope: None,
        });
        ArrayDestructuring {
            iterator_slot,
            next_slot,
            done_slot,
            enter,
        }
    }

    /// Pushes the next iterator value, or undefined once exhausted.
    pub(super) fn emit_iterator_step(&mut self, destructuring: &ArrayDestructuring) {
        self.emit(Op::LoadLocal(destructuring.iterator_slot));
        self.emit(Op::LoadLocal(destructuring.next_slot));
        self.emit(Op::IteratorStep {
            done_slot: destructuring.done_slot,
        });
    }

    /// Pushes an array holding the remaining iterator values.
    pub(super) fn emit_iterator_rest(&mut self, destructuring: &ArrayDestructuring) {
        self.emit(Op::LoadLocal(destructuring.iterator_slot));
        self.emit(Op::LoadLocal(destructuring.next_slot));
        self.emit(Op::IteratorRest {
            done_slot: destructuring.done_slot,
        });
    }

    /// Ends the protected region. A normal completion closes an unfinished
    /// iterator and lets close errors propagate; a thrown completion closes it
    /// while swallowing close errors, then rethrows the original error. A
    /// generator `return()` completion is routed through the try-finally stack,
    /// so it closes the iterator and lets close errors override the return.
    pub(super) fn end_array_destructuring(&mut self, destructuring: &ArrayDestructuring) {
        self.emit(Op::ExitTry);
        self.emit_close_unless_done(destructuring.iterator_slot, destructuring.done_slot, false);
        let over_catch = self.emit(Op::Jump(usize::MAX));

        let catch_target = self.code.len();
        self.emit(Op::ExitTry);
        self.emit_close_unless_done(destructuring.iterator_slot, destructuring.done_slot, true);
        self.emit(Op::Throw);

        let finally_target = self.code.len();
        self.emit_close_unless_done(destructuring.iterator_slot, destructuring.done_slot, false);
        self.emit(Op::EndFinally);

        let after = self.code.len();
        self.patch_jump(over_catch, after);
        if let Op::EnterTry { catch, finally, .. } = &mut self.code[destructuring.enter] {
            *catch = Some(catch_target);
            *finally = Some(finally_target);
        }
    }

    /// Emits a close of the iterator unless the done flag is set. Both paths
    /// converge after the close; the rest of the stack stays untouched.
    pub(super) fn emit_close_unless_done(
        &mut self,
        iterator_slot: usize,
        done_slot: usize,
        swallow: bool,
    ) {
        self.emit(Op::LoadLocal(done_slot));
        let skip_close = self.emit(Op::JumpIfTrue(usize::MAX));
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(iterator_slot));
        self.emit(Op::IteratorClose { swallow });
        let after = self.emit(Op::Jump(usize::MAX));
        let skip_target = self.code.len();
        self.patch_jump(skip_close, skip_target);
        self.emit(Op::Pop);
        let end = self.code.len();
        self.patch_jump(after, end);
    }
    pub(super) fn declare_var_kind_slot(&mut self, name: &str, kind: VarKind) -> usize {
        match kind {
            VarKind::Var => self.local_slot(name, true),
            VarKind::Let => self.declare_lexical_slot(name, true),
            // `using`/`await using` are immutable, block-scoped bindings. Their
            // disposal semantics are layered on separately; the binding itself
            // behaves like `const`.
            VarKind::Const | VarKind::Using | VarKind::AwaitUsing => {
                self.declare_lexical_slot(name, false)
            }
        }
    }

    fn var_initializer_slot(&self, name: &str, declared_slot: usize, kind: VarKind) -> usize {
        if kind != VarKind::Var {
            return declared_slot;
        }
        self.resolve_local_slot(name).unwrap_or(declared_slot)
    }

    pub(super) fn emit_store_var_initializer(&mut self, slot: usize, name: &str, kind: VarKind) {
        if kind == VarKind::Var && self.inside_current_with() {
            // Inside a `with`, a `var` initializer assignment is an ordinary
            // PutValue on a reference resolved through the current lexical
            // chain, which includes the with object's environment record. The
            // binding is already hoisted to `slot`, so the with-aware store
            // falls back to it when the with object lacks the property.
            let local = self.resolve_local_slot(name);
            self.emit_store_identifier(name, local, None);
            return;
        }
        let store_slot = self.var_initializer_slot(name, slot, kind);
        if store_slot != slot && kind == VarKind::Var {
            self.emit(Op::StoreLocal(store_slot));
        } else {
            self.emit_store_var_binding(store_slot, name, kind);
        }
    }

    pub(super) fn emit_store_var_binding(&mut self, slot: usize, name: &str, kind: VarKind) {
        if self.global_scope && kind == VarKind::Var {
            self.emit(Op::DefineGlobalVar(name.to_owned()));
        } else {
            self.emit(Op::StoreLocal(slot));
        }
    }
}

/// The NamedEvaluation name for a binding default, or `None` when the binding
/// is a nested pattern (which never names its default value).
pub(super) fn binding_inferred_name(binding: &BindingPattern) -> Option<&str> {
    match binding {
        BindingPattern::Identifier { name, .. } => Some(name),
        BindingPattern::Array { .. } | BindingPattern::Object { .. } => None,
    }
}
