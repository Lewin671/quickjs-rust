//! Slot-addressed reads of an object's compact property storage.
//!
//! The bytecode's named-property read caches resolve a property once and then
//! re-read it by slot. Each entry pairs the slot with whatever it needs to
//! prove the slot still means the same property: a layout revision for one
//! object, a shared literal shape, or the interned name the storage holds.
//! Keeping those reads together separates "where a property lives" from the
//! observable object semantics in the parent module.

use std::rc::Rc;

use super::{
    ObjectLiteralShape, ObjectRef, OwnDataPropertyRead, OwnDataPropertyWrite, PropertyStorage,
};
use crate::{Property, Value};

/// The insertion-ordered entries of a small or dynamic storage, where a slot
/// is an index and each entry carries its interned name -- what a shared
/// slot is validated against. A constructor that adds its twelfth property
/// moves its objects to dynamic storage (3d-raytrace's triangles); reading
/// only small storage by shared slot sent every such read to a per-object
/// cache entry, which a dozen triangles thrashed.
#[inline(always)]
fn named_entries(storage: &PropertyStorage) -> Option<&[(Rc<str>, Property)]> {
    match storage {
        PropertyStorage::Small { entries } => Some(entries),
        PropertyStorage::Dynamic(dynamic) => Some(&dynamic.entries),
        PropertyStorage::Shaped { .. } | PropertyStorage::ShapedPair { .. } => None,
    }
}

#[inline(always)]
fn named_entries_mut(storage: &mut PropertyStorage) -> Option<&mut [(Rc<str>, Property)]> {
    match storage {
        PropertyStorage::Small { entries } => Some(entries),
        PropertyStorage::Dynamic(dynamic) => Some(&mut dynamic.entries),
        PropertyStorage::Shaped { .. } | PropertyStorage::ShapedPair { .. } => None,
    }
}

impl ObjectRef {
    pub(crate) fn own_data_property_read(&self, key: &str) -> OwnDataPropertyRead {
        if self.0.module_namespace_exotic.get() {
            return OwnDataPropertyRead::NeedsSlowPath;
        }
        self.properties_for(key).borrow().own_data_read(key)
    }

    /// Resolves `key` to a stable own-property slot in this object's compact
    /// storage. Callers pair the slot with [`Self::layout_revision`]: the pair
    /// stays valid across ordinary value assignment, so a monomorphic read
    /// cache keeps hitting while a field is written every iteration.
    pub(crate) fn own_data_slot(&self, key: &str) -> Option<usize> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        match &*self.properties_for(key).borrow() {
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

    /// Resolves `key` to a data-property slot in small or dynamic storage,
    /// for a writer that validates it with [`Self::layout_revision`] and
    /// writes it with [`Self::any_storage_data_slot_write`]. The read caches
    /// keep [`Self::own_data_slot`], which leaves dynamic storage to their
    /// own entries.
    pub(crate) fn any_storage_data_slot(&self, key: &str) -> Option<usize> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        match &*self.properties_for(key).borrow() {
            PropertyStorage::Small { entries } => {
                entries.iter().position(|(candidate, property)| {
                    candidate.as_ref() == key && !property.is_accessor()
                })
            }
            PropertyStorage::Dynamic(dynamic) => dynamic
                .slot(key)
                .filter(|&slot| !dynamic.entries[slot].1.is_accessor()),
            PropertyStorage::Shaped { .. } | PropertyStorage::ShapedPair { .. } => None,
        }
    }

    /// Writes a slot resolved by [`Self::any_storage_data_slot`] under an
    /// unchanged layout; still checks writability.
    pub(crate) fn any_storage_data_slot_write(
        &self,
        slot: usize,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let result = match &mut *self.0.properties.borrow_mut() {
            PropertyStorage::Small { entries } => {
                let (_, property) = entries.get_mut(slot)?;
                super::write_existing_property(Some(property), value, |_| true)
            }
            PropertyStorage::Dynamic(dynamic) => {
                let (_, property) = dynamic.entries.get_mut(slot)?;
                super::write_existing_property(Some(property), value, |_| true)
            }
            PropertyStorage::Shaped { .. } | PropertyStorage::ShapedPair { .. } => return None,
        };
        if matches!(result, OwnDataPropertyWrite::Written) {
            self.bump_value_revision();
        }
        Some(result)
    }

