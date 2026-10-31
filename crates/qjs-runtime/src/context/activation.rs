//! Activation identity, scope roots and execution mode. Ordinary entry creates
//! its variable identity without allocating a holder/lease; exposure attaches
//! cells and holders to that identity. Views retain identity, never slot values.
use super::{CreationContext, LexicalCallContext, OwnerKind};
use crate::binding::{ScopeCursor, ScopeId, ScopeLease, VariableEnvIdentity};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExecutionMode {
    Script,
    Module,
    Function,
    Eval,
    Field,
    StaticBlock,
    ClassKey,
}
impl From<OwnerKind> for ExecutionMode {
    fn from(kind: OwnerKind) -> Self {
        match kind {
            OwnerKind::Script => Self::Script,
            OwnerKind::Module => Self::Module,
            OwnerKind::Function => Self::Function,
            OwnerKind::Field => Self::Field,
            OwnerKind::StaticBlock => Self::StaticBlock,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct ScopeRoots {
    pub(crate) module_root: bool,
    pub(crate) cursor: Option<ScopeCursor>,
    pub(crate) entry: Option<Rc<ScopeLease>>,
}

pub(crate) struct ActivationContext {
    pub(crate) creation: Rc<CreationContext>,
    // CallEnv's temporary Clone API promotes through &self. B5 removes this
    // RefCell together with CallEnv, using Context's mutable activation borrow.
    pub(crate) call: RefCell<LexicalCallContext>,
    pub(crate) scopes: ScopeRoots,
    pub(crate) variable: VariableEnvIdentity,
    pub(crate) mode: ExecutionMode,
    pub(crate) strict: bool,
}
impl ActivationContext {
    pub(crate) fn new(creation: Rc<CreationContext>, mode: ExecutionMode) -> Self {
        let activation = creation.realm.atoms().next_variable_identity();
        Self {
            creation,
            call: RefCell::default(),
            scopes: ScopeRoots::default(),
            variable: VariableEnvIdentity {
                activation,
                boundary: ScopeId(0),
            },
            mode,
            strict: false,
        }
    }
    pub(crate) fn retained_view(&self) -> Self {
        Self {
            creation: self.creation.clone(),
            call: RefCell::new(self.call.borrow_mut().shared_view()),
            scopes: self.scopes.clone(),
            variable: self.variable,
            mode: self.mode,
            strict: self.strict,
        }
    }
    pub(crate) fn fresh_function(&self) -> Self {
        let mut creation = self.creation.clone();
        if !matches!(creation.global, super::GlobalSelection::Default) {
            Rc::make_mut(&mut creation).global = super::GlobalSelection::Default;
        }
        Self::new(creation, ExecutionMode::Function)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::new_realm_sharing_atoms;
    use crate::property::atom::AtomDomain;
    use std::collections::HashMap;
    #[test]
    fn calls_get_identity_before_exposure_and_views_keep_it() {
        let atoms = AtomDomain::new();
        let realm = new_realm_sharing_atoms(HashMap::new(), atoms.clone());
        let root = ActivationContext::new(CreationContext::root(realm), ExecutionMode::Script);
        let first = root.fresh_function();
        let second = root.fresh_function();
        assert_ne!(first.variable, second.variable);
        assert_eq!(first.variable, first.retained_view().variable);
        assert!(first.scopes.cursor.is_none());
        assert!(second.scopes.cursor.is_none());
        let other_realm = new_realm_sharing_atoms(HashMap::new(), atoms);
        let other =
            ActivationContext::new(CreationContext::root(other_realm), ExecutionMode::Function);
        assert_ne!(first.variable, other.variable);
        assert_ne!(second.variable, other.variable);
    }
}
