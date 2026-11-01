//! Ordinary object identity, internal slots and live prototype edges.
//!
//! NamedStore is the sole string/symbol authority. Layouts own key leases;
//! per-object slots contain values, with accessors cold. Layouts include
//! prototype identity; prototype-mode cells guard inherited structural facts.
//! Pure String virtual-index hooks do not materialize or intern on reads.

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
mod cold;
use cold::ColdPayload;
mod construction;
mod data_write;
mod key_order;
mod literal;
mod named_properties;
mod ordered_data_builder;
mod slot_reads;
mod teardown;
mod typed_array_methods;

use super::layout::NamedStore;
pub(crate) use literal::ObjectLiteralShape;

pub(crate) use ordered_data_builder::OrderedDataPropertyBuilder;

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

    fn value_for_export(&self, export_name: &str) -> Option<Result<Value, RuntimeError>> {
        if let Some((lexical, binding_name)) = self.aliases.get(export_name) {
            return lexical.cell(binding_name).map(|cell| cell.read());
        }
        self.lexical.cell(export_name).map(|cell| cell.read())
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
    Map(super::MapRef),
    Set(super::SetRef),
}

/// A live array prototype identity; no contextual wrapper allocation.
#[derive(Clone)]
pub(crate) struct ArrayPrototypeRef(ArrayRef);

impl ArrayPrototypeRef {
    pub(crate) fn array_ref(&self) -> &ArrayRef {
        &self.0
    }

    pub(crate) fn array(&self) -> ArrayRef {
        self.0.clone()
    }

    pub(crate) fn effective_prototype_slot(&self) -> Option<Prototype> {
        self.0.prototype_slot()
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }

    pub(crate) fn property(&self, key: &str) -> Option<Property> {
        crate::array_own_property_descriptor(&self.0, key).or_else(|| {
            self.effective_prototype_slot()
                .and_then(|prototype| prototype.property(key))
        })
    }

    pub(crate) fn symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.0.own_symbol_property(symbol).or_else(|| {
            self.effective_prototype_slot()
                .and_then(|prototype| prototype.symbol_property(symbol))
        })
    }
}

impl Prototype {
    pub(crate) fn array(array: ArrayRef) -> Self {
        Self::Array(ArrayPrototypeRef(array))
    }

