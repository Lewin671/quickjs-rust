//! Per-site ordinary named-property cache shared by member reads and writes.

use std::{cell::RefCell, rc::Rc};

use crate::{
    ObjectRef, Value,
    value::{ObjectLiteralShape, ObjectWeakRef, OwnDataPropertyWrite},
};

/// Number of receiver layouts one site retains before falling back.
///
/// Two covers a monomorphic site and misses the moment a call site sees three
/// object shapes, which ordinary code produces as soon as one function reads a
/// field from objects built by different literals. Four costs 1-1.5% on the
/// workloads whose sites stay monomorphic -- the state is twice the size and
/// a thrashing site rewrites twice as many entries -- and is worth 18% where a
/// third shape exists.
///
/// Holding the extra entries behind a lazily allocated box instead was
/// measured and is worse: a site that rotates through more receivers than it
/// can hold then allocates on every other update, which cost 4.7% on
/// prototype-dispatched reads.
const POLYMORPHIC_CACHE_SLOTS: usize = 4;

#[cfg(test)]
mod tests;

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
    /// A read that resolved on the receiver's direct prototype.
    ///
    /// Every method call has this shape, and it was the one shape this cache
    /// could not hold: a receiver miss cleared the whole site and walked the
    /// chain again on the next read. Measured, that made a prototype-resolved
    /// read cost about 20 ns more than an own-property read, against 2.5 ns
    /// for QuickJS-NG.
    ///
    /// It is keyed on the *holder*, not the receiver, which is what makes it
    /// work for the many-receivers case: every instance of one constructor
    /// shares one prototype, so one entry serves them all.
    ///
    /// Three facts guard it, each re-established on every hit:
    ///
    /// - the receiver has no own `key` -- proven by the caller, with the same
    ///   `own_data_property_read` that used to clear the site, before this
    ///   entry is consulted at all;
    /// - the receiver's [[Prototype]] is still this holder, so
    ///   `Object.setPrototypeOf` on the receiver misses;
    /// - the holder's own-slot layout is unchanged, so `slot` still names the
    ///   same property. Assigning through the property keeps the layout and is
    ///   observed anyway, because the value is read from the slot rather than
    ///   cached here.
    PrototypeSlot {
        holder: ObjectWeakRef,
        holder_layout_revision: u64,
        slot: usize,
    },
}

/// What one pass over a site found.
pub(super) enum CacheProbe {
    /// An own-property answer, already fully guarded.
    Own(Value),
    /// The receiver's prototype matches a cached holder. Valid only once the
    /// caller has proven the receiver has no own property of this name.
    PrototypeCandidate {
        holder: ObjectRef,
        slot: usize,
    },
    Miss,
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

    /// One pass over the site for both answers it can give.
    ///
    /// A prototype-dominant site would otherwise be scanned twice -- once for
    /// an own-property answer that is never there, and again after the
    /// receiver miss is proven -- so the two are folded into one walk. The
    /// prototype answer is returned as an unverified candidate, because using
    /// it still requires the caller to establish that the receiver has no own
    /// property of that name.
    pub(super) fn probe(&self, object: &ObjectRef) -> CacheProbe {
        let state = self.0.borrow();
        let mut candidate = None;
        for entry in state.entries.iter().flatten() {
            if let NamedPropertyCacheEntry::PrototypeSlot {
                holder,
                holder_layout_revision,
                slot,
            } = entry
            {
                // Compare the receiver's prototype by identity before
                // upgrading, so a site rotating over several prototypes pays
                // one reference-count round trip for the entry that matches
                // rather than one per entry walked.
                if candidate.is_none()
                    && object.prototype_is_weak(holder)
                    && let Some(holder) = holder.upgrade()
                    && *holder_layout_revision == holder.layout_revision()
                {
                    candidate = Some((holder, *slot));
                }
                continue;
            }
            if let Some(value) = Self::read_entry(entry, object) {
                return CacheProbe::Own(value);
            }
        }
        match candidate {
            Some((holder, slot)) => CacheProbe::PrototypeCandidate { holder, slot },
            None => CacheProbe::Miss,
        }
    }

    /// Inlined into `probe` unconditionally: left to LLVM, whether this is
    /// inlined and the four-entry walk unrolled varied from build to build
    /// with unrelated code, and a build that keeps it out of line is about
    /// 4% slower on every property-heavy workload (3d-raytrace, access-nbody).
    #[inline(always)]
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
            // Only valid once the receiver is known to have no own `key`, so
            // it is served by `get_from_prototype` rather than from here.
            NamedPropertyCacheEntry::PrototypeSlot { .. } => return None,
        };
        Some(match value {
            CachedValue::Undefined => Value::Undefined,
            CachedValue::Null => Value::Null,
            CachedValue::Boolean(value) => Value::Boolean(*value),
            CachedValue::Number(value) => Value::Number(*value),
            CachedValue::Object(value) => Value::Object(value.upgrade()?),
        })
    }

    /// Records that `key` resolved to a data property on `receiver`'s direct
    /// prototype. Any other resolution -- an accessor, a deeper holder, a
    /// non-ordinary prototype, or storage without stable slots -- installs
    /// nothing, so the read stays on the general path.
    pub(super) fn update_from_prototype(&self, receiver: &ObjectRef, key: &str) {
        let Some(holder) = receiver.ordinary_prototype() else {
            return;
        };
        let Some(slot) = holder.prototype_data_slot(key) else {
            return;
        };
        let entry = NamedPropertyCacheEntry::PrototypeSlot {
            holder: holder.downgrade(),
            holder_layout_revision: holder.layout_revision(),
            slot,
        };
        let mut state = self.0.borrow_mut();
        // A site that already holds this exact holder is re-reading it, not
        // rotating: replacing in place keeps a polymorphic site from spending
        // all four entries on one prototype.
        if let Some(existing) = state.entries.iter_mut().flatten().find(|existing| {
            matches!(
                existing,
                NamedPropertyCacheEntry::PrototypeSlot { holder: cached, .. }
                    if cached.ptr_eq(&holder)
            )
        }) {
            *existing = entry;
            return;
        }
        let slot = state.next_slot;
        state.entries[slot] = Some(entry);
        state.next_slot = (slot + 1) % POLYMORPHIC_CACHE_SLOTS;
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
                | NamedPropertyCacheEntry::SharedSlot { .. }
                | NamedPropertyCacheEntry::PrototypeSlot { .. } => false,
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
                // A prototype entry says where a *read* resolved when the
                // receiver had no own property of that name. A write creates
                // the own property instead, which is the general path's job.
                NamedPropertyCacheEntry::Exact { .. }
                | NamedPropertyCacheEntry::LiteralShape { .. }
                | NamedPropertyCacheEntry::OwnSlot { .. }
                | NamedPropertyCacheEntry::PrototypeSlot { .. } => {}
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
