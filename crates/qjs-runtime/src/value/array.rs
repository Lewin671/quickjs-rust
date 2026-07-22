use std::{
    cell::{Cell, OnceCell, RefCell},
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
pub struct ArrayRef(Rc<ArrayData>);

struct ArrayData {
    elements: RefCell<Vec<Value>>,
    length: Cell<usize>,
    length_writable: Cell<bool>,
    extensible: Cell<bool>,
    sealed: Cell<bool>,
    frozen: Cell<bool>,
    cold: OnceCell<Box<ArrayColdData>>,
}

#[derive(Default)]
struct ArrayColdData {
    holes: RefCell<BTreeSet<usize>>,
    properties: RefCell<HashMap<String, Property>>,
    symbol_properties: RefCell<Vec<(ObjectRef, Property)>>,
    prototype: RefCell<Option<Option<Prototype>>>,
}

impl ArrayData {
    fn cold(&self) -> &ArrayColdData {
        self.cold.get_or_init(Box::default).as_ref()
    }

    fn cold_if_present(&self) -> Option<&ArrayColdData> {
        self.cold.get().map(Box::as_ref)
    }

    fn has_hole(&self, index: usize) -> bool {
        self.cold_if_present()
            .is_some_and(|cold| cold.holes.borrow().contains(&index))
    }

    fn holes_are_empty(&self) -> bool {
        self.cold_if_present()
            .is_none_or(|cold| cold.holes.borrow().is_empty())
    }

    fn properties_are_empty(&self) -> bool {
        self.cold_if_present()
            .is_none_or(|cold| cold.properties.borrow().is_empty())
    }

    fn uses_default_prototype(&self) -> bool {
        self.cold_if_present()
            .is_none_or(|cold| cold.prototype.borrow().is_none())
    }

    fn prototype_override(&self) -> Option<Option<Prototype>> {
        self.cold_if_present()
            .and_then(|cold| cold.prototype.borrow().clone())
    }
}

impl ArrayRef {
    pub(crate) fn new(elements: Vec<Value>) -> Self {
        Self::new_sparse(elements, Vec::new())
    }

    pub(crate) fn new_with_length(length: usize) -> Self {
        Self(Rc::new(ArrayData {
            elements: RefCell::new(Vec::new()),
            length: Cell::new(length),
            length_writable: Cell::new(true),
            extensible: Cell::new(true),
            sealed: Cell::new(false),
            frozen: Cell::new(false),
            cold: OnceCell::new(),
        }))
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
        let data = ArrayData {
            elements: RefCell::new(elements),
            length: Cell::new(length),
            length_writable: Cell::new(true),
            extensible: Cell::new(true),
            sealed: Cell::new(false),
            frozen: Cell::new(false),
            cold: OnceCell::new(),
        };
        if !holes.is_empty() {
            data.cold
                .set(Box::new(ArrayColdData {
                    holes: RefCell::new(holes),
                    ..ArrayColdData::default()
                }))
                .unwrap_or_else(|_| unreachable!("fresh array cold state is empty"));
        }
        Self(Rc::new(data))
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn len(&self) -> usize {
        self.0.length.get()
    }

    pub(crate) fn get(&self, index: usize) -> Option<Value> {
        if self.0.has_hole(index) {
            return None;
        }
        self.0.elements.borrow().get(index).cloned()
    }

    pub(crate) fn has_index(&self, index: usize) -> bool {
        index < self.0.elements.borrow().len() && !self.0.has_hole(index)
    }

    pub(crate) fn present_indices(&self) -> Vec<usize> {
        let len = self.0.length.get();
        let dense_len = self.0.elements.borrow().len();
        let mut indices: Vec<_> = (0..dense_len)
            .filter(|index| !self.0.has_hole(*index))
            .collect();
        if let Some(cold) = self.0.cold_if_present() {
            indices.extend(
                cold.properties
                    .borrow()
                    .keys()
                    .filter_map(|key| array_index_property_key(key))
                    .filter(|index| *index < len),
            );
        }
        indices.sort_unstable();
        indices.dedup();
        indices
    }

    pub(crate) fn property(&self, key: &str) -> Option<Property> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.properties.borrow().get(key).cloned())
    }

    pub(crate) fn symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.own_symbol_property(symbol)
    }

    pub(crate) fn to_vec(&self) -> Vec<Value> {
        self.0.elements.borrow().clone()
    }

    pub(crate) fn pop(&self) -> Option<Value> {
        let mut elements = self.0.elements.borrow_mut();
        let index = self.0.length.get().checked_sub(1)?;
        self.0.length.set(index);
        if let Some(cold) = self.0.cold_if_present() {
            cold.holes.borrow_mut().remove(&index);
            cold.properties.borrow_mut().remove(&index.to_string());
        }
        if index + 1 == elements.len() {
            elements.pop()
        } else {
            None
        }
    }

    pub(crate) fn replace_with(&self, values: Vec<Value>) {
        if self.0.frozen.get() {
            return;
        }
        if values.len() > self.0.length.get() && !self.0.extensible.get() {
            return;
        }
        self.0.length.set(values.len());
        *self.0.elements.borrow_mut() = values;
        if let Some(cold) = self.0.cold_if_present() {
            cold.holes.borrow_mut().clear();
        }
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
        self.0.uses_default_prototype()
    }

    /// Whether this array's explicit prototype slot names the supplied realm
    /// Array.prototype object. `new Array(...)` records the intrinsic object
    /// explicitly, while array literals normally use the implicit default
    /// slot; both representations have ordinary array prototype semantics.
    pub(crate) fn uses_prototype_object(&self, prototype: &ObjectRef) -> bool {
        matches!(
            self.0.prototype_override(),
            Some(Some(Prototype::Object(actual))) if actual.ptr_eq(prototype)
        )
    }

    /// Reads every element `0..length` directly out of dense storage as an
    /// argument list, returning `None` when a generic property lookup is needed
    /// instead. The fast path requires fully dense storage (length matches the
    /// element vector with no holes), no own indexed/length descriptors that
    /// could intercept the read, the default prototype, and that prototype owning
    /// no indexed property whose value an absent element would inherit.
    pub(crate) fn dense_argument_values(&self, env: &CallEnv) -> Option<Vec<Value>> {
        self.with_dense_argument_elements(env, |elements| elements.to_vec())
    }

    pub(crate) fn with_dense_argument_elements<R>(
        &self,
        env: &CallEnv,
        read: impl FnOnce(&[Value]) -> R,
    ) -> Option<R> {
        let elements = self.0.elements.borrow();
        if self.0.length.get() != elements.len() || !self.0.holes_are_empty() {
            return None;
        }
        if !self.0.properties_are_empty() {
            return None;
        }
        match self.0.prototype_override() {
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
        Some(read(&elements))
    }

    /// Reads one element directly when ordinary property lookup cannot observe a
    /// different value. Callers should re-check this per access because arrays
    /// can become sparse or gain intercepting descriptors while iteration is in
    /// progress.
    pub(crate) fn dense_index_value(&self, index: usize, env: &CallEnv) -> Option<Value> {
        let elements = self.0.elements.borrow();
        if self.0.length.get() != elements.len() || !self.0.holes_are_empty() {
            return None;
        }
        if !self.0.properties_are_empty() {
            return None;
        }
        match self.0.prototype_override() {
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
        elements.get(index).cloned()
    }

    /// Reads a present dense element when ordinary `array[index]` lookup cannot
    /// observe a different value. Unlike absent-element reads, a present own
    /// dense element always wins over the prototype chain, so the caller does
    /// not need to inspect Array.prototype's indexed properties.
    pub(crate) fn direct_dense_index_value(&self, index: usize) -> Option<Value> {
        let elements = self.0.elements.borrow();
        if self.0.length.get() != elements.len() || !self.0.holes_are_empty() {
            return None;
        }
        if !self.0.properties_are_empty() || !self.uses_default_prototype() {
            return None;
        }
        elements.get(index).cloned()
    }

    /// Searches fully dense numeric storage without generic property lookup.
    /// Callers separately guard the resolved `Array.prototype.indexOf` identity;
    /// this method rejects holes, descriptors, and prototype overrides so the
    /// array cannot expose different indexed values during the search.
    pub(crate) fn direct_dense_index_of_number(&self, search: f64, start: usize) -> Option<f64> {
        let elements = self.0.elements.borrow();
        if self.0.length.get() != elements.len() || !self.0.holes_are_empty() {
            return None;
        }
        if !self.0.properties_are_empty() || !self.uses_default_prototype() {
            return None;
        }
        Some(
            elements[start.min(elements.len())..]
                .iter()
                .position(|element| matches!(element, Value::Number(number) if *number == search))
                .map_or(-1.0, |offset| (start + offset) as f64),
        )
    }

    pub(crate) fn dense_index_store_eligible(&self, index: usize) -> bool {
        if index >= MAX_DENSE_STORAGE_LENGTH || self.0.frozen.get() || !self.0.length_writable.get()
        {
            return false;
        }
        // An index below `length` can still be a hole, and filling that hole
        // creates a new own property. Non-extensible arrays may overwrite an
        // existing dense element, but they may not materialize a hole merely
        // because dense storage has a placeholder at that position.
        if !self.0.extensible.get() && !self.has_index(index) {
            return false;
        }
        self.0.cold_if_present().is_none_or(|cold| {
            let properties = cold.properties.borrow();
            properties.is_empty() || !properties.contains_key(&index.to_string())
        })
    }

    /// Whether `CreateDataProperty` for `index` can be represented as a dense
    /// element write. This is stricter than ordinary array assignment: it only
    /// accepts mutable, extensible arrays without own special descriptors at
    /// the target index, so callers can fall back to the generic descriptor
    /// path whenever a failure or descriptor-preserving overwrite is possible.
    pub(crate) fn dense_data_property_eligible(&self, index: usize) -> bool {
        if index > MAX_ARRAY_INDEX
            || index >= MAX_DENSE_STORAGE_LENGTH
            || self.0.frozen.get()
            || self.0.sealed.get()
            || !self.0.extensible.get()
            || !self.0.length_writable.get()
        {
            return false;
        }
        self.0.cold_if_present().is_none_or(|cold| {
            let properties = cold.properties.borrow();
            properties.is_empty() || !properties.contains_key(&index.to_string())
        })
    }

    pub(crate) fn set(&self, index: usize, value: Value) {
        if index > MAX_ARRAY_INDEX {
            self.set_property(index.to_string(), value);
            return;
        }
        if index >= self.0.length.get() {
            if self.0.frozen.get() || !self.0.extensible.get() || !self.0.length_writable.get() {
                return;
            }
            self.0.length.set(index + 1);
        }
        let mut elements = self.0.elements.borrow_mut();
        if index >= elements.len() {
            if index >= MAX_DENSE_STORAGE_LENGTH {
                drop(elements);
                self.0
                    .cold()
                    .properties
                    .borrow_mut()
                    .insert(index.to_string(), Property::enumerable(value));
                return;
            }
            let old_len = elements.len();
            elements.resize(index + 1, Value::Undefined);
            if old_len < index {
                self.0.cold().holes.borrow_mut().extend(old_len..index);
            }
        }
        if self.0.frozen.get() {
            return;
        }
        elements[index] = value;
        if let Some(cold) = self.0.cold_if_present() {
            cold.holes.borrow_mut().remove(&index);
        }
    }

    pub(crate) fn delete_index(&self, index: usize) -> bool {
        let key = index.to_string();
        if let Some(cold) = self.0.cold_if_present() {
            let mut properties = cold.properties.borrow_mut();
            if properties
                .get(&key)
                .is_some_and(|property| !property.configurable)
            {
                return false;
            }
            properties.remove(&key);
        }
        // A present dense element on a sealed/frozen array is non-configurable
        // (its descriptor is synthesized from these flags), so its deletion is
        // rejected even without a `properties` map entry.
        if self.0.sealed.get() && self.has_index(index) {
            return false;
        }
        if index < self.0.elements.borrow().len() {
            self.0.cold().holes.borrow_mut().insert(index);
        }
        true
    }

    pub(crate) fn delete_indices_from(&self, length: usize) -> Option<usize> {
        let dense_len = self.0.elements.borrow().len();
        if length < dense_len {
            for index in (length..dense_len).rev() {
                if !self.delete_index(index) {
                    return Some(index + 1);
                }
            }
        }

        let mut sparse_indices: Vec<_> = self.0.cold_if_present().map_or_else(Vec::new, |cold| {
            cold.properties
                .borrow()
                .keys()
                .filter_map(|key| array_index_property_key(key))
                .filter(|index| *index >= length && *index <= MAX_ARRAY_INDEX)
                .collect()
        });
        sparse_indices.sort_unstable_by(|left, right| right.cmp(left));
        for index in sparse_indices {
            if !self.delete_index(index) {
                return Some(index + 1);
            }
        }
        None
    }

    pub(crate) fn set_property(&self, key: String, value: Value) {
        let mut properties = self.0.cold().properties.borrow_mut();
        if let Some(property) = properties.get_mut(&key) {
            if property.writable {
                property.value = value;
            }
            return;
        }
        if !self.0.extensible.get() {
            return;
        }
        properties.insert(key, Property::enumerable(value));
    }

    pub(crate) fn define_property(&self, key: String, property: Property) {
        if let Some(index) = array_index_property_key(&key) {
            if index >= self.0.length.get() {
                if !self.0.length_writable.get() {
                    return;
                }
                self.0.length.set(index + 1);
            }
            let mut elements = self.0.elements.borrow_mut();
            if index >= elements.len() {
                if !self.0.extensible.get() {
                    return;
                }
                if index >= MAX_DENSE_STORAGE_LENGTH {
                    drop(elements);
                    self.0.cold().properties.borrow_mut().insert(key, property);
                    return;
                }
                let old_len = elements.len();
                elements.resize(index + 1, Value::Undefined);
                self.0.cold().holes.borrow_mut().extend(old_len..=index);
            } else {
                self.0.cold().holes.borrow_mut().insert(index);
            }
        }
        self.0.cold().properties.borrow_mut().insert(key, property);
    }

    pub(crate) fn define_symbol_property(&self, symbol: ObjectRef, property: Property) {
        let mut properties = self.0.cold().symbol_properties.borrow_mut();
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
        if key == "length" {
            return false;
        }
        let Some(cold) = self.0.cold_if_present() else {
            return true;
        };
        let mut properties = cold.properties.borrow_mut();
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
        let mut keys: Vec<_> = self.0.cold_if_present().map_or_else(Vec::new, |cold| {
            cold.properties
                .borrow()
                .iter()
                .filter(|(_, property)| property.enumerable)
                .map(|(key, _)| key.clone())
                .collect()
        });
        keys.sort();
        keys
    }

    pub(crate) fn property_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.0.cold_if_present().map_or_else(Vec::new, |cold| {
            cold.properties.borrow().keys().cloned().collect()
        });
        names.sort();
        names
    }

    pub(crate) fn set_len(&self, length: usize) {
        let mut elements = self.0.elements.borrow_mut();
        if self.0.frozen.get() || !self.0.length_writable.get() {
            return;
        }
        let old_len = self.0.length.get();
        // ArraySetLength changes the existing non-configurable `length` data
        // property. Growing it creates holes, not indexed properties, so a
        // non-extensible array may still accept a larger writable length.
        self.0.length.set(length);
        if length < elements.len() {
            elements.truncate(length);
        } else if length <= MAX_DENSE_STORAGE_LENGTH {
            elements.resize(length, Value::Undefined);
        }
        let mut holes = self.0.cold().holes.borrow_mut();
        holes.retain(|index| *index < length);
        if length > old_len && length <= MAX_DENSE_STORAGE_LENGTH {
            holes.extend(old_len..length);
        }
        if length < old_len {
            let mut sparse_indices: Vec<_> =
                self.0.cold_if_present().map_or_else(Vec::new, |cold| {
                    cold.properties
                        .borrow()
                        .keys()
                        .filter_map(|key| array_index_property_key(key))
                        .filter(|index| *index >= length && *index <= MAX_ARRAY_INDEX)
                        .collect()
                });
            sparse_indices.sort_unstable_by(|left, right| right.cmp(left));
            let mut properties = self.0.cold().properties.borrow_mut();
            for index in sparse_indices {
                let key = index.to_string();
                if properties
                    .get(&key)
                    .is_some_and(|property| !property.configurable)
                {
                    self.0.length.set(index + 1);
                    return;
                }
                properties.remove(&key);
            }
        }
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.0.extensible.get()
    }

    pub(crate) fn has_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.0.cold_if_present().is_some_and(|cold| {
            cold.symbol_properties
                .borrow()
                .iter()
                .any(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
        })
    }

    pub(crate) fn is_length_writable(&self) -> bool {
        self.0.length_writable.get()
    }

    pub(crate) fn set_length_writable(&self, writable: bool) {
        self.0.length_writable.set(writable);
    }

    pub(crate) fn prevent_extensions(&self) {
        self.0.extensible.set(false);
    }

    pub(crate) fn seal(&self) {
        self.prevent_extensions();
        if let Some(cold) = self.0.cold_if_present() {
            for (_, property) in cold.symbol_properties.borrow_mut().iter_mut() {
                property.make_non_configurable();
            }
            // Named (non-index) own properties live in the `properties` map and
            // must also become non-configurable when the array is sealed.
            for (_, property) in cold.properties.borrow_mut().iter_mut() {
                property.make_non_configurable();
            }
        }
        self.0.sealed.set(true);
    }

    pub(crate) fn is_sealed(&self) -> bool {
        !self.0.extensible.get() && self.0.sealed.get()
    }

    pub(crate) fn freeze(&self) {
        self.seal();
        if let Some(cold) = self.0.cold_if_present() {
            for (_, property) in cold.symbol_properties.borrow_mut().iter_mut() {
                property.freeze_data();
            }
            for (_, property) in cold.properties.borrow_mut().iter_mut() {
                property.freeze_data();
            }
        }
        self.0.frozen.set(true);
        self.0.length_writable.set(false);
    }

    pub(crate) fn is_frozen(&self) -> bool {
        !self.0.extensible.get() && self.0.sealed.get() && self.0.frozen.get()
    }

    pub(crate) fn prototype_slot_override(&self) -> Option<Option<Prototype>> {
        self.0.prototype_override()
    }

    /// The effective [[Prototype]] slot, preserving array/function/proxy
    /// identities and resolving the implicit realm Array.prototype only when
    /// no explicit override is present.
    pub(crate) fn effective_prototype_slot(&self, env: &CallEnv) -> Option<Prototype> {
        self.prototype_slot_override()
            .unwrap_or_else(|| crate::array_prototype(env).map(Prototype::Object))
    }

    pub(crate) fn own_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.0.cold_if_present().and_then(|cold| {
            cold.symbol_properties
                .borrow()
                .iter()
                .find(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
                .map(|(_, property)| property.clone())
        })
    }

    pub(crate) fn delete_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        let Some(cold) = self.0.cold_if_present() else {
            return true;
        };
        let mut properties = cold.symbol_properties.borrow_mut();
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
        self.0.cold_if_present().map_or_else(Vec::new, |cold| {
            cold.symbol_properties
                .borrow()
                .iter()
                .map(|(symbol, _)| symbol.clone())
                .collect()
        })
    }

    pub(crate) fn set_prototype_slot(&self, prototype: Option<Prototype>) -> Result<(), ()> {
        if matches!(
            self.0.prototype_override().as_ref(),
            Some(current) if same_prototype_slot(current.as_ref(), prototype.as_ref())
        ) {
            return Ok(());
        }
        if !self.0.extensible.get() {
            return Err(());
        }
        if prototype
            .as_ref()
            .is_some_and(|prototype| prototype.would_cycle_array(self))
        {
            return Err(());
        }
        *self.0.cold().prototype.borrow_mut() = Some(prototype);
        Ok(())
    }
}