    /// Resolves a prototype read to a slot in small, dynamic or literal
    /// storage. The caller guards holder identity and layout before reading
    /// the slot. Keep this separate from `own_data_slot`: that API also
    /// installs write caches, whose supported storage and policy remain
    /// unchanged here. Dynamic storage matters most on this side: a class or
    /// constructor prototype with more than a dozen methods is one, and every
    /// method call on its instances reads through it.
    pub(crate) fn prototype_data_slot(&self, key: &str) -> Option<usize> {
        self.any_storage_data_slot(key)
            .or_else(|| self.literal_data_slot(key).map(|(_, slot)| slot))
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
        match &*self.properties_for(key).borrow() {
            PropertyStorage::Small { entries } => {
                entries
                    .iter()
                    .enumerate()
                    .find_map(|(slot, (name, property))| {
                        (name.as_ref() == key && !property.is_accessor())
                            .then(|| (Rc::clone(name), slot))
                    })
            }
            PropertyStorage::Dynamic(dynamic) => {
                let slot = dynamic.slot(key)?;
                let (name, property) = dynamic.entries.get(slot)?;
                (!property.is_accessor()).then(|| (Rc::clone(name), slot))
            }
            PropertyStorage::Shaped { .. } | PropertyStorage::ShapedPair { .. } => None,
        }
    }

