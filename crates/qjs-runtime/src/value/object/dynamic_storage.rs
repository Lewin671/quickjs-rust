//! Property storage for an object past the small-object limit.

use std::rc::Rc;

use crate::Property;

/// Payload for the cold, unbounded property-storage path. Boxed so an
/// object that never grows past the shaped/small paths does not pay for the
/// `HashMap` + `Vec` footprint in every `PropertyStorage` (the enum's size is
/// otherwise governed by its largest variant even when that variant is
/// inactive).
pub(super) struct DynamicPropertyStorage {
    /// The properties, each at a slot that stays put until a property is
    /// removed: a slot-keyed cache validated by the layout revision can
    /// address a large object -- the global object -- without hashing.
    pub(super) entries: Vec<(Rc<str>, Property)>,
    /// Each key's slot in `entries`.
    pub(super) index: crate::value::name_hash::NameMap<Rc<str>, usize>,
    /// Enumeration order; a key inserted unordered has a slot but no place
    /// here.
    pub(super) order: Vec<Rc<str>>,
}

impl DynamicPropertyStorage {
    /// Storage for `entries`, in the given order, all of them enumerable.
    pub(super) fn ordered(entries: Vec<(Rc<str>, Property)>) -> Self {
        let order = entries.iter().map(|(key, _)| key.clone()).collect();
        let index = entries
            .iter()
            .enumerate()
            .map(|(slot, (key, _))| (key.clone(), slot))
            .collect();
        Self {
            entries,
            index,
            order,
        }
    }

    pub(super) fn slot(&self, key: &str) -> Option<usize> {
        self.index.get(key).copied()
    }

    pub(super) fn property(&self, key: &str) -> Option<&Property> {
        let slot = self.slot(key)?;
        self.entries.get(slot).map(|(_, property)| property)
    }

    pub(super) fn property_mut(&mut self, key: &str) -> Option<&mut Property> {
        let slot = self.slot(key)?;
        self.entries.get_mut(slot).map(|(_, property)| property)
    }

    pub(super) fn insert_new(&mut self, key: Rc<str>, property: Property) {
        self.index.insert(key.clone(), self.entries.len());
        self.entries.push((key, property));
    }

    pub(super) fn remove(&mut self, key: &str) -> Option<Property> {
        let slot = self.index.remove(key)?;
        let (_, removed) = self.entries.swap_remove(slot);
        if let Some((moved, _)) = self.entries.get(slot) {
            self.index.insert(moved.clone(), slot);
        }
        self.order.retain(|existing| existing.as_ref() != key);
        Some(removed)
    }
}
