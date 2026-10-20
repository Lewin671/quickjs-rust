//! An object's string-keyed property store: one layout pointer, which owns
//! every key, its slot, attributes and order, and a payload of value slots.
//!
//! The payload holds no keys. `Slots` holds one descriptor per slot for the
//! `Small`, `Dynamic` and `Shaped` encodings; `Pair` holds the two values of
//! a two-key literal, whose attributes are implied. Every structural edit
//! goes through [`LayoutEdit`] and mirrors its slot change in the payload
//! under the same `&mut` borrow, so no reader can see the two disagree: for
//! every slot, the layout field's descriptor attributes equal those of the
//! payload descriptor, and the store's encoding copy equals the layout's.
//! Slot readers and writers rely on both, so they read no layout node.

use std::rc::Rc;

use crate::property::atom::AtomDomain;
use crate::property::layout::{
    Attributes, BulkEdit, Encoding, Field, Layout, LayoutEdit, SlotRemoval, layout_for_keys,
};
use crate::{Property, Value};

pub(super) enum Payload {
    Slots(Vec<Property>),
    Pair([Value; 2]),
}

pub(super) struct PropertyStorage {
    pub(super) layout: Layout,
    pub(super) payload: Payload,
    /// The layout's encoding, kept beside the payload so a slot access that
    /// admits by encoding reads no layout node; resynced after every edit.
    encoding: Encoding,
    /// Non-append edits; the store becomes a dictionary at a fixed count.
    edits: u8,
}

/// A read-only view of one slot's descriptor.
pub(super) enum SlotRef<'a> {
    Descriptor(&'a Property),
    PairValue(&'a Value),
}

impl SlotRef<'_> {
    pub(super) fn value(&self) -> &Value {
        match self {
            Self::Descriptor(property) => &property.value,
            Self::PairValue(value) => value,
        }
    }

    pub(super) fn is_accessor(&self) -> bool {
        matches!(self, Self::Descriptor(property) if property.is_accessor())
    }

    pub(super) fn to_property(&self) -> Property {
        match self {
            Self::Descriptor(property) => (*property).clone(),
            Self::PairValue(value) => Property::enumerable((*value).clone()),
        }
    }
}

impl PropertyStorage {
    /// An empty store of `domain`.
    pub(super) fn empty(domain: &AtomDomain, capacity: usize) -> Self {
        Self {
            layout: domain.root(Encoding::Small),
            payload: Payload::Slots(Vec::with_capacity(capacity)),
            encoding: Encoding::Small,
            edits: 0,
        }
    }

    /// A store over ordered, distinct keys, built through the domain's edges.
    pub(super) fn from_ordered(domain: &AtomDomain, entries: Vec<(Rc<str>, Property)>) -> Self {
        let encoding = if entries.len() > crate::property::layout::LINEAR_LOOKUP_LIMIT {
            Encoding::Dynamic
        } else {
            Encoding::Small
        };
        let layout = layout_for_keys(
            domain,
            encoding,
            entries
                .iter()
                .map(|(key, property)| (key.as_ref(), Attributes::of(property))),
        );
        let payload = Payload::Slots(entries.into_iter().map(|(_, property)| property).collect());
        Self {
            encoding: layout.encoding(),
            layout,
            payload,
            edits: 0,
        }
    }

    /// A literal store over a published literal layout.
    pub(super) fn literal(layout: Layout, payload: Payload) -> Self {
        debug_assert_eq!(
            matches!(payload, Payload::Pair(_)),
            layout.encoding() == Encoding::Pair
        );
        Self {
            encoding: layout.encoding(),
            layout,
            payload,
            edits: 0,
        }
    }

    /// The layout's encoding, without reading the layout.
    #[inline(always)]
    pub(super) fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// The number of slots, read from the payload.
    #[inline(always)]
    pub(super) fn slot_count(&self) -> usize {
        match &self.payload {
            Payload::Slots(properties) => properties.len(),
            Payload::Pair(values) => values.len(),
        }
    }

    #[inline(always)]
    fn sync_encoding(&mut self) {
        self.encoding = self.layout.encoding();
    }

    #[inline(always)]
    pub(super) fn len(&self) -> usize {
        self.layout.len()
    }

    #[inline(always)]
    pub(super) fn domain(&self) -> &AtomDomain {
        self.layout.domain()
    }

    #[inline(always)]
    pub(super) fn lookup(&self, key: &str) -> Option<usize> {
        self.layout.lookup(key)
    }

    /// One probe made by a mutation, counted against the one-probe rule.
    #[inline(always)]
    pub(super) fn probe_for_edit(&self, key: &str) -> Option<usize> {
        crate::diagnostics::count!(property_add_probes);
        self.layout.lookup(key)
    }

