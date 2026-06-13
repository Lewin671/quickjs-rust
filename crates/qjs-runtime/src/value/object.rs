use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
};

use crate::private::{PrivateEnvironment, PrivateStorage};
use crate::{Function, proxy::ProxyRef};

use super::{Property, Value};

/// A [[Prototype]] slot value. Most prototypes are plain objects, but a
/// function may also sit in a prototype chain (for example a subclass
/// constructor whose [[Prototype]] is its superclass, or an object created with
/// `Object.create(fn)`). The variants keep both forms first-class so property
/// lookup walks through either uniformly.
#[derive(Clone)]
pub(crate) enum Prototype {
    Object(ObjectRef),
    Function(Function),
    Proxy(ProxyRef),
}

impl Prototype {
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Object(left), Self::Object(right)) => left.ptr_eq(right),
            (Self::Function(left), Self::Function(right)) => left.ptr_eq(right),
            (Self::Proxy(left), Self::Proxy(right)) => left.ptr_eq(right),
            _ => false,
        }
    }

    /// Walks this prototype (and its own chain) for the data/accessor property
    /// `key`, returning the first match.
    fn property(&self, key: &str) -> Option<Property> {
        match self {
            Self::Object(object) => object.property(key),
            Self::Function(function) => function.chain_property(key),
            Self::Proxy(proxy) => match proxy.target_result() {
                Ok(target) => crate::property::own_or_inherited_descriptor(target, key),
                Err(_) => None,
            },
        }
    }

    fn get(&self, key: &str) -> Option<Value> {
        self.property(key).map(|property| property.value)
    }

    fn symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        match self {
            Self::Object(object) => object.symbol_property(symbol),
            Self::Function(function) => function.chain_symbol_property(symbol),
            Self::Proxy(proxy) => match proxy.target_result() {
                Ok(target) => crate::property::own_or_inherited_symbol_descriptor(target, symbol),
                Err(_) => None,
            },
        }
    }

    fn to_string_tag(&self) -> Option<String> {
        match self {
            Self::Object(object) => object.to_string_tag(),
            // Functions never carry a Symbol.toStringTag in their own chain by
            // default; stop the search here.
            Self::Function(_) => None,
            Self::Proxy(_) => None,
        }
    }

    /// Whether the object `target` appears at or beyond this point in the
    /// chain.
    pub(crate) fn chain_contains(&self, target: &ObjectRef) -> bool {
        match self {
            Self::Object(object) => object.ptr_eq(target) || object.has_prototype(target),
            Self::Function(function) => function.chain_contains_object(target),
            Self::Proxy(proxy) => match proxy.target_result() {
                Ok(Value::Object(object)) => object.ptr_eq(target) || object.has_prototype(target),
                Ok(proxy_target) => {
                    crate::property::value_has_prototype_object(proxy_target, target)
                }
                Err(_) => false,
            },
        }
    }

    /// Whether the value `target` (object or function) appears at or beyond this
    /// point in the chain, by reference identity.
    pub(crate) fn chain_contains_value(&self, target: &Value) -> bool {
        match (self, target) {
            (Self::Object(object), Value::Object(target)) => {
                object.ptr_eq(target) || object.has_prototype(target)
            }
            (Self::Object(object), _) => object
                .prototype_slot()
                .is_some_and(|next| next.chain_contains_value(target)),
            (Self::Function(function), Value::Function(target)) => {
                function.ptr_eq(target) || function.chain_contains_function(target)
            }
            (Self::Function(function), _) => function.chain_contains_value(target),
            (Self::Proxy(proxy), Value::Proxy(target)) => proxy.ptr_eq(target),
            (Self::Proxy(proxy), _) => match proxy.target_result() {
                Ok(proxy_target) => {
                    crate::property::value_has_prototype_value(proxy_target, target)
                }
                Err(_) => false,
            },
        }
    }

    /// Whether this prototype is (or descends to) the object `target`, used to
    /// reject prototype cycles.
    fn would_cycle(&self, target: &ObjectRef) -> bool {
        match self {
            Self::Object(object) => {
                object.ptr_eq(target)
                    || object
                        .prototype_slot()
                        .is_some_and(|prototype| prototype.would_cycle(target))
            }
            Self::Function(_) => false,
            Self::Proxy(_) => false,
        }
    }

    /// As an `ObjectRef`, if this prototype is an object (used where callers
    /// still expect the legacy object-only prototype).
    pub(crate) fn as_object(&self) -> Option<ObjectRef> {
        match self {
            Self::Object(object) => Some(object.clone()),
            Self::Function(_) | Self::Proxy(_) => None,
        }
    }

    /// The prototype as a JavaScript value, for `getPrototypeOf` and friends.
    pub(crate) fn to_value(&self) -> Value {
        match self {
            Self::Object(object) => Value::Object(object.clone()),
            Self::Function(function) => Value::Function(function.clone()),
            Self::Proxy(proxy) => Value::Proxy(proxy.clone()),
        }
    }
}

