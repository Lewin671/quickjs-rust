//! Frame-local slot and shared-cell initialization.
//!
//! Immutable routes come from `FrameInitPlan`; only runtime context selects
//! realm/module/dynamic identities. Positional inputs are seeded before cells
//! are created, so declaring frames and every escaping closure share one cell.

use super::{
    frame_init_plan::InitialSlot,
    ir::Bytecode,
    vm::{Slot, Vm},
};
use crate::{
    Value,
    function::{CallEnv, Upvalue},
};

impl Vm<'_> {
    pub(super) fn initial_slots(bytecode: &Bytecode, env: &CallEnv) -> Vec<Slot> {
        let mut locals = bytecode.take_local_slots(bytecode.locals.len());
        for (slot, route) in bytecode.frame_init_plan.initial_slots.iter().enumerate() {
            let name = &bytecode.locals[slot].name;
            let incoming = || {
                env.get_local(name).filter(|value| {
                    !matches!(value,
                Value::Function(function) if function.is_uninitialized_lexical_marker())
                })
            };
            locals[slot] = match route {
                InitialSlot::Empty => None,
                InitialSlot::Undefined => Some(Value::Undefined),
                InitialSlot::Frame => env.get_local(name),
                InitialSlot::FrameOrUndefined => {
                    Some(env.get_local(name).unwrap_or(Value::Undefined))
                }
                InitialSlot::Incoming => incoming(),
                InitialSlot::IncomingOrUndefined => Some(incoming().unwrap_or(Value::Undefined)),
                InitialSlot::IncomingOrRealm => incoming().or_else(|| env.get_realm(name)),
            };
        }
        locals
    }

    pub(super) fn initial_local_upvalues(
        bytecode: &Bytecode,
        _locals: &[Slot],
        upvalues: &[Upvalue],
        env: &CallEnv,
    ) -> Vec<Option<Upvalue>> {
        let mut next_received = 0;
        let mut local_upvalues = vec![None; bytecode.locals.len()];
        let direct_eval_frame = matches!(
            env.get_local(crate::DIRECT_EVAL_BINDING),
            Some(Value::Boolean(true))
        );
        if !bytecode.global_scope
            && !direct_eval_frame
            && !env.has_module_imports()
            && env.derived_this_cell().is_none()
            && env.deopt_bindings().is_none()
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
            for &slot in &bytecode.frame_init_plan.sloppy_global_slots {
                local_upvalues[slot] = env.realm_binding_cell(&bytecode.locals[slot].name);
            }
        } else {
            for (slot, local) in bytecode.locals.iter().enumerate() {
                if local.compiler_temporary {
                    continue;
                }
                if local.name == "this"
                    && let Some(cell) = env.derived_this_cell()
                {
                    local_upvalues[slot] = Some(cell);
                    if local.is_received_upvalue() {
                        next_received += 1;
                    }
                    continue;
                }
                if let Some(upvalue) = env.module_import_cell(&local.name) {
                    local_upvalues[slot] = Some(upvalue);
                    if local.is_received_upvalue() {
                        next_received += 1;
                    }
                    continue;
                }
                // A direct-eval program is compiled as a global-scope bytecode,
                // but its imported `var` slots can still name the caller's local
                // binding. Preserve that binding identity before considering the
                // same-named realm cell.
                if direct_eval_frame
                    && local.hoisted
                    && let Some(upvalue) = env.local_binding_cell(&local.name)
                {
                    local_upvalues[slot] = Some(upvalue);
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
                    local_upvalues[slot] = if direct_eval_frame {
                        env.local_binding_cell(&local.name)
                            .or_else(|| env.realm_binding_cell(&local.name))
                    } else {
                        env.realm_binding_cell(&local.name)
                    };
                    continue;
                }
                if local.is_received_upvalue() {
                    if let Some(upvalue) = upvalues.get(next_received) {
                        local_upvalues[slot] = Some(upvalue.clone());
                    } else if env.deopt_bindings().is_some() {
                        local_upvalues[slot] = env
                            .deopt_bindings()
                            .and_then(|bindings| bindings.cell(&local.name))
                            .or_else(|| env.frame_binding_cell(&local.name));
                    }
                    next_received += 1;
                }
            }
        }
        let dynamic = bytecode.contains_direct_eval()
            || bytecode.contains_with()
            || env.deopt_bindings().is_some();
        let static_slots = if dynamic {
            &bytecode.frame_init_plan.dynamic_cell_slots
        } else {
            &bytecode.frame_init_plan.cell_slots
        };
        let mut module_slots = Vec::new();
        let cell_slots = if env.has_module_live_bindings() {
            module_slots.extend_from_slice(static_slots);
            module_slots.extend(
                bytecode
                    .frame_init_plan
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
            if local.name == "this" && local_upvalues[slot].is_some() {
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
            if local.parameter
                && bytecode.needs_arguments_object()
                && let Some(upvalue) = env.frame_binding_cell(&local.name)
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
            if direct_eval_frame
                && local.hoisted
                && !bytecode
                    .global_lexical_names()
                    .iter()
                    .any(|name| name == &local.name)
                && !env.has_frame_binding(&local.name)
                && let Some(upvalue) = env
                    .deopt_bindings()
                    .and_then(|bindings| bindings.cell(&local.name))
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
            local_upvalues[slot] = Some(Upvalue::new(value));
        }
        local_upvalues
    }

    pub(super) fn initial_authoritative_slots(
        bytecode: &Bytecode,
        local_upvalues: &[Option<Upvalue>],
        env: &CallEnv,
    ) -> u128 {
        // A frame with no cells and an environment that supplies no name answers
        // the whole mask from the bytecode. Asking the environment about every
        // local's name, as the general path must, is what made building a frame
        // scale with its local count.
        if env.supplies_no_named_binding() {
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
                        || !bytecode.global_scope && env.slot_is_authoritative(&local.name)))
                .then_some(1_u128 << slot)
            })
            .fold(0, |slots, slot| slots | slot)
    }

    pub(super) fn initial_realm_binding_slots(
        bytecode: &Bytecode,
        local_upvalues: &[Option<Upvalue>],
        env: &CallEnv,
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
