use std::{
    cell::{Cell, RefCell},
    collections::{BTreeSet, HashMap},
    fmt,
    rc::Rc,
};

use super::{ObjectRef, Property, Prototype, Value};
use crate::CallEnv;

const MAX_DENSE_STORAGE_LENGTH: usize = 1_000_000;
const MAX_ARRAY_INDEX: usize = u32::MAX as usize - 1;

/// Array storage reference.
#[derive(Clone)]
pub struct ArrayRef {
    elements: Rc<RefCell<Vec<Value>>>,
    holes: Rc<RefCell<BTreeSet<usize>>>,
    properties: Rc<RefCell<HashMap<String, Property>>>,
    symbol_properties: Rc<RefCell<Vec<(ObjectRef, Property)>>>,
    length: Rc<Cell<usize>>,
    length_writable: Rc<Cell<bool>>,
    extensible: Rc<Cell<bool>>,
    sealed: Rc<Cell<bool>>,
    frozen: Rc<Cell<bool>>,
    prototype: Rc<RefCell<Option<Option<Prototype>>>>,
}

impl ArrayRef {
    pub(crate) fn new(elements: Vec<Value>) -> Self {
        Self::new_sparse(elements, Vec::new())
    }

    pub(crate) fn new_with_length(length: usize) -> Self {
        Self {
            elements: Rc::new(RefCell::new(Vec::new())),
            holes: Rc::new(RefCell::new(BTreeSet::new())),
            properties: Rc::new(RefCell::new(HashMap::new())),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            length: Rc::new(Cell::new(length)),
            length_writable: Rc::new(Cell::new(true)),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            prototype: Rc::new(RefCell::new(None)),
        }
    }

