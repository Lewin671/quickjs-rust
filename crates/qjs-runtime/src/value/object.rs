use std::{
    cell::{Cell, OnceCell, RefCell},
    collections::HashMap,
    fmt,
    rc::{Rc, Weak},
};

use crate::private::{PrivateEnvironment, PrivateStorage};
use crate::{ArrayRef, Function, RuntimeError, function::DynamicBindings, proxy::ProxyRef};

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

/// Result of trying to complete `OrdinarySet` against an existing own data
/// property. `NeedsSlowPath` preserves the full descriptor/prototype path.
pub(crate) enum OwnDataPropertyWrite {
    Written,
    ReadOnly,
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
    /// A live array object used as a [[Prototype]]. Keep the original
    /// `ArrayRef`, rather than copying its current descriptors into an
    /// `ObjectRef`, so later indexed-property mutations remain observable.
    Array(ArrayPrototypeRef),
    Function(Function),
    Proxy(ProxyRef),
}

/// Live view of an array used as a [[Prototype]]. The fallback captures the
/// array's realm-intrinsic prototype at the point the slot is installed; an
/// explicit later `Object.setPrototypeOf(array, ...)` override still wins.
/// Boxing the pair keeps `Prototype` pointer-sized per variant.
#[derive(Clone)]
pub(crate) struct ArrayPrototypeRef(Rc<ArrayPrototypeData>);

struct ArrayPrototypeData {
    array: ArrayRef,
    default_prototype: Option<ObjectRef>,
}

impl ArrayPrototypeRef {
    fn new(array: ArrayRef, default_prototype: Option<ObjectRef>) -> Self {
        Self(Rc::new(ArrayPrototypeData {
            array,
            default_prototype,
        }))
    }

    pub(crate) fn array(&self) -> ArrayRef {
        self.0.array.clone()
    }

    pub(crate) fn effective_prototype_slot(&self) -> Option<Prototype> {
        self.0
            .array
            .prototype_slot_override()
            .unwrap_or_else(|| self.0.default_prototype.clone().map(Prototype::Object))
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        self.0.array.ptr_eq(&other.0.array)
    }

    pub(crate) fn property(&self, key: &str) -> Option<Property> {
        crate::array_own_property_descriptor(&self.0.array, key).or_else(|| {
            self.effective_prototype_slot()
                .and_then(|prototype| prototype.property(key))
        })
    }

    pub(crate) fn symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.0.array.own_symbol_property(symbol).or_else(|| {
            self.effective_prototype_slot()
                .and_then(|prototype| prototype.symbol_property(symbol))
        })
    }
}

impl Prototype {
    pub(crate) fn array(array: ArrayRef, default_prototype: Option<ObjectRef>) -> Self {
        Self::Array(ArrayPrototypeRef::new(array, default_prototype))
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Object(left), Self::Object(right)) => left.ptr_eq(right),
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Function(left), Self::Function(right)) => left.ptr_eq(right),
            (Self::Proxy(left), Self::Proxy(right)) => left.ptr_eq(right),
            _ => false,
        }
    }

    /// Walks this prototype (and its own chain) for the data/accessor property
    /// `key`, returning the first match.
    pub(crate) fn property(&self, key: &str) -> Option<Property> {
        match self {
            Self::Object(object) => object.property(key),
            Self::Array(array) => array.property(key),
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

    pub(crate) fn symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        match self {
            Self::Object(object) => object.symbol_property(symbol),
            Self::Array(array) => array.symbol_property(symbol),
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
            Self::Array(_) => None,
            // Functions never carry a Symbol.toStringTag in their own chain by
            // default; stop the search here.
            Self::Function(_) => None,
            Self::Proxy(_) => None,
        }
    }

    /// Whether this prototype is (or descends to) the object `target`, used to
    /// reject prototype cycles.
    fn would_cycle(&self, target: &ObjectRef) -> bool {
        self.would_cycle_inner(target, &mut Vec::new(), &mut Vec::new(), &mut Vec::new())
    }

    fn would_cycle_inner(
        &self,
        target: &ObjectRef,
        seen_objects: &mut Vec<ObjectRef>,
        seen_arrays: &mut Vec<ArrayRef>,
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
                    prototype.would_cycle_inner(target, seen_objects, seen_arrays, seen_functions)
                })
            }
            Self::Array(array_prototype) => {
                let array = array_prototype.array();
                if seen_arrays.iter().any(|seen| seen.ptr_eq(&array)) {
                    return false;
                }
                seen_arrays.push(array);
                array_prototype
                    .effective_prototype_slot()
                    .is_some_and(|prototype| {
                        prototype.would_cycle_inner(
                            target,
                            seen_objects,
                            seen_arrays,
                            seen_functions,
                        )
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
                        prototype.would_cycle_inner(
                            target,
                            seen_objects,
                            seen_arrays,
                            seen_functions,
                        )
                    })
            }
            Self::Proxy(_) => false,
        }
    }

    pub(crate) fn would_cycle_array(&self, target: &ArrayRef) -> bool {
        self.would_cycle_array_inner(target, &mut Vec::new(), &mut Vec::new(), &mut Vec::new())
    }

    fn would_cycle_array_inner(
        &self,
        target: &ArrayRef,
        seen_objects: &mut Vec<ObjectRef>,
        seen_arrays: &mut Vec<ArrayRef>,
        seen_functions: &mut Vec<Function>,
    ) -> bool {
        match self {
            Self::Array(array) => {
                let current = array.array();
                if current.ptr_eq(target) {
                    return true;
                }
                if seen_arrays.iter().any(|seen| seen.ptr_eq(&current)) {
                    return false;
                }
                seen_arrays.push(current);
                array.effective_prototype_slot().is_some_and(|prototype| {
                    prototype.would_cycle_array_inner(
                        target,
                        seen_objects,
                        seen_arrays,
                        seen_functions,
                    )
                })
            }
            Self::Object(object) => {
                if seen_objects.iter().any(|seen| seen.ptr_eq(object)) {
                    return false;
                }
                seen_objects.push(object.clone());
                object.prototype_slot().is_some_and(|prototype| {
                    prototype.would_cycle_array_inner(
                        target,
                        seen_objects,
                        seen_arrays,
                        seen_functions,
                    )
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
                        prototype.would_cycle_array_inner(
                            target,
                            seen_objects,
                            seen_arrays,
                            seen_functions,
                        )
                    })
            }
            Self::Proxy(_) => false,
        }
    }

    /// The prototype as a JavaScript value, for `getPrototypeOf` and friends.
    pub(crate) fn to_value(&self) -> Value {
        match self {
            Self::Object(object) => Value::Object(object.clone()),
            Self::Array(array) => Value::Array(array.array()),
            Self::Function(function) => Value::Function(function.clone()),
            Self::Proxy(proxy) => Value::Proxy(proxy.clone()),
        }
    }
}

