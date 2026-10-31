//! Frame-local slot and shared-cell initialization.
//!
//! Immutable routes come from `BindingPlan`; only runtime context selects
//! realm/module/dynamic identities. Positional inputs are seeded before cells
//! are created, so declaring frames and every escaping closure share one cell.

use super::{
    binding_plan::InitialSlot,
    ir::Bytecode,
    vm::{Slot, Vm},
};
use crate::{Value, function::BindingCell};

impl Vm<'_, '_> {
    pub(super) fn initial_slots(
        bytecode: &Bytecode,
        env: &crate::context::ActivationContext,
    ) -> Vec<Slot> {
        let mut locals = bytecode.take_local_slots(bytecode.locals.len());
        for (slot, route) in bytecode.binding_plan.initial_slots.iter().enumerate() {
            let name = &bytecode.locals[slot].name;
            locals[slot] = match route {
                InitialSlot::Empty => None,
                InitialSlot::Undefined => Some(Value::Undefined),
                InitialSlot::Receiver => env.receiver_seed(),
                InitialSlot::Arguments => None,
                InitialSlot::ReceiverOrUndefined => {
                    Some(env.receiver_seed().unwrap_or(Value::Undefined))
                }
                InitialSlot::IncomingOrUndefined => Some(Value::Undefined),
                InitialSlot::IncomingOrRealm => env.get_realm(name),
            };
        }
        locals
    }

    pub(super) fn initial_local_upvalues(
        bytecode: &Bytecode,
        _locals: &[Slot],
        upvalues: &[BindingCell],
        env: &crate::context::ActivationContext,
    ) -> Vec<Option<BindingCell>> {
        let mut next_received = 0;
        let mut local_upvalues = vec![None; bytecode.locals.len()];
        let direct_eval_frame = bytecode.is_direct_eval_code();
        if !bytecode.global_scope
            && !env.has_global_binding_root()
            && !bytecode.binding_plan.has_realm_lexicals
            && !direct_eval_frame
            && !env.has_module_imports()
            && env.entry_scope().is_none()
            && env.derived_this_cell().is_none()
            && !env.has_module_live_bindings()
        {
            for (index, &slot) in bytecode
                .received_upvalue_slots()
                .iter()
                .filter(|&&slot| {
                    !bytecode.locals[slot].compiler_temporary
                        && !bytecode.locals[slot].sloppy_global_fallback
                })
                .enumerate()
            {
                local_upvalues[slot] = upvalues.get(index).cloned();
            }
            for &slot in &bytecode.binding_plan.sloppy_global_slots {
                local_upvalues[slot] = env.realm_binding_cell(&bytecode.locals[slot].name);
            }
        } else {
            for (slot, local) in bytecode.locals.iter().enumerate() {
                if local.compiler_temporary {
                    continue;
                }
                // Positional captures name their source cell. Import tables
                // cannot replace a nearer parameter/lexical homonym.
                if local.is_received_upvalue()
                    && let Some(cell) = upvalues.get(next_received)
                {
                    next_received += 1;
                    local_upvalues[slot] = Some(cell.clone());
                    continue;
                }
                if env.has_global_binding_root()
                    && (bytecode.global_scope || bytecode.is_direct_eval_code())
                    && local.hoisted
                    && !local.parameter
                {
                    if env.can_cache_realm_globals() {
                        local_upvalues[slot] = env.realm_binding_cell(&local.name);
                    }
                    continue;
                }
                if bytecode.is_direct_eval_code()
                    && let Some(root) = env.entry_scope()
                    && let Some(ordinal) = bytecode.binding_plan.root_ordinals[slot]
                {
                    local_upvalues[slot] = Some(root.cells[ordinal].cell.clone());
                    continue;
                }
                if bytecode.binding_plan.realm_lexical_slots[slot] {
                    local_upvalues[slot] = env.realm_binding_cell(&local.name);
                    continue;
                }
                let module_root_binding = env.has_module_live_bindings()
                    && bytecode.binding_plan.scopes[0]
                        .lookup
                        .get(&local.name)
                        .is_some_and(|id| bytecode.binding_plan.bindings[id.0].slot.0 == slot);
                if (local.is_received_upvalue() || module_root_binding)
                    && let Some(upvalue) = env.module_import_cell(&local.name)
                {
                    local_upvalues[slot] = Some(upvalue);
                    if local.is_received_upvalue() {
                        next_received += 1;
                    }
                    continue;
                }
                if bytecode.global_scope
                    && local.hoisted
                    && let Some(upvalue) = env.realm_binding_cell(&local.name)
                {
                    local_upvalues[slot] = Some(upvalue);
                    continue;
                }
                if local.sloppy_global_fallback {
                    local_upvalues[slot] = env.realm_binding_cell(&local.name);
                    continue;
                }
                if local.is_received_upvalue() {
                    if let Some(upvalue) = upvalues.get(next_received) {
                        local_upvalues[slot] = Some(upvalue.clone());
                    }
                    next_received += 1;
                }
            }
        }
        let dynamic = bytecode.has_dynamic_scope_tree()
            || env.scope_cursor().is_some()
            || bytecode.contains_with();
        let static_slots = if dynamic {
            &bytecode.binding_plan.dynamic_cell_slots
        } else {
            &bytecode.binding_plan.cell_slots
        };
        let mut module_slots = Vec::new();
        let cell_slots = if env.has_module_live_bindings() {
            module_slots.extend_from_slice(static_slots);
            module_slots.extend(
                bytecode
                    .binding_plan
                    .module_live_candidates
                    .iter()
                    .copied()
                    .filter(|&slot| {
                        env.module_live_binding_cell(&bytecode.locals[slot].name)
                            .is_some()
                    }),
            );
            module_slots.sort_unstable();
            module_slots.dedup();
            &module_slots
        } else {
            static_slots
        };
        for &slot in cell_slots {
            let Some(local) = bytecode.locals.get(slot) else {
                continue;
            };
            if local.compiler_temporary {
                continue;
            }
            if local_upvalues[slot].is_some() {
                continue;
            }
            if local.is_received_upvalue() {
                continue;
            }
            if bytecode.global_scope
                && local.hoisted
                && let Some(upvalue) = env.realm_binding_cell(&local.name)
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            if bytecode.local_slot(&local.name) == Some(slot)
                && let Some(upvalue) = env.module_live_binding_cell(&local.name)
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            let value = _locals
                .get(slot)
                .and_then(Option::as_ref)
                .cloned()
                .unwrap_or_else(
                    || Value::Function(crate::Function::uninitialized_lexical_marker()),
                );
            local_upvalues[slot] = Some(bytecode.binding_meta(slot).map_or_else(
                || BindingCell::new(value.clone()),
                |meta| BindingCell::with_metadata(meta, Some(value.clone())),
            ));
        }
        local_upvalues
    }

