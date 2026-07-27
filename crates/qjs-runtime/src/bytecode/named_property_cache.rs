//! Per-site ordinary named-property cache shared by member reads and writes.

use std::{cell::RefCell, rc::Rc};

use crate::{
    ObjectRef, Value,
    value::{ObjectLiteralShape, ObjectWeakRef, OwnDataPropertyWrite},
};

/// Number of receiver layouts one site retains before falling back.
const POLYMORPHIC_CACHE_SLOTS: usize = 2;

#[derive(Clone, Debug, Default)]
pub(super) struct NamedPropertyCache(Rc<RefCell<NamedPropertyCacheState>>);

#[derive(Clone, Debug, Default)]
struct NamedPropertyCacheState {
    entries: [Option<NamedPropertyCacheEntry>; POLYMORPHIC_CACHE_SLOTS],
    next_slot: usize,
    local_slot: Option<usize>,
}

#[derive(Clone, Debug)]
enum NamedPropertyCacheEntry {
    Exact {
        object: ObjectWeakRef,
        revision: u64,
        value: CachedValue,
    },
    LiteralShape {
        shape: Rc<ObjectLiteralShape>,
        slot: usize,
    },
    OwnSlot {
        object: ObjectWeakRef,
        layout_revision: u64,
        slot: usize,
    },
    SharedSlot {
        key: Rc<str>,
        slot: usize,
    },
}

#[derive(Clone, Debug)]
enum CachedValue {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    Object(ObjectWeakRef),
}

impl NamedPropertyCache {
    pub(super) fn for_local(slot: usize) -> Self {
        Self(Rc::new(RefCell::new(NamedPropertyCacheState {
            entries: Default::default(),
            next_slot: 0,
            local_slot: Some(slot),
        })))
    }

    pub(super) fn local_slot(&self) -> Option<usize> {
        self.0.borrow().local_slot
    }

    pub(super) fn get(&self, object: &ObjectRef) -> Option<Value> {
        let state = self.0.borrow();
        state
            .entries
            .iter()
            .flatten()
            .find_map(|entry| Self::read_entry(entry, object))
    }

    fn read_entry(entry: &NamedPropertyCacheEntry, object: &ObjectRef) -> Option<Value> {
        let value = match entry {
            NamedPropertyCacheEntry::Exact {
                object: cached_object,
                revision,
                value,
            } => {
                if !cached_object.ptr_eq(object) || *revision != object.property_revision() {
                    return None;
                }
                value
            }
            NamedPropertyCacheEntry::LiteralShape { shape, slot } => {
                return object.literal_data_slot_value(shape, *slot);
            }
            NamedPropertyCacheEntry::OwnSlot {
                object: cached_object,
                layout_revision,
                slot,
            } => {
                if !cached_object.ptr_eq(object) || *layout_revision != object.layout_revision() {
                    return None;
                }
                return object.own_data_slot_value(*slot);
            }
            NamedPropertyCacheEntry::SharedSlot { key, slot } => {
                return object.shared_data_slot_value(key, *slot);
            }
        };
        Some(match value {
            CachedValue::Undefined => Value::Undefined,
            CachedValue::Null => Value::Null,
            CachedValue::Boolean(value) => Value::Boolean(*value),
            CachedValue::Number(value) => Value::Number(*value),
            CachedValue::Object(value) => Value::Object(value.upgrade()?),
        })
    }