/// Object storage reference.
#[derive(Clone)]
pub struct ObjectRef(Rc<ObjectData>);

#[derive(Clone)]
pub(crate) struct ObjectWeakRef(Weak<ObjectData>);

impl fmt::Debug for ObjectWeakRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ObjectWeakRef(..)")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SymbolBrand {
    None,
    Primitive,
    Boxed,
}

struct ObjectData {
    properties: RefCell<PropertyStorage>,
    /// Invalidates monomorphic named-read caches whenever an own string
    /// property's descriptor or value changes.
    property_revision: Cell<u64>,
    /// Count of own string keys that parse as array indices. Maintained as keys
    /// are added and removed so `has_own_index_property` is an O(1) check; this
    /// keeps the `array[i] = x` fast path from scanning a prototype's keys on
    /// every write.
    index_property_count: Cell<usize>,
    extensible: Cell<bool>,
    prototype: RefCell<Option<Prototype>>,
    raw_json: Cell<bool>,
    array_prototype_exotic: Cell<bool>,
    /// Whether this object has the TypedArray internal slots. Keep the brand
    /// outside string-keyed property storage so ordinary property reads can
    /// reject the integer-indexed exotic path without a HashMap probe.
    typed_array_exotic: Cell<bool>,
    /// Whether this object carries the Symbol [[SymbolData]] internal slot.
    /// Keep the primitive/boxed distinction outside string-keyed storage so
    /// ubiquitous symbol checks do not probe the property HashMap.
    symbol_brand: Cell<SymbolBrand>,
    immutable_prototype_exotic: Cell<bool>,
    module_namespace_exotic: Cell<bool>,
    cold: OnceCell<Box<ObjectColdData>>,
}

#[derive(Default)]
struct ObjectColdData {
    symbol_properties: RefCell<Vec<(ObjectRef, Property)>>,
    to_string_tag: RefCell<Option<String>>,
    module_namespace_bindings: RefCell<Option<ModuleNamespaceBindings>>,
    /// Generator [[GeneratorState]] for generator objects; `None` for ordinary
    /// objects. Lazily allocated so non-generator objects pay only one `Rc`.
    generator_state: RefCell<Option<crate::bytecode::GeneratorState>>,
    /// Async-generator internal state (the [[AsyncGeneratorQueue]] of pending
    /// requests plus the draining flag) for async generator objects; `None` for
    /// every other object.
    async_generator_state: RefCell<Option<crate::async_generator::AsyncGeneratorInternal>>,
    /// Private-name state: per-object storage (fields and brands) and, for class
    /// prototype objects, the private environment their members resolve `#x`
    /// references through. Lazily populated.
    private_state: RefCell<crate::private::PrivateState>,
    /// Opaque byte storage for ArrayBuffer objects. The public ArrayBuffer brand
    /// remains a hidden property; bytes live here so typed-array element access
    /// does not have to encode and decode a string on every read or write.
    internal_bytes: RefCell<Option<Vec<u8>>>,
    /// TypedArray internal slots. Keeping these out of string-keyed property
    /// storage avoids repeated hashing on every indexed access and prevents
    /// guessed property names from mutating spec-internal view metadata.
    typed_array_slots: OnceCell<crate::typed_array::TypedArraySlots>,
    /// Iterator.zip helper internal state. Ordinary objects hold `None`; zip
    /// helpers store their records here so advancement does not round-trip
    /// through observable-looking property storage.
    iterator_zip_state: RefCell<Option<crate::iterator::ZipState>>,
    /// Stable intrinsic identity for a synthetic realm global. The Test262
    /// host represents additional realms as ordinary self-referential
    /// `globalThis` objects; capture their Object/Array prototypes once when
    /// that identity is established so later global rebinding cannot change
    /// the realm intrinsics used by dynamic code.
    realm_intrinsic_identity: OnceCell<RealmIntrinsicIdentity>,
    /// Cross-thread backing for a `SharedArrayBuffer` under the Test262
    /// `$262.agent` harness. When present, the buffer's bytes live in this
    /// `Arc`-shared store (so a worker agent on another OS thread observes the
    /// same memory) instead of `internal_bytes`. Gated so the default build's
    /// object layout is unchanged.
    #[cfg(feature = "agents")]
    shared_backing: RefCell<Option<crate::array_buffer::SharedBackingRef>>,
}

struct RealmIntrinsicIdentity {
    object_prototype: ObjectRef,
    array_prototype: ObjectRef,
}

impl ObjectData {
    fn cold(&self) -> &ObjectColdData {
        self.cold.get_or_init(Box::default).as_ref()
    }

    fn cold_if_present(&self) -> Option<&ObjectColdData> {
        self.cold.get().map(Box::as_ref)
    }
}

/// Shared key layout for object literals whose property names are statically
/// known. The bytecode owns one shape; each evaluated object allocates only
/// its property values until a structural mutation requires generic storage.
#[derive(Debug)]
pub(crate) struct ObjectLiteralShape {
    keys: Rc<[Rc<str>]>,
    input_slots: Rc<[usize]>,
    lookup: HashMap<Rc<str>, usize>,
    index_property_count: usize,
}

impl ObjectLiteralShape {
    pub(crate) fn new(input_keys: Vec<Rc<str>>) -> Rc<Self> {
        let mut keys = Vec::with_capacity(input_keys.len());
        let mut lookup = HashMap::with_capacity(input_keys.len());
        let mut input_slots = Vec::with_capacity(input_keys.len());
        for key in input_keys {
            let slot = match lookup.get(key.as_ref()) {
                Some(slot) => *slot,
                None => {
                    let slot = keys.len();
                    keys.push(key.clone());
                    lookup.insert(key, slot);
                    slot
                }
            };
            input_slots.push(slot);
        }
        let index_property_count = keys.iter().filter(|key| is_array_index_key(key)).count();
        Self {
            keys: keys.into(),
            input_slots: input_slots.into(),
            lookup,
            index_property_count,
        }
        .into()
    }

    pub(crate) fn input_len(&self) -> usize {
        self.input_slots.len()
    }

    pub(crate) fn unique_len(&self) -> usize {
        self.keys.len()
    }

    /// Returns the input value that supplies `key` after duplicate literal
    /// definitions have been applied from left to right.
    ///
    /// `input_slots` maps every source property to its shared storage slot,
    /// while `lookup` maps the final key to that slot. Walking the input map
    /// backwards therefore identifies the last source value without exposing
    /// the shape's storage representation to bytecode analyses.
    pub(crate) fn final_input_index(&self, key: &str) -> Option<usize> {
        let slot = *self.lookup.get(key)?;
        self.input_slots
            .iter()
            .rposition(|input_slot| *input_slot == slot)
    }
}

