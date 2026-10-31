//! Typed execution ownership. Lexical call state is separate from declarative
//! bindings and can be retained without copying an activation's name/value view.
mod call_owner;
pub(crate) use call_owner::{CallOwner, LexicalCallContext, OwnerKind, ThisBinding};
