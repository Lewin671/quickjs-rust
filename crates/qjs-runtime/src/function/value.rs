//! Function identity, captured execution state and explicit lazy descriptor hooks.
//! All materialized public keys share NamedStore; private names stay separate.
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    ops::Deref,
    rc::Rc,
};

use qjs_ast::FunctionParams;

use crate::function::{BindingCell, Realm};
use crate::{Bytecode, NativeFunction, ObjectRef, Property, PropertyKey, Prototype, Value};

const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

/// A compiled instance-field initializer attached to a class constructor. It
/// runs at construction time with `this` bound to the new instance.
#[derive(Clone)]
pub(crate) struct InstanceFieldInitializer {
    /// The property key the field installs (computed keys are resolved at class
    /// definition time).
    key: InstanceFieldKey,
    pub(crate) definition_plan: Rc<RefCell<crate::value::layout::FieldPlans>>,
    /// The initializer thunk: a function evaluated with `this` = the instance.
    /// `None` for a field with no initializer (which installs `undefined`).
    pub(crate) initializer: Option<Function>,
}

/// A class field key after class-definition-time resolution.
///
/// Public string keys are immutable for the class's lifetime, so retaining
/// them as a shared string avoids deep-copying the same source key for every
/// constructed instance. Symbols retain their existing identity handle and
/// use the generic installation path.
#[derive(Clone)]
enum InstanceFieldKey {
    String(Rc<str>),
    Symbol(ObjectRef),
}

impl InstanceFieldInitializer {
    pub(crate) fn new(key: PropertyKey, initializer: Option<Function>) -> Self {
        let key = match key {
            PropertyKey::String(key) => InstanceFieldKey::String(key.into()),
            PropertyKey::Symbol(symbol) => InstanceFieldKey::Symbol(symbol),
        };
        Self {
            key,
            initializer,
            definition_plan: Rc::default(),
        }
    }

    /// Returns the shareable public string key when the field can take the
    /// ordinary-object installation route.
    pub(crate) fn shared_string_key(&self) -> Option<&Rc<str>> {
        match &self.key {
            InstanceFieldKey::String(key) => Some(key),
            InstanceFieldKey::Symbol(_) => None,
        }
    }

    /// Rebuilds the generic key only when the field cannot use the ordinary
    /// fast path. This preserves the full symbol and exotic-receiver route.
    pub(crate) fn property_key(&self) -> PropertyKey {
        match &self.key {
            InstanceFieldKey::String(key) => PropertyKey::String(key.to_string()),
            InstanceFieldKey::Symbol(symbol) => PropertyKey::Symbol(symbol.clone()),
        }
    }
}

/// A class element applied to each instance at construction time.
#[derive(Clone)]
pub(crate) enum InstanceElementInitializer {
    PublicField(InstanceFieldInitializer),
    PrivateElement(InstancePrivateElement),
}

/// A private element applied to each instance at construction time. Methods and
/// accessors only brand the instance (the function is shared); a field both
/// brands and installs a per-instance value.
#[derive(Clone)]
pub(crate) struct InstancePrivateElement {
    /// The private-name source text, resolved through the constructor's class
    /// private environment when the instance is initialized.
    pub(crate) name: String,
    /// `Some` for a private field: the initializer thunk (or `None` for an
    /// initializer-less field, which installs `undefined`). `None` for a
    /// private method or accessor, which only brands the instance.
    pub(crate) field_initializer: Option<PrivateFieldInit>,
}

/// The initializer for an instance private field.
#[derive(Clone)]
pub(crate) struct PrivateFieldInit {
    /// The thunk evaluated with `this` = the instance, or `None` to install
    /// `undefined`.
    pub(crate) initializer: Option<Function>,
}

/// Cheap shared handle to a user-defined or native function value.
///
/// Functions are copied through the operand stack, environments, properties,
/// and argument vectors on every call. Keeping the object behind one shared
/// allocation makes those copies reference-count bumps instead of repeatedly
/// cloning the function's vectors and maps. Post-construction identity state is
/// interior-mutable inside that same allocation, so handles can never detach
/// into distinct objects through copy-on-write metadata mutation.
#[derive(Clone)]
pub struct Function(Rc<FunctionData>);

#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<FunctionData>() <= 304);

/// A non-owning handle to a [`Function`], for caches that must not keep
/// the function, or a cycle through its own code, alive.
#[derive(Clone)]
pub(crate) struct FunctionWeakRef(std::rc::Weak<FunctionData>);

impl FunctionWeakRef {
    pub(crate) fn address(&self) -> usize {
        self.0.as_ptr() as usize
    }

    pub(crate) fn upgrade(&self) -> Option<Function> {
        self.0.upgrade().map(Function)
    }
}

impl fmt::Debug for FunctionWeakRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FunctionWeakRef(..)")
    }
}

