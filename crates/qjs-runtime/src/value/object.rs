use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
};

use crate::private::{PrivateEnvironment, PrivateStorage};
use crate::{Function, RuntimeError, function::DynamicBindings, proxy::ProxyRef};

use super::{Property, Value};

type NamespaceBindingCell = DynamicBindings;
type NamespaceAliasMap = HashMap<String, (NamespaceBindingCell, String)>;

/// Result of probing one ordinary object's string-keyed storage for the VM
/// direct-get path. A data hit clones only its value; descriptors that need
/// observable accessor or module-namespace behavior stay on the slow path.
pub(crate) enum OwnDataPropertyRead {
    Missing,
    Data(Value),
    NeedsSlowPath,
}

#[derive(Clone)]
pub(crate) struct ModuleNamespaceBindings {
    lexical: NamespaceBindingCell,
    aliases: Rc<NamespaceAliasMap>,
}

impl ModuleNamespaceBindings {
    pub(crate) fn new(lexical: NamespaceBindingCell, aliases: NamespaceAliasMap) -> Self {
        Self {
            lexical,
            aliases: Rc::new(aliases),
        }
    }

    fn value_for_export(&self, export_name: &str) -> Option<Value> {
        if let Some((lexical, binding_name)) = self.aliases.get(export_name) {
            return lexical.get(binding_name);
        }
        self.lexical.get(export_name)
    }
}

