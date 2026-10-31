//! Indexed shared-upvalue construction for nested functions.

use crate::{Function, Value, function::BindingCell};

use super::ir::Bytecode;
use super::vm::Vm;

impl Vm<'_> {
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
                if self.in_parameter_prologue() {
                    let marker_name = format!(
                        "{}{}",
                        crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX,
                        local.name
                    );
                    if let Some(bindings) = self.env.deopt_bindings()
                        && bindings.contains_key(&marker_name)
                        && let Some(upvalue) = bindings.cell(&local.name)
                    {
                        return Some(upvalue);
                    }
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
                self.env
                    .module_live_binding_cell(&local.name)
                    .or_else(|| self.env.frame_binding_cell(&local.name))
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
        let value = self
            .locals
            .get(slot)
            .and_then(Option::as_ref)
            .cloned()
            .unwrap_or_else(|| Value::Function(Function::uninitialized_lexical_marker()));
        let upvalue = self.bytecode.binding_meta(slot).map_or_else(
            || BindingCell::new(value.clone()),
            |meta| BindingCell::with_metadata(meta, Some(value.clone())),
        );
        if let Some(local_upvalue) = self.local_upvalues.get_mut(slot) {
            *local_upvalue = Some(upvalue.clone());
        }
        if slot < u128::BITS as usize {
            self.authoritative_slots &= !(1_u128 << slot);
        }
        upvalue
    }

    pub(super) fn in_parameter_prologue(&self) -> bool {
        if self.bytecode.global_scope {
            return false;
        }
        if !self.bytecode.locals.iter().any(|local| local.parameter) {
            return false;
        }
        !self.bytecode.code[..self.ip]
            .iter()
            .any(|op| matches!(op, super::ir::Op::FunctionPrologueEnd))
    }
}