impl Function {
    pub(crate) fn is_last_reference(&self) -> bool {
        Rc::strong_count(&self.0) == 1
    }
    pub(crate) fn release_into(self, pending: &mut Vec<Value>) {
        if let Some(mut data) = Rc::into_inner(self.0) {
            data.named.get_mut().defer_children(pending);
        }
    }
}
impl Drop for FunctionData {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        self.named.get_mut().defer_children(&mut pending);
        if !pending.is_empty() {
            crate::value::teardown::release(pending);
        }
    }
}

/// Storage behind [`Function`]. Public only because it is the target of the
/// handle's public `Deref` implementation; the runtime does not re-export it.
#[doc(hidden)]
pub struct FunctionData {
    /// Optional internal function name.
    pub name: Option<String>,
    /// Whether `name` also creates the function body's internal name binding.
    /// Method definitions have a name property but no inner binding.
    pub(crate) has_name_binding: bool,
    /// Parameter names. Held behind `Rc` so the frequent `Function` value
    /// clones (every property read, capture sync, and call setup) only bump a
    /// refcount instead of deep-cloning the parameter AST, which dominated call
    /// cost (`tasks/archive/T011-call-performance.md`). Parameters are immutable after
    /// the function is created.
    pub params: Rc<FunctionParams>,
    /// Opaque state carried by native closure-like functions (promise
    /// reactions, async helpers, RegExp accessors, ...). User bytecode functions
    /// keep this empty: lexical captures live in indexed [`BindingCell`] cells and
    /// globals live in the shared `realm` cell below.
    native_captures: Option<Rc<crate::function::NativeCaptures>>,
    /// Stable creation realm for bytecode and native functions; intrinsic
    /// prototypes never resolve through mutable global constructor bindings.
    pub(crate) creation: Option<Rc<crate::context::CreationContext>>,
    /// Whether this function was created while the host's temporary dynamic
    /// realm marker was active. The marker object remains live realm state;
    /// this bit only keeps ordinary calls off that cold lookup path.
    pub(crate) has_dynamic_function_realm: bool,
    /// Whether this function currently has an own object-valued override for
    /// the internal dynamic-realm marker. Property mutation keeps this bit in
    /// sync so ordinary calls need not hash the hidden property name.
    pub(crate) has_dynamic_function_realm_override: Cell<bool>,
    /// Memoized result of `can_seed_direct_leaf_call`: every input it checks
    /// (function flags, parameter simplicity, bytecode analysis bits) is
    /// fixed once this function is created, so recomputing it — including an
    /// O(parameter count) scan — on every single call is pure waste.
    pub(crate) direct_leaf_call_eligible: Cell<Option<bool>>,
    /// Memoized: whether `new` on this base class constructor may run on the
    /// direct-leaf frame with its instance fields installed by the fast field
    /// loop. Resolved on the first construction, after class definition has
    /// recorded every instance element.
    pub(crate) direct_construct_eligible: Cell<Option<bool>>,
    /// Memoized, packed: whether the compact wide tier may run this function
    /// in a window of its caller's register stack as far as the function's
    /// own fixed facts decide, and the facts the driver enters it with
    /// (`compact_fn::wide`); zero until the first call. The caller-dependent
    /// checks still run on every call.
    pub(crate) wide_inline_facts: Cell<u64>,
    /// The compact tier's counterpart: whether its admission proof, as far as
    /// it depends on the function alone, holds, and the register count.
    pub(crate) compact_inline_facts: Cell<u32>,
    pub(crate) upvalues: Vec<BindingCell>,
    /// Received-upvalue slots backed by the function creation realm. The cells
    /// stay live; this mask only avoids rediscovering their identity by name on
    /// every direct call.
    pub(crate) realm_upvalue_slots: u128,
    pub(crate) local_names: Rc<Vec<String>>,
    pub(crate) bytecode: Option<Rc<Bytecode>>,
    /// Original source retained for `Function.prototype.toString`. Function
    /// source is fixed at creation, so it does not need a `RefCell` in the
    /// mutable auxiliary header.
    pub(crate) source_text: Option<Rc<str>>,
    pub(crate) native: Option<NativeFunction>,
    /// Operation-specific entry bound at native installation, never on first call.
    pub(crate) native_entry: Option<crate::native::NativeEntry>,
    pub(crate) constructable: bool,
    pub(crate) is_strict: bool,
    pub(crate) lexical_this: bool,
    pub(crate) lexical_arguments: bool,
    /// Whether this is a generator function (`function*` / `*m()`), which
    /// returns a generator object when called instead of running its body.
    pub(crate) is_generator: bool,
    /// Whether this is an async function (`async function` / `async () =>` /
    /// async method), which returns a promise when called and suspends its body
    /// on `await`.
    pub(crate) is_async: bool,
    /// Whether this is a class constructor, which must be invoked with `new`.
    pub(crate) is_class_constructor: bool,
    /// Whether this is a derived (extends) class constructor, whose `this` is
    /// uninitialized until `super(...)` runs.
    pub(crate) is_derived_constructor: bool,
    /// Call-owner restrictions for an ordinary body, field or static block.
    pub(crate) owner_kind: crate::context::OwnerKind,
    /// The method/constructor [[HomeObject]] used to resolve `super.x`. For an
    /// instance method this is the class prototype; for a static method it is
    /// the constructor; for a derived constructor it is the prototype.
    pub(crate) auxiliary: FunctionAuxiliaryState,
    pub(crate) bound: Option<Box<BoundFunction>>,
    /// Function object properties.
    named: RefCell<crate::value::layout::NamedStorage>,
}

