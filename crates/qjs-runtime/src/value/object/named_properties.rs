//! Ordinary string and symbol operations over one NamedStore. Borrowed storage
//! never survives an observable callback. Property is a cold descriptor result.
use super::*;
use crate::property::atom::KeyLease;

impl ObjectRef {
    fn define_key(&self, key: &KeyLease, property: Property) {
        let mut store = self.0.properties.borrow_mut();
        let lookup = store
            .lookup(key)
            .expect("key normalized to receiver domain");
        let added = lookup.descriptor().is_none();
        lookup.define(property);
        if added && key.array_index().is_some() {
            self.0
                .index_property_count
                .set(self.0.index_property_count.get() + 1);
        }
        drop(store);
    }
    pub(crate) fn set(&self, key: String, value: Value) {
        self.set_shared_key(key.into(), value);
    }
    pub(crate) fn set_shared_key(&self, key: Rc<str>, value: Value) {
        if self.virtual_index_property(&key).is_some() {
            return;
        }
        let identity = key.as_ref() == "globalThis"
            && matches!(&value, Value::Object(global) if self.ptr_eq(global));
        let domain = self.domain();
        let lease = domain.find(&key).unwrap_or_else(|| domain.intern(&key));
        let mut store = self.0.properties.borrow_mut();
        let lookup = store.lookup(&lease).expect("own domain");
        let property = match lookup.descriptor() {
            Some(mut property) => {
                if property.is_accessor() || !property.writable {
                    return;
                }
                property.value = value;
                property
            }
            None => {
                if !self.is_extensible() {
                    return;
                }
                if lease.array_index().is_some() {
                    self.0
                        .index_property_count
                        .set(self.0.index_property_count.get() + 1);
                }
                Property::enumerable(value)
            }
        };
        lookup.define(property);
        drop(store);

        if identity {
            self.capture_realm_intrinsic_identity();
        }
    }
    pub(crate) fn define_property(&self, key: String, property: Property) {
        let realm_identity = key == "globalThis"
            && !property.is_accessor()
            && matches!(&property.value, Value::Object(global) if self.ptr_eq(global));
        let lease = self
            .domain()
            .find(&key)
            .unwrap_or_else(|| self.domain().intern(&key));
        self.define_key(&lease, property);
        if realm_identity {
            self.capture_realm_intrinsic_identity();
        }
    }
    pub(crate) fn define_shared_property(&self, key: Rc<str>, property: Property) {
        let lease = self
            .domain()
            .find(&key)
            .unwrap_or_else(|| self.domain().intern(&key));
        self.define_key(&lease, property);
    }
    pub(crate) fn define_symbol_property(&self, symbol: ObjectRef, property: Property) {
        let lease = self
            .domain()
            .find_symbol(&symbol)
            .unwrap_or_else(|| self.domain().intern_symbol(&symbol));
        self.define_key(&lease, property);
    }
    pub(crate) fn define_non_enumerable(&self, key: String, value: Value) {
        self.define_property(key, Property::non_enumerable(value));
    }
    pub(crate) fn set_internal_non_enumerable(&self, key: &str, value: Value) {
        let property = match self.own_property(key) {
            Some(mut p) => {
                p.value = value;
                p
            }
            None => Property::non_enumerable(value),
        };
        self.define_property(key.to_owned(), property);
    }
    pub(crate) fn has_own_property(&self, key: &str) -> bool {
        self.own_property(key).is_some()
    }
    pub(crate) fn has_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.own_symbol_property(symbol).is_some()
    }
    pub(crate) fn is_extensible(&self) -> bool {
        self.0.flag(super::flags::EXTENSIBLE)
    }
    pub(crate) fn prevent_extensions(&self) {
        self.0.set_flag(super::flags::EXTENSIBLE, false);
    }
    pub(crate) fn seal(&self) {
        self.prevent_extensions();
        self.0.properties.borrow_mut().transform_descriptors(false);
    }
    pub(crate) fn freeze(&self) {
        self.prevent_extensions();
        self.0.properties.borrow_mut().transform_descriptors(true);
    }
    pub(crate) fn is_sealed(&self) -> bool {
        !self.is_extensible()
            && self
                .0
                .properties
                .borrow()
                .descriptors()
                .all(|p| !p.configurable)
    }
    pub(crate) fn is_frozen(&self) -> bool {
        !self.is_extensible()
            && self
                .0
                .properties
                .borrow()
                .descriptors()
                .all(|p| !p.configurable && !p.writable)
    }
    pub(crate) fn append_string_property(&self, key: &str, suffix: &str) -> Option<Value> {
        if self.0.flag(super::flags::STRING_INDICES) && self.virtual_index_property(key).is_some() {
            return None;
        }
        let mut store = self.0.properties.borrow_mut();
        let lease = store.domain().find(key)?;
        let slot = store.data_slot(&lease).expect("own domain")?;
        let Value::String(text) = store.data_value_mut(slot)? else {
            return None;
        };
        crate::string::push_js_str(text.make_mut(), suffix);
        let result = Value::String(text.clone());

        Some(result)
    }
    pub(crate) fn own_property(&self, key: &str) -> Option<Property> {
        let mut property = self
            .virtual_index_property(key)
            .or_else(|| self.0.properties.borrow().get_text(key))?;
        if self.0.flag(super::flags::MODULE_NAMESPACE)
            && let Some(cold) = self.0.cold_if_present()
            && let Some(bindings) = cold.module_namespace_bindings.borrow().as_ref()
            && let Some(value) = bindings.value_for_export(key)
        {
            property.value = value;
        }
        Some(property)
    }
    pub(crate) fn write_existing_own_data_property(
        &self,
        key: &str,
        value: &Value,
    ) -> OwnDataPropertyWrite {
        self.write_existing_own_data_property_if(key, value, |_| true)
    }
    pub(crate) fn write_existing_own_data_property_if(
        &self,
        key: &str,
        value: &Value,
        guard: impl FnOnce(&Value) -> bool,
    ) -> OwnDataPropertyWrite {
        if self.0.flag(super::flags::MODULE_NAMESPACE) || self.is_array_prototype_exotic() {
            return OwnDataPropertyWrite::NeedsSlowPath;
        }
        if self.0.flag(super::flags::STRING_INDICES) && self.virtual_index_property(key).is_some() {
            return OwnDataPropertyWrite::ReadOnly;
        }
        let identity =
            key == "globalThis" && matches!(value, Value::Object(global) if self.ptr_eq(global));
        let result = self
            .0
            .properties
            .borrow_mut()
            .write_existing_data(key, value, guard);
        if matches!(result, OwnDataPropertyWrite::Written) && identity {
            self.capture_realm_intrinsic_identity();
        }
        result
    }
    pub(crate) fn own_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        let store = self.0.properties.borrow();
        let lease = store.domain().find_symbol(symbol)?;
        store.get(&lease).expect("own domain")
    }
    fn delete_key(&self, lease: &KeyLease) -> bool {
        let mut store = self.0.properties.borrow_mut();
        let lookup = store.lookup(lease).expect("own domain");
        if lookup.descriptor().is_some_and(|p| !p.configurable) {
            return false;
        }
        let removed = lookup.remove().is_some();
        drop(store);
        if removed && lease.array_index().is_some() {
            self.0
                .index_property_count
                .set(self.0.index_property_count.get() - 1);
        }
        true
    }
    pub(crate) fn delete_own_property(&self, key: &str) -> bool {
        if self.0.flag(super::flags::STRING_INDICES) && self.virtual_index_property(key).is_some() {
            return false;
        }
        let Some(lease) = self.domain().find(key) else {
            return true;
        };
        self.delete_key(&lease)
    }
    pub(crate) fn delete_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        let Some(lease) = self.domain().find_symbol(symbol) else {
            return true;
        };
        self.delete_key(&lease)
    }
    pub(crate) fn own_property_symbols(&self) -> Vec<ObjectRef> {
        self.0
            .properties
            .borrow()
            .keys()
            .iter()
            .filter_map(|key| key.symbol().cloned())
            .collect()
    }
}
