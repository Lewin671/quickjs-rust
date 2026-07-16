use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use super::{ObjectRef, Prototype, Value};

/// Set storage reference.
#[derive(Clone)]
pub struct SetRef(Rc<SetData>);

struct SetData {
    entries: RefCell<Vec<Value>>,
    object: ObjectRef,
}

impl SetRef {
    pub(crate) fn new(prototype: Option<ObjectRef>) -> Self {
        Self::with_prototype_slot(prototype.map(Prototype::Object))
    }

    pub(crate) fn with_prototype_slot(prototype: Option<Prototype>) -> Self {
        let object = ObjectRef::with_prototype_slot(HashMap::new(), prototype);
        object.set_to_string_tag("Set");
        Self(Rc::new(SetData {
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

    pub(crate) fn has(&self, value: &Value) -> bool {
        self.0
            .entries
            .borrow()
            .iter()
            .any(|entry| entry.same_value_zero(value))
    }

    pub(crate) fn add(&self, value: Value) {
        let mut entries = self.0.entries.borrow_mut();
        if entries.iter().any(|entry| entry.same_value_zero(&value)) {
            return;
        }
        entries.push(canonical_set_value(value));
    }

    pub(crate) fn delete(&self, value: &Value) -> bool {
        let mut entries = self.0.entries.borrow_mut();
        let Some(index) = entries
            .iter()
            .position(|entry| entry.same_value_zero(value))
        else {
            return false;
        };
        entries.remove(index);
        true
    }

    pub(crate) fn clear(&self) {
        self.0.entries.borrow_mut().clear();
    }

    pub(crate) fn values(&self) -> Vec<Value> {
        self.0.entries.borrow().clone()
    }
}

impl fmt::Debug for SetRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SetRef")
            .field("len", &self.0.entries.borrow().len())
            .finish()
    }
}

fn canonical_set_value(value: Value) -> Value {
    if matches!(value, Value::Number(number) if number == 0.0) {
        Value::Number(0.0)
    } else {
        value
    }
}
