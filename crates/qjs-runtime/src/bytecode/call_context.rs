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
        self.code.iter().any(|op| matches!(op, Op::LoadNewTarget))
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