    pub(crate) fn identity(&self) -> (u8, usize) {
        match self {
            Self::Object(o) => (1, o.address()),
            Self::Array(a) => (2, a.array_ref().address()),
            Self::Function(f) => (3, f.address()),
            Self::Proxy(p) => (4, p.address()),
            Self::Map(m) => (5, m.address()),
            Self::Set(s) => (6, s.address()),
        }
    }
    #[inline(always)]
    pub(crate) fn load_guarded_data(&self, slot: u32) -> Option<Value> {
        use super::named::NamedTarget;
        match self {
            Self::Object(o) => NamedTarget::Object(o).load_data(slot),
            Self::Array(a) => NamedTarget::Array(a.array_ref()).load_data(slot),
            Self::Function(f) => NamedTarget::Function(f).load_data(slot),
            Self::Map(m) => NamedTarget::Map(m).load_data(slot),
            Self::Set(s) => NamedTarget::Set(s).load_data(slot),
            Self::Proxy(_) => None,
        }
    }
    pub(crate) fn into_value(self) -> Value {
        match self {
            Self::Object(v) => Value::Object(v),
            Self::Array(v) => Value::Array(v.0),
            Self::Function(v) => Value::Function(v),
            Self::Proxy(v) => Value::Proxy(v),
            Self::Map(v) => Value::Map(v),
            Self::Set(v) => Value::Set(v),
        }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Object(left), Self::Object(right)) => left.ptr_eq(right),
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Function(left), Self::Function(right)) => left.ptr_eq(right),
            (Self::Proxy(left), Self::Proxy(right)) => left.ptr_eq(right),
            (Self::Map(left), Self::Map(right)) => left.ptr_eq(right),
            (Self::Set(left), Self::Set(right)) => left.ptr_eq(right),
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

            Self::Map(_carrier) => {
                let object = &_carrier.object();
                object.property(key)
            }

            Self::Set(_carrier) => {
                let object = &_carrier.object();
                object.property(key)
            }
        }
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

            Self::Map(_carrier) => {
                let object = &_carrier.object();
                object.symbol_property(symbol)
            }

            Self::Set(_carrier) => {
                let object = &_carrier.object();
                object.symbol_property(symbol)
            }
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

            Self::Map(_carrier) => {
                let object = &_carrier.object();
                object.to_string_tag()
            }

            Self::Set(_carrier) => {
                let object = &_carrier.object();
                object.to_string_tag()
            }
        }
    }

    /// Ordinary cycle checks stop at proxies, without invoking their traps.
    /// Carrier objects use their ordinary payload identity, matching [[Prototype]].
    fn would_cycle(&self, target: &ObjectRef) -> bool {
        self.chain_contains((1, target.address()))
    }

    pub(crate) fn would_cycle_array(&self, target: &ArrayRef) -> bool {
        self.chain_contains((2, target.address()))
    }

    pub(crate) fn would_cycle_function(&self, target: &Function) -> bool {
        self.chain_contains((3, target.address()))
    }

    /// Brent's checkpoint detects even a host-created malformed cycle with
    /// constant storage. Ordinary JS chains are acyclic; proxies terminate
    /// this pure walk without invoking traps.
    fn chain_contains(&self, target: (u8, usize)) -> bool {
        let mut next = Some(self.clone());
        let mut checkpoint = None;
        let mut power = 1usize;
        let mut span = 0usize;
        while let Some(prototype) = next {
            crate::diagnostics::count!(prototype_cycle_visits);
            let identity = match &prototype {
                Self::Map(m) => (1, m.object_ref().address()),
                Self::Set(s) => (1, s.object_ref().address()),
                _ => prototype.identity(),
            };
            if identity == target {
                return true;
            }
            if checkpoint == Some(identity) {
                return false;
            }
            if span == power {
                checkpoint = Some(identity);
                power = power.saturating_mul(2);
                span = 0;
            }
            span += 1;
            next = match prototype {
                Self::Object(o) => o.prototype_slot(),
                Self::Array(a) => a.effective_prototype_slot(),
                Self::Function(f) => f.effective_internal_prototype(),
                Self::Map(m) => m.object_ref().prototype_slot(),
                Self::Set(s) => s.object_ref().prototype_slot(),
                Self::Proxy(_) => None,
            };
        }
        false
    }

    /// The prototype as a JavaScript value, for `getPrototypeOf` and friends.
    pub(crate) fn to_value(&self) -> Value {
        match self {
            Self::Object(object) => Value::Object(object.clone()),
            Self::Array(array) => Value::Array(array.array()),
            Self::Function(function) => Value::Function(function.clone()),
            Self::Proxy(proxy) => Value::Proxy(proxy.clone()),
            Self::Map(map) => Value::Map(map.clone()),
            Self::Set(set) => Value::Set(set.clone()),
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

mod flags {
    pub(super) const EXTENSIBLE: u16 = 1;
    pub(super) const RAW_JSON: u16 = 1 << 1;
    pub(super) const ARRAY_PROTOTYPE: u16 = 1 << 2;
    pub(super) const TYPED_ARRAY: u16 = 1 << 3;
    pub(super) const IMMUTABLE_PROTOTYPE: u16 = 1 << 4;
    pub(super) const MODULE_NAMESPACE: u16 = 1 << 5;
    pub(super) const STRING_INDICES: u16 = 1 << 6;
    pub(super) const SYMBOL_MASK: u16 = 3 << 7;
}

struct ObjectData {
    properties: RefCell<NamedStore>,
    /// Count of own string keys that parse as array indices. Maintained as keys
    /// are added and removed so `has_own_index_property` is an O(1) check; this
    /// keeps the `array[i] = x` fast path from scanning a prototype's keys on
    /// every write.
    index_property_count: Cell<u32>,
    flags: Cell<u16>,
    cold: OnceCell<ColdPayload>,
}

#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<ObjectData>() == 96);

#[derive(Default)]
struct ObjectColdData {
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
    fn flag(&self, mask: u16) -> bool {
        self.flags.get() & mask != 0
    }
    fn set_flag(&self, mask: u16, value: bool) {
        self.flags.set(if value {
            self.flags.get() | mask
        } else {
            self.flags.get() & !mask
        });
    }
    fn symbol_brand(&self) -> SymbolBrand {
        match (self.flags.get() & flags::SYMBOL_MASK) >> 7 {
            1 => SymbolBrand::Primitive,
            2 => SymbolBrand::Boxed,
            _ => SymbolBrand::None,
        }
    }
    fn set_symbol_brand(&self, brand: SymbolBrand) {
        self.flags
            .set((self.flags.get() & !flags::SYMBOL_MASK) | ((brand as u16) << 7));
    }
}

impl fmt::Debug for ObjectRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol_property_count = self.own_property_symbols().len();
        let to_string_tag = self
            .0
            .cold_if_present()
            .and_then(|cold| cold.to_string_tag.borrow().clone());
        formatter
            .debug_struct("ObjectRef")
            .field("properties", &self.properties().borrow().len())
            .field("symbol_count", &symbol_property_count)
            .field("has_prototype", &self.prototype_slot().is_some())
            .field("to_string_tag", &to_string_tag)
            .field("raw_json", &self.0.flag(flags::RAW_JSON))
            .field(
                "array_prototype_exotic",
                &self.0.flag(flags::ARRAY_PROTOTYPE),
            )
            .field(
                "immutable_prototype_exotic",
                &self.0.flag(flags::IMMUTABLE_PROTOTYPE),
            )
            .finish()
    }
}

