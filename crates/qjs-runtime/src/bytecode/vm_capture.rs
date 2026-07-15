//! Indexed shared-upvalue construction for nested functions.

use crate::{Function, Value, function::Upvalue};

use super::ir::Bytecode;
use super::vm::Vm;

impl Vm<'_> {
    pub(super) fn captured_upvalues_for_function(
        &mut self,
        function_bytecode: &Bytecode,
        lexical_captures: &[(String, usize)],
    ) -> Vec<Upvalue> {
        self.captured_upvalues_for_function_with_override(function_bytecode, lexical_captures, None)
    }

    pub(super) fn captured_upvalues_for_function_with_override(
        &mut self,
        function_bytecode: &Bytecode,
        lexical_captures: &[(String, usize)],
        binding_override: Option<(&str, &Upvalue)>,
    ) -> Vec<Upvalue> {
        function_bytecode
            .locals
            .iter()
            // The producer and consumer must use exactly the same positional
            // upvalue classification. Hoisted locals and call-frame metadata
            // (`this`, `arguments`, new.target, ...) are seeded by call setup;
            // including one opportunistically when a same-named outer/global
            // binding exists shifts every later lexical upvalue.
            .filter(|local| local.is_received_upvalue())
            .filter_map(|local| {
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
                let parent_slot = lexical_captures
                    .iter()
                    .find(|(name, _)| name == &local.name)
                    .map(|(_, slot)| *slot)
                    .or_else(|| {
                        self.bytecode.local_slot(&local.name).filter(|slot| {
                            !(self.bytecode.is_global_scope()
                                && self.bytecode.local_is_body_hoist_only(*slot)
                                && self.bytecode.local_name_at(*slot).is_some_and(|name| {
                                    !super::vm_bindings::is_compiler_temporary(name)
                                }))
                        })
                    });
                if let Some(slot) = parent_slot {
                    if self.bytecode.is_global_scope()
                        && self.bytecode.local_is_body_hoist_only(slot)
                    {
                        return self.env.realm_binding_cell(&local.name);
                    }
                    return Some(self.ensure_upvalue_for_parent_slot(slot));
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

    pub(super) fn ensure_upvalue_for_parent_slot(&mut self, slot: usize) -> Upvalue {
        if let Some(upvalue) = self.local_upvalues.get(slot).and_then(Option::as_ref) {
            return upvalue.clone();
        }
        if self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| local.is_received_upvalue())
        {
            let index = self.bytecode.locals[..slot]
                .iter()
                .filter(|local| local.is_received_upvalue())
                .count();
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
        let upvalue = Upvalue::new(value);
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