/// Identity-bearing mutable state that is cold for ordinary calls. Keeping
/// these cells behind one shared allocation preserves function identity across
/// cloned handles without paying for ten independent `Rc` allocations for
/// every closure.
#[doc(hidden)]
pub struct FunctionAuxiliaryState {
    /// Fresh functions expose the standard `length` and `name` data
    /// properties without allocating entries in the general property table.
    /// They are materialized together only when descriptor mutation requires
    /// identity-bearing storage.
    implicit_length_property: Cell<bool>,
    implicit_name_property: Cell<bool>,
    extensible: Cell<bool>,
    sealed: Cell<bool>,
    frozen: Cell<bool>,
    lazy_default_prototype: Cell<bool>,
    /// Metadata absent from ordinary closures. A single lazy allocation keeps
    /// the hot function object compact while preserving shared object identity
    /// when methods, classes, symbols, or prototype mutation need this state.
    cold: RefCell<Option<Box<FunctionColdState>>>,
}

struct FunctionColdState {
    lexical_scope: Option<crate::binding::ScopeCursor>,
    receiver_layout: Option<Rc<crate::value::layout::Layout>>,
    constructor_prototype_slot: Option<(crate::value::layout::LayoutVersion, usize)>,
    home_object: Option<Value>,
    /// An arrow's lexical receiver, target, home and restriction owner.
    lexical_call: Option<Rc<crate::context::CallOwner>>,
    /// For a derived constructor, the parent constructor invoked by `super()`.
    super_constructor: Option<Value>,
    /// For a class constructor, the instance-field initializers run when a new
    /// instance is constructed (base class: at construction start; derived
    /// class: immediately after `super()` returns).
    instance_elements: Rc<Vec<InstanceElementInitializer>>,
    /// Explicit [[Prototype]] override. `None` means "use the default
    /// %Function.prototype% intrinsic"; `Some(None)` means it is null.
    private_state: crate::private::PrivateState,
}

impl FunctionAuxiliaryState {
    fn new(home_object: Option<Value>, super_constructor: Option<Value>) -> Self {
        let cold = (home_object.is_some() || super_constructor.is_some())
            .then(|| Box::new(FunctionColdState::new(home_object, super_constructor)));
        Self {
            implicit_length_property: Cell::new(true),
            implicit_name_property: Cell::new(true),
            extensible: Cell::new(true),
            sealed: Cell::new(false),
            frozen: Cell::new(false),
            lazy_default_prototype: Cell::new(false),
            cold: RefCell::new(cold),
        }
    }

    fn with_cold<R>(&self, read: impl FnOnce(Option<&FunctionColdState>) -> R) -> R {
        let cold = self.cold.borrow();
        read(cold.as_deref())
    }

    fn with_cold_mut<R>(&self, mutate: impl FnOnce(&mut FunctionColdState) -> R) -> R {
        let mut cold = self.cold.borrow_mut();
        let cold = cold.get_or_insert_with(|| Box::new(FunctionColdState::new(None, None)));
        mutate(cold)
    }
}

/// See `Function::call_owner_inputs`.
pub(crate) enum CallOwnerInputs {
    Lexical(Rc<crate::context::CallOwner>),
    Root {
        home: Option<Value>,
        super_constructor: Option<Value>,
    },
}

impl FunctionColdState {
    fn new(home_object: Option<Value>, super_constructor: Option<Value>) -> Self {
        Self {
            lexical_scope: None,
            home_object,
            receiver_layout: None,
            constructor_prototype_slot: None,
            lexical_call: None,
            super_constructor,
            instance_elements: Rc::new(Vec::new()),
            private_state: crate::private::PrivateState::default(),
        }
    }
}

