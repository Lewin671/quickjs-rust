//! The dispatch loop's out-of-line opcode bodies.
//!
//! `Vm::run_current_activation` keeps only the opcodes that touch nothing but
//! the operand stack, an authoritative local slot, or the program counter. Each
//! of those is a handful of instructions and cannot observe `self.ip`, so the
//! loop can hold the counter, the code pointer and the code length in machine
//! registers across them.
//!
//! Everything else lands here. The split is not semantic: property access,
//! calls and the slow halves of the arithmetic opcodes are hot, and they are
//! out of line precisely *because* they are large. Disassembled, the single
//! 25,020-instruction dispatch function spilled `self`, the code pointer and
//! the length across every dispatch, so all ~90 opcodes paid an eight-memory-op
//! preamble before doing any work. An opcode that already costs a property
//! lookup or a call absorbs one extra call; an opcode that costs four
//! instructions cannot.
//!
//! `self.ip` is authoritative on entry and on exit, so an opcode that jumps,
//! calls, throws or suspends behaves exactly as it did inline.

use super::{FrameExit, FrameProgramView, Vm};
use crate::bytecode::ir::NamedPropertyCache;
use crate::bytecode::ir::Op;
use crate::bytecode::util::{stack_underflow, typeof_value};
use crate::{RuntimeError, Value, is_truthy};
use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};
use std::rc::Rc;

