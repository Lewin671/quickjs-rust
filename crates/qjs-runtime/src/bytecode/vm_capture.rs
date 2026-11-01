//! Indexed shared-upvalue construction for nested functions.

use crate::function::BindingCell;

use super::ir::Bytecode;
use super::vm::Vm;

impl Vm<'_, '_> {
    pub(super) fn captured_upvalues_for_function(
        &mut self,
        function_bytecode: &Bytecode,
        lexical_captures: &[(String, usize)],
    ) -> Vec<BindingCell> {
        self.captured_upvalues_for_function_with_override(function_bytecode, lexical_captures, None)
    }

    pub(super) fn captured_upvalues_for_function_with_override(
        &mut self,
        function_bytecode: &Bytecode,
        _lexical_captures: &[(String, usize)],
        binding_override: Option<(&str, &BindingCell)>,
    ) -> Vec<BindingCell> {
        function_bytecode
            .locals
            .iter()
            // The producer and consumer must use exactly the same positional
            // upvalue classification. Hoisted locals and call-frame metadata
            // (`this`, `arguments`, new.target, ...) are seeded by call setup;
            // including one opportunistically when a same-named outer/global
            // binding exists shifts every later lexical upvalue.
            .filter(|local| local.is_received_upvalue())
            .enumerate()
            .filter_map(|(received, local)| {
                if local.name == "this"
                    && let Some(cell) = self.env.derived_this_cell()
                {
                    return Some(cell);
                }
                if let Some((name, upvalue)) = binding_override
                    && local.name == name
                {
                    return Some(upvalue.clone());
                }
                match function_bytecode.binding_plan.received[received] {
                    super::binding_captures::CaptureRoute::ParentLocal(slot) => {
                        if self.bytecode.is_global_scope()
                            && self.bytecode.local_is_body_hoist_only(slot.0)
                        {
                            return self.env.realm_binding_cell(&local.name);
                        }
                        return Some(self.ensure_upvalue_for_parent_slot(slot.0));
                    }
                    super::binding_captures::CaptureRoute::ParentUpvalue(index) => {
                        let slot = self.bytecode.received_upvalue_slots()[index];
                        return Some(self.ensure_upvalue_for_parent_slot(slot));
                    }
                    super::binding_captures::CaptureRoute::Named => {}
                }
                if let Some(upvalue) = self.env.module_import_cell(&local.name) {
                    return Some(upvalue);
                }
                if self.bytecode.is_global_scope() {
                    return self.env.realm_binding_cell(&local.name);
                }
                self.env.module_live_binding_cell(&local.name).or_else(|| {
                    (local.name == "this")
                        .then(|| self.env.derived_this_cell())
                        .flatten()
                })
            })
            .collect()
    }

    pub(super) fn ensure_upvalue_for_parent_slot(&mut self, slot: usize) -> BindingCell {
        if let Some(upvalue) = self.local_upvalue_cell(slot) {
            return upvalue.clone();
        }
        if self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| local.is_received_upvalue())
        {
            let index = self.bytecode.binding_plan.received_indices[slot]
                .expect("received slot has a positional route");
            if let Some(upvalue) = self.upvalues.get(index) {
                return upvalue.clone();
            }
        }
        let value = self.locals.get_mut(slot).and_then(Option::take);
        let upvalue = crate::binding::BindingCell::with_metadata(
            self.bytecode
                .binding_meta(slot)
                .expect("exposed declaration metadata"),
            value,
        );
        if self.local_upvalues.len() <= slot {
            self.local_upvalues.resize_with(slot + 1, || None);
        }
        self.local_upvalues[slot] = Some(upvalue.clone());
        if slot < u128::BITS as usize {
            self.authoritative_slots &= !(1_u128 << slot);
        }
        if self.binding_scope.is_none() {
            self.binding_scope = self
                .bytecode
                .binding_plan
                .pc_scopes
                .get(self.ip.saturating_sub(1))
                .copied();
        }
        self.slow_flags |= super::vm::SLOW_AUTHORITY
            | if self.binding_scope.is_some() {
                super::vm::SLOW_SCOPES
            } else {
                0
            };
        self.verify_binding_authority();
        crate::diagnostics::count!(binding_promotions);
        upvalue
    }

    pub(super) fn in_parameter_prologue(&self) -> bool {
        self.bytecode
            .binding_plan
            .prologue_end
            .is_some_and(|end| self.ip <= end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn capture_moves_the_value_and_tdz_without_retaining_a_slot_copy() {
        for source in [
            "function f() { let value = 4; return () => value; } f;",
            "function f() { return () => value; let value = 4; } f;",
        ] {
            let Value::Function(function) = crate::eval(source).unwrap() else {
                panic!("function");
            };
            let bytecode = function.bytecode.as_ref().unwrap();
            let slot = bytecode.local_slot("value").unwrap();
            let activation = crate::function::enter_indexed_activation(&function);
            let mut runtime = crate::context::RuntimeServices::for_realm(activation.realm());
            runtime.binding_debug.force = false;
            let mut vm = Vm::new_with_globals(bytecode, &mut runtime, activation);
            assert!(vm.local_upvalue_cell(slot).is_none());
            let Value::Function(closure) = vm.run().unwrap() else {
                panic!("closure");
            };
            let cell = vm.local_upvalue_cell(slot).unwrap();
            assert!(cell.ptr_eq(&closure.upvalues[0]));
            assert!(vm.locals[slot].is_none());
            assert_eq!(
                cell.is_initialized(),
                source.contains("let value = 4; return")
            );
        }
    }
}