impl Deref for Function {
    type Target = FunctionData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Bound function internal slots.
#[derive(Clone)]
pub(crate) struct BoundFunction {
    pub(crate) target: Value,
    pub(crate) this_value: Value,
    pub(crate) arguments: Vec<Value>,
}

pub(crate) struct CompiledUserFunction {
    pub(crate) name: Option<String>,
    pub(crate) has_name_binding: bool,
    pub(crate) params: Rc<FunctionParams>,
    pub(crate) creation: Rc<crate::context::CreationContext>,
    pub(crate) bytecode: Rc<Bytecode>,
    pub(crate) source_text: Option<Rc<str>>,
    pub(crate) local_names: Rc<Vec<String>>,
    pub(crate) constructable: bool,
    pub(crate) is_strict: bool,
    pub(crate) lexical_this: bool,
    pub(crate) lexical_arguments: bool,
    pub(crate) lexical_call: Option<Rc<crate::context::CallOwner>>,
    pub(crate) is_generator: bool,
    pub(crate) is_async: bool,
    pub(crate) is_class_constructor: bool,
    pub(crate) is_derived_constructor: bool,
    pub(crate) owner_kind: crate::context::OwnerKind,
    pub(crate) home_object: Option<Value>,
    pub(crate) super_constructor: Option<Value>,
    pub(crate) lexical_scope: Option<crate::binding::ScopeCursor>,
    pub(crate) upvalues: Vec<BindingCell>,
}

impl fmt::Debug for Function {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Function")
            .field("name", &self.name)
            .field("length", &self.params.length())
            .field("native", &self.native)
            .field("local_names", &self.local_names.len())
            .field("bytecode", &self.bytecode.is_some())
            .field("constructable", &self.constructable)
            .field("is_strict", &self.is_strict)
            .field("lexical_this", &self.lexical_this)
            .field("lexical_arguments", &self.lexical_arguments)
            .field("bound", &self.bound.is_some())
            .finish()
    }
}

impl Function {
    pub(crate) fn native_captures(&self) -> &crate::function::NativeCaptures {
        self.native_captures
            .as_deref()
            .unwrap_or(&crate::function::NativeCaptures::None)
    }
    pub(crate) fn set_native_captures(&mut self, captures: crate::function::NativeCaptures) {
        let data =
            Rc::get_mut(&mut self.0).expect("capture payload is installed before function sharing");
        crate::diagnostics::count!(native_capture_payload_allocations);
        data.native_captures = Some(Rc::new(captures));
    }

    /// Adds an indexed capture while constructing a fresh internal native
    /// function. Keeping this mutation behind the handle prevents general
    /// copy-on-write detachment of JavaScript function identity.
    pub(crate) fn push_upvalue(&mut self, upvalue: BindingCell) {
        Rc::get_mut(&mut self.0)
            .expect("upvalues must be populated before sharing the function")
            .upvalues
            .push(upvalue);
    }

    pub(crate) fn lexical_scope(&self) -> Option<crate::binding::ScopeCursor> {
        self.auxiliary
            .with_cold(|cold| cold.and_then(|cold| cold.lexical_scope.clone()))
    }

    pub(crate) fn new_user_compiled(compiled: CompiledUserFunction) -> Self {
        let CompiledUserFunction {
            name,
            has_name_binding,
            params,
            creation,
            bytecode,
            source_text,
            local_names,
            constructable,
            is_strict,
            lexical_this,
            lexical_arguments,
            lexical_call,
            is_generator,
            is_async,
            is_class_constructor,
            is_derived_constructor,
            owner_kind,
            home_object,
            super_constructor,
            lexical_scope,
            upvalues,
        } = compiled;
        let self_name = has_name_binding.then(|| {
            BindingCell::with_metadata(
                bytecode
                    .self_name_binding()
                    .expect("compiled self-name metadata")
                    .binding
                    .clone(),
                None,
            )
        });
        let lexical_scope = if let Some(cell) = &self_name {
            Some(crate::binding::ScopeCursor::with_function_name(
                lexical_scope,
                cell.clone(),
                bytecode.self_name_binding().expect("self name plan"),
            ))
        } else {
            lexical_scope
        };
        let realm = &creation.realm;
        let realm_upvalue_slots = bytecode
            .received_upvalue_slots()
            .iter()
            .zip(&upvalues)
            .filter(|&(slot, cell)| {
                *slot < u128::BITS as usize
                    && bytecode
                        .local_name_at(*slot)
                        .is_some_and(|name| realm.is_binding_cell(name, cell))
            })
            .map(|(slot, _)| 1_u128 << *slot)
            .fold(0, |slots, slot| slots | slot);
        let has_dynamic_function_realm = realm.has_dynamic_function_realm_global()
            || !matches!(creation.global, crate::context::GlobalSelection::Default);
        let auxiliary = FunctionAuxiliaryState::new(home_object, super_constructor);
        let function = Self(Rc::new(FunctionData {
            has_name_binding,
            name,
            params,
            native_captures: None,
            creation: Some(creation),
            has_dynamic_function_realm,
            has_dynamic_function_realm_override: Cell::new(false),
            direct_leaf_call_eligible: Cell::new(None),
            direct_construct_eligible: Cell::new(None),
            wide_inline_facts: Cell::new(0),
            compact_inline_facts: Cell::new(0),
            upvalues,
            realm_upvalue_slots,
            local_names,
            bytecode: Some(bytecode),
            source_text,
            native: None,
            native_entry: None,
            constructable,
            is_strict,
            lexical_this,
            lexical_arguments,
            is_generator,
            is_async,
            is_class_constructor,
            is_derived_constructor,
            owner_kind,
            auxiliary,
            bound: None,
            named: RefCell::new(crate::value::layout::NamedStorage::Sentinel),
        }));
        if let Some(cell) = self_name {
            cell.initialize(Value::Function(function.clone()))
                .expect("fresh self binding");
        }
        if let Some(scope) = lexical_scope {
            crate::diagnostics::count!(function_scope_retentions);
            function
                .auxiliary
                .with_cold_mut(|cold| cold.lexical_scope = Some(scope));
        }
        if let Some(owner) = lexical_call {
            function
                .auxiliary
                .with_cold_mut(|cold| cold.lexical_call = Some(owner));
        }
        function.initialize_named();
        function.define_length_property();
        function.define_name_property();
        // Class constructors receive their `prototype` wiring from the class
        // builder so the property attributes and prototype object can match the
        // class semantics; ordinary functions get the default prototype here.
        if constructable && !is_class_constructor {
            function.mark_lazy_default_prototype();
        }
        function
    }

