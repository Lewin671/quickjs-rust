//! Ordinary object storage: `ObjectRef`, its property tables, and the
//! [[Prototype]] slot.
//!
//! String-keyed properties live in one of four `PropertyStorage` layouts
//! (small vector, hashed, shared literal shape, two-value shape), and an
//! object moves between them as it is mutated. Code outside this module must
//! not depend on which layout an object is in.
//!
//! Two counters back the VM's caches. `property_revision` changes whenever
//! an own string property's descriptor or value changes; `layout_revision`
//! changes only when slot positions do (insert, delete, descriptor
//! replacement). Every mutation path must bump the right one, or a
//! per-site cache serves a stale read.

use std::{
    cell::{Cell, OnceCell, RefCell},
    collections::HashMap,
    fmt,
    rc::{Rc, Weak},
};

use crate::private::{PrivateEnvironment, PrivateStorage};
use crate::{ArrayRef, Function, RuntimeError, function::DynamicBindings, proxy::ProxyRef};

use super::{Property, Value};

mod array_buffer_methods;
mod class_fields;
mod key_order;
mod ordered_data_builder;
mod slot_reads;
mod storage;
mod teardown;
mod typed_array_methods;

use crate::property::atom::{AtomDomain, WeakAtomDomain};
use crate::property::layout::{Attributes, BulkEdit, Encoding, Layout, layout_for_keys};
use storage::{Payload, PropertyStorage};

pub(crate) use ordered_data_builder::OrderedDataPropertyBuilder;
pub(crate) use slot_reads::{LiteralLayout, SharedSlot};

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

    /// The atom domain a store created with this prototype uses.
    pub(crate) fn atom_domain(&self) -> Option<AtomDomain> {
        match self {
            Self::Object(object) => object.atom_domain(),
            Self::Array(array) => array.0.default_prototype.as_ref()?.atom_domain(),
            Self::Function(function) => function.atom_domain(),
            Self::Proxy(_) => None,
        }
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
    /// Invalidates cached own-property *slots* only when the property table's
    /// layout changes: an insert, a delete, or a descriptor replacement.
    /// Overwriting an existing data property's value leaves every slot index
    /// valid, so a slot-keyed read cache survives ordinary field assignment --
    /// the case `property_revision` deliberately cannot express.
    layout_revision: Cell<u64>,
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
    /// A String wrapper whose index properties are not defined yet: they are
    /// defined, from its [[StringData]], before anything reads its property
    /// table (`ObjectRef::properties`). Boxing a string as a sloppy
    /// function's `this` used to define one property per code unit on every
    /// call, which is quadratic in a method called on a long string.
    string_indices_pending: Cell<bool>,
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
    /// Opaque byte storage for ArrayBuffer objects. The buffer brand and state
    /// are internal slots below; typed-array element access never round-trips
    /// through observable string-keyed properties.
    internal_bytes: RefCell<Option<Vec<u8>>>,
    /// Stable ArrayBuffer/SharedArrayBuffer brand. NUL-prefixed ordinary
    /// properties are observable and mutable JavaScript state, so they cannot
    /// be authoritative for buffer validation or optimized backing leases.
    array_buffer_kind: Cell<ArrayBufferKind>,
    /// Ordinary ArrayBuffer detach state. SharedArrayBuffer never detaches.
    array_buffer_detached: Cell<bool>,
    /// Resizable/growable maximum. `None` denotes fixed-length storage.
    array_buffer_max_byte_length: Cell<Option<usize>>,
    /// True for immutable ArrayBuffer backing stores. This is a real internal
    /// slot rather than a NUL-prefixed ordinary property, so JavaScript cannot
    /// delete or overwrite the state that guards indexed writes and leases.
    array_buffer_immutable: Cell<bool>,
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

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ArrayBufferKind {
    #[default]
    None,
    Ordinary,
    Shared,
}

impl ObjectData {
    fn cold(&self) -> &ObjectColdData {
        self.cold.get_or_init(Box::default).as_ref()
    }

    fn cold_if_present(&self) -> Option<&ObjectColdData> {
        self.cold.get().map(Box::as_ref)
    }
}

/// Shared key list of an object literal site whose property names are
/// statically known. The bytecode owns one; each evaluated object clones one
/// layout pointer and allocates only its values. The site keeps its layout
/// for the last domain that evaluated it, built through that domain's edges.
pub(crate) struct ObjectLiteralShape {
    keys: Rc<[Rc<str>]>,
    input_slots: Rc<[usize]>,
    index_property_count: usize,
    site: RefCell<Option<(WeakAtomDomain, Layout)>>,
}

impl fmt::Debug for ObjectLiteralShape {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ObjectLiteralShape")
            .field("keys", &self.keys)
            .field("input_slots", &self.input_slots)
            .finish_non_exhaustive()
    }
}