    /// [`Self::shared_data_slot_value`] of a number: the typed loop tier's
    /// read of a numeric field (`body.x`), without cloning a `Value` to
    /// unpack it again.
    #[inline]
    pub(crate) fn shared_data_slot_number(&self, key: &Rc<str>, slot: usize) -> Option<f64> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        // Small storage only: this read is inlined into the typed-loop
        // executor, whose hot arm grew and re-rolled its register allocation
        // with a dynamic-storage case (access-nbody +7% instructions). A
        // dynamic object takes the executor's general read instead.
        match &*self.0.properties.borrow() {
            PropertyStorage::Small { entries } => {
                let (name, property) = entries.get(slot)?;
                match &property.value {
                    Value::Number(number) if Rc::ptr_eq(name, key) && !property.is_accessor() => {
                        Some(*number)
                    }
                    _ => None,
                }
            }
            PropertyStorage::Dynamic(_)
            | PropertyStorage::Shaped { .. }
            | PropertyStorage::ShapedPair { .. } => None,
        }
    }

    /// [`Self::shared_data_slot_write`] of a number over a writable data
    /// property: the typed loop tier's field update (`body.vx -= ...`).
    /// `false`, having written nothing, for anything else.
    #[inline]
    pub(crate) fn shared_data_slot_write_number(
        &self,
        key: &Rc<str>,
        slot: usize,
        number: f64,
    ) -> bool {
        if self.0.module_namespace_exotic.get() {
            return false;
        }
        // Small storage only, like `shared_data_slot_number`.
        let written = match &mut *self.0.properties.borrow_mut() {
            PropertyStorage::Small { entries } => match entries.get_mut(slot) {
                Some((name, property))
                    if Rc::ptr_eq(name, key) && property.writable && !property.is_accessor() =>
                {
                    property.value = Value::Number(number);
                    true
                }
                _ => false,
            },
            PropertyStorage::Dynamic(_)
            | PropertyStorage::Shaped { .. }
            | PropertyStorage::ShapedPair { .. } => false,
        };
        if written {
            self.bump_value_revision();
        }
        written
    }

    /// Reads a slot recorded by [`Self::shared_data_slot`], confirming that
    /// this object holds the same interned name in that slot.
    pub(crate) fn shared_data_slot_value(&self, key: &Rc<str>, slot: usize) -> Option<Value> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let storage = self.0.properties.borrow();
        let (name, property) = named_entries(&storage)?.get(slot)?;
        (Rc::ptr_eq(name, key) && !property.is_accessor()).then(|| property.value.clone())
    }

    /// [`Self::shared_data_slot_value`] of a dynamic-storage object only: the
    /// typed loop tier's inline read already tried small storage, and a
    /// repeated miss there costs a polymorphic site a second mispredicted
    /// check.
    pub(crate) fn shared_dynamic_slot_value(&self, key: &Rc<str>, slot: usize) -> Option<Value> {
        let storage = self.0.properties.borrow();
        let PropertyStorage::Dynamic(dynamic) = &*storage else {
            return None;
        };
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let (name, property) = dynamic.entries.get(slot)?;
        (Rc::ptr_eq(name, key) && !property.is_accessor()).then(|| property.value.clone())
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

    /// Reads a guarded prototype slot without expanding the small-object
    /// reader used by own-property caches and prepared loop reads.
    #[inline]
    pub(crate) fn prototype_data_slot_value(&self, slot: usize) -> Option<Value> {
        self.own_data_slot_value(slot)
            .or_else(|| self.literal_prototype_data_slot_value(slot))
    }

    #[inline(never)]
    fn literal_prototype_data_slot_value(&self, slot: usize) -> Option<Value> {
        match &*self.0.properties.borrow() {
            PropertyStorage::Shaped { properties, .. } => {
                let property = properties.get(slot)?;
                (!property.is_accessor()).then(|| property.value.clone())
            }
            PropertyStorage::ShapedPair { values, .. } => values.get(slot).cloned(),
            PropertyStorage::Dynamic(dynamic) => {
                let (_, property) = dynamic.entries.get(slot)?;
                (!property.is_accessor()).then(|| property.value.clone())
            }
            PropertyStorage::Small { .. } => None,
        }
    }

    /// Writes a slot resolved by [`Self::own_data_slot`]. The caller already
    /// validated object identity and layout; this still checks writability so
    /// a descriptor replacement cannot skip observable `[[Set]]` behavior.
    pub(crate) fn own_data_slot_write(
        &self,
        slot: usize,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let result = match &mut *self.0.properties.borrow_mut() {
            PropertyStorage::Small { entries } => {
                let (_, property) = entries.get_mut(slot)?;
                super::write_existing_property(Some(property), value, |_| true)
            }
            PropertyStorage::Dynamic(_)
            | PropertyStorage::Shaped { .. }
            | PropertyStorage::ShapedPair { .. } => return None,
        };
        if matches!(result, OwnDataPropertyWrite::Written) {
            self.bump_value_revision();
        }
        Some(result)
    }

    /// Writes a shared small-object slot only when the recorded interned key
    /// is still present at that exact position.
    pub(crate) fn shared_data_slot_write(
        &self,
        key: &Rc<str>,
        slot: usize,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let result = {
            let mut storage = self.0.properties.borrow_mut();
            let (name, property) = named_entries_mut(&mut storage)?.get_mut(slot)?;
            if !Rc::ptr_eq(name, key) {
                return None;
            }
            super::write_existing_property(Some(property), value, |_| true)
        };
        if matches!(result, OwnDataPropertyWrite::Written) {
            self.bump_value_revision();
        }
        Some(result)
    }

    /// Returns the shared literal shape and storage slot of a data property
    /// of an object literal. Named-property caches use this to share one
    /// cache entry across distinct objects created by the same bytecode site.
    ///
    /// The shape identifies the layout: adding or deleting a property, or
    /// redefining one, leaves shaped storage for dynamic storage, so a value
    /// overwritten in place keeps the slot valid. A literal whose field is
    /// updated every iteration -- a search node's cost -- therefore stays on
    /// this path; it used to fall off it at the first write.
    pub(crate) fn literal_data_slot(&self, key: &str) -> Option<(Rc<ObjectLiteralShape>, usize)> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let properties = self.properties_for(key).borrow();
        let (shape, slot) = match &*properties {
            PropertyStorage::Shaped { shape, properties } => {
                let slot = *shape.lookup.get(key)?;
                if properties.get(slot)?.is_accessor() {
                    return None;
                }
                (shape, slot)
            }
            PropertyStorage::ShapedPair { shape, .. } => (shape, *shape.lookup.get(key)?),
            PropertyStorage::Small { .. } | PropertyStorage::Dynamic(_) => return None,
        };
        Some((shape.clone(), slot))
    }

    /// Reads a previously resolved literal slot after checking that this
    /// object still has the same shared shape and the slot still holds a data
    /// property.
    pub(crate) fn literal_data_slot_value(
        &self,
        expected_shape: &Rc<ObjectLiteralShape>,
        slot: usize,
    ) -> Option<Value> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        match &*self.0.properties.borrow() {
            PropertyStorage::Shaped { shape, properties } if Rc::ptr_eq(shape, expected_shape) => {
                let property = properties.get(slot)?;
                (!property.is_accessor()).then(|| property.value.clone())
            }
            PropertyStorage::ShapedPair { shape, values } if Rc::ptr_eq(shape, expected_shape) => {
                values.get(slot).cloned()
            }
            _ => None,
        }
    }
}

impl ObjectRef {
    #[cfg(feature = "perf-counters")]
    pub(crate) fn storage_kind_for_trace(&self) -> &'static str {
        match &*self.properties().borrow() {
            PropertyStorage::Small { .. } => "small",
            PropertyStorage::Dynamic(_) => "dynamic",
            PropertyStorage::Shaped { .. } => "shaped",
            PropertyStorage::ShapedPair { .. } => "pair",
        }
    }
}