    pub(super) fn initial_authoritative_slots(
        bytecode: &Bytecode,
        local_upvalues: &[Option<BindingCell>],
        env: &crate::context::ActivationContext,
    ) -> u128 {
        if !(bytecode.global_scope
            || bytecode.is_direct_eval_code() && env.scope_cursor().is_none())
        {
            return bytecode
                .locals
                .iter()
                .enumerate()
                .take(u128::BITS as usize)
                .filter_map(|(slot, local)| {
                    if local.sloppy_global_fallback
                        || local.is_received_upvalue()
                        || local_upvalues.get(slot).is_some_and(Option::is_some)
                    {
                        return None;
                    }
                    if env.has_global_binding_root()
                        && bytecode.is_direct_eval_code()
                        && local.hoisted
                        && !local.parameter
                    {
                        return None;
                    }
                    Some(1_u128 << slot)
                })
                .fold(0, |mask, slot| mask | slot);
        }
        // A frame with no cells and an environment that supplies no name answers
        // the whole mask from the bytecode. Asking the environment about every
        // local's name, as the general path must, is what made building a frame
        // scale with its local count.
        if env.indexed_bindings_only() {
            let occupied = local_upvalues
                .iter()
                .take(u128::BITS as usize)
                .enumerate()
                .fold(0, |mask, (slot, cell)| {
                    mask | if cell.is_some() { 1_u128 << slot } else { 0 }
                });
            return bytecode.authoritative_mask_clean() & !occupied;
        }
        bytecode
            .locals
            .iter()
            .enumerate()
            .take(u128::BITS as usize)
            .filter_map(|(slot, local)| {
                (!local.sloppy_global_fallback
                    && local_upvalues.get(slot).is_none_or(Option::is_none)
                    && (local.compiler_temporary
                        || !bytecode.global_scope && env.permits_owned_slots()))
                .then_some(1_u128 << slot)
            })
            .fold(0, |slots, slot| slots | slot)
    }

    pub(super) fn initial_realm_binding_slots(
        bytecode: &Bytecode,
        local_upvalues: &[Option<BindingCell>],
        env: &crate::context::ActivationContext,
    ) -> u128 {
        bytecode
            .locals
            .iter()
            .enumerate()
            .take(u128::BITS as usize)
            .filter_map(|(slot, local)| {
                local_upvalues
                    .get(slot)
                    .and_then(Option::as_ref)
                    .is_some_and(|cell| env.is_realm_binding_cell(&local.name, cell))
                    .then_some(1_u128 << slot)
            })
            .fold(0, |slots, slot| slots | slot)
    }
}
