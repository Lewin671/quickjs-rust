use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use super::{ObjectRef, Value};

/// Map storage reference.
#[derive(Clone)]
pub struct MapRef {
    entries: Rc<RefCell<Vec<(Value, Value)>>>,
    object: ObjectRef,
}

impl MapRef {
    pub(crate) fn new(prototype: Option<ObjectRef>) -> Self {
        let object = ObjectRef::with_prototype(HashMap::new(), prototype);
        object.set_to_string_tag("Map");
        Self {
            entries: Rc::new(RefCell::new(Vec::new())),
            object,
        }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.entries, &other.entries)
    }

    pub(crate) fn object(&self) -> ObjectRef {
        self.object.clone()
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.borrow().len()
    }

    pub(crate) fn get(&self, key: &Value) -> Option<Value> {
        self.entries
            .borrow()
            .iter()
            .find(|(entry_key, _)| entry_key.same_value_zero(key))
            .map(|(_, value)| value.clone())
    }

    pub(crate) fn has(&self, key: &Value) -> bool {
        self.entries
            .borrow()
            .iter()
            .any(|(entry_key, _)| entry_key.same_value_zero(key))
    }

    pub(crate) fn set(&self, key: Value, value: Value) {
        let mut entries = self.entries.borrow_mut();
        if let Some((_, entry_value)) = entries
            .iter_mut()
            .find(|(entry_key, _)| entry_key.same_value_zero(&key))
        {
            *entry_value = value;
            return;
        }
        entries.push((canonical_map_key(key), value));
    }

    pub(crate) fn delete(&self, key: &Value) -> bool {
        let mut entries = self.entries.borrow_mut();
        let Some(index) = entries
            .iter()
            .position(|(entry_key, _)| entry_key.same_value_zero(key))
        else {
            return false;
        };
        entries.remove(index);
        true
    }

    pub(crate) fn clear(&self) {
        self.entries.borrow_mut().clear();
    }

    pub(crate) fn entries(&self) -> Vec<(Value, Value)> {
        self.entries.borrow().clone()
    }
}

impl fmt::Debug for MapRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MapRef")
            .field("len", &self.entries.borrow().len())
            .finish()
    }
}

fn canonical_map_key(key: Value) -> Value {
    if matches!(key, Value::Number(value) if value == 0.0) {
        Value::Number(0.0)
    } else {
        key
    }
}