    pub(crate) fn new_sparse(elements: Vec<Value>, holes: Vec<usize>) -> Self {
        let length = elements.len();
        let holes: BTreeSet<_> = holes.into_iter().collect();
        let all_holes = holes.len() == length && (0..length).all(|index| holes.contains(&index));
        let (elements, holes) = if all_holes {
            (Vec::new(), BTreeSet::new())
        } else {
            (elements, holes)
        };
        Self {
            elements: Rc::new(RefCell::new(elements)),
            holes: Rc::new(RefCell::new(holes)),
            properties: Rc::new(RefCell::new(HashMap::new())),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            length: Rc::new(Cell::new(length)),
            length_writable: Rc::new(Cell::new(true)),
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
        self.length.get()
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
        let len = self.length.get();
        let dense_len = self.elements.borrow().len();
        let mut indices: Vec<_> = (0..dense_len)
            .filter(|index| !holes.contains(index))
            .collect();
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

    pub(crate) fn symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.own_symbol_property(symbol)
    }

    pub(crate) fn to_vec(&self) -> Vec<Value> {
        self.elements.borrow().clone()
    }

    pub(crate) fn pop(&self) -> Option<Value> {
        let mut elements = self.elements.borrow_mut();
        let index = self.length.get().checked_sub(1)?;
        self.length.set(index);
        self.holes.borrow_mut().remove(&index);
        self.properties.borrow_mut().remove(&index.to_string());
        if index + 1 == elements.len() {
            elements.pop()
        } else {
            None
        }
    }

    pub(crate) fn replace_with(&self, values: Vec<Value>) {
        if self.frozen.get() {
            return;
        }
        if values.len() > self.length.get() && !self.extensible.get() {
            return;
        }
        self.length.set(values.len());
        *self.elements.borrow_mut() = values;
        self.holes.borrow_mut().clear();
    }

    /// Whether `index` can be written straight into dense storage with the same
    /// observable result as the generic property-set path, assuming the array
    /// uses the default Array.prototype and that prototype owns no indexed
    /// property (both verified by the caller). Requires the index to stay in
    /// dense range, the array to be mutable and extensible enough to take the
    /// write, and to have no own special descriptor at the index (which could be
    /// an accessor or a non-writable data property).
    /// Whether this array has no [[Prototype]] override and therefore resolves
    /// to the realm's default Array.prototype.
    pub(crate) fn uses_default_prototype(&self) -> bool {
        self.prototype.borrow().is_none()
    }

    /// Reads every element `0..length` directly out of dense storage as an
    /// argument list, returning `None` when a generic property lookup is needed
    /// instead. The fast path requires fully dense storage (length matches the
    /// element vector with no holes), no own indexed/length descriptors that
    /// could intercept the read, the default prototype, and that prototype owning
    /// no indexed property whose value an absent element would inherit.
    pub(crate) fn dense_argument_values(&self, env: &CallEnv) -> Option<Vec<Value>> {
        let elements = self.elements.borrow();
        if self.length.get() != elements.len() || !self.holes.borrow().is_empty() {
            return None;
        }
        if !self.properties.borrow().is_empty() {
            return None;
        }
        match self.prototype.borrow().as_ref() {
            Some(Some(_)) => return None,
            Some(None) => {}
            None => {
                if crate::array_prototype(env)
                    .is_some_and(|prototype| prototype.has_own_index_property())
                {
                    return None;
                }
            }
        }
        Some(elements.clone())
    }

    pub(crate) fn dense_index_store_eligible(&self, index: usize) -> bool {
        if index >= MAX_DENSE_STORAGE_LENGTH || self.frozen.get() || !self.length_writable.get() {
            return false;
        }
        let within_length = index < self.length.get();
        if !within_length && !self.extensible.get() {
            return false;
        }
        let properties = self.properties.borrow();
        properties.is_empty() || !properties.contains_key(&index.to_string())
    }

    /// Whether `CreateDataProperty` for `index` can be represented as a dense
    /// element write. This is stricter than ordinary array assignment: it only
    /// accepts mutable, extensible arrays without own special descriptors at
    /// the target index, so callers can fall back to the generic descriptor
    /// path whenever a failure or descriptor-preserving overwrite is possible.
    pub(crate) fn dense_data_property_eligible(&self, index: usize) -> bool {
        if index > MAX_ARRAY_INDEX
            || index >= MAX_DENSE_STORAGE_LENGTH
            || self.frozen.get()
            || self.sealed.get()
            || !self.extensible.get()
            || !self.length_writable.get()
        {
            return false;
        }
        let properties = self.properties.borrow();
        properties.is_empty() || !properties.contains_key(&index.to_string())
    }

    pub(crate) fn set(&self, index: usize, value: Value) {
        if index > MAX_ARRAY_INDEX {
            self.set_property(index.to_string(), value);
            return;
        }
        if index >= self.length.get() {
            if self.frozen.get() || !self.extensible.get() || !self.length_writable.get() {
                return;
            }
            self.length.set(index + 1);
        }
        let mut elements = self.elements.borrow_mut();
        let mut holes = self.holes.borrow_mut();
        if index >= elements.len() {
            if index >= MAX_DENSE_STORAGE_LENGTH {
                drop(elements);
                drop(holes);
                self.properties
                    .borrow_mut()
                    .insert(index.to_string(), Property::enumerable(value));
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

    pub(crate) fn delete_indices_from(&self, length: usize) -> Option<usize> {
        let dense_len = self.elements.borrow().len();
        if length < dense_len {
            for index in (length..dense_len).rev() {
                if !self.delete_index(index) {
                    return Some(index + 1);
                }
            }
        }

        let mut sparse_indices: Vec<_> = self
            .properties
            .borrow()
            .keys()
            .filter_map(|key| key.parse::<usize>().ok())
            .filter(|index| *index >= length && *index <= MAX_ARRAY_INDEX)
            .collect();
        sparse_indices.sort_unstable_by(|left, right| right.cmp(left));
        for index in sparse_indices {
            if !self.delete_index(index) {
                return Some(index + 1);
            }
        }
        None
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
            if index > MAX_ARRAY_INDEX {
                self.properties.borrow_mut().insert(key, property);
                return;
            }
            if index >= self.length.get() {
                if !self.length_writable.get() {
                    return;
                }
                self.length.set(index + 1);
            }
            let mut elements = self.elements.borrow_mut();
            let mut holes = self.holes.borrow_mut();
            if index >= elements.len() {
                if !self.extensible.get() {
                    return;
                }
                if index >= MAX_DENSE_STORAGE_LENGTH {
                    drop(elements);
                    drop(holes);
                    self.properties.borrow_mut().insert(key, property);
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

    pub(crate) fn define_symbol_property(&self, symbol: ObjectRef, property: Property) {
        let mut properties = self.symbol_properties.borrow_mut();
        if let Some((_, existing)) = properties
            .iter_mut()
            .find(|(existing_symbol, _)| existing_symbol.ptr_eq(&symbol))
        {
            *existing = property;
            return;
        }
        properties.push((symbol, property));
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
        if self.frozen.get() || !self.length_writable.get() {
            return;
        }
        let old_len = self.length.get();
        if length > old_len && !self.extensible.get() {
            return;
        }
        self.length.set(length);
        if length < elements.len() {
            elements.truncate(length);
        } else if length <= MAX_DENSE_STORAGE_LENGTH {
            elements.resize(length, Value::Undefined);
        }
        let mut holes = self.holes.borrow_mut();
        holes.retain(|index| *index < length);
        if length > old_len && length <= MAX_DENSE_STORAGE_LENGTH {
            holes.extend(old_len..length);
        }
        if length < old_len {
            let mut sparse_indices: Vec<_> = self
                .properties
                .borrow()
                .keys()
                .filter_map(|key| key.parse::<usize>().ok())
                .filter(|index| *index >= length && *index <= MAX_ARRAY_INDEX)
                .collect();
            sparse_indices.sort_unstable_by(|left, right| right.cmp(left));
            let mut properties = self.properties.borrow_mut();
            for index in sparse_indices {
                let key = index.to_string();
                if properties
                    .get(&key)
                    .is_some_and(|property| !property.configurable)
                {
                    self.length.set(index + 1);
                    return;
                }
                properties.remove(&key);
            }
        }
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.extensible.get()
    }

    pub(crate) fn has_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.symbol_properties
            .borrow()
            .iter()
            .any(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
    }

    pub(crate) fn is_length_writable(&self) -> bool {
        self.length_writable.get()
    }

    pub(crate) fn set_length_writable(&self, writable: bool) {
        self.length_writable.set(writable);
    }

    pub(crate) fn prevent_extensions(&self) {
        self.extensible.set(false);
    }

    pub(crate) fn seal(&self) {
        self.prevent_extensions();
        for (_, property) in self.symbol_properties.borrow_mut().iter_mut() {
            property.make_non_configurable();
        }
        self.sealed.set(true);
    }

    pub(crate) fn is_sealed(&self) -> bool {
        !self.extensible.get() && self.sealed.get()
    }

    pub(crate) fn freeze(&self) {
        self.seal();
        for (_, property) in self.symbol_properties.borrow_mut().iter_mut() {
            property.freeze_data();
        }
        self.frozen.set(true);
        self.length_writable.set(false);
    }

    pub(crate) fn is_frozen(&self) -> bool {
        !self.extensible.get() && self.sealed.get() && self.frozen.get()
    }

    pub(crate) fn prototype_override(&self) -> Option<Option<ObjectRef>> {
        self.prototype
            .borrow()
            .clone()
            .map(|prototype| prototype.and_then(|prototype| prototype.as_object()))
    }

    pub(crate) fn prototype_slot_override(&self) -> Option<Option<Prototype>> {
        self.prototype.borrow().clone()
    }

    pub(crate) fn own_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.symbol_properties
            .borrow()
            .iter()
            .find(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
            .map(|(_, property)| property.clone())
    }

    pub(crate) fn delete_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        let mut properties = self.symbol_properties.borrow_mut();
        let Some(index) = properties
            .iter()
            .position(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
        else {
            return true;
        };
        if !properties[index].1.configurable {
            return false;
        }
        properties.remove(index);
        true
    }

    pub(crate) fn own_property_symbols(&self) -> Vec<ObjectRef> {
        self.symbol_properties
            .borrow()
            .iter()
            .map(|(symbol, _)| symbol.clone())
            .collect()
    }

    pub(crate) fn set_prototype_slot(&self, prototype: Option<Prototype>) -> Result<(), ()> {
        if matches!(
            self.prototype.borrow().as_ref(),
            Some(current) if same_prototype_slot(current.as_ref(), prototype.as_ref())
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
            .field("len", &self.length.get())
            .finish()
    }
}

fn same_prototype_slot(left: Option<&Prototype>, right: Option<&Prototype>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.ptr_eq(right),
        _ => false,
    }
}
