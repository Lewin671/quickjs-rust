//! A stack borrow of runtime services and one activation. This facade is never
//! cloned, stored or retained; callbacks receive a child activation and reborrow.
use super::{ActivationContext, DirectEvalSite, RuntimeServices};
use crate::{Value, native::CallInfo};

#[derive(Clone, Copy)]
pub(crate) struct NativeSite<'a> {
    pub(crate) info: &'a CallInfo<'a>,
    pub(crate) target: Option<&'a Value>,
    pub(crate) direct_eval: Option<&'a DirectEvalSite<'a>>,
}

pub(crate) struct Context<'a> {
    pub(crate) runtime: &'a mut RuntimeServices,
    pub(crate) activation: &'a mut ActivationContext,
    pub(crate) native: Option<NativeSite<'a>>,
}
impl Context<'_> {
    pub(crate) fn reborrow(&mut self) -> Context<'_> {
        Context {
            runtime: self.runtime,
            activation: self.activation,
            native: self.native,
        }
    }
    pub(crate) fn realm(&self) -> &super::Realm {
        match &self.native {
            Some(site) => site.info.callee.creation_realm(),
            None => &self.activation.creation.realm,
        }
    }
    pub(crate) fn creation_context_ref(&self) -> &std::rc::Rc<super::CreationContext> {
        match &self.native {
            Some(site) if site.direct_eval.is_none() => site.info.callee.creation_context(),
            _ => &self.activation.creation,
        }
    }
    pub(crate) fn caller(&self) -> &ActivationContext {
        self.activation
    }
    pub(crate) fn new_target(&self) -> Option<&Value> {
        self.native.as_ref().and_then(|site| site.target)
    }
    pub(crate) fn with_native<'a>(
        &'a mut self,
        info: &'a CallInfo<'a>,
        target: Option<&'a Value>,
        direct_eval: Option<&'a DirectEvalSite<'a>>,
    ) -> Context<'a> {
        Context {
            runtime: self.runtime,
            activation: self.activation,
            native: Some(NativeSite {
                info,
                target,
                direct_eval,
            }),
        }
    }
}
