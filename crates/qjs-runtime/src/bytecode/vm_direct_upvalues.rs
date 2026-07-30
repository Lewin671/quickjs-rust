//! Direct-call upvalue storage and lookup.
//!
//! Ordinary frames own a local-sized cell vector. Direct slot-seeded frames
//! can use a smaller representation, including a retained function for
//! statically read-only received cells.

use crate::function::{CallEnv, Upvalue};

use super::{ir::Bytecode, vm::Vm};

impl Vm<'_> {
    pub(super) fn initial_direct_local_upvalues(
        bytecode: &Bytecode,
        upvalues: &[Upvalue],
        received_realm_binding_slots: u128,
        env: &CallEnv,
    ) -> (Vec<Option<Upvalue>>, Option<u128>) {
        // Most direct leaf calls have no captured, module, or sloppy-global
        // cells. An empty vector represents the all-None state for those
        // frames and avoids allocating one pointer-sized entry per local on
        // every call. Direct-call eligibility excludes operations that can
        // create cells later (closures, eval, and with).
        if !bytecode.has_direct_local_upvalue_routes() && !env.has_module_imports() {
            return (Vec::new(), Some(0));
        }
        let direct_eval_frame = matches!(
            env.get_local(crate::DIRECT_EVAL_BINDING),
            Some(crate::Value::Boolean(true))
        );
        let mut next_received = 0;
        let has_module_imports = env.has_module_imports();
        let mut realm_binding_slots = 0_u128;
        let mut local_upvalues = Vec::with_capacity(bytecode.locals.len());
        for (slot, local) in bytecode.locals.iter().enumerate() {
            if local.compiler_temporary {
                local_upvalues.push(None);
                continue;
            }
            if has_module_imports && let Some(upvalue) = env.module_import_cell(&local.name) {
                if local.is_received_upvalue() {
                    next_received += 1;
                }
                local_upvalues.push(Some(upvalue));
                continue;
            }
            if local.sloppy_global_fallback {
                if direct_eval_frame && let Some(upvalue) = env.local_binding_cell(&local.name) {
                    local_upvalues.push(Some(upvalue));
                    continue;
                }
                let upvalue = env.realm_binding_cell(&local.name);
                if upvalue.is_some() && slot < u128::BITS as usize {
                    realm_binding_slots |= 1_u128 << slot;
                }
                local_upvalues.push(upvalue);
                continue;
            }
            if local.is_received_upvalue() {
                let upvalue = upvalues.get(next_received).cloned();
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
    pub(super) fn local_upvalue_cell(&self, slot: usize) -> Option<&Upvalue> {
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

    /// Whether `cell` aliases any live upvalue in this frame, including the
    /// direct read-only source used by slot-seeded frames. Numeric loop guards
    /// use this to avoid scalarizing a callee update that can reach back into
    /// the caller through a shared cell.
    pub(super) fn has_local_upvalue_cell(&self, cell: &Upvalue) -> bool {
        self.local_upvalues
            .iter()
            .flatten()
            .any(|candidate| candidate.ptr_eq(cell))
            || self
                .direct_readonly_upvalue_owner
                .as_ref()
                .is_some_and(|owner| {
                    owner
                        .upvalues
                        .iter()
                        .any(|candidate| candidate.ptr_eq(cell))
                })
    }
}
