//! Slot-addressed reads of an object's compact property storage.
//!
//! The bytecode's named-property read caches resolve a property once and then
//! re-read it by slot. Each entry pairs the slot with whatever it needs to
//! prove the slot still means the same property: a layout revision for one
//! object, a shared literal shape, or the interned name the storage holds.
//! Keeping those reads together separates "where a property lives" from the
//! observable object semantics in the parent module.

use std::rc::Rc;

use super::{ObjectLiteralShape, ObjectRef, OwnDataPropertyRead, PropertyStorage};
use crate::Value;

impl ObjectRef {
    pub(crate) fn own_data_property_read(&self, key: &str) -> OwnDataPropertyRead {
        if self.0.module_namespace_exotic.get() {
            return OwnDataPropertyRead::NeedsSlowPath;
        }
        self.0.properties.borrow().own_data_read(key)
    }

    /// Resolves `key` to a stable own-property slot in this object's compact
    /// storage. Callers pair the slot with [`Self::layout_revision`]: the pair
    /// stays valid across ordinary value assignment, so a monomorphic read
    /// cache keeps hitting while a field is written every iteration.
    pub(crate) fn own_data_slot(&self, key: &str) -> Option<usize> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        match &*self.0.properties.borrow() {
            PropertyStorage::Small { entries } => {
                entries.iter().position(|(candidate, property)| {
                    candidate.as_ref() == key && !property.is_accessor()
                })
            }
            PropertyStorage::Dynamic(_)
            | PropertyStorage::Shaped { .. }
            | PropertyStorage::ShapedPair { .. } => None,
        }
    }

    /// Resolves `key` to a slot together with the interned name the storage
    /// holds there.
    ///
    /// Objects built by the same code site share one interned name per
    /// property, so a read cache can revalidate a *different* object against
    /// the recorded name by pointer alone — no object identity and no name
    /// comparison. That is what lets one cache entry serve a loop over many
    /// objects of the same construction, which object identity keying cannot.
    pub(crate) fn shared_data_slot(&self, key: &str) -> Option<(Rc<str>, usize)> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        match &*self.0.properties.borrow() {
            PropertyStorage::Small { entries } => {
                entries
                    .iter()
                    .enumerate()
                    .find_map(|(slot, (name, property))| {
                        (name.as_ref() == key && !property.is_accessor())
                            .then(|| (Rc::clone(name), slot))
                    })
            }
            PropertyStorage::Dynamic(_)
            | PropertyStorage::Shaped { .. }
            | PropertyStorage::ShapedPair { .. } => None,
        }
    }

    /// Reads a slot recorded by [`Self::shared_data_slot`], confirming that
    /// this object holds the same interned name in that slot.
    pub(crate) fn shared_data_slot_value(&self, key: &Rc<str>, slot: usize) -> Option<Value> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        match &*self.0.properties.borrow() {
            PropertyStorage::Small { entries } => {
                let (name, property) = entries.get(slot)?;
                (Rc::ptr_eq(name, key) && !property.is_accessor()).then(|| property.value.clone())
            }
            PropertyStorage::Dynamic(_)
            | PropertyStorage::Shaped { .. }
            | PropertyStorage::ShapedPair { .. } => None,
        }
    }

    /// Reads a slot previously resolved by [`Self::own_data_slot`]. The storage
    /// kind is re-checked so a layout the revision counter cannot describe
    /// simply misses the cache instead of reading the wrong property.
    pub(crate) fn own_data_slot_value(&self, slot: usize) -> Option<Value> {
        match &*self.0.properties.borrow() {
            PropertyStorage::Small { entries } => {
                let (_, property) = entries.get(slot)?;
                (!property.is_accessor()).then(|| property.value.clone())
            }
            PropertyStorage::Dynamic(_)
            | PropertyStorage::Shaped { .. }
            | PropertyStorage::ShapedPair { .. } => None,
        }
    }

    /// Returns the shared literal shape and storage slot for an unmodified
    /// data-only object literal. Named-property caches use this to share one
    /// cache entry across distinct objects created by the same bytecode site.
    pub(crate) fn literal_data_slot(&self, key: &str) -> Option<(Rc<ObjectLiteralShape>, usize)> {
        if self.0.module_namespace_exotic.get() || self.property_revision() != 0 {
            return None;
        }
        let properties = self.0.properties.borrow();
        let shape = match &*properties {
            PropertyStorage::Shaped { shape, .. } | PropertyStorage::ShapedPair { shape, .. } => {
                shape
            }
            PropertyStorage::Small { .. } | PropertyStorage::Dynamic(_) => return None,
        };
        let slot = *shape.lookup.get(key)?;
        Some((shape.clone(), slot))
    }

    /// Reads a previously resolved literal slot after checking that this
    /// object still has the same unmodified shared shape.
    pub(crate) fn literal_data_slot_value(
        &self,
        expected_shape: &Rc<ObjectLiteralShape>,
        slot: usize,
    ) -> Option<Value> {
        if self.0.module_namespace_exotic.get() || self.property_revision() != 0 {
            return None;
        }
        match &*self.0.properties.borrow() {
            PropertyStorage::Shaped { shape, properties } if Rc::ptr_eq(shape, expected_shape) => {
                properties.get(slot).map(|property| property.value.clone())
            }
            PropertyStorage::ShapedPair { shape, values } if Rc::ptr_eq(shape, expected_shape) => {
                values.get(slot).cloned()
            }
            _ => None,
        }
    }
}