/// Parses an ECMAScript Array Index property key. Numeric-looking strings
/// such as `"01"`, `"1e0"`, and `"4294967295"` are ordinary string keys:
/// only the canonical decimal spelling of a uint32 below 2^32 - 1 is indexed.
pub(crate) fn array_index_property_key(key: &str) -> Option<usize> {
    key.parse::<u32>()
        .ok()
        .filter(|index| *index < u32::MAX && index.to_string() == key)
        .map(|index| index as usize)
}

impl fmt::Debug for ArrayRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ArrayRef")
            .field("len", &self.0.length.get())
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

#[cfg(test)]
mod tests {
    use std::{mem, rc::Rc};

    use super::{ArrayData, ArrayRef};
    use crate::Value;

    #[test]
    fn cloned_array_is_a_pointer_sized_shared_handle() {
        let array = ArrayRef::new(Vec::new());
        let cloned = array.clone();

        assert!(Rc::ptr_eq(&array.0, &cloned.0));
        assert!(array.ptr_eq(&cloned));
        assert_eq!(mem::size_of::<ArrayRef>(), mem::size_of::<Rc<ArrayData>>());
    }

    #[test]
    fn dense_array_keeps_cold_state_out_of_line() {
        let array_data_size = mem::size_of::<ArrayData>();
        assert!(
            array_data_size <= 64,
            "dense array header grew to {array_data_size} bytes"
        );

        let array = ArrayRef::new(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);
        assert!(array.0.cold.get().is_none());
        assert_eq!(array.get(2), Some(Value::Number(3.0)));
        assert!(
            array.0.cold.get().is_none(),
            "dense reads must not allocate cold array state"
        );

        assert!(array.delete_index(1));
        assert!(array.0.cold.get().is_some());
        assert_eq!(array.get(1), None);
    }
}
