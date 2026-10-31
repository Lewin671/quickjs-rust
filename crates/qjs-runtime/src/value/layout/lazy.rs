//! Empty named authorities own only their canonical layout and explicit
//! prototype. Value storage is allocated on mutation, never on a read probe.
use super::*;
use crate::Prototype;

pub(crate) enum NamedStorage {
    Sentinel,
    Empty {
        layout: Rc<Layout>,
        prototype: Option<Rc<Prototype>>,
    },
    Full(Box<NamedStore>),
}
impl NamedStorage {
    pub(crate) fn new(domain: &AtomDomain, prototype: Option<Prototype>, flavor: u16) -> Self {
        let mut store = NamedStore::empty_with_prototype(domain, prototype, flavor);
        store.materialize_layout();
        Self::Empty {
            layout: store.layout,
            prototype: store.prototype.map(Rc::new),
        }
    }
    /// Fork a realm's immutable empty seed on the first receiver mutation.
    pub(crate) fn fork_empty_store(&self) -> NamedStore {
        match self {
            Self::Empty { layout, prototype } => NamedStore {
                layout: layout.clone(),
                prototype: prototype.as_deref().cloned(),
                values: ValueSlots::Empty,
                cold: ColdStorage::default(),
            },
            _ => panic!("array seed must be an empty authority"),
        }
    }
    pub(crate) fn store(&self) -> Option<&NamedStore> {
        match self {
            Self::Full(s) => Some(s),
            _ => None,
        }
    }
    pub(crate) fn store_mut(&mut self) -> &mut NamedStore {
        if let Self::Empty { layout, prototype } = self {
            let store = NamedStore {
                layout: layout.clone(),
                prototype: prototype.as_deref().cloned(),
                values: ValueSlots::Empty,
                cold: ColdStorage::default(),
            };
            *self = Self::Full(Box::new(store));
        }
        match self {
            Self::Full(s) => s,
            _ => panic!("TDZ sentinel has no public object storage"),
        }
    }
    pub(crate) fn domain(&self) -> &AtomDomain {
        match self {
            Self::Empty { layout, .. } => &layout.domain,
            Self::Full(s) => s.domain(),
            Self::Sentinel => panic!("TDZ sentinel domain"),
        }
    }
    pub(crate) fn version(&self) -> Option<LayoutVersion> {
        match self {
            Self::Empty { layout, .. } => Some(LayoutVersion::shared(layout)),
            Self::Full(s) => Some(s.version()),
            Self::Sentinel => None,
        }
    }
    pub(crate) fn matches(&self, v: &LayoutVersion) -> bool {
        match self {
            Self::Empty { layout, .. } => v.matches_layout(layout, 0),
            Self::Full(s) => s.matches_version(v),
            Self::Sentinel => false,
        }
    }
    pub(crate) fn first_prototype_data(&self, version: &LayoutVersion, slot: u32) -> Option<Value> {
        match self {
            Self::Empty { prototype, .. } if self.matches(version) => {
                prototype.as_deref()?.load_guarded_data(slot)
            }
            Self::Full(s) => s.first_prototype_data(version, slot),
            _ => None,
        }
    }
    pub(crate) fn prototype_is_object(&self, expected: &crate::ObjectRef) -> bool {
        match self {
            Self::Empty { prototype, .. } => {
                matches!(prototype.as_deref(), Some(Prototype::Object(actual)) if actual.ptr_eq(expected))
            }
            Self::Full(store) => store.prototype_is_object(expected),
            Self::Sentinel => false,
        }
    }
    pub(crate) fn prototype(&self) -> Option<Prototype> {
        match self {
            Self::Empty { prototype, .. } => prototype.as_deref().cloned(),
            Self::Full(s) => s.prototype(),
            Self::Sentinel => None,
        }
    }
    pub(crate) fn chain_dependency(&self) -> Option<StructuralDependency> {
        match self {
            Self::Empty { prototype, .. } => prototype.as_deref()?.enter_prototype_mode()?.token(),
            Self::Full(s) => s.chain_dependency(),
            Self::Sentinel => None,
        }
    }
    pub(crate) fn set_prototype(&mut self, p: Option<Prototype>, flavor: u16) {
        match self {
            Self::Empty { layout, prototype } => {
                let owner = PropertyOwner::new(layout.domain.clone());
                let next = owner.root_for(p.as_ref(), flavor);
                *prototype = p.map(Rc::new);
                *layout = next;
            }
            Self::Full(s) => s.rebase(p, flavor),
            Self::Sentinel => {}
        }
    }
    pub(crate) fn set_flavor(&mut self, flavor: u16) {
        let p = self.prototype();
        self.set_prototype(p, flavor)
    }
    pub(crate) fn defer_children(&mut self, pending: &mut Vec<Value>) {
        match self {
            Self::Full(s) => s.defer_children(pending),
            Self::Empty { prototype, .. } => {
                if let Some(p) = prototype.take().and_then(Rc::into_inner) {
                    let mut value = p.into_value();
                    crate::value::teardown::defer_if_last(&mut value, pending);
                }
            }
            Self::Sentinel => {}
        }
    }
}