impl ObjectRef {
    fn from_named(
        mut properties: NamedStore,
        prototype: Option<Prototype>,
        index_property_count: usize,
    ) -> Self {
        properties.set_prototype(prototype);
        Self::from_ready_named(properties, index_property_count)
    }
    /// The store already owns its final explicit prototype.
    fn from_ready_named(properties: NamedStore, index_property_count: usize) -> Self {
        Self(Rc::new(ObjectData {
            properties: RefCell::new(properties),
            index_property_count: Cell::new(
                u32::try_from(index_property_count).expect("index count bounded by key namespace"),
            ),

            flags: Cell::new(flags::EXTENSIBLE),
            cold: OnceCell::new(),
        }))
    }

    /// Materialized named storage. Virtual String indices stay in the exotic hook.
    #[inline(always)]
    fn properties(&self) -> &RefCell<NamedStore> {
        &self.0.properties
    }

    #[cold]
    #[inline(never)]
    fn virtual_index_property(&self, key: &str) -> Option<Property> {
        if !self.0.flag(flags::STRING_INDICES) {
            return None;
        }
        let text = self.string_data()?;
        let value = crate::string::string_property(&text, key)?;
        Some(Property::data(value, true, false, false))
    }

    /// Enables the String virtual-index exotic hook without creating named keys.
    pub(crate) fn defer_string_indices(&self) {
        self.0.set_flag(flags::STRING_INDICES, true);
        self.refresh_named_flavor();
    }

    pub(crate) fn new(
        properties: HashMap<String, Value>,
        domain: &crate::property::atom::AtomDomain,
    ) -> Self {
        Self::with_prototype(properties, None, domain)
    }

