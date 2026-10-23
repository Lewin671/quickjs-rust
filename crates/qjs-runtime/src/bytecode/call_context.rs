//! What a function body needs from the call that entered it.
//!
//! `this`, `arguments`, and `new.target` come from the call for an ordinary
//! function and from the enclosing function for an arrow. A body that never
//! reads them does not care which, and that is what lets the call path give an
//! arrow the same slot-seeded frame an ordinary function gets.

use super::ir::{Bytecode, Op};

impl Bytecode {
    /// Whether the code reads `new.target`.
    pub(crate) fn reads_new_target(&self) -> bool {
        self.frame_init_plan.reads_new_target
    }

    /// Any direct eval/with in this body or its statically nested code.
    pub(crate) fn has_dynamic_scope_tree(&self) -> bool {
        self.frame_init_plan.dynamic_scope_tree
    }

    /// Positional inputs cannot depend on dynamic names or outlive call entry.
    pub(crate) fn supports_positional_entry(&self) -> bool {
        !self.global_scope
            && !self.suspends()
            && !self.has_dynamic_scope_tree()
            && !self.needs_arguments_object()
    }

    pub(crate) fn needs_call_receiver(&self) -> bool {
        self.frame_init_plan.needs_receiver
    }

    pub(crate) fn has_direct_seedable_parameters(&self) -> bool {
        self.frame_init_plan.parameters.direct_seedable
    }

    pub(crate) fn parameters_shadow_arguments(&self) -> bool {
        self.frame_init_plan.parameters.shadows_arguments
    }

    pub(crate) fn positional_parameter_bindings(&self) -> impl Iterator<Item = &str> {
        self.parameter_slots()
            .iter()
            .map(|&slot| self.locals[slot].name.as_str())
    }

    pub(crate) fn rest_parameter_binding(&self) -> Option<&str> {
        self.frame_init_plan
            .parameters
            .rest_slot
            .map(|slot| self.locals[slot].name.as_str())
    }

    /// Whether an immutable environment binding named `name` -- a class's
    /// inner name as seen by its methods, or a named function expression's
    /// own name -- is observed by this body only through reads of a received
    /// cell. Such a body needs no frame binding for the name: the binding's
    /// only other job is the assignment diagnostic, and the body never
    /// assigns. A body that reaches the name by global lookup, or writes it,
    /// keeps the general frame that installs the binding.
    pub(crate) fn reads_immutable_env_binding_through_cell(&self, name: &str) -> bool {
        let Some(slot) = self.local_slot(name) else {
            return false;
        };
        if !self
            .locals
            .get(slot)
            .is_some_and(|local| local.is_received_upvalue())
        {
            return false;
        }
        !self.code.iter().any(|op| match op {
            Op::StoreLocal(target) | Op::AssignLocal(target) | Op::ClearLocal(target) => {
                *target == slot
            }
            Op::StoreLocalOrGlobalSloppy { slot: target, .. } => *target == slot,
            Op::LoadGlobal(global) | Op::TypeofGlobal(global) => global == name,
            _ => false,
        })
    }

    /// Whether the code mentions `arguments` at all, either as a free name or as
    /// a binding it received from the function that created it.
    pub(crate) fn reads_arguments(&self) -> bool {
        self.local_slot("arguments").is_some()
            || self
                .code
                .iter()
                .any(|op| matches!(op, Op::LoadGlobal(name) if name == "arguments"))
    }
}