    /// Installs the class-constructor `prototype` property and its
    /// `constructor` back-reference with the attributes ECMAScript mandates:
    /// the constructor's `prototype` is non-writable, non-enumerable, and
    /// non-configurable, while the prototype's `constructor` is writable,
    /// non-enumerable, and configurable.
    pub(crate) fn install_class_prototype(&self, prototype: ObjectRef) {
        prototype.define_property(
            "constructor".to_owned(),
            Property::data(Value::Function(self.clone()), false, true, true),
        );
        self.define_property(
            "prototype".to_owned(),
            Property::data(Value::Object(prototype), false, false, false),
        );
    }

    pub(crate) fn new_native(
        name: Option<&str>,
        length: usize,
        native: NativeFunction,
        constructable: bool,
        realm: &Realm,
    ) -> Self {
        Self::new(
            name.map(str::to_owned),
            vec![String::new(); length],
            Some(native),
            constructable,
            Some(realm.atoms()),
            Some(realm.clone()),
        )
    }
    pub(crate) fn creation_context(&self) -> &Rc<crate::context::CreationContext> {
        self.creation.as_ref().expect("callable creation context")
    }
    pub(crate) fn realm(&self) -> Option<&Realm> {
        self.creation.as_ref().map(|creation| &creation.realm)
    }
    pub(crate) fn creation_realm(&self) -> &Realm {
        &self.creation_context().realm
    }

    pub(crate) fn has_module_imports(&self) -> bool {
        self.creation
            .as_ref()
            .is_some_and(|creation| !creation.imports().is_empty())
    }
    #[cfg(test)]
    pub(crate) fn new_native_in_domain(
        name: Option<&str>,
        length: usize,
        native: NativeFunction,
        constructable: bool,
        domain: &crate::property::atom::AtomDomain,
    ) -> Self {
        Self::new(
            name.map(str::to_owned),
            vec![String::new(); length],
            Some(native),
            constructable,
            Some(domain),
            Some(crate::context::new_realm_sharing_atoms(
                HashMap::new(),
                domain.clone(),
            )),
        )
    }

    pub(crate) fn uninitialized_lexical_marker() -> Self {
        // Internal TDZ sentinel: never creates an ordinary object or realm.
        Self::new(
            Some("\0\0uninitialized_lexical".to_owned()),
            Vec::new(),
            Some(NativeFunction::UninitializedLexical),
            false,
            None,
            None,
        )
    }

    pub(crate) fn property_domain(&self) -> &crate::property::atom::AtomDomain {
        if let Some(realm) = self.realm() {
            return realm.atoms();
        }
        panic!("TDZ sentinel has no property domain")
    }

    pub(crate) fn downgrade(&self) -> FunctionWeakRef {
        FunctionWeakRef(Rc::downgrade(&self.0))
    }

    pub(crate) fn is_uninitialized_lexical_marker(&self) -> bool {
        matches!(self.native, Some(NativeFunction::UninitializedLexical))
    }

    pub(crate) fn native_kind(&self) -> Option<NativeFunction> {
        self.native
    }