    #[inline(always)]
    pub(super) fn slot(&self, slot: usize) -> Option<SlotRef<'_>> {
        match &self.payload {
            Payload::Slots(properties) => properties.get(slot).map(SlotRef::Descriptor),
            Payload::Pair(values) => values.get(slot).map(SlotRef::PairValue),
        }
    }

    #[inline(always)]
    pub(super) fn field(&self, slot: usize) -> Option<&Field> {
        self.layout.field(slot)
    }

    /// The value of a slot, writable in place. Attributes are unaffected.
    #[inline(always)]
    pub(super) fn value_mut(&mut self, slot: usize) -> Option<&mut Value> {
        match &mut self.payload {
            Payload::Slots(properties) => {
                properties.get_mut(slot).map(|property| &mut property.value)
            }
            Payload::Pair(values) => values.get_mut(slot),
        }
    }

    pub(super) fn get(&self, key: &str) -> Option<Property> {
        self.slot(self.lookup(key)?).map(|slot| slot.to_property())
    }

    pub(super) fn value(&self, key: &str) -> Option<Value> {
        self.slot(self.lookup(key)?)
            .map(|slot| slot.value().clone())
    }

    pub(super) fn enumerable(&self, key: &str) -> Option<bool> {
        let slot = self.lookup(key)?;
        self.field(slot)
            .map(|field| field.attributes.is_enumerable())
    }

    pub(super) fn contains_key(&self, key: &str) -> bool {
        self.lookup(key).is_some()
    }

    /// Converts a `Pair` payload to descriptors, keeping slots.
    fn ensure_descriptors(&mut self) {
        if let Payload::Pair(values) = &mut self.payload {
            let values = std::mem::replace(values, [Value::Undefined, Value::Undefined]);
            self.payload = Payload::Slots(values.into_iter().map(Property::enumerable).collect());
            LayoutEdit::recode(&mut self.layout, Encoding::Dynamic);
            self.sync_encoding();
        }
    }

    /// Replaces the descriptor at `slot`, returning the old one.
    pub(super) fn replace(&mut self, slot: usize, property: Property) -> Property {
        let attributes = self.layout.fields()[slot]
            .attributes
            .with_descriptor_of(&property);
        if let Payload::Pair(values) = &mut self.payload
            && attributes == Attributes::DEFAULT
        {
            let old = std::mem::replace(&mut values[slot], property.value);
            return Property::enumerable(old);
        }
        self.ensure_descriptors();
        LayoutEdit::reconfigure(&mut self.layout, &mut self.edits, slot, attributes);
        self.sync_encoding();
        let Payload::Slots(properties) = &mut self.payload else {
            unreachable!("descriptor payload ensured")
        };
        std::mem::replace(&mut properties[slot], property)
    }

    /// Reserves payload room for `additional` appends.
    pub(super) fn reserve(&mut self, additional: usize) {
        if let Payload::Slots(properties) = &mut self.payload {
            properties.reserve(additional);
        }
    }

    /// Appends a key the caller proved absent; returns its slot.
    pub(super) fn append(&mut self, key: &str, property: Property, ordered: bool) -> usize {
        let mut attributes = Attributes::of(&property);
        if !ordered {
            attributes = attributes.unordered();
        }
        self.ensure_descriptors();
        let slot = LayoutEdit::append(&mut self.layout, key, attributes);
        self.sync_encoding();
        let Payload::Slots(properties) = &mut self.payload else {
            unreachable!("descriptor payload ensured")
        };
        debug_assert_eq!(slot, properties.len());
        properties.push(property);
        slot
    }

    /// Defines `key`: replaces an existing descriptor in its slot or appends
    /// a new one. One probe either way.
    #[inline]
    pub(super) fn insert(&mut self, key: &str, property: Property) -> Option<Property> {
        // An append edge from this shape proves `key` absent: its probe.
        if let Payload::Slots(properties) = &mut self.payload
            && let Some(slot) =
                LayoutEdit::append_by_edge(&mut self.layout, key, Attributes::of(&property))
        {
            crate::diagnostics::count!(property_add_probes);
            debug_assert_eq!(slot, properties.len());
            properties.push(property);
            // Append edges preserve their parent's encoding. Re-encoding uses
            // a separate edge, so this hit cannot change the payload kind.
            debug_assert_eq!(self.encoding, self.layout.encoding());
            return None;
        }
        self.insert_without_edge(key, property)
    }

    /// Replacements and unseen transitions stay out of the append-edge hit
    /// path so callers can inline one small probe and payload push.
    #[inline(never)]
    fn insert_without_edge(&mut self, key: &str, property: Property) -> Option<Property> {
        match self.probe_for_edit(key) {
            Some(slot) => Some(self.replace(slot, property)),
            None => {
                self.append(key, property, true);
                None
            }
        }
    }

    pub(super) fn remove_slot(&mut self, slot: usize) -> Property {
        self.ensure_descriptors();
        let removal = LayoutEdit::remove(&mut self.layout, &mut self.edits, slot);
        self.sync_encoding();
        let Payload::Slots(properties) = &mut self.payload else {
            unreachable!("descriptor payload ensured")
        };
        match removal {
            SlotRemoval::Shift => properties.remove(slot),
            SlotRemoval::SwapLast => properties.swap_remove(slot),
        }
    }

    /// Applies a bulk attribute edit to every slot in one layout transition.
    pub(super) fn bulk(&mut self, edit: BulkEdit) {
        self.ensure_descriptors();
        LayoutEdit::bulk(&mut self.layout, &mut self.edits, edit);
        self.sync_encoding();
        let Payload::Slots(properties) = &mut self.payload else {
            unreachable!("descriptor payload ensured")
        };
        for property in properties {
            match edit {
                BulkEdit::Seal => property.make_non_configurable(),
                BulkEdit::Freeze => property.freeze_data(),
            }
        }
    }

    /// Whether every slot's attributes satisfy `predicate`.
    pub(super) fn all_attributes(&self, predicate: impl Fn(Attributes) -> bool) -> bool {
        self.layout
            .fields()
            .iter()
            .all(|field| predicate(field.attributes))
    }

    /// Ordered keys with their slot: enumeration order, internal
    /// (unordered) keys skipped.
    pub(super) fn ordered_fields(&self) -> impl Iterator<Item = (usize, &Field)> {
        let fields = self.layout.fields();
        self.layout
            .ordered_slots()
            .map(move |slot| (slot, &fields[slot]))
            .filter(|(_, field)| field.attributes.is_ordered())
    }
}

#[cfg(test)]
impl PropertyStorage {
    pub(super) fn capacity(&self) -> usize {
        match &self.payload {
            Payload::Slots(properties) => properties.capacity(),
            Payload::Pair(_) => 2,
        }
    }
}
