//! Callback-free private-slot and numeric stack work. Debug drivers borrow this
//! work rather than reserving its operand temporaries across native re-entry.
use super::*;
impl<'a, F: crate::bytecode::vm::VmFrame<'a>> Vm<'a, '_, F> {
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline(always))]
    pub(super) fn try_private_load(&mut self, slot: usize) -> bool {
        if !self.current.direct_eval_with_stack
            && slot < 128
            && self.current.authoritative_slots & (1u128 << slot) != 0
            && let Some(Some(value)) = self.current.locals.get(slot)
        {
            let value = crate::bytecode::vm_bindings::clone_local_value(value);
            self.current.stack.push(value);
            crate::diagnostics::count!(authoritative_load_local_hits);
            return true;
        }
        false
    }
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline(always))]
    pub(super) fn try_private_store(&mut self, slot: usize, assign: bool) -> bool {
        if (!assign || !self.current.direct_eval_with_stack)
            && slot < 128
            && self.current.authoritative_slots & (1u128 << slot) != 0
            && self
                .current
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|l| l.mutable)
            && (!assign || self.current.locals.get(slot).is_some_and(Option::is_some))
            && let Some(value) = self.current.stack.pop()
        {
            self.current.locals[slot] = Some(value);
            if assign {
                crate::diagnostics::count!(authoritative_assign_local_hits);
            } else {
                crate::diagnostics::count!(authoritative_store_local_hits);
            }
            return true;
        }
        false
    }
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline(always))]
    pub(super) fn try_number_binary(&mut self, op: qjs_ast::BinaryOp) -> bool {
        let stack = &mut *self.current.stack;
        let len = stack.len();
        if len >= 2
            && let (Value::Number(left), Value::Number(right)) = (&stack[len - 2], &stack[len - 1])
            && let Some(value) =
                crate::bytecode::vm_props::fast_number_binary_numbers(*left, op, *right)
        {
            stack.truncate(len - 1);
            stack[len - 2] = value;
            return true;
        }
        false
    }
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline(always))]
    pub(super) fn try_constant(&mut self, constants: &[Value], index: usize) -> bool {
        if let Some(value) = constants.get(index) {
            self.current.stack.push(value.clone());
            return true;
        }
        false
    }
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline(always))]
    pub(super) fn try_duplicate(&mut self) -> bool {
        let stack = &mut *self.current.stack;
        if let Some(value) = stack.last() {
            let value = crate::bytecode::vm_bindings::clone_local_value(value);
            stack.push(value);
            return true;
        }
        false
    }
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline(always))]
    pub(super) fn try_pop(&mut self) -> bool {
        self.current.stack.pop().is_some()
    }
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline(always))]
    pub(super) fn inline_return(
        &mut self,
    ) -> Result<Option<crate::bytecode::frame_stack::FrameExit>, RuntimeError> {
        Ok(self.op_return()?.map(|value| {
            crate::bytecode::frame_stack::FrameExit::Completed(
                crate::bytecode::vm_result::Completion::Return(value),
            )
        }))
    }
    #[cfg(debug_assertions)]
    #[inline(never)]
    pub(super) fn verify_dispatch_interval<const SLOW: bool>(&self, authority_at_entry: u128) {
        debug_assert!(
            SLOW || (self.current.binding_scope.is_none()
                && !self.current.reference_hooks
                && self.current.authoritative_slots == authority_at_entry),
            "the fast dispatch loop outlived a slow-path change"
        );
    }
}
