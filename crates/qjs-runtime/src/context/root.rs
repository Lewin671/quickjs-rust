//! Explicit host/test roots. Internal calls borrow Context and never use this
//! builder to manufacture a native/coercion environment.
use super::{ActivationContext, Context, CreationContext, ExecutionMode, Realm, RuntimeServices};
use crate::{GLOBAL_THIS_BINDING, ObjectRef, Value};
use std::collections::HashMap;

pub(crate) struct HostRoot {
    pub(crate) runtime: RuntimeServices,
    pub(crate) activation: ActivationContext,
}
impl HostRoot {
    pub(crate) fn new(realm: Realm) -> Self {
        let runtime = RuntimeServices::for_realm(&realm);
        let activation =
            ActivationContext::new(CreationContext::root(realm), ExecutionMode::Script);
        Self {
            runtime,
            activation,
        }
    }
    pub(crate) fn script() -> Self {
        let atoms = crate::property::atom::AtomDomain::new();
        let global = Value::Object(ObjectRef::new(HashMap::new(), &atoms));
        let mut values = HashMap::new();
        values.insert(GLOBAL_THIS_BINDING.into(), global.clone());
        values.insert("undefined".into(), Value::Undefined);
        let mut root = Self::new(super::new_realm_sharing_atoms(values, atoms));
        crate::initialize_builtins(&mut root.context(), &global);
        root.activation
            .realm()
            .refresh_dynamic_function_realm_global();
        root
    }
    pub(crate) fn context(&mut self) -> Context<'_> {
        Context {
            runtime: &mut self.runtime,
            activation: &mut self.activation,
            native: None,
        }
    }
    pub(crate) fn into_parts(self) -> (RuntimeServices, ActivationContext) {
        (self.runtime, self.activation)
    }
}
