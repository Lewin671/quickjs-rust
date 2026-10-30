//! Immutable literal input-slot metadata and bounded per-domain layout links.
//! Runtime objects contain only NamedStore; this metadata is never consulted
//! by property lookup. Linked layouts own leases, preventing recycled-ID ABA.
use crate::property::atom::{AtomDomain, WeakAtomDomain};
use crate::value::{
    layout::{Layout, NamedStore},
    name_hash::NameMap,
};
use crate::{Property, Prototype, Value};
use std::{cell::RefCell, rc::Rc};

#[derive(Debug)]
pub(crate) struct ObjectLiteralShape {
    pub(super) keys: Rc<[Rc<str>]>,
    pub(super) input_slots: Rc<[usize]>,
    pub(super) index_property_count: usize,
    links: RefCell<Vec<LiteralLink>>,
}
struct LiteralLink {
    domain: WeakAtomDomain,
    layout: Rc<Layout>,
}
impl std::fmt::Debug for LiteralLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiteralLink").finish_non_exhaustive()
    }
}
impl ObjectLiteralShape {
    pub(crate) fn new(input_keys: Vec<Rc<str>>) -> Rc<Self> {
        let mut keys = Vec::new();
        let mut slots = NameMap::default();
        let mut input_slots = Vec::with_capacity(input_keys.len());
        for key in input_keys {
            let key: Rc<str> =
                Rc::from(crate::property::atom::canonical_property_text(&key).as_ref());
            let slot = *slots.entry(key.clone()).or_insert_with(|| {
                let slot = keys.len();
                keys.push(key);
                slot
            });
            input_slots.push(slot);
        }
        let index_property_count = keys
            .iter()
            .filter(|key| super::is_array_index_key(key))
            .count();
        Rc::new(Self {
            keys: keys.into(),
            input_slots: input_slots.into(),
            index_property_count,
            links: RefCell::new(Vec::new()),
        })
    }
    pub(crate) fn input_len(&self) -> usize {
        self.input_slots.len()
    }
    pub(crate) fn unique_len(&self) -> usize {
        self.keys.len()
    }
    pub(crate) fn final_input_index(&self, key: &str) -> Option<usize> {
        let slot = self
            .keys
            .iter()
            .position(|candidate| candidate.as_ref() == key)?;
        self.input_slots
            .iter()
            .rposition(|input_slot| *input_slot == slot)
    }
    pub(super) fn linked_layout(
        &self,
        domain: &AtomDomain,
        prototype: Option<Prototype>,
    ) -> Rc<Layout> {
        let mut links = self.links.borrow_mut();
        if let Some(link) = links.iter().find(|link| {
            link.domain.is(domain) && link.layout.matches_prototype(prototype.as_ref(), 0)
        }) {
            return link.layout.clone();
        }
        let mut store = NamedStore::empty_with_prototype(domain, prototype, 0);
        for key in self.keys.iter() {
            store
                .define(&domain.intern(key), Property::enumerable(Value::Undefined))
                .expect("own domain");
        }
        let layout = store.layout().clone();
        if links.len() == 4 {
            links.remove(0);
        }
        links.push(LiteralLink {
            domain: domain.downgrade(),
            layout: layout.clone(),
        });
        layout
    }
}
