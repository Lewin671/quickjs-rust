//! Typed execution ownership. Lexical call state is separate from declarative
//! bindings and can be retained without copying an activation's name/value view.
mod call_owner;
pub(crate) use call_owner::{CallOwner, LexicalCallContext, OwnerKind, ThisBinding};
mod activation;
mod creation;
pub(crate) use activation::{ActivationContext, ExecutionMode};
pub(crate) use creation::{CreationContext, GlobalSelection, IntrinsicOverrides, ScriptOrModule};
mod intrinsics;
pub(crate) use intrinsics::{
    PrimitiveIntrinsic, constructor_function_prototype, constructor_object_prototype,
};
mod eval_cache;
mod realm;
pub(crate) use creation::ModuleImports;
pub(crate) use realm::{Realm, new_realm, new_realm_sharing_atoms};
