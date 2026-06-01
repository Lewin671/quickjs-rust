use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
};

use super::{Property, Value};

/// Object storage reference.
#[derive(Clone)]
pub struct ObjectRef {
    properties: Rc<RefCell<HashMap<String, Property>>>,
    extensible: Rc<Cell<bool>>,
    prototype: Rc<RefCell<Option<ObjectRef>>>,
    to_string_tag: Rc<RefCell<Option<String>>>,
}

impl fmt::Debug for ObjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ObjectRef")
            .field("properties", &self.properties.borrow().len())
            .field("has_prototype", &self.prototype.borrow().is_some())
            .field("to_string_tag", &self.to_string_tag.borrow())
            .finish()
    }
}

impl ObjectRef {
    pub(crate) fn new(properties: HashMap<String, Value>) -> Self {
        Self::with_prototype(properties, None)
    }

    pub(crate) fn with_prototype(
        properties: HashMap<String, Value>,
        prototype: Option<ObjectRef>,
    ) -> Self {
        Self {
            properties: Rc::new(RefCell::new(
                properties
                    .into_iter()
                    .map(|(key, value)| (key, Property::enumerable(value)))
                    .collect(),
            )),
            extensible: Rc::new(Cell::new(true)),
            prototype: Rc::new(RefCell::new(prototype)),
            to_string_tag: Rc::new(RefCell::new(None)),
        }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.properties, &other.properties)
    }

    pub(crate) fn get(&self, key: &str) -> Option<Value> {
        self.properties
            .borrow()
            .get(key)
            .map(|property| property.value.clone())
            .or_else(|| {
                self.prototype
                    .borrow()
                    .as_ref()
                    .and_then(|proto| proto.get(key))
            })
    }

    pub(crate) fn set(&self, key: String, value: Value) {
        let mut properties = self.properties.borrow_mut();
        if let Some(property) = properties.get_mut(&key) {
            if property.writable {
                property.value = value;
            }
            return;
        }
        if !self.extensible.get() {
            return;
        }
        properties.insert(key, Property::enumerable(value));
    }

    pub(crate) fn define_property(&self, key: String, property: Property) {
        self.properties.borrow_mut().insert(key, property);
    }

    pub(crate) fn define_non_enumerable(&self, key: String, value: Value) {
        self.properties
            .borrow_mut()
            .insert(key, Property::non_enumerable(value));
    }

    pub(crate) fn contains_property(&self, key: &str) -> bool {
        self.properties.borrow().contains_key(key)
            || self
                .prototype
                .borrow()
                .as_ref()
                .is_some_and(|proto| proto.contains_property(key))
    }

    pub(crate) fn has_own_property(&self, key: &str) -> bool {
        self.properties.borrow().contains_key(key)
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.extensible.get()
    }

    pub(crate) fn prevent_extensions(&self) {
        self.extensible.set(false);
    }

    pub(crate) fn seal(&self) {
        self.prevent_extensions();
        for property in self.properties.borrow_mut().values_mut() {
            property.make_non_configurable();
        }
    }

    pub(crate) fn is_sealed(&self) -> bool {
        !self.extensible.get()
            && self
                .properties
                .borrow()
                .values()
                .all(|property| !property.configurable)
    }

    pub(crate) fn freeze(&self) {
        self.prevent_extensions();
        for property in self.properties.borrow_mut().values_mut() {
            property.freeze_data();
        }
    }

    pub(crate) fn is_frozen(&self) -> bool {
        !self.extensible.get()
            && self
                .properties
                .borrow()
                .values()
                .all(|property| !property.configurable && !property.writable)
    }

    pub(crate) fn own_property(&self, key: &str) -> Option<Property> {
        self.properties.borrow().get(key).cloned()
    }

    pub(crate) fn delete_own_property(&self, key: &str) -> bool {
        let mut properties = self.properties.borrow_mut();
        if properties
            .get(key)
            .is_some_and(|property| !property.configurable)
        {
            return false;
        }
        properties.remove(key);
        true
    }

    pub(crate) fn own_property_keys(&self) -> Vec<String> {
        let mut keys: Vec<_> = self
            .properties
            .borrow()
            .iter()
            .filter(|(_, property)| property.enumerable)
            .map(|(key, _)| key.clone())
            .collect();
        keys.sort();
        keys
    }

    pub(crate) fn own_property_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.properties.borrow().keys().cloned().collect();
        names.sort();
        names
    }

    pub(crate) fn has_prototype(&self, prototype: &ObjectRef) -> bool {
        self.prototype
            .borrow()
            .as_ref()
            .is_some_and(|proto| proto.ptr_eq(prototype) || proto.has_prototype(prototype))
    }

    pub(crate) fn prototype(&self) -> Option<ObjectRef> {
        self.prototype.borrow().clone()
    }

    pub(crate) fn set_prototype(&self, prototype: Option<ObjectRef>) -> Result<(), ()> {
        if self
            .prototype()
            .as_ref()
            .map_or(prototype.is_none(), |current| {
                prototype
                    .as_ref()
                    .is_some_and(|prototype| current.ptr_eq(prototype))
            })
        {
            return Ok(());
        }
        if !self.extensible.get() {
            return Err(());
        }
        if prototype
            .as_ref()
            .is_some_and(|prototype| prototype.ptr_eq(self) || prototype.has_prototype(self))
        {
            return Err(());
        }
        *self.prototype.borrow_mut() = prototype;
        Ok(())
    }

    pub(crate) fn to_string_tag(&self) -> Option<String> {
        self.to_string_tag.borrow().clone().or_else(|| {
            self.prototype
                .borrow()
                .as_ref()
                .and_then(ObjectRef::to_string_tag)
        })
    }

    pub(crate) fn set_to_string_tag(&self, tag: &str) {
        *self.to_string_tag.borrow_mut() = Some(tag.to_owned());
    }
}
