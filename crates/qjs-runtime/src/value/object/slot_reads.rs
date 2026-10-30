//! Pure own-slot probes and live data-slot access. Shared guards own a layout
//! or key lease, so recycling a numeric KeyId cannot validate a stale slot.
use super::{ObjectRef, OwnDataPropertyRead, OwnDataPropertyWrite};
use crate::{Value, property::atom::KeyLease, value::layout::Layout};
use std::rc::Rc;

impl ObjectRef {
    pub(crate) fn own_data_property_read(&self, key: &str) -> OwnDataPropertyRead {
        if self.0.flag(super::flags::MODULE_NAMESPACE) {
            return OwnDataPropertyRead::NeedsSlowPath;
        }
        if let Some(property) = self.virtual_index_property(key) {
            return OwnDataPropertyRead::Data(property.value);
        }
        self.0.properties.borrow().own_data_read(key)
    }
    pub(crate) fn own_data_slot(&self, key: &str) -> Option<usize> {
        if self.0.flag(super::flags::MODULE_NAMESPACE) || self.virtual_index_property(key).is_some()
        {
            return None;
        }
        let store = self.0.properties.borrow();
        store
            .data_slot(&store.domain().find(key)?)
            .expect("own domain")
    }
    pub(crate) fn any_storage_data_slot(&self, key: &str) -> Option<usize> {
        self.own_data_slot(key)
    }
    pub(crate) fn prototype_data_slot(&self, key: &str) -> Option<usize> {
        self.own_data_slot(key)
    }
    pub(crate) fn own_data_slot_value(&self, slot: usize) -> Option<Value> {
        if self.0.flag(super::flags::MODULE_NAMESPACE) {
            return None;
        }
        self.0.properties.borrow().load_data(slot)
    }
    pub(crate) fn prototype_data_slot_value(&self, slot: usize) -> Option<Value> {
        self.own_data_slot_value(slot)
    }
    pub(crate) fn own_data_slot_write(
        &self,
        slot: usize,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        if self.0.flag(super::flags::MODULE_NAMESPACE)
            || self.is_array_prototype_exotic()
            || self.is_typed_array_exotic()
            || self.is_symbol_primitive()
        {
            return None;
        }
        let result = self
            .0
            .properties
            .borrow_mut()
            .replace_data(slot, value.clone())?;
        if result {
            self.bump_value_revision();
        }
        Some(if result {
            OwnDataPropertyWrite::Written
        } else {
            OwnDataPropertyWrite::ReadOnly
        })
    }
    pub(crate) fn any_storage_data_slot_write(
        &self,
        slot: usize,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        self.own_data_slot_write(slot, value)
    }
    pub(crate) fn shared_data_slot(&self, key: &str) -> Option<(KeyLease, usize)> {
        if self.0.flag(super::flags::MODULE_NAMESPACE) || self.virtual_index_property(key).is_some()
        {
            return None;
        }
        let store = self.0.properties.borrow();
        let key = store.domain().find(key)?;
        let slot = store.data_slot(&key).expect("own domain")?;
        Some((key, slot))
    }
    pub(crate) fn shared_data_slot_value(&self, key: &KeyLease, slot: usize) -> Option<Value> {
        if self.0.flag(super::flags::MODULE_NAMESPACE) {
            return None;
        }
        let store = self.0.properties.borrow();
        store
            .slot_matches(slot, key)
            .then(|| store.load_data(slot))
            .flatten()
    }
    pub(crate) fn shared_data_slot_write(
        &self,
        key: &KeyLease,
        slot: usize,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        if self.0.flag(super::flags::MODULE_NAMESPACE)
            || self.is_array_prototype_exotic()
            || self.is_typed_array_exotic()
            || self.is_symbol_primitive()
        {
            return None;
        }
        let mut store = self.0.properties.borrow_mut();
        if !store.slot_matches(slot, key) {
            return None;
        }
        let result = store.replace_data(slot, value.clone())?;
        if result {
            self.bump_value_revision();
        }
        Some(if result {
            OwnDataPropertyWrite::Written
        } else {
            OwnDataPropertyWrite::ReadOnly
        })
    }
    pub(crate) fn shared_data_slot_number(&self, key: &KeyLease, slot: usize) -> Option<f64> {
        match self.shared_data_slot_value(key, slot)? {
            Value::Number(n) => Some(n),
            _ => None,
        }
    }
    pub(crate) fn shared_data_slot_write_number(
        &self,
        key: &KeyLease,
        slot: usize,
        number: f64,
    ) -> bool {
        matches!(
            self.shared_data_slot_write(key, slot, &Value::Number(number)),
            Some(OwnDataPropertyWrite::Written)
        )
    }
    pub(crate) fn layout_data_slot(&self, key: &str) -> Option<(Rc<Layout>, usize)> {
        if self.0.flag(super::flags::MODULE_NAMESPACE) || self.virtual_index_property(key).is_some()
        {
            return None;
        }
        let store = self.0.properties.borrow();
        let slot = store
            .data_slot(&store.domain().find(key)?)
            .expect("own domain")?;
        // A dictionary is private and versioned; it must never enter the
        // immutable shared-layout arm without its edit version.
        (!store.is_dictionary()).then(|| (store.layout().clone(), slot))
    }
    pub(crate) fn layout_data_slot_value(&self, layout: &Rc<Layout>, slot: usize) -> Option<Value> {
        if self.0.flag(super::flags::MODULE_NAMESPACE) {
            return None;
        }
        let store = self.0.properties.borrow();
        (Rc::ptr_eq(store.layout(), layout) && !store.is_dictionary())
            .then(|| store.load_data(slot))
            .flatten()
    }
    #[cfg(feature = "perf-counters")]
    pub(crate) fn storage_kind_for_trace(&self) -> &'static str {
        if self.0.properties.borrow().is_dictionary() {
            "dictionary"
        } else {
            "shape"
        }
    }
}

impl ObjectRef {
    /// Prelinked own read: one ID probe, with cache metadata from that probe.
    pub(crate) fn read_own_data_key(
        &self,
        key: &KeyLease,
    ) -> (OwnDataPropertyRead, Option<(KeyLease, usize)>) {
        if self.is_module_namespace_exotic() {
            return (OwnDataPropertyRead::NeedsSlowPath, None);
        }
        if let Some(p) = self.virtual_index_property(key.as_str()) {
            return (OwnDataPropertyRead::Data(p.value), None);
        }
        let store = self.0.properties.borrow();
        let key = if key.belongs_to(store.domain()) {
            key.clone()
        } else {
            let Some(key) = store.domain().find(key.as_str()) else {
                return (OwnDataPropertyRead::Missing, None);
            };
            key
        };
        let (result, slot) = store.read_data_slot(&key);
        (result, slot.map(|slot| (key, slot)))
    }
}