    pub(crate) fn new_bound(
        target: Value,
        this_value: Value,
        arguments: Vec<Value>,
        length: usize,
        _domain: &crate::property::atom::AtomDomain,
    ) -> Self {
        let constructable = match &target {
            Value::Function(function) => function.constructable,
            _ => false,
        };
        let name = bound_function_name(&target);
        let auxiliary = FunctionAuxiliaryState::new(None, None);
        let function = Self(Rc::new(FunctionData {
            name: Some(name),
            has_name_binding: false,
            params: Rc::new(FunctionParams::positional(vec![String::new(); length])),
            native_captures: None,
            creation: callable_creation(&target),
            has_dynamic_function_realm: match &target {
                Value::Function(function) => function.has_dynamic_function_realm,
                _ => false,
            },
            has_dynamic_function_realm_override: Cell::new(false),
            direct_leaf_call_eligible: Cell::new(None),
            direct_construct_eligible: Cell::new(None),
            wide_inline_facts: Cell::new(0),
            compact_inline_facts: Cell::new(0),
            upvalues: Vec::new(),
            realm_upvalue_slots: 0,
            local_names: Rc::new(Vec::new()),
            bytecode: None,
            source_text: None,
            native: None,
            native_entry: None,
            constructable,
            is_strict: false,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            owner_kind: crate::context::OwnerKind::Function,
            auxiliary,
            bound: Some(Box::new(BoundFunction {
                target,
                this_value,
                arguments,
            })),
            named: RefCell::new(crate::value::layout::NamedStorage::Sentinel),
        }));
        function.initialize_named();
        function.define_length_property();
        function.define_name_property();
        function
    }

    fn new(
        name: Option<String>,
        params: Vec<String>,
        native: Option<NativeFunction>,
        constructable: bool,
        domain: Option<&crate::property::atom::AtomDomain>,
        creation_realm: Option<Realm>,
    ) -> Self {
        let prototype = constructable.then(|| {
            ObjectRef::new(
                HashMap::new(),
                domain.expect("constructors require an explicit domain"),
            )
        });
        let auxiliary = FunctionAuxiliaryState::new(None, None);
        let function = Self(Rc::new(FunctionData {
            has_name_binding: false,
            name,
            params: Rc::new(FunctionParams::positional(params)),
            native_captures: None,
            creation: creation_realm.map(crate::context::CreationContext::root),
            has_dynamic_function_realm: false,
            has_dynamic_function_realm_override: Cell::new(false),
            direct_leaf_call_eligible: Cell::new(None),
            direct_construct_eligible: Cell::new(None),
            wide_inline_facts: Cell::new(0),
            compact_inline_facts: Cell::new(0),
            upvalues: Vec::new(),
            realm_upvalue_slots: 0,
            local_names: Rc::new(Vec::new()),
            bytecode: None,
            source_text: None,
            native,
            native_entry: native.map(crate::native::bind),
            constructable,
            is_strict: false,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            owner_kind: crate::context::OwnerKind::Function,
            auxiliary,
            bound: None,
            named: RefCell::new(crate::value::layout::NamedStorage::Sentinel),
        }));
        function.initialize_named();
        function.define_length_property();
        function.define_name_property();
        if let Some(prototype) = prototype {
            prototype
                .define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
            function.define_property(
                "prototype".to_owned(),
                // A function's `prototype` is writable and non-enumerable but
                // non-configurable (a class constructor's is also non-writable,
                // wired separately by the class builder).
                Property::data(Value::Object(prototype), false, true, false),
            );
        }
        function
    }

    fn define_length_property(&self) {
        self.auxiliary.implicit_length_property.set(true);
    }

    fn define_name_property(&self) {
        self.auxiliary.implicit_name_property.set(true);
    }

    fn default_length_property(&self) -> Property {
        Property::data(
            Value::Number(self.params.length() as f64),
            false,
            false,
            true,
        )
    }

    fn default_name_property(&self) -> Property {
        Property::data(
            Value::String(self.name.clone().unwrap_or_default().into()),
            false,
            false,
            true,
        )
    }

    pub(crate) fn symbol_property(
        &self,
        symbol: &ObjectRef,
        _env: &crate::context::Context<'_>,
    ) -> Option<Property> {
        self.own_symbol_property(symbol)
            .or_else(|| match self.effective_internal_prototype() {
                Some(Prototype::Object(prototype)) => prototype.symbol_property(symbol),
                Some(Prototype::Array(array)) => array.symbol_property(symbol),
                Some(Prototype::Function(parent)) => parent.chain_symbol_property(symbol),
                Some(Prototype::Proxy(proxy)) => proxy.target_result().ok().and_then(|target| {
                    crate::property::own_or_inherited_symbol_descriptor(target, symbol)
                }),
                None => None,

                Some(Prototype::Map(_carrier)) => {
                    let prototype = _carrier.object();
                    prototype.symbol_property(symbol)
                }

                Some(Prototype::Set(_carrier)) => {
                    let prototype = _carrier.object();
                    prototype.symbol_property(symbol)
                }
            })
    }

    /// The selected [[Prototype]] identity; construction installed the intrinsic
    /// default once. Pure descriptor walking requires no execution context.
    pub(crate) fn effective_internal_prototype(&self) -> Option<Prototype> {
        self.named.borrow().prototype()
    }

