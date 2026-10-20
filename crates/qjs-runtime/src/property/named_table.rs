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

    /// The one probe an edit makes, counted against the one-probe rule; a
    /// miss proves `key` absent for [`Self::append_absent`].
    pub(crate) fn probe(&self, key: &str) -> Option<usize> {
        crate::diagnostics::count!(property_add_probes);
        self.lookup(key)
    }

    /// The descriptor at a probed slot, borrowed for in-place mutation.
    pub(crate) fn slot_mut(&mut self, slot: usize) -> SlotMut<'_> {
        SlotMut { table: self, slot }
    }

    /// Appends `key`, which a [`Self::probe`] proved absent. A table without
    /// a layout takes `domain` -- the receiver's realm domain -- or, from a
    /// caller with no realm at hand, the shared fallback domain.
    pub(crate) fn append_absent(
        &mut self,
        domain: Option<&AtomDomain>,
        key: &str,
        property: Property,
    ) {
        let layout = self.layout.get_or_insert_with(|| match domain {
            Some(domain) => domain.root(Encoding::Small),
            None => AtomDomain::shared_fallback().root(Encoding::Small),
        });
        let slot = LayoutEdit::append(layout, key, Attributes::of(&property));
        debug_assert_eq!(slot, self.slots.len());
        self.slots.push(property);
    }

    /// Defines `key` with one probe: replaces an existing descriptor in its
    /// slot or appends a new one. Returns the replaced descriptor.
    pub(crate) fn insert_hinted(
        &mut self,
        domain: Option<&AtomDomain>,
        key: impl AsRef<str>,
        property: Property,
    ) -> Option<Property> {
        let key = key.as_ref();
        if let Some(slot) = self.probe(key) {
            let old = std::mem::replace(&mut self.slots[slot], property);
            self.publish(slot);
            return Some(old);
        }
        self.append_absent(domain, key, property);
        None
    }

    /// [`Self::insert_hinted`] with the installer's realm domain.
    pub(crate) fn insert_in(
        &mut self,
        domain: &AtomDomain,
        key: impl AsRef<str>,
        property: Property,
    ) -> Option<Property> {
        self.insert_hinted(Some(domain), key, property)
    }

    /// [`Self::insert_hinted`] for a caller with no realm at hand.
    pub(crate) fn insert(&mut self, key: impl AsRef<str>, property: Property) -> Option<Property> {
        self.insert_hinted(None, key, property)
    }

    /// Inserts absent `key` at enumeration position `index`. A position
    /// before the end rebuilds the table in the new order, which only a
    /// lazily materialized default property needs.
    pub(crate) fn insert_absent_at(
        &mut self,
        domain: Option<&AtomDomain>,
        index: usize,
        key: &str,
        property: Property,
    ) {
        if index >= self.slots.len() {
            self.append_absent(domain, key, property);
            return;
        }
        let own = self.domain().cloned();
        let domain = own.as_ref().or(domain);
        let old = std::mem::take(self);
        let mut entries: Vec<(String, Property)> = old
            .iter()
            .map(|(key, property)| (key.to_owned(), property.clone()))
            .collect();
        entries.insert(index, (key.to_owned(), property));
        for (key, property) in entries {
            self.append_absent(domain, &key, property);
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
