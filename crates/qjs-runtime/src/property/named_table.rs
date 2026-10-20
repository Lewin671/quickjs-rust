//! The ordinary named part of a function or array: string-keyed descriptors
//! behind the same layout authority as an ordinary object's store.
//!
//! The receiver keeps its own semantic hooks (`length`, `name`, `prototype`,
//! indices) and its own key-order bookkeeping; this table holds only the
//! installed named descriptors. It has no layout until its first install,
//! which takes the domain the installing code supplies -- the receiver's
//! realm -- so creating a function or array creates no layout and no domain.
//!
//! Invariant: for every slot, the layout field's descriptor attributes equal
//! those of the slot's descriptor. Every mutation that can change attributes
//! goes through [`NamedTable::get_mut`]'s guard or a bulk edit, which
//! republishes the layout before the borrow ends.

use std::ops::{Deref, DerefMut};

use super::atom::AtomDomain;
use super::layout::{
    Attributes, BulkEdit, Encoding, Layout, LayoutEdit, OrderedSlots, SlotRemoval,
};
use crate::Property;

#[derive(Default)]
pub(crate) struct NamedTable {
    layout: Option<Layout>,
    slots: Vec<Property>,
    edits: u8,
}

/// A descriptor borrowed for in-place mutation; attribute changes are
/// published to the layout when the guard drops.
pub(crate) struct SlotMut<'a> {
    table: &'a mut NamedTable,
    slot: usize,
}

impl Deref for SlotMut<'_> {
    type Target = Property;

    fn deref(&self) -> &Property {
        &self.table.slots[self.slot]
    }
}

impl DerefMut for SlotMut<'_> {
    fn deref_mut(&mut self) -> &mut Property {
        &mut self.table.slots[self.slot]
    }
}

impl Drop for SlotMut<'_> {
    fn drop(&mut self) {
        self.table.publish(self.slot);
    }
}

impl NamedTable {
    fn lookup(&self, key: &str) -> Option<usize> {
        self.layout.as_ref()?.lookup(key)
    }

    /// Republishes `slot`'s attributes after an in-place descriptor change.
    fn publish(&mut self, slot: usize) {
        let Some(layout) = self.layout.as_mut() else {
            return;
        };
        let current = layout.fields()[slot].attributes;
        let attributes = current.with_descriptor_of(&self.slots[slot]);
        if attributes != current {
            LayoutEdit::reconfigure(layout, &mut self.edits, slot, attributes);
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.slots.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// The domain fixed by the first install.
    pub(crate) fn domain(&self) -> Option<&AtomDomain> {
        self.layout.as_ref().map(|layout| layout.domain())
    }

    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.lookup(key).is_some()
    }

    pub(crate) fn get(&self, key: &str) -> Option<&Property> {
        self.slots.get(self.lookup(key)?)
    }

    pub(crate) fn get_mut(&mut self, key: &str) -> Option<SlotMut<'_>> {
        let slot = self.lookup(key)?;
        Some(SlotMut { table: self, slot })
    }

    /// Defines `key` in a table whose domain is already fixed or is fixed
    /// now from `domain`. Returns the replaced descriptor.
    pub(crate) fn insert_in(
        &mut self,
        domain: &AtomDomain,
        key: impl AsRef<str>,
        property: Property,
    ) -> Option<Property> {
        let key = key.as_ref();
        if let Some(slot) = self.lookup(key) {
            let old = std::mem::replace(&mut self.slots[slot], property);
            self.publish(slot);
            return Some(old);
        }
        let layout = self
            .layout
            .get_or_insert_with(|| domain.root(Encoding::Small));
        let slot = LayoutEdit::append(layout, key, Attributes::of(&property));
        debug_assert_eq!(slot, self.slots.len());
        self.slots.push(property);
        None
    }

    /// [`Self::insert_in`] with the receiver's realm domain when the caller
    /// knows it, falling back to [`Self::insert`].
    pub(crate) fn insert_hinted(
        &mut self,
        domain: Option<&AtomDomain>,
        key: impl AsRef<str>,
        property: Property,
    ) -> Option<Property> {
        match (self.layout.is_some(), domain) {
            (false, Some(domain)) => self.insert_in(domain, key, property),
            _ => self.insert(key, property),
        }
    }

    /// [`Self::insert_in`] for a caller with no realm at hand: the table's
    /// own domain once it has one.
    pub(crate) fn insert(&mut self, key: impl AsRef<str>, property: Property) -> Option<Property> {
        match self.layout.as_ref().map(|layout| layout.domain().clone()) {
            Some(domain) => self.insert_in(&domain, key, property),
            None => self.insert_in(&unshared_domain(), key, property),
        }
    }

    pub(crate) fn remove(&mut self, key: &str) -> Option<Property> {
        let slot = self.lookup(key)?;
        let layout = self.layout.as_mut()?;
        Some(match LayoutEdit::remove(layout, &mut self.edits, slot) {
            SlotRemoval::Shift => self.slots.remove(slot),
            SlotRemoval::SwapLast => self.slots.swap_remove(slot),
        })
    }

    /// Applies a bulk attribute edit to every descriptor.
    pub(crate) fn bulk(&mut self, edit: BulkEdit) {
        for property in &mut self.slots {
            match edit {
                BulkEdit::Seal => property.make_non_configurable(),
                BulkEdit::Freeze => property.freeze_data(),
            }
        }
        if let Some(layout) = self.layout.as_mut() {
            LayoutEdit::bulk(layout, &mut self.edits, edit);
        }
    }

    /// Keys and descriptors in layout order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &Property)> {
        let (fields, order) = match &self.layout {
            Some(layout) => (layout.fields(), layout.ordered_slots()),
            None => (&[][..], OrderedSlots::Shape(0..0)),
        };
        order.map(move |slot| (fields[slot].text().as_ref(), &self.slots[slot]))
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &str> {
        self.iter().map(|(key, _)| key)
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &Property> {
        self.slots.iter()
    }
}

/// A domain for a table first installed by code with no realm at hand.
/// Counted with every other domain, so the obligation tests show which
/// install paths still reach it.
#[cold]
fn unshared_domain() -> AtomDomain {
    AtomDomain::new()
}