impl ObjectLiteralShape {
    pub(crate) fn new(input_keys: Vec<Rc<str>>) -> Rc<Self> {
        let mut keys: Vec<Rc<str>> = Vec::with_capacity(input_keys.len());
        let mut input_slots = Vec::with_capacity(input_keys.len());
        for key in input_keys {
            let slot = match keys.iter().position(|existing| *existing == key) {
                Some(slot) => slot,
                None => {
                    keys.push(key);
                    keys.len() - 1
                }
            };
            input_slots.push(slot);
        }
        let index_property_count = keys.iter().filter(|key| is_array_index_key(key)).count();
        Self {
            keys: keys.into(),
            input_slots: input_slots.into(),
            index_property_count,
            site: RefCell::new(None),
        }
        .into()
    }

    pub(crate) fn input_len(&self) -> usize {
        self.input_slots.len()
    }

    pub(crate) fn unique_len(&self) -> usize {
        self.keys.len()
    }

    /// The payload encoding of this site's objects.
    fn encoding(&self) -> Encoding {
        if self.keys.len() == 2 {
            Encoding::Pair
        } else {
            Encoding::Shaped
        }
    }

    /// This site's published layout in `domain`.
    #[inline]
    fn layout_in(&self, domain: &AtomDomain) -> Layout {
        if let Some((site_domain, layout)) = &*self.site.borrow()
            && site_domain.is(domain)
        {
            return Rc::clone(layout);
        }
        let layout = layout_for_keys(
            domain,
            self.encoding(),
            self.keys
                .iter()
                .map(|key| (key.as_ref(), Attributes::DEFAULT)),
        );
        *self.site.borrow_mut() = Some((domain.downgrade(), Rc::clone(&layout)));
        layout
    }

    /// The layout for an object with `prototype`: in the prototype's domain,
    /// or, for a null-prototype literal, in the domain the site last used.
    #[inline]
    fn layout_for(&self, prototype: Option<&ObjectRef>) -> Layout {
        if prototype.is_none()
            && let Some((_, layout)) = &*self.site.borrow()
        {
            return Rc::clone(layout);
        }
        self.layout_in(&literal_domain(prototype))
    }

    /// Returns the input value that supplies `key` after duplicate literal
    /// definitions have been applied from left to right: the last input
    /// that maps to `key`'s slot.
    pub(crate) fn final_input_index(&self, key: &str) -> Option<usize> {
        let slot = self
            .keys
            .iter()
            .position(|existing| existing.as_ref() == key)?;
        self.input_slots
            .iter()
            .rposition(|input_slot| *input_slot == slot)
    }
}

fn own_data_read(storage: &PropertyStorage, key: &str) -> OwnDataPropertyRead {
    let Some(slot) = storage.lookup(key) else {
        return OwnDataPropertyRead::Missing;
    };
    match storage.slot(slot) {
        None => OwnDataPropertyRead::Missing,
        Some(slot) if slot.is_accessor() => OwnDataPropertyRead::NeedsSlowPath,
        Some(slot) => OwnDataPropertyRead::Data(slot.value().clone()),
    }
}