/// Payload for the cold, unbounded property-storage path. Boxed so an
/// object that never grows past the shaped/small paths does not pay for the
/// `HashMap` + `Vec` footprint in every `PropertyStorage` (the enum's size is
/// otherwise governed by its largest variant even when that variant is
/// inactive).
struct DynamicPropertyStorage {
    properties: HashMap<Rc<str>, Property>,
    order: Vec<Rc<str>>,
}

enum PropertyStorage {
    /// Most ordinary objects have only a handful of properties. Keep their
    /// descriptors and insertion order in one allocation, and pay hashing plus
    /// a second order-vector allocation only after the object grows past the
    /// small-object threshold.
    Small {
        entries: Vec<(Rc<str>, Property)>,
    },
    Dynamic(Box<DynamicPropertyStorage>),
    Shaped {
        shape: Rc<ObjectLiteralShape>,
        properties: Vec<Property>,
    },
    ShapedPair {
        shape: Rc<ObjectLiteralShape>,
        values: [Value; 2],
    },
}

impl PropertyStorage {
    const SMALL_LIMIT: usize = 8;

    fn dynamic(properties: HashMap<Rc<str>, Property>, order: Vec<Rc<str>>) -> Self {
        if properties.len() <= Self::SMALL_LIMIT {
            let mut properties = properties;
            let entries = order
                .into_iter()
                .map(|key| {
                    let property = properties
                        .remove(&key)
                        .expect("property order must cover every initial property");
                    (key, property)
                })
                .collect();
            debug_assert!(properties.is_empty());
            return Self::Small { entries };
        }
        Self::Dynamic(Box::new(DynamicPropertyStorage { properties, order }))
    }

    fn len(&self) -> usize {
        match self {
            Self::Small { entries } => entries.len(),
            Self::Dynamic(dynamic) => dynamic.properties.len(),
            Self::Shaped { properties, .. } => properties.len(),
            Self::ShapedPair { .. } => 2,
        }
    }

    fn get(&self, key: &str) -> Option<Property> {
        match self {
            Self::Small { entries } => entries
                .iter()
                .find(|(candidate, _)| candidate.as_ref() == key)
                .map(|(_, property)| property.clone()),
            Self::Dynamic(dynamic) => dynamic.properties.get(key).cloned(),
            Self::Shaped { shape, properties } => shape
                .lookup
                .get(key)
                .and_then(|slot| properties.get(*slot))
                .cloned(),
            Self::ShapedPair { shape, values } => {
                let value = values.get(*shape.lookup.get(key)?)?.clone();
                Some(Property::enumerable(value))
            }
        }
    }