/// Object storage reference.
#[derive(Clone)]
pub struct ObjectRef {
    properties: Rc<RefCell<HashMap<String, Property>>>,
    property_order: Rc<RefCell<Vec<String>>>,
    /// Count of own string keys that parse as array indices. Maintained as keys
    /// are added and removed so `has_own_index_property` is an O(1) check; this
    /// keeps the `array[i] = x` fast path from scanning a prototype's keys on
    /// every write.
    index_property_count: Rc<Cell<usize>>,
    symbol_properties: Rc<RefCell<Vec<(ObjectRef, Property)>>>,
    extensible: Rc<Cell<bool>>,
    prototype: Rc<RefCell<Option<Prototype>>>,
    to_string_tag: Rc<RefCell<Option<String>>>,
    raw_json: Rc<Cell<bool>>,
    array_prototype_exotic: Rc<Cell<bool>>,
    immutable_prototype_exotic: Rc<Cell<bool>>,
    /// Generator [[GeneratorState]] for generator objects; `None` for ordinary
    /// objects. Lazily allocated so non-generator objects pay only one `Rc`.
    generator_state: Rc<RefCell<Option<crate::bytecode::GeneratorState>>>,
    /// Async-generator internal state (the [[AsyncGeneratorQueue]] of pending
    /// requests plus the draining flag) for async generator objects; `None` for
    /// every other object.
    async_generator_state: Rc<RefCell<Option<crate::async_generator::AsyncGeneratorInternal>>>,
    /// Private-name state: per-object storage (fields and brands) and, for class
    /// prototype objects, the private environment their members resolve `#x`
    /// references through. Lazily populated.
    private_state: Rc<RefCell<crate::private::PrivateState>>,
}