    pub(crate) fn with_prototype(
        properties: HashMap<String, Value>,
        prototype: Option<ObjectRef>,
        domain: &crate::property::atom::AtomDomain,
    ) -> Self {
        Self::with_prototype_slot(properties, prototype.map(Prototype::Object), domain)
    }

    pub(crate) fn with_prototype_slot(
        properties: HashMap<String, Value>,
        prototype: Option<Prototype>,
        domain: &crate::property::atom::AtomDomain,
    ) -> Self {
        assert!(
            properties.is_empty(),
            "nonempty host input requires explicit order"
        );
        Self::with_ordered_prototype_slot(Vec::new(), prototype, domain)
    }

    /// Ordered host creation: duplicate keys keep their first insertion position
    /// and last value. A HashMap cannot supply a JavaScript insertion order.
    pub(crate) fn new_ordered(
        properties: Vec<(String, Value)>,
        domain: &crate::property::atom::AtomDomain,
    ) -> Self {
        Self::with_ordered_prototype_slot(properties, None, domain)
    }

    pub(crate) fn with_ordered_prototype(
        properties: Vec<(String, Value)>,
        prototype: Option<ObjectRef>,
        domain: &crate::property::atom::AtomDomain,
    ) -> Self {
        Self::with_ordered_prototype_slot(properties, prototype.map(Prototype::Object), domain)
    }

    fn with_ordered_prototype_slot(
        properties: Vec<(String, Value)>,
        prototype: Option<Prototype>,
        domain: &crate::property::atom::AtomDomain,
    ) -> Self {
        let mut builder =
            OrderedDataPropertyBuilder::with_prototype(properties.len(), domain, prototype.clone());
        for (key, value) in properties {
            builder.insert(Rc::from(key), value);
        }
        builder.finish_slot(prototype)
    }

    /// Builds a plain object literal from statically known string keys without
    /// re-running observable property-key conversion or copying the keys for
    /// every object. Duplicate keys retain their first insertion position and
    /// their last value, matching CreateDataProperty semantics.
    /// Host input with explicit order validates the complete key permutation
    /// before allocating any keys. Key creation itself is infallible.
    #[cfg(test)]
    pub(crate) fn with_property_order(
        mut properties: HashMap<String, Value>,
        order: Vec<String>,
        prototype: Option<Prototype>,
        domain: &crate::property::atom::AtomDomain,
    ) -> Result<Self, RuntimeError> {
        let unique: std::collections::HashSet<_> = order.iter().collect();
        if order.len() != properties.len()
            || unique.len() != order.len()
            || order.iter().any(|key| !properties.contains_key(key))
        {
            return Err(RuntimeError {
                thrown: None,
                message: "invalid host property order".to_owned(),
            });
        }
        let mut builder =
            OrderedDataPropertyBuilder::with_prototype(order.len(), domain, prototype.clone());
        for key in order {
            let value = properties.remove(&key).expect("validated host order");
            builder.insert(Rc::from(key), value);
        }
        Ok(builder.finish_slot(prototype))
    }

    #[cfg(test)]
    pub(crate) fn test_set_layout_epoch(&self, epoch: u64) {
        self.0.properties.borrow_mut().test_set_epoch(epoch);
    }

    pub(crate) fn with_literal_properties(
        shape: Rc<ObjectLiteralShape>,
        mut values: Vec<Value>,
        prototype: Option<ObjectRef>,
        domain: &crate::property::atom::AtomDomain,
    ) -> Self {
        assert_eq!(shape.input_len(), values.len());
        // Input slots never point ahead: each unique key gets its slot on
        // first encounter. Compact duplicates in the producer's vector.
        // Equal input/unique cardinalities prove that the first-occurrence
        // slot map is the identity. Most literals need no remapping at all.
        if shape.input_len() != shape.unique_len() {
            for (input, slot) in shape.input_slots.iter().copied().enumerate() {
                if input != slot {
                    let value = std::mem::replace(&mut values[input], Value::Undefined);
                    values[slot] = value;
                }
            }
            values.truncate(shape.unique_len());
        }
        let prototype = prototype.map(Prototype::Object);
        let store = NamedStore::from_data_layout(
            shape.linked_layout(domain, prototype.as_ref()),
            values,
            prototype,
        );
        Self::from_ready_named(store, shape.index_property_count)
    }

