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
        self.binding_plan.reads_new_target
    }

    /// Any direct eval/with in this body or its statically nested code.
    pub(crate) fn has_dynamic_scope_tree(&self) -> bool {
        self.binding_plan.dynamic_scope_tree
    }

    /// Includes inline keys and lexical arrows using this call's
    /// receiver. Receiver-owning children are separate call-context boundaries.
    pub(crate) fn needs_lexical_owner(&self) -> bool {
        self.binding_plan.needs_lexical_owner
    }

    pub(crate) fn needs_call_receiver(&self) -> bool {
        self.binding_plan.needs_receiver
    }

    pub(crate) fn has_direct_seedable_parameters(&self) -> bool {
        self.binding_plan.parameters.direct_seedable
    }

    pub(crate) fn parameters_shadow_arguments(&self) -> bool {
        self.binding_plan.parameters.shadows_arguments
    }

    pub(crate) fn mapped_argument_slot(&self, index: usize) -> Option<usize> {
        self.binding_plan
            .parameters
            .mapped_slots
            .get(index)
            .copied()
            .flatten()
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
