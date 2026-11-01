//! Direct-call upvalue storage and lookup.
//!
//! Ordinary frames own a local-sized cell vector. Direct slot-seeded frames
//! can use a smaller representation, including a retained function for
//! statically read-only received cells.

use crate::function::BindingCell;

use super::{ir::Bytecode, vm::Vm};

impl Vm<'_, '_> {
    pub(super) fn initial_direct_local_upvalues(
        bytecode: &Bytecode,
        upvalues: &[BindingCell],
        received_realm_binding_slots: u128,
        env: &crate::context::ActivationContext,
    ) -> (Vec<Option<BindingCell>>, Option<u128>) {
        // Most direct leaf calls have no captured, module, or sloppy-global
        // cells. An empty vector represents the all-None state for those
        // frames and avoids allocating one pointer-sized entry per local on
        // every call. Direct-call eligibility excludes operations that can
        // create cells later (closures, eval, and with).
        if !bytecode.has_direct_local_upvalue_routes() && !env.has_module_imports() {
            return (Vec::new(), Some(0));
        }
        let _direct_eval_frame = bytecode.is_direct_eval_code();
        let mut next_received = 0;
        let mut realm_binding_slots = 0_u128;
        let mut local_upvalues = Vec::with_capacity(bytecode.locals.len());
        for (slot, local) in bytecode.locals.iter().enumerate() {
            if local.compiler_temporary {
                local_upvalues.push(None);
                continue;
            }
            if local.sloppy_global_fallback {
                let upvalue = env.realm_binding_cell(&local.name);
                if upvalue.is_some() && slot < u128::BITS as usize {
                    realm_binding_slots |= 1_u128 << slot;
                }
                local_upvalues.push(upvalue);
                continue;
            }
            if local.is_received_upvalue() {
                // Positional captures select a declaration, not a name. A
                // module import homonym cannot replace a nearer captured cell.
                let upvalue = upvalues
                    .get(next_received)
                    .cloned()
                    .or_else(|| env.module_import_cell(&local.name));
                next_received += 1;
                if slot < u128::BITS as usize
                    && received_realm_binding_slots & (1_u128 << slot) != 0
                {
                    realm_binding_slots |= 1_u128 << slot;
                }
                local_upvalues.push(upvalue);
                continue;
            }
            local_upvalues.push(None);
        }
        (local_upvalues, Some(realm_binding_slots))
    }

    /// Returns the live cell for a local slot. Ordinary frames own the
    /// local-sized option vector; a proven read-only direct frame instead
    /// resolves the same slot from the function it retains for its duration.
    pub(super) fn local_upvalue_cell(&self, slot: usize) -> Option<&BindingCell> {
        self.local_upvalues
            .get(slot)
            .and_then(Option::as_ref)
            .or_else(|| {
                let owner = self.direct_readonly_upvalue_owner.as_ref()?;
                let slot_bit = (slot < u128::BITS as usize).then(|| 1_u128 << slot)?;
                (self.direct_readonly_upvalue_slots & slot_bit != 0).then_some(())?;
                let index = self.bytecode.direct_readonly_received_upvalue_index(slot)?;
                owner.upvalues.get(index)
            })
    }
}