impl fmt::Debug for ObjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ObjectRef")
            .field("properties", &self.properties.borrow().len())
            .field("symbol_properties", &self.symbol_properties.borrow().len())
            .field("has_prototype", &self.prototype.borrow().is_some())
            .field("to_string_tag", &self.to_string_tag.borrow())
            .field("raw_json", &self.raw_json.get())
            .field("array_prototype_exotic", &self.array_prototype_exotic.get())
            .field(
                "immutable_prototype_exotic",
                &self.immutable_prototype_exotic.get(),
            )
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
        Self::with_prototype_slot(properties, prototype.map(Prototype::Object))
    }

    pub(crate) fn with_prototype_slot(
        properties: HashMap<String, Value>,
        prototype: Option<Prototype>,
    ) -> Self {
        let mut property_order: Vec<_> = properties.keys().cloned().collect();
        property_order.sort();
        let index_property_count = property_order
            .iter()
            .filter(|key| is_array_index_key(key))
            .count();
        Self {
            properties: Rc::new(RefCell::new(
                properties
                    .into_iter()
                    .map(|(key, value)| (key, Property::enumerable(value)))
                    .collect(),
            )),
            property_order: Rc::new(RefCell::new(property_order)),
            index_property_count: Rc::new(Cell::new(index_property_count)),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            extensible: Rc::new(Cell::new(true)),
            prototype: Rc::new(RefCell::new(prototype)),
            to_string_tag: Rc::new(RefCell::new(None)),
            raw_json: Rc::new(Cell::new(false)),
            array_prototype_exotic: Rc::new(Cell::new(false)),
            immutable_prototype_exotic: Rc::new(Cell::new(false)),
            generator_state: Rc::new(RefCell::new(None)),
            async_generator_state: Rc::new(RefCell::new(None)),
            private_state: Rc::new(RefCell::new(crate::private::PrivateState::default())),
        }
    }

    /// The generator [[GeneratorState]] cell for this object. Non-generator
    /// objects hold `None`; generator objects store their resumable state here.
    pub(crate) fn generator_state(&self) -> &Rc<RefCell<Option<crate::bytecode::GeneratorState>>> {
        &self.generator_state
    }

    /// The async-generator internal-state cell for this object. Ordinary objects
    /// (and plain generators) hold `None`; async generator objects store their
    /// request queue and draining flag here.
    pub(crate) fn async_generator_state(
        &self,
    ) -> &Rc<RefCell<Option<crate::async_generator::AsyncGeneratorInternal>>> {
        &self.async_generator_state
    }

    /// Returns the object's private-name storage, creating it on first use.
    pub(crate) fn private_storage(&self) -> PrivateStorage {
        self.private_state
            .borrow_mut()
            .storage
            .get_or_insert_with(PrivateStorage::new)
            .clone()
    }

    /// Sets the private environment carried by a class prototype object.
    pub(crate) fn set_private_environment(&self, environment: PrivateEnvironment) {
        self.private_state.borrow_mut().environment = Some(environment);
    }

    /// Returns the private environment carried by this object, if any.
    pub(crate) fn private_environment(&self) -> Option<PrivateEnvironment> {
        self.private_state.borrow().environment.clone()
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.properties, &other.properties)
    }

    pub(crate) fn mark_raw_json(&self) {
        self.raw_json.set(true);
    }

    pub(crate) fn is_raw_json(&self) -> bool {
        self.raw_json.get()
    }

    pub(crate) fn mark_array_prototype_exotic(&self) {
        self.array_prototype_exotic.set(true);
    }

    pub(crate) fn is_array_prototype_exotic(&self) -> bool {
        self.array_prototype_exotic.get()
    }

    pub(crate) fn mark_immutable_prototype_exotic(&self) {
        self.immutable_prototype_exotic.set(true);
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

    pub(crate) fn property(&self, key: &str) -> Option<Property> {
        self.properties.borrow().get(key).cloned().or_else(|| {
            self.prototype
                .borrow()
                .as_ref()
                .and_then(|proto| proto.property(key))
        })
    }

    /// Whether any own property key parses as an array index. Used to gate the
    /// dense `array[i] = x` fast path: a default prototype with no own indexed
    /// property cannot intercept an index store with an inherited accessor or a
    /// non-writable data property.
    pub(crate) fn has_own_index_property(&self) -> bool {
        self.index_property_count.get() > 0
    }

    pub(crate) fn symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.own_symbol_property(symbol).or_else(|| {
            self.prototype
                .borrow()
                .as_ref()
                .and_then(|proto| proto.symbol_property(symbol))
        })
    }

    /// The raw [[Prototype]] slot, distinguishing object and function
    /// prototypes.
    pub(crate) fn prototype_slot(&self) -> Option<Prototype> {
        self.prototype.borrow().clone()
    }

    pub(crate) fn set_prototype_slot(&self, prototype: Option<Prototype>) -> Result<(), ()> {
        if same_prototype_slot(self.prototype.borrow().as_ref(), prototype.as_ref()) {
            return Ok(());
        }
        if self.immutable_prototype_exotic.get() {
            return Err(());
        }
        if !self.extensible.get() {
            return Err(());
        }
        if prototype
            .as_ref()
            .is_some_and(|prototype| prototype.would_cycle(self))
        {
            return Err(());
        }
        *self.prototype.borrow_mut() = prototype;
        Ok(())
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
        if self.array_prototype_exotic.get()
            && let Some(index) = array_index_property_key(&key)
            && let Some(length_property) = properties.get_mut("length")
            && length_property.writable
        {
            let current_length = match length_property.value {
                Value::Number(length) if length.is_finite() && length >= 0.0 => length as u32,
                _ => 0,
            };
            if index >= current_length {
                length_property.value = Value::Number((index + 1) as f64);
            }
        }
        if is_array_index_key(&key) {
            self.index_property_count
                .set(self.index_property_count.get() + 1);
        }
        self.property_order.borrow_mut().push(key.clone());
        properties.insert(key, Property::enumerable(value));
    }

    pub(crate) fn define_property(&self, key: String, property: Property) {
        let mut properties = self.properties.borrow_mut();
        if !properties.contains_key(&key) {
            if is_array_index_key(&key) {
                self.index_property_count
                    .set(self.index_property_count.get() + 1);
            }
            self.property_order.borrow_mut().push(key.clone());
        }
        properties.insert(key, property);
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

    pub(crate) fn define_non_enumerable(&self, key: String, value: Value) {
        self.define_property(key, Property::non_enumerable(value));
    }

    /// Whether the object `prototype` appears as a function prototype anywhere
    /// in this object's chain. Used by `isPrototypeOf`/`instanceof` to walk past
    /// a function sitting mid-chain.
    pub(crate) fn has_prototype_object(&self, prototype: &ObjectRef) -> bool {
        self.prototype
            .borrow()
            .as_ref()
            .is_some_and(|proto| proto.chain_contains(prototype))
    }

    pub(crate) fn has_own_property(&self, key: &str) -> bool {
        self.properties.borrow().contains_key(key)
    }

    pub(crate) fn has_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.symbol_properties
            .borrow()
            .iter()
            .any(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
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
        for (_, property) in self.symbol_properties.borrow_mut().iter_mut() {
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
            && self
                .symbol_properties
                .borrow()
                .iter()
                .all(|(_, property)| !property.configurable)
    }

    pub(crate) fn freeze(&self) {
        self.prevent_extensions();
        for property in self.properties.borrow_mut().values_mut() {
            property.freeze_data();
        }
        for (_, property) in self.symbol_properties.borrow_mut().iter_mut() {
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
            && self
                .symbol_properties
                .borrow()
                .iter()
                .all(|(_, property)| !property.configurable && !property.writable)
    }

    pub(crate) fn own_property(&self, key: &str) -> Option<Property> {
        self.properties.borrow().get(key).cloned()
    }

    pub(crate) fn own_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.symbol_properties
            .borrow()
            .iter()
            .find(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
            .map(|(_, property)| property.clone())
    }

    pub(crate) fn delete_own_property(&self, key: &str) -> bool {
        let mut properties = self.properties.borrow_mut();
        if properties
            .get(key)
            .is_some_and(|property| !property.configurable)
        {
            return false;
        }
        let removed = properties.remove(key);
        if removed.is_some() && is_array_index_key(key) {
            self.index_property_count
                .set(self.index_property_count.get().saturating_sub(1));
        }
        self.property_order
            .borrow_mut()
            .retain(|existing| existing != key);
        true
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

    pub(crate) fn own_property_keys(&self) -> Vec<String> {
        self.ordered_property_names(|property| property.enumerable)
    }

    pub(crate) fn own_property_names(&self) -> Vec<String> {
        self.ordered_property_names(|_| true)
    }

    fn ordered_property_names(&self, include: impl Fn(&Property) -> bool) -> Vec<String> {
        let properties = self.properties.borrow();
        let mut indices = Vec::new();
        let mut strings = Vec::new();

        for key in self.property_order.borrow().iter() {
            if is_internal_property_key(key) {
                continue;
            }
            let Some(property) = properties.get(key.as_str()) else {
                continue;
            };
            if !include(property) {
                continue;
            }
            if let Some(index) = array_index_property_key(key) {
                indices.push((index, key.clone()));
            } else {
                strings.push(key.clone());
            }
        }

        indices.sort_by_key(|(index, _)| *index);
        indices
            .into_iter()
            .map(|(_, key)| key)
            .chain(strings)
            .collect()
    }

    pub(crate) fn own_property_symbols(&self) -> Vec<ObjectRef> {
        self.symbol_properties
            .borrow()
            .iter()
            .map(|(symbol, _)| symbol.clone())
            .collect()
    }

    pub(crate) fn has_prototype(&self, prototype: &ObjectRef) -> bool {
        self.has_prototype_object(prototype)
    }

    /// The [[Prototype]] as an object, or `None` if absent or a function. Use
    /// [`ObjectRef::prototype_slot`] when the function case matters.
    pub(crate) fn prototype(&self) -> Option<ObjectRef> {
        self.prototype
            .borrow()
            .as_ref()
            .and_then(Prototype::as_object)
    }

    pub(crate) fn set_prototype(&self, prototype: Option<ObjectRef>) -> Result<(), ()> {
        self.set_prototype_slot(prototype.map(Prototype::Object))
    }

    pub(crate) fn to_string_tag(&self) -> Option<String> {
        self.to_string_tag.borrow().clone().or_else(|| {
            self.prototype
                .borrow()
                .as_ref()
                .and_then(Prototype::to_string_tag)
        })
    }

    pub(crate) fn set_to_string_tag(&self, tag: &str) {
        *self.to_string_tag.borrow_mut() = Some(tag.to_owned());
    }
}

fn same_prototype_slot(left: Option<&Prototype>, right: Option<&Prototype>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.ptr_eq(right),
        _ => false,
    }
}

fn is_internal_property_key(key: &str) -> bool {
    key.starts_with('\0')
}

fn array_index_property_key(key: &str) -> Option<u32> {
    key.parse::<u32>()
        .ok()
        .filter(|index| *index < u32::MAX && index.to_string() == key)
}

fn is_array_index_key(key: &str) -> bool {
    array_index_property_key(key).is_some()
}