/// A [[Prototype]] slot value. Most prototypes are plain objects, but a
/// function may also sit in a prototype chain (for example a subclass
/// constructor whose [[Prototype]] is its superclass, or an object created with
/// `Object.create(fn)`). The variants keep both forms first-class so property
/// lookup walks through either uniformly.
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
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

    /// Whether this prototype is (or descends to) the object `target`, used to
    /// reject prototype cycles.
    fn would_cycle(&self, target: &ObjectRef) -> bool {
        self.would_cycle_inner(target, &mut Vec::new(), &mut Vec::new())
    }

    fn would_cycle_inner(
        &self,
        target: &ObjectRef,
        seen_objects: &mut Vec<ObjectRef>,
        seen_functions: &mut Vec<Function>,
    ) -> bool {
        match self {
            Self::Object(object) => {
                if object.ptr_eq(target) {
                    return true;
                }
                if seen_objects.iter().any(|seen| seen.ptr_eq(object)) {
                    return false;
                }
                seen_objects.push(object.clone());
                object.prototype_slot().is_some_and(|prototype| {
                    prototype.would_cycle_inner(target, seen_objects, seen_functions)
                })
            }
            Self::Function(function) => {
                if seen_functions.iter().any(|seen| seen.ptr_eq(function)) {
                    return false;
                }
                seen_functions.push(function.clone());
                function
                    .effective_internal_prototype()
                    .is_some_and(|prototype| {
                        prototype.would_cycle_inner(target, seen_objects, seen_functions)
                    })
            }
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
pub struct ObjectRef(Rc<ObjectData>);

#[derive(Clone, Copy, PartialEq, Eq)]
enum SymbolBrand {
    None,
    Primitive,
    Boxed,
}

struct ObjectData {
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
    /// Whether this object has the TypedArray internal slots. Keep the brand
    /// outside string-keyed property storage so ordinary property reads can
    /// reject the integer-indexed exotic path without a HashMap probe.
    typed_array_exotic: Cell<bool>,
    /// Whether this object carries the Symbol [[SymbolData]] internal slot.
    /// Keep the primitive/boxed distinction outside string-keyed storage so
    /// ubiquitous symbol checks do not probe the property HashMap.
    symbol_brand: Cell<SymbolBrand>,
    immutable_prototype_exotic: Rc<Cell<bool>>,
    module_namespace_exotic: Rc<Cell<bool>>,
    module_namespace_bindings: Rc<RefCell<Option<ModuleNamespaceBindings>>>,
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
    /// Opaque byte storage for ArrayBuffer objects. The public ArrayBuffer brand
    /// remains a hidden property; bytes live here so typed-array element access
    /// does not have to encode and decode a string on every read or write.
    internal_bytes: Rc<RefCell<Option<Vec<u8>>>>,
    /// Iterator.zip helper internal state. Ordinary objects hold `None`; zip
    /// helpers store their records here so advancement does not round-trip
    /// through observable-looking property storage.
    iterator_zip_state: Rc<RefCell<Option<crate::iterator::ZipState>>>,
    /// Cross-thread backing for a `SharedArrayBuffer` under the Test262
    /// `$262.agent` harness. When present, the buffer's bytes live in this
    /// `Arc`-shared store (so a worker agent on another OS thread observes the
    /// same memory) instead of `internal_bytes`. Gated so the default build's
    /// object layout is unchanged.
    #[cfg(feature = "agents")]
    shared_backing: Rc<RefCell<Option<crate::array_buffer::SharedBackingRef>>>,
}

impl fmt::Debug for ObjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ObjectRef")
            .field("properties", &self.0.properties.borrow().len())
            .field(
                "symbol_properties",
                &self.0.symbol_properties.borrow().len(),
            )
            .field("has_prototype", &self.0.prototype.borrow().is_some())
            .field("to_string_tag", &self.0.to_string_tag.borrow())
            .field("raw_json", &self.0.raw_json.get())
            .field(
                "array_prototype_exotic",
                &self.0.array_prototype_exotic.get(),
            )
            .field(
                "immutable_prototype_exotic",
                &self.0.immutable_prototype_exotic.get(),
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
        Self(Rc::new(ObjectData {
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
            typed_array_exotic: Cell::new(false),
            symbol_brand: Cell::new(SymbolBrand::None),
            immutable_prototype_exotic: Rc::new(Cell::new(false)),
            module_namespace_exotic: Rc::new(Cell::new(false)),
            module_namespace_bindings: Rc::new(RefCell::new(None)),
            generator_state: Rc::new(RefCell::new(None)),
            async_generator_state: Rc::new(RefCell::new(None)),
            private_state: Rc::new(RefCell::new(crate::private::PrivateState::default())),
            internal_bytes: Rc::new(RefCell::new(None)),
            iterator_zip_state: Rc::new(RefCell::new(None)),
            #[cfg(feature = "agents")]
            shared_backing: Rc::new(RefCell::new(None)),
        }))
    }

    /// The generator [[GeneratorState]] cell for this object. Non-generator
    /// objects hold `None`; generator objects store their resumable state here.
    pub(crate) fn generator_state(&self) -> &Rc<RefCell<Option<crate::bytecode::GeneratorState>>> {
        &self.0.generator_state
    }

    /// The async-generator internal-state cell for this object. Ordinary objects
    /// (and plain generators) hold `None`; async generator objects store their
    /// request queue and draining flag here.
    pub(crate) fn async_generator_state(
        &self,
    ) -> &Rc<RefCell<Option<crate::async_generator::AsyncGeneratorInternal>>> {
        &self.0.async_generator_state
    }

    /// Returns the object's private-name storage, creating it on first use.
    pub(crate) fn private_storage(&self) -> PrivateStorage {
        self.0
            .private_state
            .borrow_mut()
            .storage
            .get_or_insert_with(PrivateStorage::new)
            .clone()
    }

    /// Sets the private environment carried by a class prototype object.
    pub(crate) fn set_private_environment(&self, environment: PrivateEnvironment) {
        self.0.private_state.borrow_mut().environment = Some(environment);
    }

    /// Returns the private environment carried by this object, if any.
    pub(crate) fn private_environment(&self) -> Option<PrivateEnvironment> {
        self.0.private_state.borrow().environment.clone()
    }

    pub(crate) fn internal_bytes(&self) -> Option<Vec<u8>> {
        self.0.internal_bytes.borrow().clone()
    }

    pub(crate) fn with_internal_bytes<T>(&self, f: impl FnOnce(Option<&[u8]>) -> T) -> T {
        let bytes = self.0.internal_bytes.borrow();
        f(bytes.as_deref())
    }

    pub(crate) fn set_internal_bytes(&self, bytes: Vec<u8>) {
        *self.0.internal_bytes.borrow_mut() = Some(bytes);
    }

    pub(crate) fn clear_internal_bytes(&self) {
        *self.0.internal_bytes.borrow_mut() = None;
    }

    pub(crate) fn with_internal_bytes_mut<T>(
        &self,
        f: impl FnOnce(&mut Vec<u8>) -> T,
    ) -> Option<T> {
        self.0.internal_bytes.borrow_mut().as_mut().map(f)
    }

    /// The cross-thread `SharedArrayBuffer` backing for this object, if one was
    /// installed (agents harness only).
    #[cfg(feature = "agents")]
    pub(crate) fn shared_backing(&self) -> Option<crate::array_buffer::SharedBackingRef> {
        self.0.shared_backing.borrow().clone()
    }

    /// Installs the cross-thread `SharedArrayBuffer` backing for this object.
    #[cfg(feature = "agents")]
    pub(crate) fn set_shared_backing(&self, backing: crate::array_buffer::SharedBackingRef) {
        *self.0.shared_backing.borrow_mut() = Some(backing);
    }

    pub(crate) fn set_iterator_zip_state(&self, state: crate::iterator::ZipState) {
        *self.0.iterator_zip_state.borrow_mut() = Some(state);
    }

    pub(crate) fn with_iterator_zip_state_mut<T>(
        &self,
        f: impl FnOnce(&mut crate::iterator::ZipState) -> T,
    ) -> Option<T> {
        self.0.iterator_zip_state.borrow_mut().as_mut().map(f)
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn mark_raw_json(&self) {
        self.0.raw_json.set(true);
    }

    pub(crate) fn is_raw_json(&self) -> bool {
        self.0.raw_json.get()
    }

    pub(crate) fn mark_array_prototype_exotic(&self) {
        self.0.array_prototype_exotic.set(true);
    }

    pub(crate) fn is_array_prototype_exotic(&self) -> bool {
        self.0.array_prototype_exotic.get()
    }

    pub(crate) fn mark_typed_array_exotic(&self) {
        self.0.typed_array_exotic.set(true);
    }

    pub(crate) fn is_typed_array_exotic(&self) -> bool {
        self.0.typed_array_exotic.get()
    }

    pub(crate) fn mark_symbol_primitive(&self) {
        self.0.symbol_brand.set(SymbolBrand::Primitive);
    }

    pub(crate) fn mark_symbol_boxed(&self) {
        self.0.symbol_brand.set(SymbolBrand::Boxed);
    }

    pub(crate) fn is_symbol_object(&self) -> bool {
        self.0.symbol_brand.get() != SymbolBrand::None
    }

    pub(crate) fn is_symbol_primitive(&self) -> bool {
        self.0.symbol_brand.get() == SymbolBrand::Primitive
    }

    pub(crate) fn mark_immutable_prototype_exotic(&self) {
        self.0.immutable_prototype_exotic.set(true);
    }

    pub(crate) fn mark_module_namespace_exotic(&self) {
        self.0.module_namespace_exotic.set(true);
    }

    pub(crate) fn is_module_namespace_exotic(&self) -> bool {
        self.0.module_namespace_exotic.get()
    }

    pub(crate) fn set_module_namespace_bindings(&self, bindings: ModuleNamespaceBindings) {
        *self.0.module_namespace_bindings.borrow_mut() = Some(bindings);
    }

    pub(crate) fn get(&self, key: &str) -> Option<Value> {
        self.0
            .properties
            .borrow()
            .get(key)
            .map(|property| property.value.clone())
            .or_else(|| {
                self.0
                    .prototype
                    .borrow()
                    .as_ref()
                    .and_then(|proto| proto.get(key))
            })
    }

    pub(crate) fn property(&self, key: &str) -> Option<Property> {
        self.0.properties.borrow().get(key).cloned().or_else(|| {
            self.0
                .prototype
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
        self.0.index_property_count.get() > 0
    }

    pub(crate) fn symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.own_symbol_property(symbol).or_else(|| {
            self.0
                .prototype
                .borrow()
                .as_ref()
                .and_then(|proto| proto.symbol_property(symbol))
        })
    }

    /// The raw [[Prototype]] slot, distinguishing object and function
    /// prototypes.
    pub(crate) fn prototype_slot(&self) -> Option<Prototype> {
        self.0.prototype.borrow().clone()
    }

    pub(crate) fn set_prototype_slot(&self, prototype: Option<Prototype>) -> Result<(), ()> {
        if same_prototype_slot(self.0.prototype.borrow().as_ref(), prototype.as_ref()) {
            return Ok(());
        }
        if self.0.immutable_prototype_exotic.get() {
            return Err(());
        }
        if !self.0.extensible.get() {
            return Err(());
        }
        if prototype
            .as_ref()
            .is_some_and(|prototype| prototype.would_cycle(self))
        {
            return Err(());
        }
        *self.0.prototype.borrow_mut() = prototype;
        Ok(())
    }

    pub(crate) fn set(&self, key: String, value: Value) {
        let mut properties = self.0.properties.borrow_mut();
        if let Some(property) = properties.get_mut(&key) {
            if property.writable {
                property.value = value;
            }
            return;
        }
        if !self.0.extensible.get() {
            return;
        }
        if self.0.array_prototype_exotic.get()
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
            self.0
                .index_property_count
                .set(self.0.index_property_count.get() + 1);
        }
        self.0.property_order.borrow_mut().push(key.clone());
        properties.insert(key, Property::enumerable(value));
    }

    pub(crate) fn define_property(&self, key: String, property: Property) {
        let mut properties = self.0.properties.borrow_mut();
        if !properties.contains_key(&key) {
            if is_array_index_key(&key) {
                self.0
                    .index_property_count
                    .set(self.0.index_property_count.get() + 1);
            }
            self.0.property_order.borrow_mut().push(key.clone());
        }
        properties.insert(key, property);
    }

    pub(crate) fn define_symbol_property(&self, symbol: ObjectRef, property: Property) {
        let mut properties = self.0.symbol_properties.borrow_mut();
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

    pub(crate) fn set_internal_non_enumerable(&self, key: &str, value: Value) {
        let mut properties = self.0.properties.borrow_mut();
        if let Some(property) = properties.get_mut(key) {
            property.value = value;
            return;
        }
        properties.insert(key.to_owned(), Property::non_enumerable(value));
    }

    /// Whether the object `prototype` appears as a function prototype anywhere
    /// in this object's chain. Used by `isPrototypeOf`/`instanceof` to walk past
    /// a function sitting mid-chain.
    pub(crate) fn has_own_property(&self, key: &str) -> bool {
        self.0.properties.borrow().contains_key(key)
    }

    pub(crate) fn has_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.0
            .symbol_properties
            .borrow()
            .iter()
            .any(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.0.extensible.get()
    }

    pub(crate) fn prevent_extensions(&self) {
        self.0.extensible.set(false);
    }

    pub(crate) fn seal(&self) {
        self.prevent_extensions();
        for property in self.0.properties.borrow_mut().values_mut() {
            property.make_non_configurable();
        }
        for (_, property) in self.0.symbol_properties.borrow_mut().iter_mut() {
            property.make_non_configurable();
        }
    }

    pub(crate) fn append_string_property(&self, key: &str, suffix: &str) -> Option<Value> {
        let mut properties = self.0.properties.borrow_mut();
        let property = properties.get_mut(key)?;
        if !property.writable || property.accessor {
            return None;
        }
        let Value::String(string) = &mut property.value else {
            return None;
        };
        std::rc::Rc::make_mut(string).push_str(suffix);
        Some(Value::String(string.clone()))
    }

    pub(crate) fn is_sealed(&self) -> bool {
        !self.0.extensible.get()
            && self
                .0
                .properties
                .borrow()
                .values()
                .all(|property| !property.configurable)
            && self
                .0
                .symbol_properties
                .borrow()
                .iter()
                .all(|(_, property)| !property.configurable)
    }

    pub(crate) fn freeze(&self) {
        self.prevent_extensions();
        for property in self.0.properties.borrow_mut().values_mut() {
            property.freeze_data();
        }
        for (_, property) in self.0.symbol_properties.borrow_mut().iter_mut() {
            property.freeze_data();
        }
    }

    pub(crate) fn is_frozen(&self) -> bool {
        !self.0.extensible.get()
            && self
                .0
                .properties
                .borrow()
                .values()
                .all(|property| !property.configurable && !property.writable)
            && self
                .0
                .symbol_properties
                .borrow()
                .iter()
                .all(|(_, property)| !property.configurable && !property.writable)
    }

    pub(crate) fn own_property(&self, key: &str) -> Option<Property> {
        let mut property = self.0.properties.borrow().get(key).cloned()?;
        if self.0.module_namespace_exotic.get()
            && let Some(bindings) = self.0.module_namespace_bindings.borrow().as_ref()
            && let Some(value) = bindings.value_for_export(key)
        {
            property.value = value.clone();
        }
        Some(property)
    }

    pub(crate) fn own_data_property_read(&self, key: &str) -> OwnDataPropertyRead {
        if self.0.module_namespace_exotic.get() {
            return OwnDataPropertyRead::NeedsSlowPath;
        }
        let properties = self.0.properties.borrow();
        match properties.get(key) {
            None => OwnDataPropertyRead::Missing,
            Some(property) if property.get.is_some() || property.accessor => {
                OwnDataPropertyRead::NeedsSlowPath
            }
            Some(property) => OwnDataPropertyRead::Data(property.value.clone()),
        }
    }

    pub(crate) fn module_namespace_export_property(
        &self,
        key: &str,
    ) -> Result<Option<Property>, RuntimeError> {
        if !self.0.module_namespace_exotic.get() {
            return Ok(None);
        }
        let mut property = match self.0.properties.borrow().get(key).cloned() {
            Some(property) => property,
            None => return Ok(None),
        };
        if let Some(bindings) = self.0.module_namespace_bindings.borrow().as_ref()
            && let Some(value) = bindings.value_for_export(key)
        {
            if value.is_uninitialized_lexical_marker() {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{key}`"),
                });
            }
            property.value = value;
        }
        Ok(Some(property))
    }

    pub(crate) fn own_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.0
            .symbol_properties
            .borrow()
            .iter()
            .find(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
            .map(|(_, property)| property.clone())
    }

    pub(crate) fn delete_own_property(&self, key: &str) -> bool {
        let mut properties = self.0.properties.borrow_mut();
        if properties
            .get(key)
            .is_some_and(|property| !property.configurable)
        {
            return false;
        }
        let removed = properties.remove(key);
        if removed.is_some() && is_array_index_key(key) {
            self.0
                .index_property_count
                .set(self.0.index_property_count.get().saturating_sub(1));
        }
        self.0
            .property_order
            .borrow_mut()
            .retain(|existing| existing != key);
        true
    }

    pub(crate) fn delete_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        let mut properties = self.0.symbol_properties.borrow_mut();
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
        let properties = self.0.properties.borrow();
        let order = self.0.property_order.borrow();
        if self.0.index_property_count.get() == 0 {
            return order
                .iter()
                .filter_map(|key| {
                    if is_internal_property_key(key) {
                        return None;
                    }
                    let property = properties.get(key.as_str())?;
                    include(property).then(|| key.clone())
                })
                .collect();
        }

        let mut indices = Vec::new();
        let mut strings = Vec::new();

        for key in order.iter() {
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
        self.0
            .symbol_properties
            .borrow()
            .iter()
            .map(|(symbol, _)| symbol.clone())
            .collect()
    }

    /// The [[Prototype]] as an object, or `None` if absent or a function. Use
    /// [`ObjectRef::prototype_slot`] when the function case matters.
    pub(crate) fn prototype(&self) -> Option<ObjectRef> {
        self.0
            .prototype
            .borrow()
            .as_ref()
            .and_then(Prototype::as_object)
    }

    pub(crate) fn set_prototype(&self, prototype: Option<ObjectRef>) -> Result<(), ()> {
        self.set_prototype_slot(prototype.map(Prototype::Object))
    }

    pub(crate) fn to_string_tag(&self) -> Option<String> {
        self.0.to_string_tag.borrow().clone().or_else(|| {
            self.0
                .prototype
                .borrow()
                .as_ref()
                .and_then(Prototype::to_string_tag)
        })
    }

    pub(crate) fn set_to_string_tag(&self, tag: &str) {
        *self.0.to_string_tag.borrow_mut() = Some(tag.to_owned());
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, mem, rc::Rc};

    use super::{ObjectData, ObjectRef};

    #[test]
    fn cloned_object_is_a_pointer_sized_shared_handle() {
        let object = ObjectRef::new(HashMap::new());
        let cloned = object.clone();

        assert!(Rc::ptr_eq(&object.0, &cloned.0));
        assert!(object.ptr_eq(&cloned));
        assert_eq!(
            mem::size_of::<ObjectRef>(),
            mem::size_of::<Rc<ObjectData>>()
        );
    }
}
