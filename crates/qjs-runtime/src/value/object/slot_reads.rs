//! Slot-addressed reads of an object's property store.
//!
//! The bytecode's named-property read caches resolve a property once and then
//! re-read it by slot. Each entry pairs the slot with whatever it needs to
//! prove the slot still means the same property: a layout revision for one
//! object, a literal site's layout, or the interned name the layout holds at
//! that slot. The layout owns names and attributes; the payload holds values.
//! Which encodings each reader admits is unchanged from the payload-keyed
//! stores these readers replaced, so caches keep their coverage.

use std::rc::Rc;

use super::storage::{Payload, PropertyStorage};
use super::{ObjectRef, OwnDataPropertyRead, OwnDataPropertyWrite, own_data_read, write_slot};
use crate::Value;
use std::rc::Weak;

use crate::property::layout::{Encoding, Layout};

/// A literal site's published layout, as a read cache records it.
pub(crate) type LiteralLayout = Layout;

/// A data slot of a `Small` or `Dynamic` store as a cache records it: the
/// interned name held there, and the shape it was resolved on.
#[derive(Clone, Debug)]
pub(crate) struct SharedSlot {
    pub(crate) key: Rc<str>,
    pub(crate) slot: usize,
    writable: bool,
    small: bool,
    /// `None` for a dictionary, which a cache must not share.
    shape: Option<Layout>,
    /// A weak independent dictionary identity plus the owner's layout
    /// revision. Never borrow the node weakly: Rc::get_mut must remain able
    /// to edit a uniquely owned dictionary in place after a cached read.
    dictionary: Option<(Weak<()>, u64)>,
}

impl SharedSlot {
    /// Whether `storage` is still on the recorded shape, which fixes the
    /// slot's key, attributes and encoding as recorded.
    #[inline(always)]
    fn on_shape(&self, storage: &PropertyStorage) -> bool {
        self.shape
            .as_ref()
            .is_some_and(|shape| Rc::ptr_eq(shape, &storage.layout))
    }

    /// [`Self::on_shape`], or still on the recorded dictionary unedited
    /// since: either fixes the slot's key and attributes as recorded, and
    /// the dictionary's independent identity prevents address reuse (ABA).
    #[inline(always)]
    fn on_layout(&self, object: &ObjectRef, storage: &PropertyStorage) -> bool {
        self.on_shape(storage)
            || self
                .dictionary
                .as_ref()
                .is_some_and(|(identity, revision)| {
                    object.layout_revision() == *revision
                        && storage.layout.as_dictionary().is_some_and(|dictionary| {
                            std::ptr::eq(identity.as_ptr(), Rc::as_ptr(dictionary.identity()))
                        })
                })
    }
}

#[inline(always)]
fn named_encoding(storage: &PropertyStorage) -> bool {
    matches!(storage.encoding(), Encoding::Small | Encoding::Dynamic)
}

/// The data slot `key` names in `storage`, when `admit` accepts its encoding.
#[inline(always)]
fn data_slot(
    storage: &PropertyStorage,
    key: &str,
    admit: impl Fn(Encoding) -> bool,
) -> Option<usize> {
    if !admit(storage.encoding()) {
        return None;
    }
    let slot = storage.lookup(key)?;
    (!storage.field(slot)?.attributes.is_accessor()).then_some(slot)
}

/// The value of data slot `slot` whose layout names `key` there.
#[inline(always)]
fn named_slot_value(storage: &PropertyStorage, key: &Rc<str>, slot: usize) -> Option<Value> {
    let field = storage.field(slot)?;
    if !Rc::ptr_eq(field.text(), key) || field.attributes.is_accessor() {
        return None;
    }
    storage.slot(slot).map(|slot| slot.value().clone())
}

impl ObjectRef {
    pub(crate) fn own_data_property_read(&self, key: &str) -> OwnDataPropertyRead {
        if self.0.module_namespace_exotic.get() {
            return OwnDataPropertyRead::NeedsSlowPath;
        }
        own_data_read(&self.properties_for(key).borrow(), key)
    }