    pub(super) fn update(&self, object: &ObjectRef, key: &str, value: &Value) {
        let value_entry_went_stale = self.0.borrow().entries.iter().flatten().any(|entry| {
            matches!(
                entry,
                NamedPropertyCacheEntry::Exact { object: cached, .. } if cached.ptr_eq(object)
            )
        });
        let saw_other_object = self
            .0
            .borrow()
            .entries
            .iter()
            .flatten()
            .any(|entry| match entry {
                NamedPropertyCacheEntry::Exact { object: cached, .. }
                | NamedPropertyCacheEntry::OwnSlot { object: cached, .. } => !cached.ptr_eq(object),
                NamedPropertyCacheEntry::LiteralShape { .. }
                | NamedPropertyCacheEntry::SharedSlot { .. } => false,
            });
        let entry = if let Some((shape, slot)) = object.literal_data_slot(key) {
            NamedPropertyCacheEntry::LiteralShape { shape, slot }
        } else if let Some((key, slot)) = saw_other_object
            .then(|| object.shared_data_slot(key))
            .flatten()
        {
            NamedPropertyCacheEntry::SharedSlot { key, slot }
        } else if let Some(slot) = value_entry_went_stale
            .then(|| object.own_data_slot(key))
            .flatten()
        {
            NamedPropertyCacheEntry::OwnSlot {
                object: object.downgrade(),
                layout_revision: object.layout_revision(),
                slot,
            }
        } else {
            let value = match value {
                Value::Undefined => CachedValue::Undefined,
                Value::Null => CachedValue::Null,
                Value::Boolean(value) => CachedValue::Boolean(*value),
                Value::Number(value) => CachedValue::Number(*value),
                Value::Object(value) => CachedValue::Object(value.downgrade()),
                _ => {
                    self.clear();
                    return;
                }
            };
            NamedPropertyCacheEntry::Exact {
                object: object.downgrade(),
                revision: object.property_revision(),
                value,
            }
        };
        let mut state = self.0.borrow_mut();
        let slot = state.next_slot;
        state.entries[slot] = Some(entry);
        state.next_slot = (slot + 1) % POLYMORPHIC_CACHE_SLOTS;
    }

    pub(super) fn write(
        &self,
        object: &ObjectRef,
        key: &str,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        let mut state = self.0.borrow_mut();
        for entry in state.entries.iter_mut().flatten() {
            match entry {
                NamedPropertyCacheEntry::Exact {
                    object: cached,
                    revision,
                    ..
                } if cached.ptr_eq(object) && *revision == object.property_revision() => {
                    let slot = object.own_data_slot(key)?;
                    let result = object.own_data_slot_write(slot, value)?;
                    *entry = NamedPropertyCacheEntry::OwnSlot {
                        object: object.downgrade(),
                        layout_revision: object.layout_revision(),
                        slot,
                    };
                    return Some(result);
                }
                NamedPropertyCacheEntry::OwnSlot {
                    object: cached,
                    layout_revision,
                    slot,
                } if cached.ptr_eq(object) && *layout_revision == object.layout_revision() => {
                    return object.own_data_slot_write(*slot, value);
                }
                NamedPropertyCacheEntry::SharedSlot { key, slot } => {
                    if let Some(result) = object.shared_data_slot_write(key, *slot, value) {
                        return Some(result);
                    }
                }
                NamedPropertyCacheEntry::Exact { .. }
                | NamedPropertyCacheEntry::LiteralShape { .. }
                | NamedPropertyCacheEntry::OwnSlot { .. } => {}
            }
        }
        None
    }

    pub(super) fn record_write(&self, object: &ObjectRef, key: &str) {
        let mut state = self.0.borrow_mut();
        let saw_other_object = state.entries.iter().flatten().any(|entry| {
            matches!(
                entry,
                NamedPropertyCacheEntry::Exact { object: cached, .. }
                    | NamedPropertyCacheEntry::OwnSlot { object: cached, .. }
                    if !cached.ptr_eq(object)
            )
        });
        let entry = if saw_other_object {
            object
                .shared_data_slot(key)
                .map(|(key, slot)| NamedPropertyCacheEntry::SharedSlot { key, slot })
        } else {
            object
                .own_data_slot(key)
                .map(|slot| NamedPropertyCacheEntry::OwnSlot {
                    object: object.downgrade(),
                    layout_revision: object.layout_revision(),
                    slot,
                })
        };
        let Some(entry) = entry else {
            return;
        };
        let slot = state.next_slot;
        state.entries[slot] = Some(entry);
        state.next_slot = (slot + 1) % POLYMORPHIC_CACHE_SLOTS;
    }

    pub(super) fn clear(&self) {
        let mut state = self.0.borrow_mut();
        state.entries = Default::default();
        state.next_slot = 0;
    }
}