/// Writes the existing writable data property at `slot` when `guard`
/// accepts its current value; a rejected one is `NeedsSlowPath`, unchanged.
fn write_slot(
    storage: &mut PropertyStorage,
    slot: usize,
    value: &Value,
    guard: impl FnOnce(&Value) -> bool,
) -> OwnDataPropertyWrite {
    let Some(attributes) = storage.field(slot).map(|field| field.attributes) else {
        return OwnDataPropertyWrite::NeedsSlowPath;
    };
    if attributes.is_accessor() {
        return OwnDataPropertyWrite::NeedsSlowPath;
    }
    let Some(current) = storage.value_mut(slot) else {
        return OwnDataPropertyWrite::NeedsSlowPath;
    };
    if !guard(current) {
        return OwnDataPropertyWrite::NeedsSlowPath;
    }
    if !attributes.is_writable() {
        return OwnDataPropertyWrite::ReadOnly;
    }
    *current = value.clone();
    OwnDataPropertyWrite::Written
}

fn write_existing_data(
    storage: &mut PropertyStorage,
    key: &str,
    value: &Value,
    guard: impl FnOnce(&Value) -> bool,
) -> OwnDataPropertyWrite {
    match storage.lookup(key) {
        Some(slot) => write_slot(storage, slot, value, guard),
        None => OwnDataPropertyWrite::NeedsSlowPath,
    }
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
            .field("properties", &self.properties().borrow().len())
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
    /// The own string-keyed property table, after defining a String
    /// wrapper's pending index properties.
    #[inline(always)]
    fn properties(&self) -> &RefCell<PropertyStorage> {
        if self.0.string_indices_pending.get() {
            self.define_pending_string_indices();
        }
        &self.0.properties
    }

    /// The property table for an operation on `key` alone: a String
    /// wrapper's pending index properties matter only to an index key.
    #[inline(always)]
    fn properties_for(&self, key: &str) -> &RefCell<PropertyStorage> {
        if self.0.string_indices_pending.get() && is_array_index_key(key) {
            self.define_pending_string_indices();
        }
        &self.0.properties
    }

    #[cold]
    #[inline(never)]
    fn define_pending_string_indices(&self) {
        self.0.string_indices_pending.set(false);
        let Some(text) = self.string_data() else {
            return;
        };
        for (index, code_unit) in crate::string::code_units(&text).enumerate() {
            self.define_shared_property(
                Rc::from(index.to_string()),
                Property::data(
                    Value::String(crate::string::js_string_from_code_unit(code_unit)),
                    true,
                    false,
                    false,
                ),
            );
        }
    }

    /// Defers a String wrapper's index properties -- one per code unit of
    /// its [[StringData]] -- until its property table is next read.
    pub(crate) fn defer_string_indices(&self) {
        self.0.string_indices_pending.set(true);
    }

    /// A String wrapper's [[StringData]], read without defining its index
    /// properties.
    pub(crate) fn string_data(&self) -> Option<crate::JsString> {
        match self
            .0
            .properties
            .borrow()
            .value(crate::string::STRING_DATA_PROPERTY)
        {
            Some(Value::String(text)) => Some(text),
            _ => None,
        }
    }

    pub(crate) fn new(properties: HashMap<String, Value>) -> Self {
        Self::with_prototype_slot(properties, None)
    }

    /// An object with no prototype whose store uses `domain`: a
    /// null-prototype object takes its domain from the realm creating it.
    pub(crate) fn new_in(domain: &AtomDomain, properties: HashMap<String, Value>) -> Self {
        Self::with_prototype_slot_in(domain, properties, None)
    }

    /// [`Self::new_like`] of an object value; any other value has no domain.
    pub(crate) fn new_like_value(like: &Value, properties: HashMap<String, Value>) -> Self {
        match like {
            Value::Object(object) => Self::new_like(object, properties),
            _ => Self::new(properties),
        }
    }

    /// A null-prototype object in the domain of `like`, for an internal
    /// record created alongside an existing object.
    pub(crate) fn new_like(like: &ObjectRef, properties: HashMap<String, Value>) -> Self {
        match like.atom_domain() {
            Some(domain) => Self::new_in(&domain, properties),
            None => Self::new(properties),
        }
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
        if properties.is_empty()
            && let Some(Prototype::Object(object)) = &prototype
            && let Some(storage) = object
                .0
                .properties
                .try_borrow()
                .ok()
                .map(|storage| PropertyStorage::empty(storage.domain(), 0))
        {
            return Self::from_storage(storage, 0, prototype);
        }
        let domain = store_domain(prototype.as_ref());
        Self::with_prototype_slot_in(&domain, properties, prototype)
    }

    /// An empty ordinary object whose store takes its domain from
    /// `prototype`.
    #[cfg(test)]
    pub(crate) fn empty_with_prototype(prototype: ObjectRef) -> Self {
        let storage = match prototype.0.properties.try_borrow() {
            Ok(properties) => PropertyStorage::empty(properties.domain(), 0),
            Err(_) => PropertyStorage::empty(&fallback_domain(), 0),
        };
        Self::from_storage(storage, 0, Some(Prototype::Object(prototype)))
    }

    pub(crate) fn with_prototype_slot_in(
        domain: &AtomDomain,
        properties: HashMap<String, Value>,
        prototype: Option<Prototype>,
    ) -> Self {
        if properties.is_empty() {
            return Self::from_storage(PropertyStorage::empty(domain, 0), 0, prototype);
        }
        let mut entries: Vec<(Rc<str>, Property)> = properties
            .into_iter()
            .map(|(key, value)| (Rc::from(key), Property::enumerable(value)))
            .collect();
        entries.sort_by(|(left, _), (right, _)| left.cmp(right));
        let index_property_count = entries
            .iter()
            .filter(|(key, _)| is_array_index_key(key))
            .count();
        Self::from_storage(
            PropertyStorage::from_ordered(domain, entries),
            index_property_count,
            prototype,
        )
    }

    fn from_storage(
        storage: PropertyStorage,
        index_property_count: usize,
        prototype: Option<Prototype>,
    ) -> Self {
        Self(Rc::new(ObjectData {
            properties: RefCell::new(storage),
            property_revision: Cell::new(0),
            layout_revision: Cell::new(0),
            index_property_count: Cell::new(index_property_count),
            extensible: Cell::new(true),
            prototype: RefCell::new(prototype),
            raw_json: Cell::new(false),
            array_prototype_exotic: Cell::new(false),
            typed_array_exotic: Cell::new(false),
            symbol_brand: Cell::new(SymbolBrand::None),
            immutable_prototype_exotic: Cell::new(false),
            module_namespace_exotic: Cell::new(false),
            string_indices_pending: Cell::new(false),
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
        let layout = shape.layout_for(prototype.as_ref());
        let payload = if shape.encoding() == Encoding::Pair {
            let mut pair = [Value::Undefined, Value::Undefined];
            for (slot, value) in shape.input_slots.iter().copied().zip(values) {
                pair[slot] = value;
            }
            Payload::Pair(pair)
        } else if shape.input_slots.len() == shape.keys.len() {
            Payload::Slots(values.into_iter().map(Property::enumerable).collect())
        } else {
            let mut properties = vec![None; shape.keys.len()];
            for (slot, value) in shape.input_slots.iter().copied().zip(values) {
                properties[slot] = Some(Property::enumerable(value));
            }
            Payload::Slots(
                properties
                    .into_iter()
                    .map(|property| property.expect("literal shape slot must have a value"))
                    .collect(),
            )
        };
        Self::from_storage(
            PropertyStorage::literal(layout, payload),
            shape.index_property_count,
            prototype.map(Prototype::Object),
        )
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
        let layout = shape.layout_for(prototype.as_ref());
        Self::from_storage(
            PropertyStorage::literal(layout, Payload::Pair(values)),
            shape.index_property_count,
            prototype.map(Prototype::Object),
        )
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

    /// The atom domain of this object's store.
    pub(crate) fn atom_domain(&self) -> Option<AtomDomain> {
        self.0
            .properties
            .try_borrow()
            .ok()
            .map(|properties| properties.domain().clone())
    }

    /// Identity of the shared storage, for hashing this reference as a key.
    pub(crate) fn address(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    pub(crate) fn downgrade(&self) -> ObjectWeakRef {
        ObjectWeakRef(Rc::downgrade(&self.0))
    }

    pub(crate) fn property_revision(&self) -> u64 {
        self.0.property_revision.get()
    }

    pub(crate) fn layout_revision(&self) -> u64 {
        self.0.layout_revision.get()
    }

    /// Records a change that keeps every own-property slot index valid: the
    /// value of an existing data property was overwritten in place.
    fn bump_value_revision(&self) {
        self.0
            .property_revision
            .set(self.0.property_revision.get().wrapping_add(1));
    }

    fn bump_property_revision(&self) {
        self.0
            .layout_revision
            .set(self.0.layout_revision.get().wrapping_add(1));
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

    /// The enumerable flag of an own string-keyed property of an ordinary
    /// object, or `None` when the property is absent. Exotic own-property
    /// behaviour (a module namespace, a typed array) is not answered here;
    /// callers check for it first.
    pub(crate) fn own_property_enumerable(&self, key: &str) -> Option<bool> {
        self.properties_for(key).borrow().enumerable(key)
    }

    pub(crate) fn set_module_namespace_bindings(&self, bindings: ModuleNamespaceBindings) {
        *self.0.cold().module_namespace_bindings.borrow_mut() = Some(bindings);
    }

    pub(crate) fn get(&self, key: &str) -> Option<Value> {
        self.properties_for(key).borrow().value(key).or_else(|| {
            self.0
                .prototype
                .borrow()
                .as_ref()
                .and_then(|proto| proto.get(key))
        })
    }

    pub(crate) fn property(&self, key: &str) -> Option<Property> {
        self.properties_for(key).borrow().get(key).or_else(|| {
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
        let _ = self.properties();
        self.0.index_property_count.get() > 0
    }

    /// Whether creating a new indexed property on an object with this
    /// prototype could be intercepted anywhere up the chain.
    ///
    /// `CreateDataProperty` on a receiver that has no own property at the key
    /// still walks the prototype chain looking for a setter or a non-writable
    /// data property, so a fast path that writes an element directly may only
    /// run when the whole chain is ordinary and indexless. An exotic link — a
    /// proxy, a callable, or a live array prototype — declines.
    pub(crate) fn chain_intercepts_index_write(&self) -> bool {
        let mut current = self.clone();
        for _ in 0..16 {
            if current.has_own_index_property() {
                return true;
            }
            match current.prototype_slot() {
                None => return false,
                Some(Prototype::Object(next)) => current = next,
                Some(_) => return true,
            }
        }
        true
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

    /// This object's [[Prototype]] when it is an ordinary object.
    ///
    /// Separate from `prototype_slot` because a property-read cache only ever
    /// wants the ordinary case and should not clone a slot it will discard.
    pub(crate) fn ordinary_prototype(&self) -> Option<ObjectRef> {
        match &*self.0.prototype.borrow() {
            Some(Prototype::Object(prototype)) => Some(prototype.clone()),
            _ => None,
        }
    }

    /// Whether this object's [[Prototype]] is exactly `candidate`.
    ///
    /// One pointer comparison behind a `RefCell` borrow, with no clone: this
    /// is the per-read guard of the prototype property cache, so it runs on
    /// every method call.
    /// Whether this object's [[Prototype]] is the object `candidate` weakly
    /// refers to, by identity and without upgrading the weak reference: an
    /// allocation a live `Weak` names cannot be reused, so a pointer match
    /// means the prototype slot itself holds that object alive.
    pub(crate) fn prototype_is_weak(&self, candidate: &ObjectWeakRef) -> bool {
        match &*self.0.prototype.borrow() {
            Some(Prototype::Object(prototype)) => candidate.ptr_eq(prototype),
            _ => false,
        }
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
        let mut properties = self.properties_for(&key).borrow_mut();
        if let Some(slot) = properties.probe_for_edit(&key) {
            if properties
                .field(slot)
                .is_some_and(|field| field.attributes.is_writable())
            {
                if let Some(current) = properties.value_mut(slot) {
                    *current = value;
                }
                self.bump_value_revision();
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
            && let Some(length_slot) = properties.lookup("length")
            && properties
                .field(length_slot)
                .is_some_and(|field| field.attributes.is_writable())
            && let Some(length) = properties.value_mut(length_slot)
        {
            let current_length = match length {
                Value::Number(length) if length.is_finite() && *length >= 0.0 => *length as u32,
                _ => 0,
            };
            if index >= current_length {
                *length = Value::Number((index + 1) as f64);
            }
        }
        if is_array_index_key(&key) {
            self.0
                .index_property_count
                .set(self.0.index_property_count.get() + 1);
        }
        properties.append(&key, Property::enumerable(value), true);
        self.bump_property_revision();
        drop(properties);
        if establishes_realm_identity {
            self.capture_realm_intrinsic_identity();
        }
    }

    /// Creates an own data property whose absence on this object the caller has
    /// already established.
    ///
    /// Initializing a field in a constructor -- `this.a = a` -- used to scan
    /// this object's storage three times: once through
    /// `write_existing_own_data_property`, once through
    /// `own_data_property_read`, and once more inside `set_shared_key` before
    /// the push. On `new O(a, b)`, whose two field initializations are the
    /// whole constructor body, those scans and the re-checks around them were
    /// 232 of the profile's samples.
    ///
    /// Returns `false` for a receiver whose creation has side effects this
    /// shortcut does not perform, leaving it to `set_shared_key`.
    pub(crate) fn create_absent_own_data_property(&self, key: Rc<str>, value: Value) -> bool {
        if self.0.array_prototype_exotic.get() || !self.0.extensible.get() {
            return false;
        }
        let establishes_realm_identity = key.as_ref() == "globalThis"
            && matches!(&value, Value::Object(global_this) if self.ptr_eq(global_this));
        if is_array_index_key(&key) {
            self.0
                .index_property_count
                .set(self.0.index_property_count.get() + 1);
        }
        {
            let mut properties = self.properties_for(&key).borrow_mut();
            debug_assert!(!properties.contains_key(&key));
            properties.append(&key, Property::enumerable(value), true);
            self.bump_property_revision();
        }
        if establishes_realm_identity {
            self.capture_realm_intrinsic_identity();
        }
        true
    }

    pub(crate) fn define_property(&self, key: String, property: Property) {
        let establishes_realm_identity = key == "globalThis"
            && !property.is_accessor()
            && matches!(&property.value, Value::Object(global_this) if self.ptr_eq(global_this));
        let mut properties = self.properties_for(&key).borrow_mut();
        if properties.insert(&key, property).is_none() && is_array_index_key(&key) {
            self.0
                .index_property_count
                .set(self.0.index_property_count.get() + 1);
        }
        self.bump_property_revision();
        drop(properties);
        if establishes_realm_identity {
            self.capture_realm_intrinsic_identity();
        }
    }

    /// `define_property` for a key the caller already shares, such as a
    /// realm-cached String object key; it never copies the key. A separate
    /// entry rather than a change to `define_property`, whose callers are the
    /// ordinary object core.
    pub(crate) fn define_shared_property(&self, key: Rc<str>, property: Property) {
        debug_assert_ne!(
            &*key, "globalThis",
            "the realm identity capture needs define_property"
        );
        let mut properties = self.properties_for(&key).borrow_mut();
        if properties.insert(&key, property).is_none() && is_array_index_key(&key) {
            self.0
                .index_property_count
                .set(self.0.index_property_count.get() + 1);
        }
        self.bump_property_revision();
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
        let mut properties = self.properties_for(key).borrow_mut();
        if let Some(slot) = properties.probe_for_edit(key) {
            if let Some(current) = properties.value_mut(slot) {
                *current = value;
            }
            self.bump_property_revision();
            return;
        }
        properties.append(key, Property::non_enumerable(value), false);
        self.bump_property_revision();
    }

    /// Whether the object `prototype` appears as a function prototype anywhere
    /// in this object's chain. Used by `isPrototypeOf`/`instanceof` to walk past
    /// a function sitting mid-chain.
    pub(crate) fn has_own_property(&self, key: &str) -> bool {
        self.properties_for(key).borrow().contains_key(key)
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
        self.properties().borrow_mut().bulk(BulkEdit::Seal);
        if let Some(cold) = self.0.cold_if_present() {
            for (_, property) in cold.symbol_properties.borrow_mut().iter_mut() {
                property.make_non_configurable();
            }
        }
    }

    pub(crate) fn append_string_property(&self, key: &str, suffix: &str) -> Option<Value> {
        let mut properties = self.properties_for(key).borrow_mut();
        let slot = properties.lookup(key)?;
        let attributes = properties.field(slot)?.attributes;
        if !attributes.is_writable() || attributes.is_accessor() {
            return None;
        }
        let Value::String(string) = properties.value_mut(slot)? else {
            return None;
        };
        crate::string::push_js_str(string.make_mut(), suffix);
        self.bump_value_revision();
        Some(Value::String(string.clone()))
    }

    pub(crate) fn is_sealed(&self) -> bool {
        !self.0.extensible.get()
            && self
                .properties()
                .borrow()
                .all_attributes(|attributes| !attributes.is_configurable())
            && self.0.cold_if_present().is_none_or(|cold| {
                cold.symbol_properties
                    .borrow()
                    .iter()
                    .all(|(_, property)| !property.configurable)
            })
    }

    pub(crate) fn freeze(&self) {
        self.prevent_extensions();
        self.properties().borrow_mut().bulk(BulkEdit::Freeze);
        if let Some(cold) = self.0.cold_if_present() {
            for (_, property) in cold.symbol_properties.borrow_mut().iter_mut() {
                property.freeze_data();
            }
        }
    }

    pub(crate) fn is_frozen(&self) -> bool {
        !self.0.extensible.get()
            && self.properties().borrow().all_attributes(|attributes| {
                !attributes.is_configurable() && !attributes.is_writable()
            })
            && self.0.cold_if_present().is_none_or(|cold| {
                cold.symbol_properties
                    .borrow()
                    .iter()
                    .all(|(_, property)| !property.configurable && !property.writable)
            })
    }

    pub(crate) fn own_property(&self, key: &str) -> Option<Property> {
        let mut property = self.properties_for(key).borrow().get(key)?;
        if self.0.module_namespace_exotic.get()
            && let Some(cold) = self.0.cold_if_present()
            && let Some(bindings) = cold.module_namespace_bindings.borrow().as_ref()
            && let Some(value) = bindings.value_for_export(key)
        {
            property.value = value.clone();
        }
        Some(property)
    }

    /// Updates an existing ordinary own data property without cloning its
    /// descriptor or walking the prototype chain. Accessors, missing keys, and
    /// module namespace exports retain their observable slow-path behavior.
    pub(crate) fn write_existing_own_data_property(
        &self,
        key: &str,
        value: &Value,
    ) -> OwnDataPropertyWrite {
        self.write_existing_own_data_property_if(key, value, |_| true)
    }

    /// [`Self::write_existing_own_data_property`] of a property whose current
    /// value `guard` accepts, in the same lookup; a rejected one is
    /// `NeedsSlowPath`, unchanged.
    pub(crate) fn write_existing_own_data_property_if(
        &self,
        key: &str,
        value: &Value,
        guard: impl FnOnce(&Value) -> bool,
    ) -> OwnDataPropertyWrite {
        if self.0.module_namespace_exotic.get() {
            return OwnDataPropertyWrite::NeedsSlowPath;
        }
        let establishes_realm_identity = key == "globalThis"
            && matches!(value, Value::Object(global_this) if self.ptr_eq(global_this));
        let result = write_existing_data(
            &mut self.properties_for(key).borrow_mut(),
            key,
            value,
            guard,
        );
        if matches!(result, OwnDataPropertyWrite::Written) {
            self.bump_value_revision();
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
        let mut property = match self.properties().borrow().get(key) {
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
        let mut properties = self.properties_for(key).borrow_mut();
        let Some(slot) = properties.probe_for_edit(key) else {
            return true;
        };
        if properties
            .field(slot)
            .is_some_and(|field| !field.attributes.is_configurable())
        {
            return false;
        }
        let removed = Some(properties.remove_slot(slot));
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

/// The domain of a new store with `prototype`: the prototype's own, or a
/// fresh one when the prototype names none.
fn store_domain(prototype: Option<&Prototype>) -> AtomDomain {
    prototype
        .and_then(Prototype::atom_domain)
        .unwrap_or_else(fallback_domain)
}

#[inline]
fn literal_domain(prototype: Option<&ObjectRef>) -> AtomDomain {
    match prototype.map(|prototype| prototype.0.properties.try_borrow()) {
        Some(Ok(properties)) => properties.domain().clone(),
        _ => fallback_domain(),
    }
}

/// A domain for a store created with no realm or prototype to take one from.
/// Counted with every other domain, so the obligation tests show which
/// creation paths still reach it.
#[cold]
fn fallback_domain() -> AtomDomain {
    AtomDomain::new()
}

fn is_internal_property_key(key: &str) -> bool {
    key.starts_with('\0')
}

/// Parses an array index in its canonical decimal spelling.
///
/// This is a predicate on the element path -- every string-keyed array access
/// asks it, and `has_index`, the element read, and the element write each ask
/// it again -- and it used to answer by formatting the parsed number back into
/// a fresh `String` to compare against the key. That is a heap allocation per
/// array access.
///
/// The canonical form is exactly what the digits already say: no sign, no
/// leading zero unless the key is `"0"`, and a value below `u32::MAX` (the
/// array length itself is not an index).
fn array_index_property_key(key: &str) -> Option<u32> {
    let bytes = key.as_bytes();
    // `u32::MAX` has ten digits, so anything longer is out of range; a leading
    // zero is a distinct string property rather than an index.
    if bytes.is_empty() || bytes.len() > 10 || (bytes[0] == b'0' && bytes.len() > 1) {
        return None;
    }
    let mut index: u32 = 0;
    for byte in bytes {
        let digit = byte.wrapping_sub(b'0');
        if digit > 9 {
            return None;
        }
        // Ten digits still overflow above `4294967295`, so the accumulation is
        // checked rather than merely bounded by the length.
        index = index.checked_mul(10)?.checked_add(u32::from(digit))?;
    }
    (index < u32::MAX).then_some(index)
}

fn is_array_index_key(key: &str) -> bool {
    array_index_property_key(key).is_some()
}

#[cfg(test)]
mod tests;
