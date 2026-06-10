//! Binding-pattern destructuring compilation shared by declarations,
//! function parameters, loop heads, and catch parameters.

use qjs_ast::{BinaryOp, BindingPattern, VarKind};

use crate::{RuntimeError, Value};

use super::compiler::Compiler;
use super::ir::Op;

/// Compiler state for one protected array-destructuring region driven by a
/// lazily stepped iterator.
pub(super) struct ArrayDestructuring {
    iterator_slot: usize,
    next_slot: usize,
    done_slot: usize,
    enter: usize,
}

impl Compiler {
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
                    self.compile_binding_default(element.default.as_ref())?;
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
                for property in properties {
                    self.emit(Op::LoadLocal(source_slot));
                    let key = self.const_slot(Value::String(property.key.clone()));
                    self.emit(Op::LoadConst(key));
                    self.emit(Op::GetProp);
                    self.compile_binding_default(property.default.as_ref())?;
                    self.compile_binding_initializer(&property.binding, kind)?;
                }
                if let Some(rest) = rest {
                    self.emit(Op::LoadLocal(source_slot));
                    self.emit(Op::ObjectRestExcluding {
                        excluded: properties
                            .iter()
                            .map(|property| property.key.clone())
                            .collect(),
                    });
                    self.compile_binding_initializer(rest, kind)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn compile_binding_uninitialized(
        &mut self,
        pattern: &BindingPattern,
        kind: VarKind,
    ) -> Result<(), RuntimeError> {
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
        self.compile_expr(default)?;
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
        let next_key = self.const_slot(Value::String("next".to_owned()));
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
    /// iterator and lets close errors propagate; an abrupt completion closes
    /// it while swallowing close errors, then rethrows the original error.
    pub(super) fn end_array_destructuring(&mut self, destructuring: &ArrayDestructuring) {
        self.emit(Op::ExitTry);
        let normal_done = self.emit_close_if_not_done(destructuring, false);
        let over_catch = self.emit(Op::Jump(usize::MAX));
        self.patch_jump(normal_done, over_catch);

        let catch_target = self.code.len();
        let abrupt_done = self.emit_close_if_not_done(destructuring, true);
        let rethrow = self.emit(Op::Throw);
        self.patch_jump(abrupt_done, rethrow);

        let after = self.code.len();
        self.patch_jump(over_catch, after);
        if let Op::EnterTry { catch, .. } = &mut self.code[destructuring.enter] {
            *catch = Some(catch_target);
        }
    }

    /// Emits a close of the iterator unless the done flag is set, returning
    /// the jump emitted on the skip path so the caller can patch it past the
    /// close. Leaves the rest of the stack untouched.
    fn emit_close_if_not_done(
        &mut self,
        destructuring: &ArrayDestructuring,
        swallow: bool,
    ) -> usize {
        self.emit(Op::LoadLocal(destructuring.done_slot));
        let skip_close = self.emit(Op::JumpIfTrue(usize::MAX));
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(destructuring.iterator_slot));
        self.emit(Op::IteratorClose { swallow });
        let done = self.emit(Op::Jump(usize::MAX));
        let skip_target = self.code.len();
        self.patch_jump(skip_close, skip_target);
        self.emit(Op::Pop);
        done
    }
}
