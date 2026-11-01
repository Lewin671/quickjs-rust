//! Activation identity, scope roots and execution mode. Ordinary entry creates
//! its variable identity without allocating a holder/lease; exposure attaches
//! cells and holders to that identity. Views retain identity, never slot values.
use super::{CreationContext, LexicalCallContext, OwnerKind};
use crate::binding::{ScopeCursor, ScopeId, ScopeLease, VariableEnvIdentity};
use std::rc::Rc;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GlobalCodeMode {
    Script,
    Eval,
}

#[derive(Clone, Default)]
pub(crate) struct ScopeRoots {
    pub(crate) module_root: bool,
    pub(crate) cursor: Option<ScopeCursor>,
    pub(crate) entry: Option<Rc<ScopeLease>>,
    pub(crate) global: Option<GlobalCodeMode>,
}

pub(crate) struct ActivationContext {
    pub(crate) creation: Rc<CreationContext>,
    pub(crate) call: LexicalCallContext,
    pub(crate) scopes: ScopeRoots,
    pub(crate) variable: VariableEnvIdentity,
    pub(crate) mode: ExecutionMode,
    pub(crate) strict: bool,
    /// References a tier without a VM frame selected before an RHS, by plan
    /// lifetime (`reference_admission`). Normally `None`; an interpreter
    /// continuation of the activation adopts them.
    pub(crate) saved_references: Option<Box<SavedReferences>>,
}
pub(crate) type SavedReferences = Vec<(usize, crate::binding::ResolvedReference)>;
impl ActivationContext {
    pub(crate) fn new(creation: Rc<CreationContext>, mode: ExecutionMode) -> Self {
        let activation = creation.realm.atoms().next_variable_identity();
        Self {
            creation,
            call: LexicalCallContext::default(),
            scopes: ScopeRoots::default(),
            variable: VariableEnvIdentity {
                activation,
                boundary: ScopeId(0),
            },
            mode,
            strict: false,
            saved_references: None,
        }
    }

    pub(crate) fn retain_continuation(&mut self) -> Self {
        Self {
            creation: self.creation.clone(),
            call: self.call.shared_view(),
            scopes: self.scopes.clone(),
            variable: self.variable,
            mode: self.mode,
            strict: self.strict,
            saved_references: None,
        }
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
        let first = ActivationContext::new(root.creation.clone(), ExecutionMode::Function);
        let second = ActivationContext::new(root.creation.clone(), ExecutionMode::Function);
        assert_ne!(first.variable, second.variable);
        assert!(first.scopes.cursor.is_none());
        assert!(second.scopes.cursor.is_none());
        let other_realm = new_realm_sharing_atoms(HashMap::new(), atoms);
        let other =
            ActivationContext::new(CreationContext::root(other_realm), ExecutionMode::Function);
        assert_ne!(first.variable, other.variable);
        assert_ne!(second.variable, other.variable);
    }
}