    /// Resolves `key` to a stable own-property slot of a `Small` store.
    /// Callers pair the slot with [`Self::layout_revision`]: the pair stays
    /// valid across ordinary value assignment, so a monomorphic read cache
    /// keeps hitting while a field is written every iteration.
    pub(crate) fn own_data_slot(&self, key: &str) -> Option<usize> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        data_slot(&self.properties_for(key).borrow(), key, |encoding| {
            encoding == Encoding::Small
        })
    }

    /// Resolves `key` to a data-property slot of a `Small` or `Dynamic`
    /// store, for a writer that validates it with [`Self::layout_revision`]
    /// and writes it with [`Self::any_storage_data_slot_write`].
    pub(crate) fn any_storage_data_slot(&self, key: &str) -> Option<usize> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        data_slot(&self.properties_for(key).borrow(), key, |encoding| {
            matches!(encoding, Encoding::Small | Encoding::Dynamic)
        })
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
        let result = {
            let mut storage = self.0.properties.borrow_mut();
            if !named_encoding(&storage) || slot >= storage.slot_count() {
                return None;
            }
            write_slot(&mut storage, slot, value, |_| true)
        };
        if matches!(result, OwnDataPropertyWrite::Written) {
            self.bump_value_revision();
        }
        Some(result)
    }

    /// Resolves a prototype read to a data slot of any store encoding. The
    /// caller guards holder identity and layout before reading the slot.
    pub(crate) fn prototype_data_slot(&self, key: &str) -> Option<usize> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        data_slot(&self.properties_for(key).borrow(), key, |_| true)
    }

    /// Resolves `key` to a slot of a `Small` or `Dynamic` store together with
    /// the interned name the layout holds there.
    ///
    /// Objects built by the same code site share one interned name per
    /// property, so a read cache can revalidate a *different* object against
    /// the recorded name by pointer alone -- no object identity and no name
    /// comparison. That is what lets one cache entry serve a loop over many
    /// objects of the same construction, which object identity keying cannot.
    /// The record also keeps the shape it was resolved on: a receiver still
    /// on that shape is answered by one pointer comparison.
    pub(crate) fn shared_data_slot(&self, key: &str) -> Option<SharedSlot> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let storage = self.properties_for(key).borrow();
        let slot = data_slot(&storage, key, |encoding| {
            matches!(encoding, Encoding::Small | Encoding::Dynamic)
        })?;
        let field = storage.field(slot)?;
        Some(SharedSlot {
            key: Rc::clone(field.text()),
            slot,
            writable: field.attributes.is_writable(),
            small: storage.encoding() == Encoding::Small,
            shape: (!storage.layout.is_dictionary()).then(|| Rc::clone(&storage.layout)),
            dictionary: storage
                .layout
                .as_dictionary()
                .map(|dictionary| (Rc::downgrade(dictionary.identity()), self.layout_revision())),
        })
    }

    /// The interned text the layout holds for own key `key`.
    pub(crate) fn own_property_name(&self, key: &str) -> Option<Rc<str>> {
        let storage = self.0.properties.borrow();
        let slot = storage.lookup(key)?;
        Some(Rc::clone(storage.field(slot)?.text()))
    }

    /// [`Self::shared_data_slot_value`] of a number in a `Small` store: the
    /// typed loop tier's read of a numeric field (`body.x`), without cloning a
    /// `Value` to unpack it again.
    // Keep the shape/fallback guards outside the typed dispatch loop. Their
    // size otherwise changes that loop even for scalar-only programs.
    #[inline(never)]
    pub(crate) fn shared_data_slot_number(&self, way: &SharedSlot) -> Option<f64> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        // Keep the existing Small-only admission of the numeric cache.
        let storage = self.0.properties.borrow();
        let slot = way.slot;
        if !(way.small && way.on_shape(&storage)) {
            let field = storage.layout.small_field(slot)?;
            if !Rc::ptr_eq(field.text(), &way.key) || field.attributes.is_accessor() {
                return None;
            }
        }
        match &storage.payload {
            Payload::Slots(properties) => match properties.get(slot)?.value {
                Value::Number(number) => Some(number),
                _ => None,
            },
            Payload::Pair(_) => None,
        }
    }

    /// [`Self::shared_data_slot_write`] of a number over a writable data
    /// property of a `Small` store: the typed loop tier's field update
    /// (`body.vx -= ...`). `false`, having written nothing, for anything else.
    #[inline(never)]
    pub(crate) fn shared_data_slot_write_number(&self, way: &SharedSlot, number: f64) -> bool {
        if self.0.module_namespace_exotic.get() {
            return false;
        }
        let written = {
            let mut storage = self.0.properties.borrow_mut();
            let slot = way.slot;
            if !(way.small && way.writable && way.on_shape(&storage)) {
                let Some(field) = storage.layout.small_field(slot) else {
                    return false;
                };
                if !Rc::ptr_eq(field.text(), &way.key)
                    || !field.attributes.is_writable()
                    || field.attributes.is_accessor()
                {
                    return false;
                }
            }
            match storage.value_mut(slot) {
                Some(value) => {
                    *value = Value::Number(number);
                    true
                }
                None => false,
            }
        };
        if written {
            self.bump_value_revision();
        }
        written
    }

    /// Reads a slot recorded by [`Self::shared_data_slot`], confirming that
    /// this object is on the recorded shape or that its layout holds the same
    /// interned name in that slot.
    pub(crate) fn shared_data_slot_value(&self, way: &SharedSlot) -> Option<Value> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let storage = self.0.properties.borrow();
        if way.on_layout(self, &storage) {
            return storage.slot(way.slot).map(|slot| slot.value().clone());
        }
        if !named_encoding(&storage) {
            return None;
        }
        named_slot_value(&storage, &way.key, way.slot)
    }

    /// [`Self::shared_data_slot_value`] of a `Dynamic` store only: the typed
    /// loop tier's inline read already tried `Small`, and a repeated miss
    /// there costs a polymorphic site a second mispredicted check.
    pub(crate) fn shared_dynamic_slot_value(&self, way: &SharedSlot) -> Option<Value> {
        let storage = self.0.properties.borrow();
        if storage.encoding() != Encoding::Dynamic {
            return None;
        }
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        if way.on_layout(self, &storage) {
            return storage.slot(way.slot).map(|slot| slot.value().clone());
        }
        named_slot_value(&storage, &way.key, way.slot)
    }

    /// Reads a slot previously resolved by [`Self::own_data_slot`]. The
    /// encoding is re-checked so a layout the revision counter cannot
    /// describe simply misses the cache instead of reading the wrong slot.
    pub(crate) fn own_data_slot_value(&self, slot: usize) -> Option<Value> {
        let storage = self.0.properties.borrow();
        if storage.encoding() != Encoding::Small {
            return None;
        }
        // The payload descriptor carries the layout field's attributes.
        let slot = storage.slot(slot)?;
        (!slot.is_accessor()).then(|| slot.value().clone())
    }

    /// Reads a guarded prototype slot of any encoding. Keep the layout/payload
    /// dispatch out of the wide executor's property arms: changing that
    /// inlining graph can spill unrelated scalar and upvalue operations.
    #[inline(never)]
    pub(crate) fn prototype_data_slot_value(&self, slot: usize) -> Option<Value> {
        let storage = self.0.properties.borrow();
        let slot = storage.slot(slot)?;
        (!slot.is_accessor()).then(|| slot.value().clone())
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
        let result = {
            let mut storage = self.0.properties.borrow_mut();
            if storage.encoding() != Encoding::Small || slot >= storage.slot_count() {
                return None;
            }
            write_slot(&mut storage, slot, value, |_| true)
        };
        if matches!(result, OwnDataPropertyWrite::Written) {
            self.bump_value_revision();
        }
        Some(result)
    }

    /// Writes a shared slot of a `Small` or `Dynamic` store only when the
    /// object is on the recorded shape or its layout still holds the recorded
    /// interned key at that exact position.
    pub(crate) fn shared_data_slot_write(
        &self,
        way: &SharedSlot,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let result = {
            let mut storage = self.0.properties.borrow_mut();
            if way.writable && way.on_layout(self, &storage) {
                *storage.value_mut(way.slot)? = value.clone();
                OwnDataPropertyWrite::Written
            } else {
                if !named_encoding(&storage)
                    || !Rc::ptr_eq(storage.field(way.slot)?.text(), &way.key)
                {
                    return None;
                }
                write_slot(&mut storage, way.slot, value, |_| true)
            }
        };
        if matches!(result, OwnDataPropertyWrite::Written) {
            self.bump_value_revision();
        }
        Some(result)
    }

    /// Returns the literal layout and slot of a data property of an object
    /// literal. Named-property caches use this to share one cache entry across
    /// distinct objects created by the same bytecode site.
    ///
    /// The layout identifies keys and attributes: adding, deleting or
    /// redefining a property moves the object to another layout, so a value
    /// overwritten in place keeps the slot valid. A literal whose field is
    /// updated every iteration -- a search node's cost -- stays on this path.
    pub(crate) fn literal_data_slot(&self, key: &str) -> Option<(LiteralLayout, usize)> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let storage = self.properties_for(key).borrow();
        let slot = data_slot(&storage, key, |encoding| {
            matches!(encoding, Encoding::Shaped | Encoding::Pair)
        })?;
        Some((Rc::clone(&storage.layout), slot))
    }

    /// Reads a previously resolved literal slot after checking that this
    /// object still has the same literal layout.
    pub(crate) fn literal_data_slot_value(
        &self,
        expected: &LiteralLayout,
        slot: usize,
    ) -> Option<Value> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        let storage = self.0.properties.borrow();
        if !Rc::ptr_eq(&storage.layout, expected) {
            return None;
        }
        // The layout fixes the slot's attributes, recorded as data.
        storage.slot(slot).map(|slot| slot.value().clone())
    }
}

impl ObjectRef {
    #[cfg(feature = "perf-counters")]
    pub(crate) fn storage_kind_for_trace(&self) -> &'static str {
        let storage = self.properties().borrow();
        if storage.layout.is_dictionary() {
            return "dictionary";
        }
        match storage.encoding() {
            Encoding::Small => "small",
            Encoding::Dynamic => "dynamic",
            Encoding::Shaped => "shaped",
            Encoding::Pair => "pair",
        }
    }
}
