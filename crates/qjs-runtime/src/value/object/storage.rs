//! Payload adapters for the uniform ordinary property layout. All name probes
//! resolve through `layout`; structural edits refresh it under the same borrow.
//! Descriptor/value payloads keep the four existing encodings during M1.2.

use super::{
    DynamicPropertyStorage, ObjectLiteralShape, OwnDataPropertyRead, OwnDataPropertyWrite,
};
use crate::property::{
    atom::AtomDomain,
    layout::{Attributes, Encoding, Field, PropertyLayout},
};
use crate::{Property, Value};
use std::rc::Rc;

pub(super) enum PropertyPayload {
    Small {
        entries: Vec<(Rc<str>, Property)>,
    },
    Dynamic(Box<DynamicPropertyStorage>),
    Shaped {
        shape: Rc<ObjectLiteralShape>,
        properties: Vec<Property>,
    },
    ShapedPair {
        shape: Rc<ObjectLiteralShape>,
        values: [Value; 2],
    },
}
pub(super) struct PropertyStorage {
    pub(super) payload: PropertyPayload,
    pub(super) layout: PropertyLayout,
}
impl PropertyStorage {
    pub(super) const SMALL_LIMIT: usize = 12;
    pub(super) fn new(payload: PropertyPayload, domain: AtomDomain) -> Self {
        let mut storage = Self {
            payload,
            layout: PropertyLayout::new(domain, Encoding::Small),
        };
        let order = (0..storage.len()).collect();
        storage.publish_with_order(order);
        storage
    }
    pub(super) fn empty(capacity: usize) -> Self {
        Self::new(
            PropertyPayload::Small {
                entries: Vec::with_capacity(capacity),
            },
            AtomDomain::new(),
        )
    }
    pub(super) fn relink(&mut self, domain: AtomDomain) {
        let order = self.layout.order().collect();
        self.layout = PropertyLayout::new(domain, self.encoding());
        self.publish_with_order(order);
    }
    pub(super) fn dynamic(
        mut properties: crate::value::name_hash::NameMap<Rc<str>, Property>,
        order: Vec<Rc<str>>,
        domain: AtomDomain,
    ) -> Self {
        let mut inputs: Vec<_> = order
            .iter()
            .filter_map(|key| {
                properties
                    .remove(key)
                    .map(|property| (key.clone(), property))
            })
            .collect();
        inputs.extend(properties);
        // Host maps can contain both stored encodings of one code-unit key.
        // Normalize before the table becomes observable, keeping first order
        // and last descriptor just like sequential definitions.
        let mut index: crate::value::name_hash::NameMap<Rc<str>, usize> = Default::default();
        let mut entries: Vec<(Rc<str>, Property)> = Vec::with_capacity(inputs.len());
        for (key, property) in inputs {
            let canonical = crate::property::atom::canonical_property_text(&key);
            let key: Rc<str> = match canonical {
                std::borrow::Cow::Borrowed(_) => key,
                std::borrow::Cow::Owned(text) => text.into(),
            };
            if let Some(&slot) = index.get(key.as_ref()) {
                entries[slot].1 = property;
            } else {
                index.insert(key.clone(), entries.len());
                entries.push((key, property));
            }
        }
        let payload = if entries.len() <= Self::SMALL_LIMIT {
            PropertyPayload::Small { entries }
        } else {
            PropertyPayload::Dynamic(Box::new(DynamicPropertyStorage::ordered(entries)))
        };
        Self::new(payload, domain)
    }
    pub(super) fn entries(&self) -> Option<&[(Rc<str>, Property)]> {
        match &self.payload {
            PropertyPayload::Small { entries } => Some(entries),
            PropertyPayload::Dynamic(dynamic) => Some(&dynamic.entries),
            _ => None,
        }
    }
    fn encoding(&self) -> Encoding {
        match &self.payload {
            PropertyPayload::Small { .. } => Encoding::Small,
            PropertyPayload::Dynamic(_) => Encoding::Dynamic,
            PropertyPayload::Shaped { .. } => Encoding::Shaped,
            PropertyPayload::ShapedPair { .. } => Encoding::Pair,
        }
    }
    pub(super) fn publish(&mut self) {
        self.publish_with_order(self.layout.order().collect());
    }
    fn publish_with_order(&mut self, order: Vec<usize>) {
        let domain = self.layout.domain();
        let fields: Vec<Field> = match &self.payload {
            PropertyPayload::Small { entries } => entries
                .iter()
                .map(|(key, property)| Field {
                    key: domain.intern_key(key),
                    attributes: Attributes::of(property),
                })
                .collect(),
            PropertyPayload::Dynamic(dynamic) => dynamic
                .entries
                .iter()
                .map(|(key, property)| Field {
                    key: domain.intern_key(key),
                    attributes: Attributes::of(property),
                })
                .collect(),
            PropertyPayload::Shaped { shape, properties } => shape
                .keys
                .iter()
                .zip(properties)
                .map(|(key, property)| Field {
                    key: domain.intern_key(key),
                    attributes: Attributes::of(property),
                })
                .collect(),
            PropertyPayload::ShapedPair { shape, .. } => shape
                .keys
                .iter()
                .map(|key| Field {
                    key: domain.intern_key(key),
                    attributes: Attributes::DEFAULT,
                })
                .collect(),
        };
        self.layout.publish(self.encoding(), fields, order);
    }
    pub(super) fn len(&self) -> usize {
        match &self.payload {
            PropertyPayload::Small { entries } => entries.len(),
            PropertyPayload::Dynamic(dynamic) => dynamic.entries.len(),
            PropertyPayload::Shaped { properties, .. } => properties.len(),
            PropertyPayload::ShapedPair { .. } => 2,
        }
    }
    pub(super) fn descriptor(&self, slot: usize) -> Option<&Property> {
        match &self.payload {
            PropertyPayload::Small { entries } => entries.get(slot).map(|(_, property)| property),
            PropertyPayload::Dynamic(dynamic) => {
                dynamic.entries.get(slot).map(|(_, property)| property)
            }
            PropertyPayload::Shaped { properties, .. } => properties.get(slot),
            PropertyPayload::ShapedPair { .. } => None,
        }
    }
    fn descriptor_mut(&mut self, slot: usize) -> Option<&mut Property> {
        match &mut self.payload {
            PropertyPayload::Small { entries } => {
                entries.get_mut(slot).map(|(_, property)| property)
            }
            PropertyPayload::Dynamic(dynamic) => {
                dynamic.entries.get_mut(slot).map(|(_, property)| property)
            }
            PropertyPayload::Shaped { properties, .. } => properties.get_mut(slot),
            PropertyPayload::ShapedPair { .. } => None,
        }
    }
    pub(super) fn get(&self, key: &str) -> Option<Property> {
        let slot = self.layout.lookup(key)?.slot;
        self.get_slot(slot)
    }
    pub(super) fn get_slot(&self, slot: usize) -> Option<Property> {
        if let PropertyPayload::ShapedPair { values, .. } = &self.payload {
            Some(Property::enumerable(values.get(slot)?.clone()))
        } else {
            self.descriptor(slot).cloned()
        }
    }
    pub(super) fn enumerable(&self, key: &str) -> Option<bool> {
        Some(self.layout.lookup(key)?.attributes.enumerable)
    }
    pub(super) fn value(&self, key: &str) -> Option<Value> {
        let slot = self.layout.lookup(key)?.slot;
        match &self.payload {
            PropertyPayload::ShapedPair { values, .. } => values.get(slot).cloned(),
            _ => self.descriptor(slot).map(|property| property.value.clone()),
        }
    }
    pub(super) fn get_mut(&mut self, key: &str) -> Option<&mut Property> {
        let slot = self.layout.lookup(key)?.slot;
        if matches!(&self.payload, PropertyPayload::ShapedPair { .. }) {
            self.ensure_dynamic();
        }
        self.descriptor_mut(slot)
    }
    pub(super) fn contains_key(&self, key: &str) -> bool {
        self.layout.lookup(key).is_some()
    }
    pub(super) fn own_data_read(&self, key: &str) -> OwnDataPropertyRead {
        let Some(entry) = self.layout.lookup(key) else {
            return OwnDataPropertyRead::Missing;
        };
        self.read_entry(entry)
    }
    pub(super) fn read_entry(&self, entry: crate::property::layout::Entry) -> OwnDataPropertyRead {
        if entry.attributes.accessor {
            return OwnDataPropertyRead::NeedsSlowPath;
        }
        let value = match &self.payload {
            PropertyPayload::ShapedPair { values, .. } => values[entry.slot].clone(),
            _ => self
                .descriptor(entry.slot)
                .expect("layout slot has payload")
                .value
                .clone(),
        };
        OwnDataPropertyRead::Data(value)
    }
    pub(super) fn write_existing_data(
        &mut self,
        key: &str,
        value: &Value,
        guard: impl FnOnce(&Value) -> bool,
    ) -> OwnDataPropertyWrite {
        let Some(entry) = self.layout.lookup(key) else {
            return OwnDataPropertyWrite::NeedsSlowPath;
        };
        if entry.attributes.accessor {
            return OwnDataPropertyWrite::NeedsSlowPath;
        }
        if let PropertyPayload::ShapedPair { values, .. } = &mut self.payload {
            if !guard(&values[entry.slot]) {
                return OwnDataPropertyWrite::NeedsSlowPath;
            }
            values[entry.slot] = value.clone();
            OwnDataPropertyWrite::Written
        } else {
            super::write_existing_property(self.descriptor_mut(entry.slot), value, guard)
        }
    }
    pub(super) fn for_each_mut(&mut self, mut apply: impl FnMut(&mut Property)) {
        if matches!(&self.payload, PropertyPayload::ShapedPair { .. }) {
            self.ensure_dynamic();
        }
        match &mut self.payload {
            PropertyPayload::Small { entries } => entries.iter_mut().for_each(|(_, p)| apply(p)),
            PropertyPayload::Dynamic(dynamic) => {
                dynamic.entries.iter_mut().for_each(|(_, p)| apply(p))
            }
            PropertyPayload::Shaped { properties, .. } => properties.iter_mut().for_each(apply),
            PropertyPayload::ShapedPair { .. } => unreachable!(),
        }
        self.publish();
    }
    pub(super) fn all(&self, predicate: impl Fn(&Property) -> bool) -> bool {
        (0..self.len()).all(|slot| match &self.payload {
            PropertyPayload::ShapedPair { values, .. } => {
                predicate(&Property::enumerable(values[slot].clone()))
            }
            _ => predicate(self.descriptor(slot).expect("layout slot")),
        })
    }
    fn ensure_dynamic(&mut self) {
        let entries = match &mut self.payload {
            PropertyPayload::Dynamic(_) => return,
            PropertyPayload::Small { entries } => std::mem::take(entries),
            PropertyPayload::Shaped { shape, properties } => shape
                .keys
                .iter()
                .cloned()
                .zip(std::mem::take(properties))
                .collect(),
            PropertyPayload::ShapedPair { shape, values } => shape
                .keys
                .iter()
                .cloned()
                .zip(values.iter().cloned().map(Property::enumerable))
                .collect(),
        };
        self.payload = PropertyPayload::Dynamic(Box::new(DynamicPropertyStorage::ordered(entries)));
        self.layout.change_encoding(Encoding::Dynamic);
    }
    pub(super) fn insert(&mut self, key: Rc<str>, property: Property) -> Option<Property> {
        if let Some(entry) = self.layout.lookup(&key) {
            let attributes = Attributes::of(&property);
            if matches!(&self.payload, PropertyPayload::ShapedPair { .. }) {
                self.ensure_dynamic();
            }
            let previous = std::mem::replace(
                self.descriptor_mut(entry.slot).expect("layout slot"),
                property,
            );
            self.layout.reconfigure(entry.slot, attributes);
            return Some(previous);
        }
        self.insert_new(key, property, true);
        None
    }
    fn insert_new(&mut self, key: Rc<str>, property: Property, ordered: bool) {
        let key = match crate::property::atom::canonical_property_text(&key) {
            std::borrow::Cow::Borrowed(_) => key,
            std::borrow::Cow::Owned(canonical) => canonical.into(),
        };
        let attributes = Attributes::of(&property);
        if ordered
            && let PropertyPayload::Small { entries } = &mut self.payload
            && entries.len() < Self::SMALL_LIMIT
        {
            entries.push((key.clone(), property));
            self.layout.append(&key, attributes, Encoding::Small, true);
            return;
        }
        self.ensure_dynamic();
        let PropertyPayload::Dynamic(dynamic) = &mut self.payload else {
            unreachable!()
        };
        dynamic.entries.push((key.clone(), property));
        self.layout
            .append(&key, attributes, Encoding::Dynamic, ordered);
    }
    pub(super) fn insert_absent(&mut self, key: Rc<str>, property: Property) {
        // Conversion/interner boundaries can make differently encoded texts
        // alias an existing key; never trust a byte-spelling absence proof.
        self.insert(key, property);
    }
    pub(super) fn insert_unordered(
        &mut self,
        key: Rc<str>,
        property: Property,
    ) -> Option<Property> {
        if self.contains_key(&key) {
            return self.insert(key, property);
        }
        self.insert_new(key, property, false);
        None
    }
    pub(super) fn remove(&mut self, key: &str) -> Option<Property> {
        let slot = self.layout.lookup(key)?.slot;
        let small = matches!(self.payload, PropertyPayload::Small { .. });
        let removed = if let PropertyPayload::Small { entries } = &mut self.payload {
            entries.remove(slot).1
        } else {
            self.ensure_dynamic();
            let PropertyPayload::Dynamic(dynamic) = &mut self.payload else {
                unreachable!()
            };
            dynamic.entries.swap_remove(slot).1
        };
        self.layout.remove(slot, !small);
        Some(removed)
    }
}
