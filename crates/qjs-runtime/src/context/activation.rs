//! Activation identity, scope roots and execution mode. Ordinary entry mints no
//! variable identity and allocates no holder/lease: the identity is minted on
//! first demand (exposure or a continuation view), and exposure attaches cells
//! and holders to it. Views retain identity, never slot values.
//!
//! Reuse invariant: only a never-exposed activation is released for reuse
//! (`release_for_reuse`): unminted identity, inline owner, no scopes, holders
//! or saved references. Nothing can then reference it -- leases, holders,
//! eval scopes, continuations and generators reach an activation only through
//! a minted identity or a shared owner, and cells/closures/arguments objects
//! never hold one. Released activations live only in bounded pools; under
//! `QJS_POISON_RELEASED_ACTIVATIONS` (debug) their receiver and new.target are
//! poisoned and checked intact when rebound.
use super::{CreationContext, LexicalCallContext, OwnerKind};
use crate::binding::{ScopeCursor, ScopeId, ScopeLease, VariableEnvIdentity};
use std::{cell::Cell, rc::Rc};

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
impl ScopeRoots {
    fn is_empty(&self) -> bool {
        !self.module_root && self.cursor.is_none() && self.entry.is_none() && self.global.is_none()
    }
}

pub(crate) struct ActivationContext {
    pub(crate) creation: Rc<CreationContext>,
    pub(crate) call: LexicalCallContext,
    pub(crate) scopes: ScopeRoots,
    /// Zero until first demanded (`variable_identity`); realms mint from one.
    variable_activation: Cell<u64>,
    variable_boundary: ScopeId,
    pub(crate) mode: ExecutionMode,
    pub(crate) strict: bool,
    /// References a tier without a VM frame selected before an RHS, by plan
    /// lifetime (`reference_admission`). Normally `None`; an interpreter
    /// continuation of the activation adopts them.
    pub(crate) saved_references: Option<Box<SavedReferences>>,
}
pub(crate) type SavedReferences = Vec<(usize, crate::binding::ResolvedReference)>;

/// Marks a released activation whose receiver/new.target hold the stress
/// poison; no compiled scope has this id.
const POISONED_BOUNDARY: ScopeId = ScopeId(usize::MAX);
impl ActivationContext {
    pub(crate) fn new(creation: Rc<CreationContext>, mode: ExecutionMode) -> Self {
        Self {
            creation,
            call: LexicalCallContext::default(),
            scopes: ScopeRoots::default(),
            variable_activation: Cell::new(0),
            variable_boundary: ScopeId(0),
            mode,
            strict: false,
            saved_references: None,
        }
    }

    /// An activation whose call owner is known at entry, built in one step.
    #[inline]
    pub(crate) fn for_call(
        creation: Rc<CreationContext>,
        call: LexicalCallContext,
        strict: bool,
    ) -> Self {
        let mode = match &call {
            LexicalCallContext::Inline(owner) => {
                debug_assert_ne!(
                    owner.kind,
                    OwnerKind::Module,
                    "module roots use set_call_owner"
                );
                owner.kind.into()
            }
            _ => ExecutionMode::Function,
        };
        Self {
            creation,
            call,
            scopes: ScopeRoots::default(),
            variable_activation: Cell::new(0),
            variable_boundary: ScopeId(0),
            mode,
            strict,
            saved_references: None,
        }
    }

    /// Releases a returned ordinary call's receiver and new.target so a
    /// pool may rebind this activation to a later indexed call (module
    /// header). A method's class record stays for the rebinding entry to
    /// update (`Function::rebind_class_state`). False (keep nothing) once the
    /// activation was ever exposed or retained: a minted identity, a shared
    /// owner, a derived receiver or constructor, scopes or saved references
    /// all belong to that activation alone. `poison` (debug stress) fills
    /// the released receiver and new.target with a sentinel.
    pub(crate) fn release_for_reuse(&mut self, poison: bool) -> bool {
        let LexicalCallContext::Inline(owner) = &mut self.call else {
            return false;
        };
        if self.variable_activation.get() != 0
            || owner.active_constructor().is_some()
            || !matches!(owner.this, super::ThisBinding::Initialized(_))
            || self.saved_references.is_some()
            || !self.scopes.is_empty()
        {
            return false;
        }
        // Release builds never poison (`BindingDebug`); keep the check out.
        if cfg!(debug_assertions) && poison {
            owner.this = super::ThisBinding::Initialized(crate::Value::Number(f64::NAN));
            owner.new_target = Some(crate::Value::Number(f64::NAN));
            self.variable_boundary = POISONED_BOUNDARY;
        } else {
            owner.this = super::ThisBinding::Initialized(crate::Value::Undefined);
            owner.new_target = None;
        }
        true
    }