    fn value(&self, key: &str) -> Option<Value> {
        match self {
            Self::Small { entries } => entries
                .iter()
                .find(|(candidate, _)| candidate.as_ref() == key)
                .map(|(_, property)| property.value.clone()),
            Self::Dynamic(dynamic) => dynamic
                .properties
                .get(key)
                .map(|property| property.value.clone()),
            Self::Shaped { shape, properties } => shape
                .lookup
                .get(key)
                .and_then(|slot| properties.get(*slot))
                .map(|property| property.value.clone()),
            Self::ShapedPair { shape, values } => values.get(*shape.lookup.get(key)?).cloned(),
        }
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut Property> {
        if matches!(self, Self::ShapedPair { .. }) {
            self.ensure_dynamic();
        }
        match self {
            Self::Small { entries } => entries
                .iter_mut()
                .find(|(candidate, _)| candidate.as_ref() == key)
                .map(|(_, property)| property),
            Self::Dynamic(dynamic) => dynamic.properties.get_mut(key),
            Self::Shaped { shape, properties } => {
                let slot = *shape.lookup.get(key)?;
                properties.get_mut(slot)
            }
            Self::ShapedPair { .. } => unreachable!("literal pair was converted to dynamic"),
        }
    }

    fn contains_key(&self, key: &str) -> bool {
        match self {
            Self::Small { entries } => entries
                .iter()
                .any(|(candidate, _)| candidate.as_ref() == key),
            Self::Dynamic(dynamic) => dynamic.properties.contains_key(key),
            Self::Shaped { shape, .. } | Self::ShapedPair { shape, .. } => {
                shape.lookup.contains_key(key)
            }
        }
    }

    fn own_data_read(&self, key: &str) -> OwnDataPropertyRead {
        match self {
            Self::Small { entries } => data_property_read(
                entries
                    .iter()
                    .find(|(candidate, _)| candidate.as_ref() == key)
                    .map(|(_, property)| property),
            ),
            Self::ShapedPair { shape, values } => shape
                .lookup
                .get(key)
                .and_then(|slot| values.get(*slot))
                .map_or(OwnDataPropertyRead::Missing, |value| {
                    OwnDataPropertyRead::Data(value.clone())
                }),
            Self::Dynamic(dynamic) => data_property_read(dynamic.properties.get(key)),
            Self::Shaped { shape, properties } => {
                data_property_read(shape.lookup.get(key).and_then(|slot| properties.get(*slot)))
            }
        }
    }

    fn writable_number(&self, key: &str) -> Option<f64> {
        match self {
            Self::Small { entries } => writable_property_number(
                &entries
                    .iter()
                    .find(|(candidate, _)| candidate.as_ref() == key)?
                    .1,
            ),
            Self::ShapedPair { shape, values } => match values.get(*shape.lookup.get(key)?)? {
                Value::Number(value) => Some(*value),
                _ => None,
            },
            Self::Dynamic(dynamic) => writable_property_number(dynamic.properties.get(key)?),
            Self::Shaped { shape, properties } => {
                writable_property_number(properties.get(*shape.lookup.get(key)?)?)
            }
        }
    }

    fn write_existing_data(&mut self, key: &str, value: &Value) -> OwnDataPropertyWrite {
        match self {
            Self::Small { entries } => write_existing_property(
                entries
                    .iter_mut()
                    .find(|(candidate, _)| candidate.as_ref() == key)
                    .map(|(_, property)| property),
                value,
            ),
            Self::ShapedPair { shape, values } => {
                let Some(slot) = shape.lookup.get(key) else {
                    return OwnDataPropertyWrite::NeedsSlowPath;
                };
                values[*slot] = value.clone();
                OwnDataPropertyWrite::Written
            }
            Self::Dynamic(dynamic) => {
                write_existing_property(dynamic.properties.get_mut(key), value)
            }
            Self::Shaped { shape, properties } => {
                let property = shape
                    .lookup
                    .get(key)
                    .and_then(|slot| properties.get_mut(*slot));
                write_existing_property(property, value)
            }
        }
    }

    fn for_each_mut(&mut self, apply: impl FnMut(&mut Property)) {
        if matches!(self, Self::ShapedPair { .. }) {
            self.ensure_dynamic();
        }
        match self {
            Self::Small { entries } => entries
                .iter_mut()
                .map(|(_, property)| property)
                .for_each(apply),
            Self::Dynamic(dynamic) => dynamic.properties.values_mut().for_each(apply),
            Self::Shaped { properties, .. } => properties.iter_mut().for_each(apply),
            Self::ShapedPair { .. } => unreachable!("literal pair was converted to dynamic"),
        }
    }

    fn all(&self, predicate: impl Fn(&Property) -> bool) -> bool {
        match self {
            Self::Small { entries } => entries.iter().all(|(_, property)| predicate(property)),
            Self::Dynamic(dynamic) => dynamic.properties.values().all(predicate),
            Self::Shaped { properties, .. } => properties.iter().all(predicate),
            Self::ShapedPair { values, .. } => values
                .iter()
                .all(|value| predicate(&Property::enumerable(value.clone()))),
        }
    }

    fn order(&self) -> Option<&[Rc<str>]> {
        match self {
            Self::Small { .. } => None,
            Self::Dynamic(dynamic) => Some(&dynamic.order),
            Self::Shaped { shape, .. } => Some(&shape.keys),
            Self::ShapedPair { shape, .. } => Some(&shape.keys),
        }
    }

    fn ensure_dynamic(&mut self) {
        match self {
            Self::Dynamic(_) => {}
            Self::Small { entries } => {
                let entries = std::mem::take(entries);
                let order = entries.iter().map(|(key, _)| key.clone()).collect();
                let properties = entries.into_iter().collect();
                *self = Self::Dynamic(Box::new(DynamicPropertyStorage { properties, order }));
            }
            Self::Shaped { shape, properties } => {
                let properties = std::mem::take(properties);
                let order = shape.keys.to_vec();
                let properties = order.iter().cloned().zip(properties).collect();
                *self = Self::Dynamic(Box::new(DynamicPropertyStorage { properties, order }));
            }
            Self::ShapedPair { shape, values } => {
                let order = shape.keys.to_vec();
                let properties = order
                    .iter()
                    .cloned()
                    .zip(values.iter().cloned().map(Property::enumerable))
                    .collect();
                *self = Self::Dynamic(Box::new(DynamicPropertyStorage { properties, order }));
            }
        }
    }

    fn insert(&mut self, key: Rc<str>, property: Property) -> Option<Property> {
        if let Some(existing) = self.get_mut(&key) {
            return Some(std::mem::replace(existing, property));
        }
        if let Self::Small { entries } = self
            && entries.len() < Self::SMALL_LIMIT
        {
            entries.push((key, property));
            return None;
        }
        self.ensure_dynamic();
        let Self::Dynamic(dynamic) = self else {
            unreachable!("property storage was converted to dynamic")
        };
        dynamic.order.push(key.clone());
        dynamic.properties.insert(key, property)
    }

    fn insert_unordered(&mut self, key: Rc<str>, property: Property) -> Option<Property> {
        if let Some(existing) = self.get_mut(&key) {
            return Some(std::mem::replace(existing, property));
        }
        self.ensure_dynamic();
        let Self::Dynamic(dynamic) = self else {
            unreachable!("property storage was converted to dynamic")
        };
        dynamic.properties.insert(key, property)
    }

    fn remove(&mut self, key: &str) -> Option<Property> {
        if let Self::Small { entries } = self {
            let index = entries
                .iter()
                .position(|(candidate, _)| candidate.as_ref() == key)?;
            return Some(entries.remove(index).1);
        }
        if !self.contains_key(key) {
            return None;
        }
        self.ensure_dynamic();
        let Self::Dynamic(dynamic) = self else {
            unreachable!("property storage was converted to dynamic")
        };
        let removed = dynamic.properties.remove(key);
        dynamic.order.retain(|existing| existing.as_ref() != key);
        removed
    }
}

fn data_property_read(property: Option<&Property>) -> OwnDataPropertyRead {
    match property {
        None => OwnDataPropertyRead::Missing,
        Some(property) if property.is_accessor() => OwnDataPropertyRead::NeedsSlowPath,
        Some(property) => OwnDataPropertyRead::Data(property.value.clone()),
    }
}

fn writable_property_number(property: &Property) -> Option<f64> {
    if property.is_accessor() || !property.writable {
        return None;
    }
    match property.value {
        Value::Number(value) => Some(value),
        _ => None,
    }
}

fn write_existing_property(property: Option<&mut Property>, value: &Value) -> OwnDataPropertyWrite {
    let Some(property) = property else {
        return OwnDataPropertyWrite::NeedsSlowPath;
    };
    if property.is_accessor() {
        return OwnDataPropertyWrite::NeedsSlowPath;
    }
    if !property.writable {
        return OwnDataPropertyWrite::ReadOnly;
    }
    property.value = value.clone();
    OwnDataPropertyWrite::Written
}

impl fmt::Debug for ObjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol_property_count = self
            .0
            .cold_if_present()
            .map_or(0, |cold| cold.symbol_properties.borrow().len());
        let to_string_tag = self
            .0
            .cold_if_present()
            .and_then(|cold| cold.to_string_tag.borrow().clone());
        formatter
            .debug_struct("ObjectRef")
            .field("properties", &self.0.properties.borrow().len())
            .field("symbol_properties", &symbol_property_count)
            .field("has_prototype", &self.0.prototype.borrow().is_some())
            .field("to_string_tag", &to_string_tag)
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
        let properties: HashMap<Rc<str>, Property> = properties
            .into_iter()
            .map(|(key, value)| (Rc::from(key), Property::enumerable(value)))
            .collect();
        let mut property_order: Vec<_> = properties.keys().cloned().collect();
        property_order.sort();
        let index_property_count = property_order
            .iter()
            .filter(|key| is_array_index_key(key))
            .count();
        Self(Rc::new(ObjectData {
            properties: RefCell::new(PropertyStorage::dynamic(properties, property_order)),
            property_revision: Cell::new(0),
            index_property_count: Cell::new(index_property_count),
            extensible: Cell::new(true),
            prototype: RefCell::new(prototype),
            raw_json: Cell::new(false),
            array_prototype_exotic: Cell::new(false),
            typed_array_exotic: Cell::new(false),
            symbol_brand: Cell::new(SymbolBrand::None),
            immutable_prototype_exotic: Cell::new(false),
            module_namespace_exotic: Cell::new(false),
            cold: OnceCell::new(),
        }))
    }

    /// Builds a plain object literal from statically known string keys without
    /// re-running observable property-key conversion or copying the keys for
    /// every object. Duplicate keys retain their first insertion position and
    /// their last value, matching CreateDataProperty semantics.
    pub(crate) fn with_literal_properties(
        shape: Rc<ObjectLiteralShape>,
        values: Vec<Value>,
        prototype: Option<ObjectRef>,
    ) -> Self {
        debug_assert_eq!(shape.input_slots.len(), values.len());
        let mut properties = vec![None; shape.keys.len()];
        for (slot, value) in shape.input_slots.iter().copied().zip(values) {
            properties[slot] = Some(Property::enumerable(value));
        }
        let properties = properties
            .into_iter()
            .map(|property| property.expect("literal shape slot must have a value"))
            .collect::<Vec<_>>();
        let index_property_count = shape.index_property_count;
        let properties = if properties.len() == 2 {
            let properties: [Property; 2] = properties
                .try_into()
                .unwrap_or_else(|_| unreachable!("literal pair length checked"));
            PropertyStorage::ShapedPair {
                shape,
                values: properties.map(|property| property.value),
            }
        } else {
            PropertyStorage::Shaped { shape, properties }
        };
        Self(Rc::new(ObjectData {
            properties: RefCell::new(properties),
            property_revision: Cell::new(0),
            index_property_count: Cell::new(index_property_count),
            extensible: Cell::new(true),
            prototype: RefCell::new(prototype.map(Prototype::Object)),
            raw_json: Cell::new(false),
            array_prototype_exotic: Cell::new(false),
            typed_array_exotic: Cell::new(false),
            symbol_brand: Cell::new(SymbolBrand::None),
            immutable_prototype_exotic: Cell::new(false),
            module_namespace_exotic: Cell::new(false),
            cold: OnceCell::new(),
        }))
    }

    /// Builds the common two-property literal without intermediate value or
    /// descriptor vectors. The shape guard guarantees distinct source keys in
    /// first-insertion order, so operand-stack order is storage-slot order.
    pub(crate) fn with_literal_pair(
        shape: Rc<ObjectLiteralShape>,
        values: [Value; 2],
        prototype: Option<ObjectRef>,
    ) -> Self {
        debug_assert_eq!(shape.input_len(), 2);
        debug_assert_eq!(shape.unique_len(), 2);
        let index_property_count = shape.index_property_count;
        Self(Rc::new(ObjectData {
            properties: RefCell::new(PropertyStorage::ShapedPair { shape, values }),
            property_revision: Cell::new(0),
            index_property_count: Cell::new(index_property_count),
            extensible: Cell::new(true),
            prototype: RefCell::new(prototype.map(Prototype::Object)),
            raw_json: Cell::new(false),
            array_prototype_exotic: Cell::new(false),
            typed_array_exotic: Cell::new(false),
            symbol_brand: Cell::new(SymbolBrand::None),
            immutable_prototype_exotic: Cell::new(false),
            module_namespace_exotic: Cell::new(false),
            cold: OnceCell::new(),
        }))
    }

    /// The generator [[GeneratorState]] cell for this object. Non-generator
    /// objects hold `None`; generator objects store their resumable state here.
    pub(crate) fn generator_state(&self) -> &RefCell<Option<crate::bytecode::GeneratorState>> {
        &self.0.cold().generator_state
    }

    /// The async-generator internal-state cell for this object. Ordinary objects
    /// (and plain generators) hold `None`; async generator objects store their
    /// request queue and draining flag here.
    pub(crate) fn async_generator_state(
        &self,
    ) -> &RefCell<Option<crate::async_generator::AsyncGeneratorInternal>> {
        &self.0.cold().async_generator_state
    }

    /// Returns the object's private-name storage, creating it on first use.
    pub(crate) fn private_storage(&self) -> PrivateStorage {
        self.0
            .cold()
            .private_state
            .borrow_mut()
            .storage
            .get_or_insert_with(PrivateStorage::new)
            .clone()
    }

    /// Sets the private environment carried by a class prototype object.
    pub(crate) fn set_private_environment(&self, environment: PrivateEnvironment) {
        self.0.cold().private_state.borrow_mut().environment = Some(environment);
    }

    /// Returns the private environment carried by this object, if any.
    pub(crate) fn private_environment(&self) -> Option<PrivateEnvironment> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.private_state.borrow().environment.clone())
    }

    pub(crate) fn internal_bytes(&self) -> Option<Vec<u8>> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.internal_bytes.borrow().clone())
    }

    pub(crate) fn with_internal_bytes<T>(&self, f: impl FnOnce(Option<&[u8]>) -> T) -> T {
        let Some(cold) = self.0.cold_if_present() else {
            return f(None);
        };
        let bytes = cold.internal_bytes.borrow();
        f(bytes.as_deref())
    }

    pub(crate) fn set_internal_bytes(&self, bytes: Vec<u8>) {
        *self.0.cold().internal_bytes.borrow_mut() = Some(bytes);
    }

    pub(crate) fn clear_internal_bytes(&self) {
        if let Some(cold) = self.0.cold_if_present() {
            *cold.internal_bytes.borrow_mut() = None;
        }
    }

    pub(crate) fn with_internal_bytes_mut<T>(
        &self,
        f: impl FnOnce(&mut Vec<u8>) -> T,
    ) -> Option<T> {
        self.0
            .cold_if_present()?
            .internal_bytes
            .borrow_mut()
            .as_mut()
            .map(f)
    }

    /// The cross-thread `SharedArrayBuffer` backing for this object, if one was
    /// installed (agents harness only).
    #[cfg(feature = "agents")]
    pub(crate) fn shared_backing(&self) -> Option<crate::array_buffer::SharedBackingRef> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.shared_backing.borrow().clone())
    }

    /// Installs the cross-thread `SharedArrayBuffer` backing for this object.
    #[cfg(feature = "agents")]
    pub(crate) fn set_shared_backing(&self, backing: crate::array_buffer::SharedBackingRef) {
        *self.0.cold().shared_backing.borrow_mut() = Some(backing);
    }

    pub(crate) fn set_iterator_zip_state(&self, state: crate::iterator::ZipState) {
        *self.0.cold().iterator_zip_state.borrow_mut() = Some(state);
    }

    pub(crate) fn with_iterator_zip_state_mut<T>(
        &self,
        f: impl FnOnce(&mut crate::iterator::ZipState) -> T,
    ) -> Option<T> {
        self.0
            .cold_if_present()?
            .iterator_zip_state
            .borrow_mut()
            .as_mut()
            .map(f)
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn downgrade(&self) -> ObjectWeakRef {
        ObjectWeakRef(Rc::downgrade(&self.0))
    }

    pub(crate) fn property_revision(&self) -> u64 {
        self.0.property_revision.get()
    }

    fn bump_property_revision(&self) {
        self.0
            .property_revision
            .set(self.0.property_revision.get().wrapping_add(1));
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

    fn mark_typed_array_exotic(&self) {
        self.0.typed_array_exotic.set(true);
    }

    pub(crate) fn install_typed_array_slots(&self, slots: crate::typed_array::TypedArraySlots) {
        self.mark_typed_array_exotic();
        let installed = self.0.cold().typed_array_slots.set(slots);
        debug_assert!(
            installed.is_ok(),
            "TypedArray internal slots must only be installed once"
        );
    }

    pub(crate) fn typed_array_slots(&self) -> Option<&crate::typed_array::TypedArraySlots> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.typed_array_slots.get())
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
        *self.0.cold().module_namespace_bindings.borrow_mut() = Some(bindings);
    }

    pub(crate) fn get(&self, key: &str) -> Option<Value> {
        self.0.properties.borrow().value(key).or_else(|| {
            self.0
                .prototype
                .borrow()
                .as_ref()
                .and_then(|proto| proto.get(key))
        })
    }

    pub(crate) fn property(&self, key: &str) -> Option<Property> {
        self.0.properties.borrow().get(key).or_else(|| {
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
        self.set_shared_key(key.into(), value);
    }

    /// Sets a statically owned property key without copying it out of bytecode.
    /// Existing properties still retain their original insertion-order key.
    pub(crate) fn set_shared_key(&self, key: Rc<str>, value: Value) {
        let establishes_realm_identity = key.as_ref() == "globalThis"
            && matches!(&value, Value::Object(global_this) if self.ptr_eq(global_this));
        let mut properties = self.0.properties.borrow_mut();
        if let Some(property) = properties.get_mut(&key) {
            if property.writable {
                property.value = value;
                self.bump_property_revision();
                drop(properties);
                if establishes_realm_identity {
                    self.capture_realm_intrinsic_identity();
                }
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
        properties.insert(key, Property::enumerable(value));
        self.bump_property_revision();
        drop(properties);
        if establishes_realm_identity {
            self.capture_realm_intrinsic_identity();
        }
    }

    pub(crate) fn define_property(&self, key: String, property: Property) {
        let establishes_realm_identity = key == "globalThis"
            && !property.is_accessor()
            && matches!(&property.value, Value::Object(global_this) if self.ptr_eq(global_this));
        let mut properties = self.0.properties.borrow_mut();
        if let Some(existing) = properties.get_mut(key.as_str()) {
            *existing = property;
        } else {
            if is_array_index_key(&key) {
                self.0
                    .index_property_count
                    .set(self.0.index_property_count.get() + 1);
            }
            let key: Rc<str> = key.into();
            properties.insert(key, property);
        }
        self.bump_property_revision();
        drop(properties);
        if establishes_realm_identity {
            self.capture_realm_intrinsic_identity();
        }
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

    pub(crate) fn define_non_enumerable(&self, key: String, value: Value) {
        self.define_property(key, Property::non_enumerable(value));
    }

    pub(crate) fn set_internal_non_enumerable(&self, key: &str, value: Value) {
        let mut properties = self.0.properties.borrow_mut();
        if let Some(property) = properties.get_mut(key) {
            property.value = value;
            self.bump_property_revision();
            return;
        }
        properties.insert_unordered(Rc::from(key), Property::non_enumerable(value));
        self.bump_property_revision();
    }

    /// Whether the object `prototype` appears as a function prototype anywhere
    /// in this object's chain. Used by `isPrototypeOf`/`instanceof` to walk past
    /// a function sitting mid-chain.
    pub(crate) fn has_own_property(&self, key: &str) -> bool {
        self.0.properties.borrow().contains_key(key)
    }

    pub(crate) fn has_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.0.cold_if_present().is_some_and(|cold| {
            cold.symbol_properties
                .borrow()
                .iter()
                .any(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
        })
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.0.extensible.get()
    }

    pub(crate) fn prevent_extensions(&self) {
        self.0.extensible.set(false);
    }

    pub(crate) fn seal(&self) {
        self.prevent_extensions();
        self.0
            .properties
            .borrow_mut()
            .for_each_mut(Property::make_non_configurable);
        if let Some(cold) = self.0.cold_if_present() {
            for (_, property) in cold.symbol_properties.borrow_mut().iter_mut() {
                property.make_non_configurable();
            }
        }
    }

    pub(crate) fn append_string_property(&self, key: &str, suffix: &str) -> Option<Value> {
        let mut properties = self.0.properties.borrow_mut();
        let property = properties.get_mut(key)?;
        if !property.writable || property.is_accessor() {
            return None;
        }
        let Value::String(string) = &mut property.value else {
            return None;
        };
        std::rc::Rc::make_mut(string).push_str(suffix);
        self.bump_property_revision();
        Some(Value::String(string.clone()))
    }

    pub(crate) fn is_sealed(&self) -> bool {
        !self.0.extensible.get()
            && self
                .0
                .properties
                .borrow()
                .all(|property| !property.configurable)
            && self.0.cold_if_present().is_none_or(|cold| {
                cold.symbol_properties
                    .borrow()
                    .iter()
                    .all(|(_, property)| !property.configurable)
            })
    }

    pub(crate) fn freeze(&self) {
        self.prevent_extensions();
        self.0
            .properties
            .borrow_mut()
            .for_each_mut(Property::freeze_data);
        if let Some(cold) = self.0.cold_if_present() {
            for (_, property) in cold.symbol_properties.borrow_mut().iter_mut() {
                property.freeze_data();
            }
        }
    }

    pub(crate) fn is_frozen(&self) -> bool {
        !self.0.extensible.get()
            && self
                .0
                .properties
                .borrow()
                .all(|property| !property.configurable && !property.writable)
            && self.0.cold_if_present().is_none_or(|cold| {
                cold.symbol_properties
                    .borrow()
                    .iter()
                    .all(|(_, property)| !property.configurable && !property.writable)
            })
    }

    pub(crate) fn own_property(&self, key: &str) -> Option<Property> {
        let mut property = self.0.properties.borrow().get(key)?;
        if self.0.module_namespace_exotic.get()
            && let Some(cold) = self.0.cold_if_present()
            && let Some(bindings) = cold.module_namespace_bindings.borrow().as_ref()
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
        self.0.properties.borrow().own_data_read(key)
    }

    /// Returns the shared literal shape and storage slot for an unmodified
    /// data-only object literal. Named-property caches use this to share one
    /// cache entry across distinct objects created by the same bytecode site.
    pub(crate) fn literal_data_slot(&self, key: &str) -> Option<(Rc<ObjectLiteralShape>, usize)> {
        if self.0.module_namespace_exotic.get() || self.property_revision() != 0 {
            return None;
        }
        let properties = self.0.properties.borrow();
        let shape = match &*properties {
            PropertyStorage::Shaped { shape, .. } | PropertyStorage::ShapedPair { shape, .. } => {
                shape
            }
            PropertyStorage::Small { .. } | PropertyStorage::Dynamic(_) => return None,
        };
        let slot = *shape.lookup.get(key)?;
        Some((shape.clone(), slot))
    }

    /// Reads a previously resolved literal slot after checking that this
    /// object still has the same unmodified shared shape.
    pub(crate) fn literal_data_slot_value(
        &self,
        expected_shape: &Rc<ObjectLiteralShape>,
        slot: usize,
    ) -> Option<Value> {
        if self.0.module_namespace_exotic.get() || self.property_revision() != 0 {
            return None;
        }
        match &*self.0.properties.borrow() {
            PropertyStorage::Shaped { shape, properties } if Rc::ptr_eq(shape, expected_shape) => {
                properties.get(slot).map(|property| property.value.clone())
            }
            PropertyStorage::ShapedPair { shape, values } if Rc::ptr_eq(shape, expected_shape) => {
                values.get(slot).cloned()
            }
            _ => None,
        }
    }

    /// Reads a writable ordinary own numeric data property for scalar
    /// replacement. Exotic namespaces, accessors, read-only descriptors, and
    /// non-numeric values stay on the observable property path.
    pub(crate) fn writable_own_data_number(&self, key: &str) -> Option<f64> {
        if self.0.module_namespace_exotic.get() {
            return None;
        }
        self.0.properties.borrow().writable_number(key)
    }

    /// Updates an existing ordinary own data property without cloning its
    /// descriptor or walking the prototype chain. Accessors, missing keys, and
    /// module namespace exports retain their observable slow-path behavior.
    pub(crate) fn write_existing_own_data_property(
        &self,
        key: &str,
        value: &Value,
    ) -> OwnDataPropertyWrite {
        if self.0.module_namespace_exotic.get() {
            return OwnDataPropertyWrite::NeedsSlowPath;
        }
        let establishes_realm_identity = key == "globalThis"
            && matches!(value, Value::Object(global_this) if self.ptr_eq(global_this));
        let result = self
            .0
            .properties
            .borrow_mut()
            .write_existing_data(key, value);
        if matches!(result, OwnDataPropertyWrite::Written) {
            self.bump_property_revision();
            if establishes_realm_identity {
                self.capture_realm_intrinsic_identity();
            }
        }
        result
    }

    /// Returns the stable Object/Array prototype identity captured for a
    /// synthetic realm global. This metadata is an internal slot: it never
    /// participates in property reflection, enumeration, or snapshots.
    pub(crate) fn realm_intrinsic_prototype_identity(&self) -> Option<(ObjectRef, ObjectRef)> {
        let identity = &self.0.cold_if_present()?.realm_intrinsic_identity;
        let identity = identity.get()?;
        Some((
            identity.object_prototype.clone(),
            identity.array_prototype.clone(),
        ))
    }

    fn capture_realm_intrinsic_identity(&self) {
        if self
            .0
            .cold_if_present()
            .is_some_and(|cold| cold.realm_intrinsic_identity.get().is_some())
        {
            return;
        }
        let object_prototype = self.own_constructor_prototype("Object");
        let array_prototype = self.own_constructor_prototype("Array");
        let (Some(object_prototype), Some(array_prototype)) = (object_prototype, array_prototype)
        else {
            return;
        };
        let initialized = self
            .0
            .cold()
            .realm_intrinsic_identity
            .set(RealmIntrinsicIdentity {
                object_prototype,
                array_prototype,
            })
            .is_ok();
        debug_assert!(initialized, "realm intrinsic identity initialized twice");
    }

    fn own_constructor_prototype(&self, name: &str) -> Option<ObjectRef> {
        let Value::Function(constructor) = self.own_property(name)?.value else {
            return None;
        };
        crate::function_prototype(&constructor)
    }

    pub(crate) fn module_namespace_export_property(
        &self,
        key: &str,
    ) -> Result<Option<Property>, RuntimeError> {
        if !self.0.module_namespace_exotic.get() {
            return Ok(None);
        }
        let mut property = match self.0.properties.borrow().get(key) {
            Some(property) => property,
            None => return Ok(None),
        };
        if let Some(cold) = self.0.cold_if_present()
            && let Some(bindings) = cold.module_namespace_bindings.borrow().as_ref()
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
            .cold_if_present()?
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
        if removed.is_some() {
            self.bump_property_revision();
        }
        if removed.is_some() && is_array_index_key(key) {
            self.0
                .index_property_count
                .set(self.0.index_property_count.get().saturating_sub(1));
        }
        true
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

    pub(crate) fn own_property_keys(&self) -> Vec<String> {
        self.ordered_property_names(|property| property.enumerable)
    }

    pub(crate) fn own_property_names(&self) -> Vec<String> {
        self.ordered_property_names(|_| true)
    }

    fn ordered_property_names(&self, include: impl Fn(&Property) -> bool) -> Vec<String> {
        let properties = self.0.properties.borrow();
        if let PropertyStorage::Small { entries } = &*properties {
            if self.0.index_property_count.get() == 0 {
                return entries
                    .iter()
                    .filter_map(|(key, property)| {
                        if is_internal_property_key(key) {
                            return None;
                        }
                        include(property).then(|| key.to_string())
                    })
                    .collect();
            }

            let mut indices = Vec::new();
            let mut strings = Vec::new();
            for (key, property) in entries {
                if is_internal_property_key(key) || !include(property) {
                    continue;
                }
                if let Some(index) = array_index_property_key(key) {
                    indices.push((index, key.to_string()));
                } else {
                    strings.push(key.to_string());
                }
            }
            indices.sort_by_key(|(index, _)| *index);
            return indices
                .into_iter()
                .map(|(_, key)| key)
                .chain(strings)
                .collect();
        }

        let order = properties
            .order()
            .expect("non-small property storage has a separate order");
        if self.0.index_property_count.get() == 0 {
            return order
                .iter()
                .filter_map(|key| {
                    if is_internal_property_key(key) {
                        return None;
                    }
                    let property = properties.get(key.as_ref())?;
                    include(&property).then(|| key.to_string())
                })
                .collect();
        }

        let mut indices = Vec::new();
        let mut strings = Vec::new();

        for key in order.iter() {
            if is_internal_property_key(key) {
                continue;
            }
            let Some(property) = properties.get(key.as_ref()) else {
                continue;
            };
            if !include(&property) {
                continue;
            }
            if let Some(index) = array_index_property_key(key) {
                indices.push((index, key.to_string()));
            } else {
                strings.push(key.to_string());
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
        self.0.cold_if_present().map_or_else(Vec::new, |cold| {
            cold.symbol_properties
                .borrow()
                .iter()
                .map(|(symbol, _)| symbol.clone())
                .collect()
        })
    }

    pub(crate) fn set_prototype(&self, prototype: Option<ObjectRef>) -> Result<(), ()> {
        self.set_prototype_slot(prototype.map(Prototype::Object))
    }

    pub(crate) fn to_string_tag(&self) -> Option<String> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.to_string_tag.borrow().clone())
            .or_else(|| {
                self.0
                    .prototype
                    .borrow()
                    .as_ref()
                    .and_then(Prototype::to_string_tag)
            })
    }

    pub(crate) fn set_to_string_tag(&self, tag: &str) {
        *self.0.cold().to_string_tag.borrow_mut() = Some(tag.to_owned());
    }
}

impl ObjectWeakRef {
    pub(crate) fn ptr_eq(&self, object: &ObjectRef) -> bool {
        self.0.as_ptr() == Rc::as_ptr(&object.0)
    }

    pub(crate) fn upgrade(&self) -> Option<ObjectRef> {
        self.0.upgrade().map(ObjectRef)
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

    use super::{ObjectData, ObjectLiteralShape, ObjectRef, OwnDataPropertyWrite, PropertyStorage};
    use crate::{Property, Value};

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

    #[test]
    fn ordinary_object_keeps_cold_state_unallocated() {
        let object = ObjectRef::new(HashMap::from([
            ("a".to_owned(), Value::Number(1.0)),
            ("b".to_owned(), Value::Number(2.0)),
        ]));

        assert_eq!(object.get("a"), Some(Value::Number(1.0)));
        assert!(object.own_property_symbols().is_empty());
        assert!(object.to_string_tag().is_none());
        assert!(object.0.cold.get().is_none());
        // Boxing the cold `Dynamic` property-storage payload (HashMap + Vec)
        // keeps this at 104 bytes instead of the 136 it cost when that
        // payload sized the whole `PropertyStorage` enum for every object.
        assert!(mem::size_of::<ObjectData>() <= 112);
    }

    #[test]
    fn ordinary_small_object_promotes_only_after_eight_properties() {
        let object = ObjectRef::new(HashMap::new());

        for index in 0..8 {
            object.set(format!("field{index}"), Value::Number(index as f64));
        }
        assert!(matches!(
            &*object.0.properties.borrow(),
            PropertyStorage::Small { entries } if entries.len() == 8
        ));

        object.set("field8".to_owned(), Value::Number(8.0));
        assert!(matches!(
            &*object.0.properties.borrow(),
            PropertyStorage::Dynamic(dynamic)
                if dynamic.properties.len() == 9 && dynamic.order.len() == 9
        ));
        assert_eq!(
            object.own_property_names(),
            (0..9)
                .map(|index| format!("field{index}"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn ordinary_object_retains_shared_static_property_key() {
        let object = ObjectRef::new(HashMap::new());
        let key: Rc<str> = Rc::from("field");

        object.set_shared_key(Rc::clone(&key), Value::Number(1.0));

        assert!(matches!(
            &*object.0.properties.borrow(),
            PropertyStorage::Small { entries }
                if entries.len() == 1 && Rc::ptr_eq(&entries[0].0, &key)
        ));
        assert_eq!(object.get("field"), Some(Value::Number(1.0)));
    }

    #[test]
    fn small_object_removal_preserves_property_order() {
        let object = ObjectRef::new(HashMap::new());
        object.set("first".to_owned(), Value::Number(1.0));
        object.set("second".to_owned(), Value::Number(2.0));
        object.set("third".to_owned(), Value::Number(3.0));

        assert!(object.delete_own_property("second"));
        object.set("second".to_owned(), Value::Number(4.0));

        assert_eq!(object.own_property_names(), ["first", "third", "second"]);
        assert!(matches!(
            &*object.0.properties.borrow(),
            PropertyStorage::Small { entries } if entries.len() == 3
        ));
    }

    #[test]
    fn small_object_enumerates_indices_before_strings() {
        let object = ObjectRef::new(HashMap::new());
        object.set("10".to_owned(), Value::Number(10.0));
        object.set("label".to_owned(), Value::Number(0.0));
        object.set("2".to_owned(), Value::Number(2.0));

        assert_eq!(object.own_property_names(), ["2", "10", "label"]);
        assert!(matches!(
            &*object.0.properties.borrow(),
            PropertyStorage::Small { entries } if entries.len() == 3
        ));
    }

    #[test]
    fn existing_own_data_write_updates_or_rejects_without_slow_path() {
        let object = ObjectRef::new(HashMap::from([("writable".to_owned(), Value::Number(1.0))]));
        object.define_property(
            "readonly".to_owned(),
            Property::data(Value::Number(2.0), true, false, true),
        );

        assert!(matches!(
            object.write_existing_own_data_property("writable", &Value::Number(3.0)),
            OwnDataPropertyWrite::Written
        ));
        assert_eq!(object.get("writable"), Some(Value::Number(3.0)));
        assert!(matches!(
            object.write_existing_own_data_property("readonly", &Value::Number(4.0)),
            OwnDataPropertyWrite::ReadOnly
        ));
        assert_eq!(object.get("readonly"), Some(Value::Number(2.0)));
        assert!(matches!(
            object.write_existing_own_data_property("missing", &Value::Number(5.0)),
            OwnDataPropertyWrite::NeedsSlowPath
        ));
    }

    #[test]
    fn literal_pair_keeps_inline_values_until_descriptor_mutation() {
        let shape = ObjectLiteralShape::new(vec![Rc::from("a"), Rc::from("b")]);
        let object =
            ObjectRef::with_literal_pair(shape, [Value::Number(1.0), Value::Number(2.0)], None);

        assert!(matches!(
            &*object.0.properties.borrow(),
            PropertyStorage::ShapedPair { .. }
        ));
        assert!(matches!(
            object.write_existing_own_data_property("a", &Value::Number(3.0)),
            OwnDataPropertyWrite::Written
        ));
        assert_eq!(object.get("a"), Some(Value::Number(3.0)));
        assert!(matches!(
            &*object.0.properties.borrow(),
            PropertyStorage::ShapedPair { .. }
        ));

        object.define_property(
            "a".to_owned(),
            Property::data(Value::Number(4.0), false, false, true),
        );
        assert!(matches!(
            &*object.0.properties.borrow(),
            PropertyStorage::Dynamic(_)
        ));
        let descriptor = object.own_property("a").expect("defined property");
        assert_eq!(descriptor.value, Value::Number(4.0));
        assert!(!descriptor.enumerable);
        assert!(!descriptor.writable);
    }

    #[test]
    fn module_namespace_own_data_write_stays_on_slow_path() {
        let object = ObjectRef::new(HashMap::from([("exported".to_owned(), Value::Number(1.0))]));
        object.mark_module_namespace_exotic();

        assert!(matches!(
            object.write_existing_own_data_property("exported", &Value::Number(2.0)),
            OwnDataPropertyWrite::NeedsSlowPath
        ));
        assert_eq!(object.get("exported"), Some(Value::Number(1.0)));
    }
}
