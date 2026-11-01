//! Services at the existing communicating-realm/atom boundary. Registry entries
//! are weak metadata; they neither retain realms nor copy global values. The
//! released-activation pool holds only creation handles of ordinary calls
//! that returned, bounded and dropped with the services.
use super::{Realm, realm::RealmState};
use crate::{ObjectRef, property::atom::AtomDomain};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
};

pub(crate) struct GroupState {
    atoms: AtomDomain,
    realms: RefCell<HashMap<usize, Weak<RealmState>>>,
    cleanup_at: Cell<usize>,
    multiple_realms: Cell<bool>,
}
#[derive(Clone)]
pub(crate) struct RuntimeGroup(Rc<GroupState>);
impl RuntimeGroup {
    pub(crate) fn for_atoms(atoms: &AtomDomain) -> Self {
        if let Some(group) = atoms.runtime_group().borrow().upgrade() {
            return Self(group);
        }
        let group = Rc::new(GroupState {
            atoms: atoms.clone(),
            realms: RefCell::default(),
            cleanup_at: Cell::new(32),
            multiple_realms: Cell::new(false),
        });
        *atoms.runtime_group().borrow_mut() = Rc::downgrade(&group);
        Self(group)
    }
    pub(crate) fn register(&self, realm: &Realm) {
        if let Some(crate::Value::Object(global)) = realm.global_this() {
            let mut realms = self.0.realms.borrow_mut();
            realms.insert(global.address(), Rc::downgrade(realm));
            if realms.len() >= self.0.cleanup_at.get() {
                realms.retain(|_, realm| realm.strong_count() != 0);
                self.0
                    .cleanup_at
                    .set(realms.len().saturating_mul(2).max(32));
            }
            self.0.multiple_realms.set(realms.len() > 1);
        }
    }
    pub(crate) fn global_realm(&self, object: &ObjectRef) -> Option<Realm> {
        let realm = self.0.realms.borrow().get(&object.address())?.upgrade()?;
        realm.is_global_object(object).then_some(realm)
    }
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn has_foreign_realms(&self) -> bool {
        self.0.multiple_realms.get()
    }
    pub(crate) fn atoms(&self) -> &AtomDomain {
        &self.0.atoms
    }
}

/// Resolver ownership stays with ScriptOrModule; this service selects that
/// canonical host and never creates a second loader or job queue.
#[derive(Default)]
pub(crate) struct HostServices;
impl HostServices {
    pub(crate) fn module<'a>(
        &self,
        creation: &'a super::CreationContext,
    ) -> Option<&'a crate::module::ModuleHostRef> {
        creation.referrer.host()
    }
}

pub(crate) struct RuntimeServices {
    pub(crate) group: RuntimeGroup,
    pub(crate) host: HostServices,
    pub(crate) binding_debug: crate::binding::BindingDebug,
    #[cfg(feature = "agents")]
    pub(crate) agent: Option<crate::agent::AgentContextRef>,
    /// Released callback activations (`ActivationContext::release_for_reuse`)
    /// a later indexed entry rebinds instead of rebuilding.
    pub(crate) released_activations: Vec<super::ActivationContext>,
}
impl RuntimeServices {
    pub(crate) fn for_realm(realm: &Realm) -> Self {
        debug_assert!(realm.group().atoms().ptr_eq(realm.atoms()));
        Self {
            group: realm.group().clone(),
            host: HostServices,
            binding_debug: crate::binding::BindingDebug::capture(),
            #[cfg(feature = "agents")]
            agent: None,
            released_activations: Vec::new(),
        }
    }
    /// Explicit retention for the host EvalOutcome Clone API. Internal calls
    /// reborrow services; they cannot clone an execution facade.
    pub(crate) fn retain_host_root(&self) -> Self {
        Self {
            group: self.group.clone(),
            host: HostServices,
            binding_debug: self.binding_debug,
            #[cfg(feature = "agents")]
            agent: self.agent.clone(),
            released_activations: Vec::new(),
        }
    }
}