    /// Walks this function's own properties, then its [[Prototype]] chain, for a
    /// string-keyed property. Used when a function sits inside another value's
    /// prototype chain.
    pub(crate) fn chain_property(&self, key: &str) -> Option<Property> {
        self.own_property(key)
            .or_else(|| match self.effective_internal_prototype() {
                Some(Prototype::Object(prototype)) => prototype.property(key),
                Some(Prototype::Array(array)) => array.property(key),
                Some(Prototype::Function(parent)) => parent.chain_property(key),
                Some(Prototype::Proxy(proxy)) => proxy
                    .target_result()
                    .ok()
                    .and_then(|target| crate::property::own_or_inherited_descriptor(target, key)),
                None => None,

                Some(Prototype::Map(_carrier)) => {
                    let prototype = _carrier.object();
                    prototype.property(key)
                }

                Some(Prototype::Set(_carrier)) => {
                    let prototype = _carrier.object();
                    prototype.property(key)
                }
            })
    }

    pub(crate) fn chain_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.own_symbol_property(symbol)
            .or_else(|| match self.effective_internal_prototype() {
                Some(Prototype::Object(prototype)) => prototype.symbol_property(symbol),
                Some(Prototype::Array(array)) => array.symbol_property(symbol),
                Some(Prototype::Function(parent)) => parent.chain_symbol_property(symbol),
                Some(Prototype::Proxy(proxy)) => proxy.target_result().ok().and_then(|target| {
                    crate::property::own_or_inherited_symbol_descriptor(target, symbol)
                }),
                None => None,

                Some(Prototype::Map(_carrier)) => {
                    let prototype = _carrier.object();
                    prototype.symbol_property(symbol)
                }

                Some(Prototype::Set(_carrier)) => {
                    let prototype = _carrier.object();
                    prototype.symbol_property(symbol)
                }
            })
    }

    /// Returns the function's private-name storage, creating it on first use.
    pub(crate) fn private_storage(&self) -> crate::private::PrivateStorage {
        self.auxiliary.with_cold_mut(|cold| {
            cold.private_state
                .storage
                .get_or_insert_with(crate::private::PrivateStorage::new)
                .clone()
        })
    }

    pub(crate) fn private_environment(&self) -> Option<crate::private::PrivateEnvironment> {
        self.creation
            .as_ref()
            .and_then(|creation| creation.private.clone())
    }

    /// Records an instance private element (a field initializer or a
    /// method/accessor brand) applied to each instance at construction time.
    pub(crate) fn push_instance_private_element(&self, element: InstancePrivateElement) {
        self.auxiliary.with_cold_mut(|cold| {
            Rc::make_mut(&mut cold.instance_elements)
                .push(InstanceElementInitializer::PrivateElement(element));
        });
    }

    /// Records a public instance field applied at construction time.
    pub(crate) fn push_instance_public_field(&self, field: InstanceFieldInitializer) {
        self.auxiliary.with_cold_mut(|cold| {
            Rc::make_mut(&mut cold.instance_elements)
                .push(InstanceElementInitializer::PublicField(field));
        });
    }

    /// Returns this constructor's instance elements. The list is shared, so a
    /// construction borrows it for one reference-count increment instead of
    /// cloning every initializer thunk per instance.
    pub(crate) fn instance_elements(&self) -> Rc<Vec<InstanceElementInitializer>> {
        self.auxiliary.with_cold(|cold| {
            cold.map_or_else(
                || Rc::new(Vec::new()),
                |cold| Rc::clone(&cold.instance_elements),
            )
        })
    }

    /// The raw [[Prototype]] override slot, preserving a function prototype.
    pub(crate) fn internal_prototype_slot(&self) -> Option<Option<Prototype>> {
        Some(self.effective_internal_prototype())
    }

    /// The function's original source text, when retained.
    pub(crate) fn source_text(&self) -> Option<Rc<str>> {
        self.source_text.clone()
    }

    pub(crate) fn set_internal_prototype_slot(
        &self,
        prototype: Option<Prototype>,
    ) -> Result<(), ()> {
        if same_prototype_slot(&self.effective_internal_prototype(), &prototype) {
            return Ok(());
        }
        if !self.auxiliary.extensible.get() {
            return Err(());
        }
        if prototype
            .as_ref()
            .is_some_and(|prototype| prototype_chain_contains_function(prototype, self))
        {
            return Err(());
        }
        self.named
            .borrow_mut()
            .set_prototype(prototype, self.named_shape_flavor());
        Ok(())
    }

    /// Whether this function carries either piece of cold lexical state that
    /// a shared-environment call admission has to rule out.
    ///
    /// Asking through `private_environment` and `home_object` costs two
    /// `RefCell` borrows and two clones -- one of them a full `Value` -- to
    /// discover, on essentially every call, that both are absent.
    pub(crate) fn has_cold_lexical_state(&self) -> bool {
        self.creation
            .as_ref()
            .is_some_and(|creation| creation.private.is_some())
            || self.auxiliary.with_cold(|cold| {
                cold.is_some_and(|cold| {
                    cold.home_object.is_some()
                        || cold.lexical_scope.is_some()
                        || cold.lexical_call.is_some()
                })
            })
    }

    pub(crate) fn home_object(&self) -> Option<Value> {
        self.auxiliary
            .with_cold(|cold| cold.and_then(|cold| cold.home_object.clone()))
    }

    pub(crate) fn set_home_object(&self, home_object: Value) {
        self.auxiliary
            .with_cold_mut(|cold| cold.home_object = Some(home_object));
    }

    pub(crate) fn super_constructor(&self) -> Option<Value> {
        self.auxiliary
            .with_cold(|cold| cold.and_then(|cold| cold.super_constructor.clone()))
    }

    pub(crate) fn lexical_call(&self) -> Option<Rc<crate::context::CallOwner>> {
        self.auxiliary
            .with_cold(|cold| cold.and_then(|cold| cold.lexical_call.clone()))
    }

    /// Makes a released root owner's class state what an indexed entry of
    /// this function binds (`call_owner_inputs`, compared under one borrow):
    /// its home object and super constructor, reusing the owner's class
    /// record. False, changing nothing, when the entry needs a retained
    /// lexical owner instead.
    #[inline]
    pub(crate) fn rebind_class_state(
        &self,
        class: &mut Option<Box<crate::context::ClassOwner>>,
    ) -> bool {
        // An ordinary function without cold state binds none.
        if class.is_none() && self.auxiliary.with_cold(|cold| cold.is_none()) {
            return true;
        }
        self.rebind_method_class_state(class)
    }

    #[inline(never)]
    fn rebind_method_class_state(
        &self,
        class: &mut Option<Box<crate::context::ClassOwner>>,
    ) -> bool {
        fn same(left: Option<&Value>, right: Option<&Value>) -> bool {
            match (left, right) {
                (None, None) => true,
                (Some(left), Some(right)) => left.same_value(right),
                _ => false,
            }
        }
        self.auxiliary.with_cold(|cold| {
            let (home, super_constructor) = match cold {
                None => (None, None),
                Some(cold) if cold.lexical_call.is_none() => {
                    (cold.home_object.as_ref(), cold.super_constructor.as_ref())
                }
                Some(_) => return false,
            };
            match class {
                None if home.is_none() && super_constructor.is_none() => {}
                Some(_) if home.is_none() && super_constructor.is_none() => *class = None,
                Some(state)
                    if same(home, state.home.as_ref())
                        && same(super_constructor, state.super_constructor.as_ref()) => {}
                Some(state) => {
                    state.home = home.cloned();
                    state.super_constructor = super_constructor.cloned();
                }
                None => {
                    *class = Some(Box::new(crate::context::ClassOwner {
                        home: home.cloned(),
                        super_constructor: super_constructor.cloned(),
                        active_constructor: None,
                    }))
                }
            }
            true
        })
    }

    /// The call-owner inputs an indexed entry needs, under one borrow: a
    /// retained lexical owner, else the home object and super constructor.
    /// A function without cold state answers without cloning anything.
    pub(crate) fn call_owner_inputs(&self) -> CallOwnerInputs {
        self.auxiliary.with_cold(|cold| match cold {
            None => CallOwnerInputs::Root {
                home: None,
                super_constructor: None,
            },
            Some(cold) => match &cold.lexical_call {
                Some(owner) => CallOwnerInputs::Lexical(owner.clone()),
                None => CallOwnerInputs::Root {
                    home: cold.home_object.clone(),
                    super_constructor: cold.super_constructor.clone(),
                },
            },
        })
    }

    #[cfg(test)]
    fn property_order(&self) -> Vec<String> {
        self.with_named(|s| s.map_or_else(Vec::new, |s| s.text_keys()))
    }
}