impl Vm<'_> {
    /// Runs one opcode the dispatch loop did not answer inline.
    ///
    /// `Ok(None)` continues the activation; `Ok(Some(exit))` is the exit the
    /// dispatch loop returns.
    #[inline(never)]
    pub(super) fn run_general_op(
        &mut self,
        op: &Op,
        program: &FrameProgramView<'_>,
    ) -> Result<Option<FrameExit>, RuntimeError> {
        let bytecode = program.bytecode;
        match op {
            Op::LoadConst(index) => {
                let Some(value) = bytecode.constants.get(*index) else {
                    return Err(RuntimeError {
                        thrown: None,
                        message: "bytecode constant index out of bounds".to_owned(),
                    });
                };
                let value = value.clone();
                self.stack.push(value);
            }
            Op::LoadLocalOrUndefined(slot) => {
                let value = self.load_local_or_undefined(*slot)?;
                self.stack.push(value);
            }
            op @ (Op::AppendStringLiteralLocal { .. } | Op::AppendStringLiteralGlobal { .. }) => {
                self.run_string_append_op(op.clone())?
            }
            Op::Pop => {
                if self.current.stack.pop().is_none() {
                    return Err(stack_underflow());
                }
            }
            Op::Dup => {
                let stack = &mut *self.current.stack;
                let Some(value) = stack.last() else {
                    return Err(stack_underflow());
                };
                let value = crate::bytecode::vm_bindings::clone_local_value(value);
                stack.push(value);
            }
            Op::NewArray { elements } => self.new_array(elements)?,
            Op::NewObjectLiteral => self.new_object_literal(),
            Op::NewObjectDataLiteral { shape } => self.new_object_data_literal(shape.clone())?,
            Op::LoadVirtualNumber { value, skip } => {
                self.stack.push(Value::Number(*value));
                self.ip += *skip;
            }
            op @ (Op::InitVirtualObject { .. }
            | Op::InitVirtualConstants { .. }
            | Op::LoadVirtualValue { .. }
            | Op::StoreVirtualValue { .. }
            | Op::LoadVirtualLength { .. }
            | Op::GuardVirtualObject
            | Op::LoadVirtualBinary { .. }
            | Op::BinaryAssignLocals { .. }
            | Op::IncrementLocal { .. }
            | Op::CopyLocal { .. }
            | Op::CompareLocalsJumpFalse { .. }
            | Op::InitVirtualFunction { .. }
            | Op::CallVirtualFunction { .. }) => self.run_virtual_object_op(program, op)?,
            Op::DefineObjectProperty(meta) => self.define_object_property(*meta)?,
            Op::EnumerateKeys { cache } => self.enumerate_keys(cache)?,
            Op::ForInKeyIsEnumerable => self.for_in_key_is_enumerable()?,
            Op::Typeof => {
                let value = self.pop()?;
                self.stack.push(Value::String(typeof_value(value).into()));
            }
            Op::FreshIterationScope(slots) => self.fresh_iteration_scope(slots),
            Op::JumpIfFalse(target) => {
                if !is_truthy(self.stack.last().ok_or_else(stack_underflow)?) {
                    self.ip = *target;
                }
            }
            Op::JumpIfTrue(target) => {
                if is_truthy(self.stack.last().ok_or_else(stack_underflow)?) {
                    self.ip = *target;
                }
            }
            Op::JumpIfNotNullish(target) => {
                if !matches!(self.stack.last(), Some(Value::Null | Value::Undefined)) {
                    self.ip = *target;
                }
            }
            // Everything a benchmark reaches at most once per closure,
            // class, `with` block, iterator step or suspension. Listing the
            // variants keeps this match exhaustive, so a new opcode is a
            // compile error here rather than a silent arrival on the cold
            // path; see `rare_ops` for why they are not inline.
            // Answered by the dispatch loop itself, which calls the matching
            // `op_*` body directly rather than routing through this match.
            Op::LoadLocal(_)
            | Op::StoreLocal(_)
            | Op::AssignLocal(_)
            | Op::LoadGlobal(_)
            | Op::StoreGlobalStrict(_)
            | Op::StoreGlobalSloppy { .. }
            | Op::GetPropNamed { .. }
            | Op::GetPropIndex(_)
            | Op::GetProp
            | Op::SetProp { .. }
            | Op::SetPropIndex { .. }
            | Op::SetPropNamed { .. }
            | Op::Call(_)
            | Op::CallResolved(_)
            | Op::CallResolvedGuardedMathUnary
            | Op::New(_)
            | Op::ToNumeric
            | Op::Unary(_)
            | Op::Update(_)
            | Op::Binary(_)
            | Op::Jump(_)
            | Op::Return => {
                return Err(RuntimeError {
                    thrown: None,
                    message: "bytecode opcode is dispatched inline, not generally".to_owned(),
                });
            }
            op @ (Op::LoadNewTarget
            | Op::ClearLocal(_)
            | Op::DefineGlobalVar(_)
            | Op::StoreLocalOrGlobalSloppy { .. }
            | Op::TypeofGlobal(_)
            | Op::EnterWith
            | Op::ExitWith
            | Op::LoadIdentWith { .. }
            | Op::ResolveIdentWith { .. }
            | Op::LoadResolvedIdentWith { .. }
            | Op::StoreIdentWith { .. }
            | Op::StoreResolvedIdentWith { .. }
            | Op::TypeofIdentWith { .. }
            | Op::DeleteIdentWith { .. }
            | Op::NewTemplateObject { .. }
            | Op::EnterDisposableScope
            | Op::RegisterDisposable
            | Op::RegisterAsyncDisposable
            | Op::DisposeScope { .. }
            | Op::SetComputedFunctionName(_)
            | Op::CopyObjectSpread
            | Op::GetIterator
            | Op::GetAsyncIterator
            | Op::AsyncIteratorComplete { .. }
            | Op::IteratorStep { .. }
            | Op::IteratorRest { .. }
            | Op::ObjectRestExcluding { .. }
            | Op::RequireObjectCoercible
            | Op::GetPrivate(_)
            | Op::SetPrivate(_)
            | Op::PrivateIn(_)
            | Op::DeleteProp { .. }
            | Op::DeleteIdent(_)
            | Op::RequireCallable
            | Op::CallDirectEval { .. }
            | Op::CallSpread
            | Op::CallDirectEvalSpread { .. }
            | Op::IteratorClose { .. }
            | Op::NewSpread
            | Op::NewFunction { .. }
            | Op::NewClass { .. }
            | Op::SuperGet { .. }
            | Op::SuperReference
            | Op::SuperGetComputed
            | Op::SuperSet { .. }
            | Op::SuperSetComputed { .. }
            | Op::SuperMethod { .. }
            | Op::SuperMethodComputed
            | Op::CallResolvedSpread
            | Op::SuperCall(_)
            | Op::SuperCallSpread
            | Op::ToString
            | Op::ToPropertyKey
            | Op::ToPropertyKeyForAccess
            | Op::AbruptJump(_)
            | Op::EnterTry { .. }
            | Op::ExitTry
            | Op::EndFinally
            | Op::DiscardPendingAbrupt
            | Op::Throw
            | Op::ThrowReferenceError(_)
            | Op::FunctionPrologueEnd
            | Op::Yield
            | Op::Await
            | Op::YieldDelegate { .. }
            | Op::ImportCall { .. }
            | Op::ImportMeta) => {
                return self.run_rare_op(op);
            }
        }
        Ok(None)
    }

    // One out-of-line body per opcode the dispatch loop dispatches directly.
    // Inlining these is what made the dispatch function 25,020 instructions
    // long and cost every opcode a spilled program counter; calling them costs
    // one branch on top of work that already runs into the tens of
    // nanoseconds. `self.ip` is authoritative across each call.

    /// A local the dispatch loop could not read from an authoritative slot.
    #[inline(never)]
    pub(super) fn op_load_local(&mut self, slot: usize) -> Result<(), RuntimeError> {
        let result = if self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot) {
            let name = self.bytecode.locals[slot].name.clone();
            self.load_ident_with(&name, Some(slot), self.bytecode.is_strict())
        } else {
            self.load_local(slot)
        };
        if let Some(value) = self.handle_runtime_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    /// A local the dispatch loop could not write to an authoritative slot.
    #[inline(never)]
    pub(super) fn op_store_local(&mut self, slot: usize) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let result = self.store_local(slot, value);
        self.handle_runtime_result(result)?;
        Ok(())
    }

    /// An assignment the dispatch loop could not make to an initialized slot.
    #[inline(never)]
    pub(super) fn op_assign_local(&mut self, slot: usize) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let result = if self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot) {
            let name = self.bytecode.locals[slot].name.clone();
            self.store_ident_with(&name, Some(slot), self.bytecode.is_strict(), value)
        } else {
            self.assign_local(slot, value)
        };
        self.handle_runtime_result(result)?;
        Ok(())
    }

    /// Reads a free name that resolved to the global environment.
    #[inline(never)]
    pub(super) fn op_load_global(&mut self, name: &str) -> Result<(), RuntimeError> {
        let result = if self.direct_eval_with_stack {
            self.load_ident_with(name, None, self.bytecode.is_strict())
        } else {
            self.load_global(name)
        };
        if let Some(value) = self.handle_runtime_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    /// Writes a free name in strict code, where an unresolvable name throws.
    #[inline(never)]
    pub(super) fn op_store_global_strict(&mut self, name: &str) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let result = if self.direct_eval_with_stack {
            self.store_ident_with(name, None, true, value)
        } else {
            self.store_global_strict(name, value)
        };
        self.handle_runtime_result(result)?;
        Ok(())
    }

    /// Writes a free name in sloppy code, creating the global when absent.
    #[inline(never)]
    pub(super) fn op_store_global_sloppy(
        &mut self,
        slot: usize,
        name: &str,
    ) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let result = if self.direct_eval_with_stack {
            self.store_ident_with(name, None, false, value)
        } else {
            self.store_global_sloppy_at_slot(slot, name, value)
        };
        self.handle_runtime_result(result)?;
        Ok(())
    }

    /// `obj.key`, through the site's inline cache.
    #[inline(never)]
    pub(super) fn op_get_prop_named(
        &mut self,
        key: &Rc<str>,
        cache: &NamedPropertyCache,
    ) -> Result<(), RuntimeError> {
        crate::diagnostics::count!(named_property_reads);
        let result = self.get_named_prop(key, cache);
        self.handle_runtime_result(result)?;
        Ok(())
    }

    /// `obj[0]`, with the constant index fused into the opcode.
    #[inline(never)]
    pub(super) fn op_get_prop_index(&mut self, index: usize) -> Result<(), RuntimeError> {
        crate::diagnostics::count!(computed_property_reads);
        let result = self.get_index_prop(index);
        self.handle_runtime_result(result)?;
        Ok(())
    }

    /// `obj[key]` with a computed key.
    #[inline(never)]
    pub(super) fn op_get_prop(&mut self) -> Result<(), RuntimeError> {
        crate::diagnostics::count!(computed_property_reads);
        let result = self.get_prop();
        self.handle_runtime_result(result)?;
        Ok(())
    }

    /// `obj[key] = value` with a computed key.
    #[inline(never)]
    pub(super) fn op_set_prop(&mut self, is_strict: bool) -> Result<(), RuntimeError> {
        crate::diagnostics::count!(computed_property_writes);
        let result = self.set_prop(is_strict);
        self.handle_runtime_result(result)?;
        Ok(())
    }

    /// `obj[0] = value`, with the constant index fused into the opcode.
    #[inline(never)]
    pub(super) fn op_set_prop_index(
        &mut self,
        index: usize,
        is_strict: bool,
    ) -> Result<(), RuntimeError> {
        crate::diagnostics::count!(computed_property_writes);
        let result = self.set_index_prop(index, is_strict);
        self.handle_runtime_result(result)?;
        Ok(())
    }

    /// `obj.key = value`, through the site's inline cache.
    #[inline(never)]
    pub(super) fn op_set_prop_named(
        &mut self,
        key: &Rc<str>,
        cache: Option<&NamedPropertyCache>,
        is_strict: bool,
    ) -> Result<(), RuntimeError> {
        crate::diagnostics::count!(named_property_writes);
        let result = self.set_named_prop(key, cache, is_strict);
        self.handle_runtime_result(result)?;
        Ok(())
    }

    /// Coerces a non-number operand; the dispatch loop answers numbers.
    #[inline(never)]
    pub(super) fn op_to_numeric(&mut self) -> Result<(), RuntimeError> {
        let result = self.eval_to_numeric();
        if let Some(value) = self.handle_runtime_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    /// A unary operator on any operand.
    #[inline(never)]
    pub(super) fn op_unary(&mut self, op: UnaryOp) -> Result<(), RuntimeError> {
        let result = self.eval_unary(op);
        if let Some(value) = self.handle_runtime_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    /// `++`/`--` on a non-number operand.
    #[inline(never)]
    pub(super) fn op_update(&mut self, op: UpdateOp) -> Result<(), RuntimeError> {
        let result = self.eval_update(op);
        if let Some(value) = self.handle_runtime_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    /// A binary operator the number-number form in the dispatch loop declined.
    #[inline(never)]
    pub(super) fn op_binary(&mut self, op: BinaryOp) -> Result<(), RuntimeError> {
        let result = self.eval_binary(op);
        if let Some(value) = self.handle_runtime_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    /// A backward edge, which is where the loop accelerators are consulted.
    #[inline(never)]
    pub(super) fn op_jump(&mut self, program: &FrameProgramView<'_>, target: usize) {
        let backedge = self.ip - 1;
        self.jump_with_loop_plans(program.loop_plans(), target, backedge);
    }

    /// Pops the return value and lets any `finally` claim it first.
    #[inline(never)]
    pub(super) fn op_return(&mut self) -> Result<Option<Value>, RuntimeError> {
        let value = self.stack.pop().unwrap_or(Value::Undefined);
        self.return_value(value)
    }
}
