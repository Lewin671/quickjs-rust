use std::{
    cell::{Cell, OnceCell, Ref, RefCell, RefMut},
    collections::{BTreeSet, HashMap},
    fmt,
    rc::Rc,
};

use super::{ObjectRef, Property, Prototype, Value};
use crate::CallEnv;

pub(crate) const MAX_DENSE_STORAGE_LENGTH: usize = 1_000_000;
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

    fn has_property_at_index(&self, index: usize) -> bool {
        self.cold_if_present().is_some_and(|cold| {
            let properties = cold.properties.borrow();
            if properties.is_empty() {
                return false;
            }

            // Format the canonical decimal index into a stack buffer so a
            // cold named property does not reintroduce a heap allocation on
            // every otherwise-direct numeric read. Three bytes per pointer
            // byte comfortably covers every supported `usize` decimal width.
            let mut digits = [0_u8; std::mem::size_of::<usize>() * 3];
            let mut cursor = digits.len();
            let mut remaining = index;
            loop {
                cursor -= 1;
                digits[cursor] = b'0' + (remaining % 10) as u8;
                remaining /= 10;
                if remaining == 0 {
                    break;
                }
            }
            let key = std::str::from_utf8(&digits[cursor..])
                .expect("decimal array indices are valid UTF-8");
            properties.contains_key(key)
        })
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
    /// dense element always wins over the prototype chain, so unrelated holes,
    /// descriptors, and prototype overrides do not block this target-specific
    /// read. An own special descriptor at `index` still takes the generic
    /// property path; the usual descriptor representation marks a dense hole,
    /// while the explicit target-key check also protects transitional storage
    /// states without making unrelated descriptors reject the read.
    pub(crate) fn direct_dense_index_value(&self, index: usize) -> Option<Value> {
        if index >= self.0.length.get()
            || self.0.has_hole(index)
            || self.0.has_property_at_index(index)
        {
            return None;
        }
        self.0.elements.borrow().get(index).cloned()
    }

    /// Temporarily exposes fully dense, writable indexed storage to a
    /// semantics-preserving numeric loop accelerator.
    ///
    /// The closure must not call back into JavaScript or access this array:
    /// `elements` is mutably borrowed for its whole duration. Prototype state
    /// is deliberately irrelevant because every index in `0..length` is a
    /// present own data property. Sealed/non-extensible arrays and arrays with
    /// a non-writable length still permit overwriting those existing elements;
    /// frozen arrays do not.
    pub(crate) fn with_dense_writable_elements<R>(
        &self,
        mutate: impl FnOnce(&mut [Value]) -> R,
    ) -> Option<R> {
        if self.0.frozen.get()
            || self.0.cold_if_present().is_some_and(|cold| {
                !cold.holes.try_borrow().is_ok_and(|holes| holes.is_empty())
                    || !cold
                        .properties
                        .try_borrow()
                        .is_ok_and(|properties| properties.is_empty())
            })
        {
            return None;
        }
        let mut elements = self.0.elements.try_borrow_mut().ok()?;
        if self.0.length.get() != elements.len() {
            return None;
        }
        Some(mutate(&mut elements))
    }

    /// Temporarily exposes several distinct dense arrays to one numeric loop
    /// region. Every receiver is validated before any element borrow is
    /// returned, and duplicate object identities fail closed so the caller
    /// never has to account for cross-receiver aliasing. A borrow conflict on
    /// any receiver releases earlier leases without invoking `mutate`.
    pub(crate) fn with_distinct_dense_writable_elements<'a, R>(
        arrays: &'a [Self],
        mutate: impl FnOnce(&mut [RefMut<'a, Vec<Value>>]) -> R,
    ) -> Option<R> {
        for (index, array) in arrays.iter().enumerate() {
            if arrays[..index]
                .iter()
                .any(|existing| existing.ptr_eq(array))
                || array.0.frozen.get()
                || array.0.cold_if_present().is_some_and(|cold| {
                    !cold.holes.try_borrow().is_ok_and(|holes| holes.is_empty())
                        || !cold
                            .properties
                            .try_borrow()
                            .is_ok_and(|properties| properties.is_empty())
                })
            {
                return None;
            }
        }

        let mut elements = Vec::with_capacity(arrays.len());
        for array in arrays {
            let lease = array.0.elements.try_borrow_mut().ok()?;
            if array.0.length.get() != lease.len() {
                return None;
            }
            elements.push(lease);
        }
        Some(mutate(&mut elements))
    }

    /// Temporarily leases one implicit hole tail for append-only writes and
    /// every other receiver for fully-dense reads.
    ///
    /// The writer must be an extensible ordinary Array whose materialized
    /// prefix has neither holes nor special indexed descriptors. `start_index`
    /// must equal that prefix length and remain below the array's existing
    /// logical length, so the closure can only materialize existing holes and
    /// must not change `length`. A non-writable length is therefore safe: an
    /// indexed property below the current length does not invoke
    /// ArraySetLength. The caller must separately guard the writer's effective
    /// prototype chain because that requires realm/VM state.
    ///
    /// The writer may not alias a readable receiver. Read/read aliases are
    /// allowed. All structural checks and element borrows finish before
    /// `mutate` runs; any conflict releases prior borrows and fails closed.
    pub(crate) fn with_dense_hole_tail_append_and_readable_elements<'a, R>(
        arrays: &'a [Self],
        writer_receiver: usize,
        start_index: usize,
        mutate: impl FnOnce(&mut Vec<Value>, &[Ref<'a, Vec<Value>>], usize) -> R,
    ) -> Option<R> {
        let writer = arrays.get(writer_receiver)?;
        if arrays
            .iter()
            .enumerate()
            .any(|(receiver, array)| receiver != writer_receiver && writer.ptr_eq(array))
            || !writer.0.extensible.get()
            || writer.0.frozen.get()
            || start_index >= MAX_DENSE_STORAGE_LENGTH
        {
            return None;
        }
        if writer.0.cold_if_present().is_some_and(|cold| {
            !cold.holes.try_borrow().is_ok_and(|holes| holes.is_empty())
                || !cold
                    .properties
                    .try_borrow()
                    .is_ok_and(|properties| properties.is_empty())
        }) {
            return None;
        }
        for (receiver, array) in arrays.iter().enumerate() {
            if receiver == writer_receiver {
                continue;
            }
            if array.0.cold_if_present().is_some_and(|cold| {
                !cold.holes.try_borrow().is_ok_and(|holes| holes.is_empty())
                    || !cold
                        .properties
                        .try_borrow()
                        .is_ok_and(|properties| properties.is_empty())
            }) {
                return None;
            }
        }

        let logical_length = writer.0.length.get();
        let mut writer_elements = writer.0.elements.try_borrow_mut().ok()?;
        if writer_elements.len() != start_index || start_index >= logical_length {
            return None;
        }

        let mut readable_elements = Vec::with_capacity(arrays.len().saturating_sub(1));
        for (receiver, array) in arrays.iter().enumerate() {
            if receiver == writer_receiver {
                continue;
            }
            let elements = array.0.elements.try_borrow().ok()?;
            if array.0.length.get() != elements.len() {
                return None;
            }
            readable_elements.push(elements);
        }

        let result = mutate(&mut writer_elements, &readable_elements, logical_length);
        debug_assert_eq!(writer.0.length.get(), logical_length);
        Some(result)
    }

    /// Temporarily exposes fully dense indexed storage to a pure read-only
    /// accelerator. The closure must not call back into JavaScript or access
    /// this array while the element borrow is live. Prototype state is
    /// deliberately irrelevant: every index in `0..length` is a present own
    /// data property, so an inherited getter cannot intercept these reads.
    pub(crate) fn with_dense_readable_elements<R>(
        &self,
        read: impl FnOnce(&[Value]) -> R,
    ) -> Option<R> {
        if self.0.cold_if_present().is_some_and(|cold| {
            !cold.holes.try_borrow().is_ok_and(|holes| holes.is_empty())
                || !cold
                    .properties
                    .try_borrow()
                    .is_ok_and(|properties| properties.is_empty())
        }) {
            return None;
        }
        let elements = self.0.elements.try_borrow().ok()?;
        if self.0.length.get() != elements.len() {
            return None;
        }
        Some(read(&elements))
    }

    /// Temporarily exposes several dense arrays to one pure read-only region.
    /// Receiver aliases are deliberately allowed because shared element
    /// borrows observe the same immutable storage safely. Every receiver is
    /// validated and borrowed before `read` runs; a mutable-borrow conflict on
    /// any receiver releases earlier leases and fails closed.
    pub(crate) fn with_dense_readable_element_sets<'a, R>(
        arrays: &'a [Self],
        read: impl FnOnce(&[Ref<'a, Vec<Value>>]) -> R,
    ) -> Option<R> {
        for array in arrays {
            if array.0.cold_if_present().is_some_and(|cold| {
                !cold.holes.try_borrow().is_ok_and(|holes| holes.is_empty())
                    || !cold
                        .properties
                        .try_borrow()
                        .is_ok_and(|properties| properties.is_empty())
            }) {
                return None;
            }
        }

        let mut elements = Vec::with_capacity(arrays.len());
        for array in arrays {
            let lease = array.0.elements.try_borrow().ok()?;
            if array.0.length.get() != lease.len() {
                return None;
            }
            elements.push(lease);
        }
        Some(read(&elements))
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
        if index >= MAX_DENSE_STORAGE_LENGTH
            || self.0.frozen.get()
            || (index >= self.0.length.get() && !self.0.length_writable.get())
        {
            return false;
        }
        // A non-writable length blocks only stores that would extend the
        // array. Materializing a hole below the existing logical length leaves
        // the length descriptor untouched and remains an ordinary indexed set.
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

    use super::{ArrayData, ArrayRef, MAX_DENSE_STORAGE_LENGTH};
    use crate::{Property, Value};

    #[test]
    fn cloned_array_is_a_pointer_sized_shared_handle() {
        let array = ArrayRef::new(Vec::new());
        let cloned = array.clone();

        assert!(Rc::ptr_eq(&array.0, &cloned.0));
        assert!(array.ptr_eq(&cloned));
        assert_eq!(mem::size_of::<ArrayRef>(), mem::size_of::<Rc<ArrayData>>());
    }

    #[test]
    fn distinct_dense_write_leases_fail_closed_for_aliases_and_borrow_conflicts() {
        let first = ArrayRef::new(vec![Value::Number(1.0)]);
        let second = ArrayRef::new(vec![Value::Number(2.0)]);

        let mut invoked = false;
        assert!(
            ArrayRef::with_distinct_dense_writable_elements(
                &[first.clone(), first.clone()],
                |_| invoked = true,
            )
            .is_none()
        );
        assert!(!invoked);

        let held_lease = second.0.elements.borrow_mut();
        assert!(
            ArrayRef::with_distinct_dense_writable_elements(
                &[first.clone(), second.clone()],
                |_| invoked = true,
            )
            .is_none()
        );
        assert!(!invoked);
        drop(held_lease);
        assert!(first.0.elements.try_borrow_mut().is_ok());

        assert!(
            ArrayRef::with_distinct_dense_writable_elements(
                &[first.clone(), second.clone()],
                |elements| {
                    elements[0][0] = Value::Number(3.0);
                    elements[1][0] = Value::Number(5.0);
                },
            )
            .is_some()
        );
        assert_eq!(first.get(0), Some(Value::Number(3.0)));
        assert_eq!(second.get(0), Some(Value::Number(5.0)));
    }

    #[test]
    fn dense_read_leases_allow_aliases_and_fail_closed_on_mutable_borrows() {
        let first = ArrayRef::new(vec![Value::Number(1.0)]);
        let second = ArrayRef::new(vec![Value::Number(2.0)]);

        assert_eq!(
            ArrayRef::with_dense_readable_element_sets(
                &[first.clone(), first.clone()],
                |elements| elements[0][0].clone() == elements[1][0].clone(),
            ),
            Some(true)
        );

        let held_lease = second.0.elements.borrow_mut();
        let mut invoked = false;
        assert!(
            ArrayRef::with_dense_readable_element_sets(&[first.clone(), second.clone()], |_| {
                invoked = true
            },)
            .is_none()
        );
        assert!(!invoked);
        drop(held_lease);
        assert!(second.0.elements.try_borrow_mut().is_ok());
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

    #[test]
    fn direct_dense_index_read_only_checks_the_target_element() {
        let array = ArrayRef::new(vec![
            Value::Number(11.0),
            Value::Number(13.0),
            Value::Number(17.0),
        ]);

        assert!(array.delete_index(1));
        array.define_property(
            "2".to_owned(),
            Property::data(Value::Number(19.0), false, false, false),
        );
        // Some generic mutation paths can materialize a dense value without
        // dropping the cold descriptor. The target-key guard must still win.
        array.set(2, Value::Number(23.0));

        assert_eq!(array.direct_dense_index_value(0), Some(Value::Number(11.0)));
        assert_eq!(array.direct_dense_index_value(1), None);
        assert_eq!(array.direct_dense_index_value(2), None);
        assert_eq!(array.direct_dense_index_value(3), None);
    }

    #[test]
    fn dense_writable_lease_allows_existing_elements_on_sealed_arrays() {
        let array = ArrayRef::new(vec![Value::Number(1.0), Value::Number(2.0)]);
        array.seal();
        array.set_length_writable(false);

        assert_eq!(
            array.with_dense_writable_elements(|elements| {
                elements[1] = Value::Number(3.0);
                elements.len()
            }),
            Some(2)
        );
        assert_eq!(array.get(1), Some(Value::Number(3.0)));
    }

    #[test]
    fn dense_writable_lease_rejects_frozen_sparse_and_special_elements() {
        let frozen = ArrayRef::new(vec![Value::Number(1.0)]);
        frozen.freeze();
        assert!(frozen.with_dense_writable_elements(|_| ()).is_none());

        let sparse = ArrayRef::new(vec![Value::Number(1.0), Value::Number(2.0)]);
        assert!(sparse.delete_index(1));
        assert!(sparse.with_dense_writable_elements(|_| ()).is_none());

        let described = ArrayRef::new(vec![Value::Number(1.0)]);
        described.define_property(
            "0".to_owned(),
            Property::data(Value::Number(1.0), false, true, false),
        );
        assert!(described.with_dense_writable_elements(|_| ()).is_none());
    }

    #[test]
    fn dense_writable_lease_fails_closed_on_borrow_conflict() {
        let array = ArrayRef::new(vec![Value::Number(1.0)]);
        let outstanding_read = array.0.elements.borrow();
        assert!(array.with_dense_writable_elements(|_| ()).is_none());
        drop(outstanding_read);
        assert!(array.with_dense_writable_elements(|_| ()).is_some());

        let cold = array.0.cold();
        let outstanding_holes_write = cold.holes.borrow_mut();
        assert!(array.with_dense_writable_elements(|_| ()).is_none());
        drop(outstanding_holes_write);
    }

    #[test]
    fn dense_index_store_allows_holes_below_a_non_writable_length_only() {
        let array = ArrayRef::new_with_length(2);
        array.set_length_writable(false);

        assert!(array.dense_index_store_eligible(0));
        assert!(array.dense_index_store_eligible(1));
        assert!(!array.dense_index_store_eligible(2));

        array.set(0, Value::Number(7.0));
        array.set(2, Value::Number(11.0));
        assert_eq!(array.len(), 2);
        assert_eq!(array.get(0), Some(Value::Number(7.0)));
        assert_eq!(array.get(2), None);
    }

    #[test]
    fn hole_tail_append_lease_materializes_only_below_the_existing_length() {
        let writer = ArrayRef::new_with_length(3);
        writer.set_length_writable(false);
        let source = ArrayRef::new(vec![
            Value::Number(2.0),
            Value::Number(3.0),
            Value::Number(5.0),
        ]);

        assert_eq!(
            ArrayRef::with_dense_hole_tail_append_and_readable_elements(
                &[source.clone(), writer.clone()],
                1,
                0,
                |writer, readable, logical_length| {
                    assert_eq!(logical_length, 3);
                    writer.extend(readable[0].iter().cloned());
                    writer.len()
                },
            ),
            Some(3)
        );
        assert_eq!(writer.len(), 3);
        assert_eq!(writer.to_vec(), source.to_vec());
        assert!(!writer.is_length_writable());

        assert!(
            ArrayRef::with_dense_hole_tail_append_and_readable_elements(
                &[source, writer],
                1,
                3,
                |_, _, _| (),
            )
            .is_none()
        );
    }

    #[test]
    fn hole_tail_append_lease_rejects_integrity_shape_and_descriptor_hazards() {
        let readable = ArrayRef::new(vec![Value::Number(1.0)]);
        let invoke = |writer: ArrayRef, start_index| {
            let mut invoked = false;
            let result = ArrayRef::with_dense_hole_tail_append_and_readable_elements(
                &[readable.clone(), writer],
                1,
                start_index,
                |_, _, _| invoked = true,
            );
            (result, invoked)
        };

        let non_extensible = ArrayRef::new_with_length(1);
        non_extensible.prevent_extensions();
        assert_eq!(invoke(non_extensible, 0), (None, false));

        let sealed = ArrayRef::new_with_length(1);
        sealed.seal();
        assert_eq!(invoke(sealed, 0), (None, false));

        let frozen = ArrayRef::new_with_length(1);
        frozen.freeze();
        assert_eq!(invoke(frozen, 0), (None, false));

        let sparse_prefix =
            ArrayRef::new_sparse(vec![Value::Number(1.0), Value::Undefined], vec![1]);
        sparse_prefix.set_len(3);
        assert_eq!(invoke(sparse_prefix, 2), (None, false));

        let described = ArrayRef::new_with_length(1);
        described.define_property(
            "0".to_owned(),
            Property::data(Value::Number(7.0), true, true, true),
        );
        assert_eq!(invoke(described, 1), (None, false));

        let wrong_start = ArrayRef::new_with_length(2);
        wrong_start.set(0, Value::Number(1.0));
        assert_eq!(invoke(wrong_start, 0), (None, false));

        let oversized = ArrayRef::new_with_length(MAX_DENSE_STORAGE_LENGTH + 1);
        assert_eq!(invoke(oversized, MAX_DENSE_STORAGE_LENGTH), (None, false));
    }

    #[test]
    fn hole_tail_append_lease_rejects_aliases_and_borrow_conflicts() {
        let writer = ArrayRef::new_with_length(2);
        assert!(
            ArrayRef::with_dense_hole_tail_append_and_readable_elements(
                &[writer.clone(), writer.clone()],
                1,
                0,
                |_, _, _| (),
            )
            .is_none()
        );

        let readable = ArrayRef::new(vec![Value::Number(1.0)]);
        let held_writer_read = writer.0.elements.borrow();
        assert!(
            ArrayRef::with_dense_hole_tail_append_and_readable_elements(
                &[readable.clone(), writer.clone()],
                1,
                0,
                |_, _, _| (),
            )
            .is_none()
        );
        drop(held_writer_read);

        let held_readable_write = readable.0.elements.borrow_mut();
        assert!(
            ArrayRef::with_dense_hole_tail_append_and_readable_elements(
                &[readable.clone(), writer.clone()],
                1,
                0,
                |_, _, _| (),
            )
            .is_none()
        );
        drop(held_readable_write);

        let cold = writer.0.cold();
        let held_holes = cold.holes.borrow_mut();
        assert!(
            ArrayRef::with_dense_hole_tail_append_and_readable_elements(
                &[readable, writer.clone()],
                1,
                0,
                |_, _, _| (),
            )
            .is_none()
        );
        drop(held_holes);
    }
}