    pub(crate) fn with_literal_pair(
        shape: Rc<ObjectLiteralShape>,
        values: [Value; 2],
        prototype: Option<ObjectRef>,
        domain: &crate::property::atom::AtomDomain,
    ) -> Self {
        if shape.input_len() != 2 || shape.unique_len() != 2 {
            return Self::with_literal_properties(shape, Vec::from(values), prototype, domain);
        }
        let prototype = prototype.map(Prototype::Object);
        let store = NamedStore::from_data_pair(
            shape.linked_layout(domain, prototype.as_ref()),
            values,
            prototype,
        );
        Self::from_ready_named(store, shape.index_property_count)
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

    fn refresh_named_flavor(&self) {
        let flavor = self.named_kind().map(u16::from).unwrap_or(9);
        self.0.properties.borrow_mut().set_flavor(flavor);
    }
    pub(crate) fn domain(&self) -> crate::property::atom::AtomDomain {
        self.0.properties.borrow().domain().clone()
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Identity of the shared storage, for hashing this reference as a key.
    pub(crate) fn address(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    pub(crate) fn downgrade(&self) -> ObjectWeakRef {
        ObjectWeakRef(Rc::downgrade(&self.0))
    }

    pub(crate) fn layout_revision(&self) -> super::layout::LayoutVersion {
        self.0.properties.borrow().version()
    }

    pub(crate) fn mark_raw_json(&self) {
        self.0.set_flag(flags::RAW_JSON, true);
    }

    pub(crate) fn is_raw_json(&self) -> bool {
        self.0.flag(flags::RAW_JSON)
    }

    pub(crate) fn mark_array_prototype_exotic(&self) {
        self.0.set_flag(flags::ARRAY_PROTOTYPE, true);
        self.refresh_named_flavor();
    }

    pub(crate) fn is_array_prototype_exotic(&self) -> bool {
        self.0.flag(flags::ARRAY_PROTOTYPE)
    }

    pub(crate) fn mark_symbol_primitive(&self) {
        self.0.set_symbol_brand(SymbolBrand::Primitive);
        self.refresh_named_flavor();
    }

    pub(crate) fn mark_symbol_boxed(&self) {
        self.0.set_symbol_brand(SymbolBrand::Boxed);
        self.refresh_named_flavor();
    }

    pub(crate) fn is_symbol_object(&self) -> bool {
        self.0.symbol_brand() != SymbolBrand::None
    }

    pub(crate) fn is_symbol_primitive(&self) -> bool {
        self.0.symbol_brand() == SymbolBrand::Primitive
    }

    pub(crate) fn mark_immutable_prototype_exotic(&self) {
        self.0.set_flag(flags::IMMUTABLE_PROTOTYPE, true);
    }

    pub(crate) fn mark_module_namespace_exotic(&self) {
        self.0.set_flag(flags::MODULE_NAMESPACE, true);
        self.refresh_named_flavor();
    }

    pub(crate) fn is_module_namespace_exotic(&self) -> bool {
        self.0.flag(flags::MODULE_NAMESPACE)
    }

    /// The enumerable flag of an own string-keyed property of an ordinary
    /// object, or `None` when the property is absent. Exotic own-property
    /// behaviour (a module namespace, a typed array) is not answered here;
    /// callers check for it first.
    pub(crate) fn own_property_enumerable(&self, key: &str) -> Option<bool> {
        self.own_property(key).map(|p| p.enumerable)
    }

    pub(crate) fn set_module_namespace_bindings(&self, bindings: ModuleNamespaceBindings) {
        *self.0.cold().module_namespace_bindings.borrow_mut() = Some(bindings);
    }

    pub(crate) fn get(&self, key: &str) -> Option<Value> {
        self.property(key).map(|p| p.value)
    }

    pub(crate) fn property(&self, key: &str) -> Option<Property> {
        if self.is_typed_array_exotic()
            && crate::typed_array::canonical_numeric_index(key).is_some()
        {
            return crate::typed_array::typed_array_own_property_descriptor(self, key);
        }
        self.own_property(key)
            .or_else(|| self.prototype_slot().as_ref().and_then(|p| p.property(key)))
    }

    /// Whether any own property key parses as an array index. Used to gate the
    /// dense `array[i] = x` fast path: a default prototype with no own indexed
    /// property cannot intercept an index store with an inherited accessor or a
    /// non-writable data property.
    pub(crate) fn has_own_index_property(&self) -> bool {
        self.0.index_property_count.get() > 0
            || (self.0.flag(flags::STRING_INDICES)
                && self
                    .string_data()
                    .is_some_and(|s| crate::string::js_string_code_unit_len(&s) > 0))
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
            self.prototype_slot()
                .as_ref()
                .and_then(|proto| proto.symbol_property(symbol))
        })
    }

    /// The raw [[Prototype]] slot, distinguishing object and function
    /// prototypes.
    pub(crate) fn prototype_slot(&self) -> Option<Prototype> {
        self.0.properties.borrow().prototype()
    }

    /// This object's [[Prototype]] when it is an ordinary object.
    ///
    /// Separate from `prototype_slot` because a property-read cache only ever
    /// wants the ordinary case and should not clone a slot it will discard.
    pub(crate) fn ordinary_prototype(&self) -> Option<ObjectRef> {
        match self.prototype_slot().as_ref() {
            Some(Prototype::Object(prototype)) => Some(prototype.clone()),
            _ => None,
        }
    }

    pub(crate) fn set_prototype_slot(&self, prototype: Option<Prototype>) -> Result<(), ()> {
        if same_prototype_slot(self.prototype_slot().as_ref(), prototype.as_ref()) {
            return Ok(());
        }
        if self.0.flag(flags::IMMUTABLE_PROTOTYPE) {
            return Err(());
        }
        if !self.0.flag(flags::EXTENSIBLE) {
            return Err(());
        }
        if prototype
            .as_ref()
            .is_some_and(|prototype| prototype.would_cycle(self))
        {
            return Err(());
        }
        self.0.properties.borrow_mut().set_prototype(prototype);
        Ok(())
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
        if !self.0.flag(flags::MODULE_NAMESPACE) {
            return Ok(None);
        }
        let mut property = match self.properties().borrow().get_text(key) {
            Some(property) => property,
            None => return Ok(None),
        };
        if let Some(cold) = self.0.cold_if_present()
            && let Some(bindings) = cold.module_namespace_bindings.borrow().as_ref()
            && let Some(value) = bindings.value_for_export(key)
        {
            property.value = value?;
        }
        Ok(Some(property))
    }

    pub(crate) fn set_prototype(&self, prototype: Option<ObjectRef>) -> Result<(), ()> {
        self.set_prototype_slot(prototype.map(Prototype::Object))
    }

    pub(crate) fn to_string_tag(&self) -> Option<String> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.to_string_tag.borrow().clone())
            .or_else(|| {
                self.prototype_slot()
                    .as_ref()
                    .and_then(Prototype::to_string_tag)
            })
    }

    pub(crate) fn set_to_string_tag(&self, tag: &str) {
        *self.0.cold().to_string_tag.borrow_mut() = Some(tag.to_owned());
    }
}

impl ObjectWeakRef {
    pub(crate) fn address(&self) -> usize {
        self.0.as_ptr() as usize
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
    key.starts_with('\0') && !matches!(key, "\0StringData" | "\0DateValue")
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
