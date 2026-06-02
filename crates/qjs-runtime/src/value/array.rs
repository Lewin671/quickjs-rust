use std::{
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashMap},
    fmt,
    rc::Rc,
};

use super::{ObjectRef, Property, Value};

/// Array storage reference.
#[derive(Clone)]
pub struct ArrayRef {
    elements: Rc<RefCell<Vec<Value>>>,
    holes: Rc<RefCell<BTreeSet<usize>>>,
    properties: Rc<RefCell<HashMap<String, Property>>>,
    extensible: Rc<Cell<bool>>,
    sealed: Rc<Cell<bool>>,
    frozen: Rc<Cell<bool>>,
    prototype: Rc<RefCell<Option<Option<ObjectRef>>>>,
}

impl ArrayRef {
    pub(crate) fn new(elements: Vec<Value>) -> Self {
        Self::new_sparse(elements, Vec::new())
    }

    pub(crate) fn new_sparse(elements: Vec<Value>, holes: Vec<usize>) -> Self {
        Self {
            elements: Rc::new(RefCell::new(elements)),
            holes: Rc::new(RefCell::new(holes.into_iter().collect())),
            properties: Rc::new(RefCell::new(HashMap::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            prototype: Rc::new(RefCell::new(None)),
        }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.elements, &other.elements)
    }

    pub(crate) fn len(&self) -> usize {
        self.elements.borrow().len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.elements.borrow().is_empty()
    }

    pub(crate) fn get(&self, index: usize) -> Option<Value> {
        if self.holes.borrow().contains(&index) {
            return None;
        }
        self.elements.borrow().get(index).cloned()
    }

    pub(crate) fn has_index(&self, index: usize) -> bool {
        index < self.elements.borrow().len() && !self.holes.borrow().contains(&index)
    }

    pub(crate) fn present_indices(&self) -> Vec<usize> {
        let holes = self.holes.borrow();
        let len = self.elements.borrow().len();
        let mut indices: Vec<_> = (0..len).filter(|index| !holes.contains(index)).collect();
        indices.extend(
            self.properties
                .borrow()
                .keys()
                .filter_map(|key| key.parse::<usize>().ok())
                .filter(|index| *index < len),
        );
        indices.sort_unstable();
        indices.dedup();
        indices
    }

    pub(crate) fn property(&self, key: &str) -> Option<Property> {
        self.properties.borrow().get(key).cloned()
    }

    pub(crate) fn to_vec(&self) -> Vec<Value> {
        self.elements.borrow().clone()
    }

    pub(crate) fn push(&self, value: Value) -> usize {
        let mut elements = self.elements.borrow_mut();
        elements.push(value);
        elements.len()
    }

    pub(crate) fn pop(&self) -> Option<Value> {
        let mut elements = self.elements.borrow_mut();
        let index = elements.len().checked_sub(1)?;
        self.holes.borrow_mut().remove(&index);
        elements.pop()
    }

    pub(crate) fn shift(&self) -> Option<Value> {
        let mut elements = self.elements.borrow_mut();
        if elements.is_empty() {
            None
        } else {
            let mut holes = self.holes.borrow_mut();
            *holes = holes
                .iter()
                .filter_map(|index| index.checked_sub(1))
                .collect();
            Some(elements.remove(0))
        }
    }

    pub(crate) fn unshift(&self, values: &[Value]) -> usize {
        let mut elements = self.elements.borrow_mut();
        let offset = values.len();
        if offset > 0 {
            let mut holes = self.holes.borrow_mut();
            *holes = holes.iter().map(|index| index + offset).collect();
        }
        elements.splice(0..0, values.iter().cloned());
        elements.len()
    }

    pub(crate) fn reverse(&self) {
        let len = self.elements.borrow().len();
        self.elements.borrow_mut().reverse();
        let mut holes = self.holes.borrow_mut();
        *holes = holes.iter().map(|index| len - 1 - index).collect();
    }

    pub(crate) fn replace_with(&self, values: Vec<Value>) {
        if self.frozen.get() {
            return;
        }
        if values.len() > self.elements.borrow().len() && !self.extensible.get() {
            return;
        }
        *self.elements.borrow_mut() = values;
        self.holes.borrow_mut().clear();
    }

    pub(crate) fn splice(&self, start: usize, delete_count: usize, items: &[Value]) -> Vec<Value> {
        if self.frozen.get() {
            return Vec::new();
        }

        let mut elements = self.elements.borrow_mut();
        let end = start + delete_count.min(elements.len().saturating_sub(start));
        let new_len = elements.len() - (end - start) + items.len();
        if new_len > elements.len() && !self.extensible.get() {
            return Vec::new();
        }
        self.holes.borrow_mut().clear();
        elements.splice(start..end, items.iter().cloned()).collect()
    }

    pub(crate) fn fill(&self, start: usize, end: usize, value: Value) {
        let mut elements = self.elements.borrow_mut();
        for element in &mut elements[start..end] {
            *element = value.clone();
        }
    }

    pub(crate) fn set(&self, index: usize, value: Value) {
        let mut elements = self.elements.borrow_mut();
        let mut holes = self.holes.borrow_mut();
        if index >= elements.len() {
            if !self.extensible.get() {
                return;
            }
            let old_len = elements.len();
            elements.resize(index + 1, Value::Undefined);
            holes.extend(old_len..index);
        }
        if self.frozen.get() {
            return;
        }
        elements[index] = value;
        holes.remove(&index);
    }

    pub(crate) fn delete_index(&self, index: usize) -> bool {
        let key = index.to_string();
        let mut properties = self.properties.borrow_mut();
        if properties
            .get(&key)
            .is_some_and(|property| !property.configurable)
        {
            return false;
        }
        properties.remove(&key);
        drop(properties);

        if index < self.elements.borrow().len() {
            self.holes.borrow_mut().insert(index);
        }
        true
    }

    pub(crate) fn set_property(&self, key: String, value: Value) {
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
        if let Ok(index) = key.parse::<usize>() {
            let mut elements = self.elements.borrow_mut();
            let mut holes = self.holes.borrow_mut();
            if index >= elements.len() {
                if !self.extensible.get() {
                    return;
                }
                let old_len = elements.len();
                elements.resize(index + 1, Value::Undefined);
                holes.extend(old_len..=index);
            } else {
                holes.insert(index);
            }
        }
        self.properties.borrow_mut().insert(key, property);
    }

    pub(crate) fn delete_property(&self, key: &str) -> bool {
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

    pub(crate) fn property_keys(&self) -> Vec<String> {
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

    pub(crate) fn property_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.properties.borrow().keys().cloned().collect();
        names.sort();
        names
    }

    pub(crate) fn set_len(&self, length: usize) {
        let mut elements = self.elements.borrow_mut();
        if self.frozen.get() {
            return;
        }
        if length > elements.len() && !self.extensible.get() {
            return;
        }
        elements.resize(length, Value::Undefined);
        self.holes.borrow_mut().retain(|index| *index < length);
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.extensible.get()
    }

    pub(crate) fn prevent_extensions(&self) {
        self.extensible.set(false);
    }

    pub(crate) fn seal(&self) {
        self.prevent_extensions();
        self.sealed.set(true);
    }

    pub(crate) fn is_sealed(&self) -> bool {
        !self.extensible.get() && self.sealed.get()
    }

    pub(crate) fn freeze(&self) {
        self.seal();
        self.frozen.set(true);
    }

    pub(crate) fn is_frozen(&self) -> bool {
        !self.extensible.get() && self.sealed.get() && self.frozen.get()
    }

    pub(crate) fn prototype_override(&self) -> Option<Option<ObjectRef>> {
        self.prototype.borrow().clone()
    }

    pub(crate) fn set_prototype(&self, prototype: Option<ObjectRef>) -> Result<(), ()> {
        if matches!(
            self.prototype.borrow().as_ref(),
            Some(current) if same_prototype(current, &prototype)
        ) {
            return Ok(());
        }
        if !self.extensible.get() {
            return Err(());
        }
        *self.prototype.borrow_mut() = Some(prototype);
        Ok(())
    }
}

impl fmt::Debug for ArrayRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ArrayRef")
            .field("len", &self.elements.borrow().len())
            .finish()
    }
}

fn same_prototype(left: &Option<ObjectRef>, right: &Option<ObjectRef>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.ptr_eq(right),
        _ => false,
    }
}
