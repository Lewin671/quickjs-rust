//! Pure own-slot probes and live data-slot access. Shared guards own a layout
//! or key lease, so recycling a numeric KeyId cannot validate a stale slot.
use super::{ObjectRef, OwnDataPropertyRead, OwnDataPropertyWrite};
use crate::Value;
#[cfg(test)]
use crate::value::layout::Layout;
#[cfg(test)]
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
    #[cfg(test)]
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
    #[cfg(test)]
    pub(crate) fn layout_data_slot_value(&self, layout: &Rc<Layout>, slot: usize) -> Option<Value> {
        if self.0.flag(super::flags::MODULE_NAMESPACE) {
            return None;
        }
        let store = self.0.properties.borrow();
        (Rc::ptr_eq(store.layout(), layout) && !store.is_dictionary())
            .then(|| store.load_data(slot))
            .flatten()
    }
}

impl ObjectRef {
    pub(crate) fn with_named<R>(
        &self,
        read: impl FnOnce(&super::super::layout::NamedStore) -> R,
    ) -> R {
        read(&self.0.properties.borrow())
    }
    pub(crate) fn with_named_mut<R>(
        &self,
        write: impl FnOnce(&mut super::super::layout::NamedStore) -> R,
    ) -> R {
        write(&mut self.0.properties.borrow_mut())
    }
}

impl ObjectRef {
    pub(crate) fn named_kind(&self) -> Option<u8> {
        if self.is_module_namespace_exotic() || self.is_symbol_primitive() {
            return None;
        }
        Some(if self.is_typed_array_exotic() {
            1
        } else if self.is_array_prototype_exotic() {
            2
        } else if self.0.flag(super::flags::STRING_INDICES) {
            3
        } else {
            0
        })
    }
}