fn callable_creation(value: &Value) -> Option<Rc<crate::context::CreationContext>> {
    match value {
        Value::Function(function) => function.creation.clone(),
        Value::Proxy(proxy) => proxy
            .target_result()
            .ok()
            .and_then(|value| callable_creation(&value)),
        _ => None,
    }
}

fn bound_function_name(target: &Value) -> String {
    let target_name = match target {
        Value::Function(function) => function
            .own_property("name")
            .and_then(|property| match &property.value {
                Value::String(name) => Some(name.to_string()),
                _ => None,
            })
            .or_else(|| function.name.clone())
            .unwrap_or_default(),
        _ => String::new(),
    };
    format!("bound {target_name}")
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

fn same_prototype_slot(left: &Option<Prototype>, right: &Option<Prototype>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.ptr_eq(right),
        _ => false,
    }
}

fn prototype_chain_contains_function(prototype: &Prototype, target: &Function) -> bool {
    prototype_chain_contains_function_inner(prototype, target, &mut Vec::new(), &mut Vec::new())
}

fn prototype_chain_contains_function_inner(
    prototype: &Prototype,
    target: &Function,
    _seen_functions: &mut Vec<Function>,
    _seen_objects: &mut Vec<ObjectRef>,
) -> bool {
    prototype.would_cycle_function(target)
}

#[cfg(test)]
mod tests;

mod construction;
mod named;
