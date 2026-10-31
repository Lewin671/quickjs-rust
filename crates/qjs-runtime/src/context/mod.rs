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
    ErrorKind, PrimitiveIntrinsic, constructor_function_prototype, constructor_object_prototype,
};
mod eval_cache;
mod realm;
pub(crate) use realm::{Realm, new_realm_sharing_atoms};

/// Parse/declaration facts belonging to the immediate direct-eval call site.
/// B5 borrows this through NativeSite instead of retaining it on a service view.
#[derive(Clone, Copy)]
pub(crate) struct DirectEvalSite<'a> {
    pub(crate) strict: bool,
    pub(crate) in_parameters: bool,
    pub(crate) scope: Option<&'a crate::binding::ScopeCursor>,
    pub(crate) private: Option<&'a crate::private::PrivateEnvironment>,
}

mod services;
pub(crate) use services::{GroupState, RuntimeGroup, RuntimeServices};
mod facade;
pub(crate) use facade::Context;
mod activation_api;
mod capabilities;

mod root;
pub(crate) use root::HostRoot;

#[cfg(test)]
pub(crate) use realm::new_realm;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use creation::ModuleImports;
