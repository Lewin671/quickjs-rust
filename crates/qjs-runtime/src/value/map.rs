use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use super::{ObjectRef, Prototype, Value};

/// Map storage reference.
#[derive(Clone)]
pub struct MapRef(Rc<MapData>);

struct MapData {
    entries: RefCell<Vec<(Value, Value)>>,
    object: ObjectRef,
}

impl MapRef {
    pub(crate) fn new(prototype: Option<ObjectRef>) -> Self {
        Self::with_prototype_slot(prototype.map(Prototype::Object))
    }

    pub(crate) fn with_prototype_slot(prototype: Option<Prototype>) -> Self {
        let object = ObjectRef::with_prototype_slot(HashMap::new(), prototype);
        object.set_to_string_tag("Map");
        Self(Rc::new(MapData {
            entries: RefCell::new(Vec::new()),
            object,
        }))
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn object(&self) -> ObjectRef {
        self.0.object.clone()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.entries.borrow().len()
    }

    pub(crate) fn get(&self, key: &Value) -> Option<Value> {
        self.0
            .entries
            .borrow()
            .iter()
            .find(|(entry_key, _)| entry_key.same_value_zero(key))
            .map(|(_, value)| value.clone())
    }

    pub(crate) fn has(&self, key: &Value) -> bool {
        self.0
            .entries
            .borrow()
            .iter()
            .any(|(entry_key, _)| entry_key.same_value_zero(key))
    }

    pub(crate) fn set(&self, key: Value, value: Value) {
        let mut entries = self.0.entries.borrow_mut();
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
        let mut entries = self.0.entries.borrow_mut();
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
        self.0.entries.borrow_mut().clear();
    }

    pub(crate) fn entries(&self) -> Vec<(Value, Value)> {
        self.0.entries.borrow().clone()
    }
}

impl fmt::Debug for MapRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MapRef")
            .field("len", &self.0.entries.borrow().len())
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