    /// Rebinds a released activation (`release_for_reuse`) to an ordinary
    /// call that `for_call` would build with this creation and owner kind,
    /// receiver unbound, once `rebind_class` has set its class state. The
    /// identity is minted afresh on demand.
    #[inline]
    pub(crate) fn reuse_for_call(
        &mut self,
        creation: &Rc<CreationContext>,
        kind: OwnerKind,
        strict: bool,
        rebind_class: impl FnOnce(&mut Option<Box<super::ClassOwner>>) -> bool,
    ) -> bool {
        if !Rc::ptr_eq(&self.creation, creation) || kind == OwnerKind::Module {
            return false;
        }
        let LexicalCallContext::Inline(owner) = &mut self.call else {
            return false;
        };
        if !rebind_class(&mut owner.class) {
            return false;
        }
        if cfg!(debug_assertions) && self.variable_boundary == POISONED_BOUNDARY {
            assert!(
                matches!(&owner.this, super::ThisBinding::Initialized(crate::Value::Number(value)) if value.is_nan())
                    && matches!(&owner.new_target, Some(crate::Value::Number(value)) if value.is_nan())
                    && self.variable_activation.get() == 0,
                "released activation poison changed"
            );
            owner.this = super::ThisBinding::Initialized(crate::Value::Undefined);
            owner.new_target = None;
        }
        owner.kind = kind;
        self.mode = kind.into();
        self.strict = strict;
        self.variable_activation.set(0);
        self.variable_boundary = ScopeId(0);
        true
    }

    /// This activation's variable identity, minted on first demand so an
    /// ordinary call that never exposes a scope pays nothing for it.
    pub(crate) fn variable_identity(&self) -> VariableEnvIdentity {
        let mut activation = self.variable_activation.get();
        if activation == 0 {
            activation = self.creation.realm.atoms().next_variable_identity();
            self.variable_activation.set(activation);
        }
        VariableEnvIdentity {
            activation,
            boundary: self.variable_boundary,
        }
    }

    pub(crate) fn select_variable_boundary(&mut self, scope: ScopeId) {
        self.variable_boundary = scope;
    }

    pub(crate) fn adopt_variable_identity(&mut self, identity: VariableEnvIdentity) {
        self.variable_activation.set(identity.activation);
        self.variable_boundary = identity.boundary;
    }

    pub(crate) fn retain_continuation(&mut self) -> Self {
        // A view shares this activation's identity, so mint it first.
        let identity = self.variable_identity();
        Self {
            creation: self.creation.clone(),
            call: self.call.shared_view(),
            scopes: self.scopes.clone(),
            variable_activation: Cell::new(identity.activation),
            variable_boundary: identity.boundary,
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
        assert_ne!(first.variable_identity(), second.variable_identity());
        assert!(first.scopes.cursor.is_none());
        assert!(second.scopes.cursor.is_none());
        let other_realm = new_realm_sharing_atoms(HashMap::new(), atoms);
        let other =
            ActivationContext::new(CreationContext::root(other_realm), ExecutionMode::Function);
        assert_ne!(first.variable_identity(), other.variable_identity());
        assert_ne!(second.variable_identity(), other.variable_identity());
    }

    fn call_activation() -> ActivationContext {
        let realm = new_realm_sharing_atoms(HashMap::new(), AtomDomain::new());
        let owner = super::super::CallOwner::root(crate::Value::Number(1.0), OwnerKind::Function);
        ActivationContext::for_call(
            CreationContext::root(realm),
            LexicalCallContext::Inline(owner),
            false,
        )
    }

    fn rebind(activation: &mut ActivationContext) -> bool {
        let creation = activation.creation.clone();
        activation.reuse_for_call(&creation, OwnerKind::Function, true, |_| true)
    }

    #[test]
    fn only_never_exposed_activations_are_released() {
        let mut plain = call_activation();
        assert!(plain.release_for_reuse(false));
        assert!(rebind(&mut plain));
        assert!(
            plain
                .this_value()
                .unwrap()
                .same_value(&crate::Value::Undefined)
        );
        assert!(plain.strict);

        // A minted identity may be held by a lease, holder or eval scope.
        let mut exposed = call_activation();
        let _ = exposed.variable_identity();
        assert!(!exposed.release_for_reuse(false));
        // A continuation view shares the owner.
        let mut viewed = call_activation();
        let _view = viewed.retain_continuation();
        assert!(!viewed.release_for_reuse(false));
        let mut scoped = call_activation();
        scoped.scopes.global = Some(GlobalCodeMode::Eval);
        assert!(!scoped.release_for_reuse(false));
    }

    #[test]
    fn poisoned_releases_are_checked_and_cleared_on_rebind() {
        let mut activation = call_activation();
        assert!(activation.release_for_reuse(true));
        assert!(matches!(
            activation.this_value(),
            Ok(crate::Value::Number(value)) if value.is_nan()
        ));
        assert!(rebind(&mut activation));
        assert!(
            activation
                .this_value()
                .unwrap()
                .same_value(&crate::Value::Undefined)
        );
        assert!(activation.lexical_new_target().is_none());
    }

    #[test]
    #[should_panic(expected = "released activation poison changed")]
    fn a_write_to_a_poisoned_release_is_caught_on_rebind() {
        let mut activation = call_activation();
        assert!(activation.release_for_reuse(true));
        if let LexicalCallContext::Inline(owner) = &mut activation.call {
            owner.this = super::super::ThisBinding::Initialized(crate::Value::Number(2.0));
        }
        rebind(&mut activation);
    }
}
