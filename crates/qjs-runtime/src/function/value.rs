use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    collections::HashMap,
    fmt,
    ops::Deref,
    rc::Rc,
};

use qjs_ast::{FunctionParams, Stmt};

use crate::CallEnv;
use crate::function::{DynamicBindings, ModuleImports, Realm, Upvalue};
use crate::module::ModuleHostRef;
use crate::{
    Bytecode, NativeFunction, ObjectRef, Property, PropertyKey, Prototype, Value,
    bytecode::compile_function_body,
    function::{collect_function_local_names, is_strict_function_body},
    object_prototype,
};

const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

fn dynamic_function_realm_global(realm: &Realm) -> Option<ObjectRef> {
    realm.dynamic_function_realm_global()
}

/// A compiled instance-field initializer attached to a class constructor. It
/// runs at construction time with `this` bound to the new instance.
#[derive(Clone)]
pub(crate) struct InstanceFieldInitializer {
    /// The property key the field installs (computed keys are resolved at class
    /// definition time).
    pub(crate) key: PropertyKey,
    /// The initializer thunk: a function evaluated with `this` = the instance.
    /// `None` for a field with no initializer (which installs `undefined`).
    pub(crate) initializer: Option<Function>,
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

/// Storage behind [`Function`]. Public only because it is the target of the
/// handle's public `Deref` implementation; the runtime does not re-export it.
#[doc(hidden)]
pub struct FunctionData {
    /// Optional internal function name.
    pub name: Option<String>,
    /// Whether `name` also creates the function body's internal name binding.
    /// Method definitions have a name property but no inner binding.
    pub(crate) has_name_binding: bool,
    /// Whether the internal name binding is the *immutable* binding of a named
    /// function expression (`var f = function g() {}`). Assigning to `g` inside
    /// the body is a silent no-op in sloppy mode and a TypeError in strict mode.
    /// False for function declarations (whose name is an ordinary mutable outer
    /// binding) and for methods.
    pub(crate) immutable_name_binding: bool,
    /// An immutable binding supplied through the captured environment rather
    /// than the function's own name, currently used for class inner names.
    pub(crate) immutable_env_binding: Option<String>,
    /// Value for the rare immutable outer name that is not represented by a
    /// compiler-generated received-upvalue slot (notably the self binding of a
    /// named function expression referenced only by a nested function).
    pub(crate) immutable_env_value: Option<Upvalue>,
    /// Parameter names. Held behind `Rc` so the frequent `Function` value
    /// clones (every property read, capture sync, and call setup) only bump a
    /// refcount instead of deep-cloning the parameter AST, which dominated call
    /// cost (`tasks/T011-call-performance.md`). Parameters are immutable after
    /// the function is created.
    pub params: Rc<FunctionParams>,
    /// Opaque state carried by native closure-like functions (promise
    /// reactions, async helpers, RegExp accessors, ...). User bytecode functions
    /// keep this empty: lexical captures live in indexed [`Upvalue`] cells and
    /// globals live in the shared `realm` cell below.
    pub(crate) native_context: NativeContext,
    /// The creation realm of a user bytecode function. Native functions keep
    /// `None` and receive their active realm through `CallEnv`.
    pub(crate) realm: Option<Realm>,
    /// Cached internal global for functions created by a cross-realm dynamic
    /// Function constructor. Ordinary user functions keep this empty, avoiding
    /// hidden string-property and realm-map lookups on every call.
    pub(crate) dynamic_function_realm_global: Option<ObjectRef>,
    /// Whether this function currently has an own object-valued override for
    /// the internal dynamic-realm marker. Property mutation keeps this bit in
    /// sync so ordinary calls need not hash the hidden property name.
    pub(crate) has_dynamic_function_realm_override: Cell<bool>,
    pub(crate) deopt_bindings: Option<DynamicBindings>,
    pub(crate) module_host: Option<ModuleHostRef>,
    pub(crate) module_imports: ModuleImports,
    pub(crate) with_stack: Vec<Value>,
    pub(crate) upvalues: Vec<Upvalue>,
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
    pub(crate) constructable: bool,
    pub(crate) is_strict: bool,
    pub(crate) lexical_this: bool,
    pub(crate) lexical_arguments: bool,
    /// `new.target` captured when an arrow is created. Unlike `this` and
    /// `arguments`, bytecode reads this through a dedicated opcode, so it is
    /// retained explicitly rather than as an ordinary received upvalue.
    pub(crate) lexical_new_target: Option<Upvalue>,
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
    /// Whether this function is the implicit thunk for a class field
    /// initializer. Direct eval inside such a thunk gets initializer-specific
    /// early errors.
    pub(crate) is_field_initializer: bool,
    /// The method/constructor [[HomeObject]] used to resolve `super.x`. For an
    /// instance method this is the class prototype; for a static method it is
    /// the constructor; for a derived constructor it is the prototype.
    pub(crate) auxiliary: FunctionAuxiliaryState,
    pub(crate) bound: Option<Box<BoundFunction>>,
    /// Function object properties.
    pub(crate) properties: LazyFunctionProperties,
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
    home_object: Option<Value>,
    /// For a derived constructor, the parent constructor invoked by `super()`.
    super_constructor: Option<Value>,
    /// For a class constructor, the instance-field initializers run when a new
    /// instance is constructed (base class: at construction start; derived
    /// class: immediately after `super()` returns).
    /// Immutable after class definition. Share the complete list across
    /// constructions so each instance does not clone every field key and
    /// initializer handle before running them.
    instance_elements: Option<Rc<Vec<InstanceElementInitializer>>>,
    property_order: Vec<String>,
    symbol_properties: Vec<(ObjectRef, Property)>,
    /// Explicit [[Prototype]] override. `None` means "use the default
    /// %Function.prototype% intrinsic"; `Some(None)` means it is null.
    internal_prototype: Option<Option<Prototype>>,
    private_state: crate::private::PrivateState,
}

/// Lazily allocated state captured by native closure-like functions.
///
/// Ordinary user bytecode functions never use this map. Keeping the empty
/// state as `None` avoids a separate heap allocation every time a JavaScript
/// closure is created while preserving the existing lookup API for native
/// helpers that do capture values.
#[derive(Clone, Default)]
pub(crate) struct NativeContext(Option<Rc<HashMap<String, Value>>>);

impl NativeContext {
    fn from_map(map: HashMap<String, Value>) -> Self {
        if map.is_empty() {
            Self::default()
        } else {
            Self(Some(Rc::new(map)))
        }
    }

    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        self.0.as_deref().and_then(|context| context.get(key))
    }

    pub(crate) fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.iter().flat_map(|context| context.keys())
    }

    fn insert(&mut self, key: String, value: Value) {
        let context = self.0.get_or_insert_with(|| Rc::new(HashMap::new()));
        Rc::make_mut(context).insert(key, value);
    }

    pub(crate) fn clone_map(&self) -> HashMap<String, Value> {
        self.0.as_deref().cloned().unwrap_or_default()
    }

    #[cfg(test)]
    fn is_allocated(&self) -> bool {
        self.0.is_some()
    }
}

/// Compact property storage for functions that have not materialized an own
/// property table.
///
/// The standard `length`, `name`, and ordinary-function `prototype` slots are
/// already represented implicitly. Most short-lived closures therefore never
/// need a general map at all. Explicit property observation or mutation keeps
/// the existing `RefCell<HashMap<...>>` borrowing behavior while moving the
/// much larger empty map header out of every function allocation.
#[derive(Default)]
// The indirection is deliberate: it removes the 56-byte empty map header from
// every short-lived closure and is allocated only on explicit property access.
#[allow(clippy::box_collection)]
pub(crate) struct LazyFunctionProperties(RefCell<Option<Box<HashMap<String, Property>>>>);

impl LazyFunctionProperties {
    pub(crate) fn borrow(&self) -> Ref<'_, HashMap<String, Property>> {
        self.ensure_allocated();
        Ref::map(self.0.borrow(), |properties| {
            properties
                .as_deref()
                .expect("function properties initialized before borrowing")
        })
    }

    pub(crate) fn borrow_mut(&self) -> RefMut<'_, HashMap<String, Property>> {
        self.ensure_allocated();
        RefMut::map(self.0.borrow_mut(), |properties| {
            properties
                .as_deref_mut()
                .expect("function properties initialized before borrowing")
        })
    }

    fn ensure_allocated(&self) {
        if self.0.borrow().is_none() {
            *self.0.borrow_mut() = Some(Box::default());
        }
    }

    #[cfg(test)]
    fn is_allocated(&self) -> bool {
        self.0.borrow().is_some()
    }
}

impl FunctionAuxiliaryState {
    fn new(home_object: Option<Value>, super_constructor: Option<Value>) -> Self {
        let cold = (home_object.is_some() || super_constructor.is_some())
            .then(|| Box::new(FunctionColdState::new(home_object, super_constructor)));
        Self {
            implicit_length_property: Cell::new(false),
            implicit_name_property: Cell::new(false),
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

impl FunctionColdState {
    fn new(home_object: Option<Value>, super_constructor: Option<Value>) -> Self {
        Self {
            home_object,
            super_constructor,
            instance_elements: None,
            property_order: Vec::new(),
            symbol_properties: Vec::new(),
            internal_prototype: None,
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
    pub(crate) immutable_name_binding: bool,
    pub(crate) immutable_env_binding: Option<String>,
    pub(crate) immutable_env_value: Option<Upvalue>,
    pub(crate) params: Rc<FunctionParams>,
    pub(crate) realm: Realm,
    pub(crate) module_host: Option<ModuleHostRef>,
    pub(crate) module_imports: ModuleImports,
    pub(crate) bytecode: Rc<Bytecode>,
    pub(crate) source_text: Option<Rc<str>>,
    pub(crate) local_names: Rc<Vec<String>>,
    pub(crate) constructable: bool,
    pub(crate) is_strict: bool,
    pub(crate) lexical_this: bool,
    pub(crate) lexical_arguments: bool,
    pub(crate) lexical_new_target: Option<Upvalue>,
    pub(crate) is_generator: bool,
    pub(crate) is_async: bool,
    pub(crate) is_class_constructor: bool,
    pub(crate) is_derived_constructor: bool,
    pub(crate) is_field_initializer: bool,
    pub(crate) home_object: Option<Value>,
    pub(crate) super_constructor: Option<Value>,
    pub(crate) deopt_bindings: Option<DynamicBindings>,
    pub(crate) with_stack: Vec<Value>,
    pub(crate) upvalues: Vec<Upvalue>,
}

#[derive(Clone, Copy)]
struct LexicalBindings {
    this: bool,
    arguments: bool,
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
    /// Inserts a binding into the function's creation environment. Used to seed
    /// a freshly created native reaction's captured state; `Rc::make_mut` is
    /// cheap here because the `Rc` is uniquely held immediately after creation.
    pub(crate) fn insert_native_context(&mut self, key: String, value: Value) {
        let data = Rc::get_mut(&mut self.0)
            .expect("native context must be populated before sharing the function");
        data.native_context.insert(key, value);
    }

    /// Adds an indexed capture while constructing a fresh internal native
    /// function. Keeping this mutation behind the handle prevents general
    /// copy-on-write detachment of JavaScript function identity.
    pub(crate) fn push_upvalue(&mut self, upvalue: Upvalue) {
        Rc::get_mut(&mut self.0)
            .expect("upvalues must be populated before sharing the function")
            .upvalues
            .push(upvalue);
    }

    pub(crate) fn new_user(
        name: Option<String>,
        params: FunctionParams,
        body: Vec<Stmt>,
        env: HashMap<String, Value>,
    ) -> Result<Self, crate::RuntimeError> {
        Self::new_user_with_constructable(name, params, body, env, true)
    }

    pub(crate) fn new_user_with_constructable(
        name: Option<String>,
        params: FunctionParams,
        body: Vec<Stmt>,
        env: HashMap<String, Value>,
        constructable: bool,
    ) -> Result<Self, crate::RuntimeError> {
        Self::new_user_with_bytecode(name, params, body, env, None, constructable)
    }

    pub(crate) fn new_user_with_bytecode(
        name: Option<String>,
        params: FunctionParams,
        body: Vec<Stmt>,
        env: HashMap<String, Value>,
        bytecode: Option<Rc<Bytecode>>,
        constructable: bool,
    ) -> Result<Self, crate::RuntimeError> {
        Self::new_user_with_bytecode_and_lexical_this(
            name,
            params,
            body,
            env,
            bytecode,
            constructable,
            LexicalBindings {
                this: false,
                arguments: false,
            },
        )
    }

    fn new_user_with_bytecode_and_lexical_this(
        name: Option<String>,
        params: FunctionParams,
        body: Vec<Stmt>,
        env: HashMap<String, Value>,
        bytecode: Option<Rc<Bytecode>>,
        constructable: bool,
        lexical_bindings: LexicalBindings,
    ) -> Result<Self, crate::RuntimeError> {
        let realm = super::env::new_realm(env);
        let dynamic_function_realm_global = dynamic_function_realm_global(&realm);
        let prototype = ObjectRef::with_prototype(
            HashMap::new(),
            object_prototype(&crate::CallEnv::new(Rc::clone(&realm))),
        );
        let local_names = collect_function_local_names(
            name.as_ref(),
            &params,
            &body,
            !lexical_bindings.arguments,
        );
        let is_strict = is_strict_function_body(&body);
        let bytecode = match bytecode {
            Some(bytecode) => bytecode,
            None => Rc::new(compile_function_body(&params, &body)?),
        };
        let auxiliary = FunctionAuxiliaryState::new(None, None);
        let function = Self(Rc::new(FunctionData {
            has_name_binding: name.is_some(),
            immutable_name_binding: false,
            immutable_env_binding: None,
            immutable_env_value: None,
            name,
            params: Rc::new(params),
            native_context: NativeContext::default(),
            realm: Some(realm),
            dynamic_function_realm_global,
            has_dynamic_function_realm_override: Cell::new(false),
            deopt_bindings: None,
            module_host: None,
            module_imports: Default::default(),
            with_stack: Vec::new(),
            upvalues: Vec::new(),
            realm_upvalue_slots: 0,
            local_names: Rc::new(local_names),
            bytecode: Some(bytecode),
            source_text: None,
            native: None,
            constructable,
            is_strict,
            lexical_this: lexical_bindings.this,
            lexical_arguments: lexical_bindings.arguments,
            lexical_new_target: None,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            is_field_initializer: false,
            auxiliary,
            bound: None,
            properties: LazyFunctionProperties::default(),
        }));
        function.define_length_property();
        function.define_name_property();
        if constructable {
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
        Ok(function)
    }

    pub(crate) fn new_user_compiled(compiled: CompiledUserFunction) -> Self {
        let CompiledUserFunction {
            name,
            has_name_binding,
            immutable_name_binding,
            immutable_env_binding,
            immutable_env_value,
            params,
            realm,
            module_host,
            module_imports,
            bytecode,
            source_text,
            local_names,
            constructable,
            is_strict,
            lexical_this,
            lexical_arguments,
            lexical_new_target,
            is_generator,
            is_async,
            is_class_constructor,
            is_derived_constructor,
            is_field_initializer,
            home_object,
            super_constructor,
            deopt_bindings,
            with_stack,
            upvalues,
        } = compiled;
        let realm_upvalue_slots = bytecode
            .received_upvalue_slots()
            .iter()
            .zip(&upvalues)
            .filter_map(|(slot, cell)| {
                (*slot < u128::BITS as usize
                    && bytecode
                        .local_name_at(*slot)
                        .is_some_and(|name| realm.is_binding_cell(name, cell)))
                .then_some(1_u128 << *slot)
            })
            .fold(0, |slots, slot| slots | slot);
        let dynamic_function_realm_global = dynamic_function_realm_global(&realm);
        let auxiliary = FunctionAuxiliaryState::new(home_object, super_constructor);
        let function = Self(Rc::new(FunctionData {
            has_name_binding,
            immutable_name_binding,
            immutable_env_binding,
            immutable_env_value,
            name,
            params,
            native_context: NativeContext::default(),
            realm: Some(realm),
            dynamic_function_realm_global,
            has_dynamic_function_realm_override: Cell::new(false),
            deopt_bindings,
            module_host,
            module_imports,
            with_stack,
            upvalues,
            realm_upvalue_slots,
            local_names,
            bytecode: Some(bytecode),
            source_text,
            native: None,
            constructable,
            is_strict,
            lexical_this,
            lexical_arguments,
            lexical_new_target,
            is_generator,
            is_async,
            is_class_constructor,
            is_derived_constructor,
            is_field_initializer,
            auxiliary,
            bound: None,
            properties: LazyFunctionProperties::default(),
        }));
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
    ) -> Self {
        Self::new(
            name.map(str::to_owned),
            vec![String::new(); length],
            HashMap::new(),
            Some(native),
            constructable,
        )
    }

    pub(crate) fn uninitialized_lexical_marker() -> Self {
        Self::new_native(
            Some("\u{0}\u{0}uninitialized_lexical"),
            0,
            NativeFunction::UninitializedLexical,
            false,
        )
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
            immutable_name_binding: false,
            immutable_env_binding: None,
            immutable_env_value: None,
            params: Rc::new(FunctionParams::positional(vec![String::new(); length])),
            native_context: NativeContext::default(),
            realm: match &target {
                Value::Function(function) => function.realm.clone(),
                _ => None,
            },
            dynamic_function_realm_global: match &target {
                Value::Function(function) => function.dynamic_function_realm_global.clone(),
                _ => None,
            },
            has_dynamic_function_realm_override: Cell::new(false),
            deopt_bindings: None,
            module_host: None,
            module_imports: Default::default(),
            with_stack: Vec::new(),
            upvalues: Vec::new(),
            realm_upvalue_slots: 0,
            local_names: Rc::new(Vec::new()),
            bytecode: None,
            source_text: None,
            native: None,
            constructable,
            is_strict: false,
            lexical_this: false,
            lexical_arguments: false,
            lexical_new_target: None,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            is_field_initializer: false,
            auxiliary,
            bound: Some(Box::new(BoundFunction {
                target,
                this_value,
                arguments,
            })),
            properties: LazyFunctionProperties::default(),
        }));
        function.define_length_property();
        function.define_name_property();
        function
    }

    fn new(
        name: Option<String>,
        params: Vec<String>,
        env: HashMap<String, Value>,
        native: Option<NativeFunction>,
        constructable: bool,
    ) -> Self {
        let prototype = ObjectRef::new(HashMap::new());
        let auxiliary = FunctionAuxiliaryState::new(None, None);
        let function = Self(Rc::new(FunctionData {
            has_name_binding: false,
            immutable_name_binding: false,
            immutable_env_binding: None,
            immutable_env_value: None,
            name,
            params: Rc::new(FunctionParams::positional(params)),
            native_context: NativeContext::from_map(env),
            realm: None,
            dynamic_function_realm_global: None,
            has_dynamic_function_realm_override: Cell::new(false),
            deopt_bindings: None,
            module_host: None,
            module_imports: Default::default(),
            with_stack: Vec::new(),
            upvalues: Vec::new(),
            realm_upvalue_slots: 0,
            local_names: Rc::new(Vec::new()),
            bytecode: None,
            source_text: None,
            native,
            constructable,
            is_strict: false,
            lexical_this: false,
            lexical_arguments: false,
            lexical_new_target: None,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            is_field_initializer: false,
            auxiliary,
            bound: None,
            properties: LazyFunctionProperties::default(),
        }));
        function.define_length_property();
        function.define_name_property();
        if constructable {
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

    /// Moves the implicit standard data properties into the general table.
    /// Inserting at the front preserves the creation order (`length`, `name`,
    /// then `prototype`) even when a constructable function has already
    /// recorded its lazy prototype slot.
    fn materialize_default_data_properties(&self) {
        self.materialize_default_prototype_order();
        let had_length = self.auxiliary.implicit_length_property.replace(false);
        let had_name = self.auxiliary.implicit_name_property.replace(false);
        if !had_length && !had_name {
            return;
        }

        let mut properties = self.properties.borrow_mut();
        self.auxiliary.with_cold_mut(|cold| {
            let mut insert_at = 0;
            if had_length {
                properties
                    .entry("length".to_owned())
                    .or_insert_with(|| self.default_length_property());
                cold.property_order.insert(insert_at, "length".to_owned());
                insert_at += 1;
            }
            if had_name {
                properties
                    .entry("name".to_owned())
                    .or_insert_with(|| self.default_name_property());
                cold.property_order.insert(insert_at, "name".to_owned());
            }
        });
    }

    fn mark_lazy_default_prototype(&self) {
        self.auxiliary.lazy_default_prototype.set(true);
    }

    /// Records the ordinary function's original `prototype` property slot
    /// only when property observation or mutation needs ordering state. This
    /// keeps unobserved closures from allocating a `Vec` buffer and owned key.
    fn materialize_default_prototype_order(&self) {
        if !self.auxiliary.lazy_default_prototype.get() {
            return;
        }
        self.auxiliary.with_cold_mut(|cold| {
            if !cold.property_order.iter().any(|key| key == "prototype") {
                cold.property_order.push("prototype".to_owned());
            }
        });
    }

    fn ensure_default_prototype(&self) {
        self.materialize_default_prototype_order();
        if !self.auxiliary.lazy_default_prototype.replace(false) {
            return;
        }
        let prototype =
            ObjectRef::with_prototype(HashMap::new(), object_prototype(&self.creation_env()));
        prototype.define_non_enumerable("constructor".to_owned(), Value::Function(self.clone()));
        self.properties.borrow_mut().insert(
            "prototype".to_owned(),
            Property::data(Value::Object(prototype), false, true, false),
        );
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.auxiliary.extensible.get()
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn prevent_extensions(&self) {
        self.ensure_default_prototype();
        self.auxiliary.extensible.set(false);
    }

    pub(crate) fn seal(&self) {
        self.materialize_default_data_properties();
        self.ensure_default_prototype();
        self.prevent_extensions();
        self.auxiliary.sealed.set(true);
        for property in self.properties.borrow_mut().values_mut() {
            property.make_non_configurable();
        }
        self.auxiliary.with_cold_mut(|cold| {
            for (_, property) in &mut cold.symbol_properties {
                property.make_non_configurable();
            }
        });
    }

    pub(crate) fn is_sealed(&self) -> bool {
        self.ensure_default_prototype();
        !self.auxiliary.extensible.get()
            && self.auxiliary.sealed.get()
            && self
                .properties
                .borrow()
                .values()
                .all(|property| !property.configurable)
            && self.auxiliary.with_cold(|cold| {
                cold.is_none_or(|cold| {
                    cold.symbol_properties
                        .iter()
                        .all(|(_, property)| !property.configurable)
                })
            })
    }

    pub(crate) fn freeze(&self) {
        self.materialize_default_data_properties();
        self.ensure_default_prototype();
        self.prevent_extensions();
        self.auxiliary.sealed.set(true);
        self.auxiliary.frozen.set(true);
        for property in self.properties.borrow_mut().values_mut() {
            property.freeze_data();
        }
        self.auxiliary.with_cold_mut(|cold| {
            for (_, property) in &mut cold.symbol_properties {
                property.freeze_data();
            }
        });
    }

    pub(crate) fn is_frozen(&self) -> bool {
        self.ensure_default_prototype();
        !self.auxiliary.extensible.get()
            && self.auxiliary.sealed.get()
            && self.auxiliary.frozen.get()
            && self
                .properties
                .borrow()
                .values()
                .all(|property| !property.configurable && !property.writable)
            && self.auxiliary.with_cold(|cold| {
                cold.is_none_or(|cold| {
                    cold.symbol_properties
                        .iter()
                        .all(|(_, property)| !property.configurable && !property.writable)
                })
            })
    }

    pub(crate) fn set_property(&self, key: String, value: Value) {
        if (key == "length" && self.auxiliary.implicit_length_property.get())
            || (key == "name" && self.auxiliary.implicit_name_property.get())
        {
            // Install-time built-ins may have replaced an implicit default by
            // writing directly to the compatibility property table. Treat
            // that entry as the same original slot; otherwise the untouched
            // standard default is non-writable and needs no materialization.
            if let Some(property) = self.properties.borrow_mut().get_mut(&key) {
                if property.writable {
                    property.value = value;
                }
            }
            return;
        }
        if key == "prototype" {
            self.ensure_default_prototype();
        }
        let mut properties = self.properties.borrow_mut();
        if let Some(property) = properties.get_mut(&key) {
            if property.writable {
                property.value = value;
            }
            drop(properties);
            self.refresh_dynamic_function_realm_override(&key);
            return;
        }
        if !self.auxiliary.extensible.get() {
            return;
        }
        self.materialize_default_prototype_order();
        self.auxiliary
            .with_cold_mut(|cold| cold.property_order.push(key.clone()));
        properties.insert(key.clone(), Property::enumerable(value));
        drop(properties);
        self.refresh_dynamic_function_realm_override(&key);
    }

    pub(crate) fn define_property(&self, key: String, property: Property) {
        self.materialize_default_prototype_order();
        if (key == "length" && self.auxiliary.implicit_length_property.get())
            || (key == "name" && self.auxiliary.implicit_name_property.get())
        {
            self.materialize_default_data_properties();
        }
        if key == "prototype" {
            self.ensure_default_prototype();
        }
        let mut properties = self.properties.borrow_mut();
        if !properties.contains_key(&key) {
            self.auxiliary
                .with_cold_mut(|cold| cold.property_order.push(key.clone()));
        }
        properties.insert(key.clone(), property);
        drop(properties);
        self.refresh_dynamic_function_realm_override(&key);
    }

    fn refresh_dynamic_function_realm_override(&self, key: &str) {
        if key != DYNAMIC_FUNCTION_REALM_GLOBAL {
            return;
        }
        self.has_dynamic_function_realm_override.set(
            self.properties
                .borrow()
                .get(key)
                .is_some_and(|property| matches!(&property.value, Value::Object(_))),
        );
    }

    pub(crate) fn own_property(&self, key: &str) -> Option<Property> {
        if key == "length" && self.auxiliary.implicit_length_property.get() {
            return self
                .properties
                .borrow()
                .get(key)
                .cloned()
                .or_else(|| Some(self.default_length_property()));
        }
        if key == "name" && self.auxiliary.implicit_name_property.get() {
            return self
                .properties
                .borrow()
                .get(key)
                .cloned()
                .or_else(|| Some(self.default_name_property()));
        }
        if key == "prototype" {
            self.ensure_default_prototype();
        }
        self.properties.borrow().get(key).cloned()
    }

    pub(crate) fn own_property_keys(&self) -> Vec<String> {
        self.ordered_property_names(|property| property.enumerable)
    }

    pub(crate) fn own_property_names(&self) -> Vec<String> {
        self.ordered_property_names(|_| true)
    }

    fn ordered_property_names(&self, include: impl Fn(&Property) -> bool) -> Vec<String> {
        self.ensure_default_prototype();
        let properties = self.properties.borrow();
        let property_order = self
            .auxiliary
            .with_cold(|cold| cold.map_or_else(Vec::new, |cold| cold.property_order.clone()));
        let mut indices = Vec::new();
        let mut strings = Vec::new();
        let mut fallback_strings = Vec::new();

        let implicit_length = self.auxiliary.implicit_length_property.get();
        let implicit_name = self.auxiliary.implicit_name_property.get();
        if implicit_length {
            let property = properties
                .get("length")
                .cloned()
                .unwrap_or_else(|| self.default_length_property());
            if include(&property) {
                strings.push("length".to_owned());
            }
        }
        if implicit_name {
            let property = properties
                .get("name")
                .cloned()
                .unwrap_or_else(|| self.default_name_property());
            if include(&property) {
                strings.push("name".to_owned());
            }
        }

        for key in property_order.iter() {
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

        for (key, property) in properties.iter() {
            if (implicit_length && key == "length")
                || (implicit_name && key == "name")
                || property_order.iter().any(|ordered| ordered == key)
                || !include(property)
            {
                continue;
            }
            if let Some(index) = array_index_property_key(key) {
                indices.push((index, key.clone()));
            } else {
                fallback_strings.push(key.clone());
            }
        }

        indices.sort_by_key(|(index, _)| *index);
        fallback_strings.sort();
        strings.extend(fallback_strings);
        indices
            .into_iter()
            .map(|(_, key)| key)
            .chain(strings)
            .collect()
    }

    pub(crate) fn delete_own_property(&self, key: &str) -> bool {
        let implicit_default = (key == "length" && self.auxiliary.implicit_length_property.get())
            || (key == "name" && self.auxiliary.implicit_name_property.get());
        if implicit_default {
            let mut properties = self.properties.borrow_mut();
            if let Some(property) = properties.get(key) {
                if !property.configurable {
                    return false;
                }
                properties.remove(key);
            }
            if key == "length" {
                self.auxiliary.implicit_length_property.set(false);
            } else {
                self.auxiliary.implicit_name_property.set(false);
            }
            return true;
        }
        if key == "prototype" {
            self.ensure_default_prototype();
        }
        let mut properties = self.properties.borrow_mut();
        if properties
            .get(key)
            .is_some_and(|property| !property.configurable)
        {
            return false;
        }
        properties.remove(key);
        drop(properties);
        if key == DYNAMIC_FUNCTION_REALM_GLOBAL {
            self.has_dynamic_function_realm_override.set(false);
        }
        self.auxiliary
            .with_cold_mut(|cold| cold.property_order.retain(|existing| existing != key));
        true
    }

    /// Removes an own property regardless of its `[[Configurable]]` attribute,
    /// for install-time setup of native objects that must not expose a property
    /// the generic builder added (e.g. `Proxy` has no own `prototype`).
    pub(crate) fn remove_own_property_unchecked(&self, key: &str) {
        if key == "length" {
            self.auxiliary.implicit_length_property.set(false);
        }
        if key == "name" {
            self.auxiliary.implicit_name_property.set(false);
        }
        if key == "prototype" {
            self.ensure_default_prototype();
        }
        self.properties.borrow_mut().remove(key);
        if key == DYNAMIC_FUNCTION_REALM_GLOBAL {
            self.has_dynamic_function_realm_override.set(false);
        }
        self.auxiliary
            .with_cold_mut(|cold| cold.property_order.retain(|existing| existing != key));
    }

    pub(crate) fn symbol_property(&self, symbol: &ObjectRef, env: &CallEnv) -> Option<Property> {
        self.own_symbol_property(symbol).or_else(|| {
            match self.effective_internal_prototype_with_env(env) {
                Some(Prototype::Object(prototype)) => prototype.symbol_property(symbol),
                Some(Prototype::Function(parent)) => parent.chain_symbol_property(symbol),
                Some(Prototype::Proxy(proxy)) => proxy.target_result().ok().and_then(|target| {
                    crate::property::own_or_inherited_symbol_descriptor(target, symbol)
                }),
                None => None,
            }
        })
    }

    /// The function's resolved [[Prototype]] slot, resolving the implicit
    /// default to %Function.prototype% via the function's captured environment.
    /// Returns `None` only when the prototype is explicitly `null` or the
    /// intrinsic cannot be resolved (for example a native function with no
    /// captured globals).
    pub(crate) fn effective_internal_prototype(&self) -> Option<Prototype> {
        let env = self.creation_env();
        self.effective_internal_prototype_with_env(&env)
    }

    fn effective_internal_prototype_with_env(&self, env: &CallEnv) -> Option<Prototype> {
        match self
            .auxiliary
            .with_cold(|cold| cold.and_then(|cold| cold.internal_prototype.clone()))
        {
            Some(slot) => slot,
            None => crate::function_intrinsic_prototype_slot(env),
        }
    }

    /// Walks this function's own properties, then its [[Prototype]] chain, for a
    /// string-keyed property. Used when a function sits inside another value's
    /// prototype chain.
    pub(crate) fn chain_property(&self, key: &str) -> Option<Property> {
        let env = self.creation_env();
        self.chain_property_with_env(key, &env)
    }

    fn creation_env(&self) -> crate::CallEnv {
        self.realm.as_ref().map_or_else(
            || crate::CallEnv::from_map(self.native_context.clone_map()),
            |realm| crate::CallEnv::new(Rc::clone(realm)),
        )
    }

    pub(crate) fn chain_property_with_env(&self, key: &str, env: &CallEnv) -> Option<Property> {
        self.own_property(key)
            .or_else(|| match self.effective_internal_prototype_with_env(env) {
                Some(Prototype::Object(prototype)) => prototype.property(key),
                Some(Prototype::Function(parent)) => parent.chain_property_with_env(key, env),
                Some(Prototype::Proxy(proxy)) => proxy
                    .target_result()
                    .ok()
                    .and_then(|target| crate::property::own_or_inherited_descriptor(target, key)),
                None => None,
            })
    }

    pub(crate) fn chain_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.own_symbol_property(symbol)
            .or_else(|| match self.effective_internal_prototype() {
                Some(Prototype::Object(prototype)) => prototype.symbol_property(symbol),
                Some(Prototype::Function(parent)) => parent.chain_symbol_property(symbol),
                Some(Prototype::Proxy(proxy)) => proxy.target_result().ok().and_then(|target| {
                    crate::property::own_or_inherited_symbol_descriptor(target, symbol)
                }),
                None => None,
            })
    }

    pub(crate) fn define_symbol_property(&self, symbol: ObjectRef, property: Property) {
        self.auxiliary.with_cold_mut(|cold| {
            if let Some((_, existing)) = cold
                .symbol_properties
                .iter_mut()
                .find(|(existing_symbol, _)| existing_symbol.ptr_eq(&symbol))
            {
                *existing = property;
                return;
            }
            cold.symbol_properties.push((symbol, property));
        });
    }

    pub(crate) fn has_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.auxiliary.with_cold(|cold| {
            cold.is_some_and(|cold| {
                cold.symbol_properties
                    .iter()
                    .any(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
            })
        })
    }

    pub(crate) fn own_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.auxiliary.with_cold(|cold| {
            cold.and_then(|cold| {
                cold.symbol_properties
                    .iter()
                    .find(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
                    .map(|(_, property)| property.clone())
            })
        })
    }

    pub(crate) fn delete_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.auxiliary.with_cold_mut(|cold| {
            let Some(index) = cold
                .symbol_properties
                .iter()
                .position(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
            else {
                return true;
            };
            if !cold.symbol_properties[index].1.configurable {
                return false;
            }
            cold.symbol_properties.remove(index);
            true
        })
    }

    pub(crate) fn own_property_symbols(&self) -> Vec<ObjectRef> {
        self.auxiliary.with_cold(|cold| {
            cold.map_or_else(Vec::new, |cold| {
                cold.symbol_properties
                    .iter()
                    .map(|(symbol, _)| symbol.clone())
                    .collect()
            })
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

    /// Sets the private environment carried by a class constructor.
    pub(crate) fn set_private_environment(&self, environment: crate::private::PrivateEnvironment) {
        self.auxiliary
            .with_cold_mut(|cold| cold.private_state.environment = Some(environment));
    }

    /// Returns the private environment carried by this constructor, if any.
    pub(crate) fn private_environment(&self) -> Option<crate::private::PrivateEnvironment> {
        self.auxiliary
            .with_cold(|cold| cold.and_then(|cold| cold.private_state.environment.clone()))
    }

    /// Records an instance private element (a field initializer or a
    /// method/accessor brand) applied to each instance at construction time.
    pub(crate) fn push_instance_private_element(&self, element: InstancePrivateElement) {
        self.auxiliary.with_cold_mut(|cold| {
            Rc::make_mut(
                cold.instance_elements
                    .get_or_insert_with(|| Rc::new(Vec::new())),
            )
            .push(InstanceElementInitializer::PrivateElement(element));
        });
    }

    /// Records a public instance field applied at construction time.
    pub(crate) fn push_instance_public_field(&self, field: InstanceFieldInitializer) {
        self.auxiliary.with_cold_mut(|cold| {
            Rc::make_mut(
                cold.instance_elements
                    .get_or_insert_with(|| Rc::new(Vec::new())),
            )
            .push(InstanceElementInitializer::PublicField(field));
        });
    }

    /// Returns a shared snapshot of this constructor's instance elements.
    pub(crate) fn instance_elements(&self) -> Option<Rc<Vec<InstanceElementInitializer>>> {
        self.auxiliary
            .with_cold(|cold| cold?.instance_elements.as_ref().map(Rc::clone))
    }

    /// The explicit [[Prototype]] override as an object slot. A function-valued
    /// override collapses to `Some(None)` here; callers that must observe the
    /// function use [`Function::internal_prototype_slot`].
    pub(crate) fn internal_prototype_override(&self) -> Option<Option<ObjectRef>> {
        self.auxiliary
            .with_cold(|cold| cold.and_then(|cold| cold.internal_prototype.clone()))
            .map(|slot| slot.and_then(|prototype| prototype.as_object()))
    }

    /// The raw [[Prototype]] override slot, preserving a function prototype.
    pub(crate) fn internal_prototype_slot(&self) -> Option<Option<Prototype>> {
        self.auxiliary
            .with_cold(|cold| cold.and_then(|cold| cold.internal_prototype.clone()))
    }

    /// The function's original source text, when retained.
    pub(crate) fn source_text(&self) -> Option<Rc<str>> {
        self.source_text.clone()
    }

    pub(crate) fn set_internal_prototype_slot(
        &self,
        prototype: Option<Prototype>,
    ) -> Result<(), ()> {
        if self.auxiliary.with_cold(|cold| {
            cold.and_then(|cold| cold.internal_prototype.as_ref())
                .is_some_and(|current| same_prototype_slot(current, &prototype))
        }) {
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
        self.auxiliary
            .with_cold_mut(|cold| cold.internal_prototype = Some(prototype));
        Ok(())
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

    #[cfg(test)]
    fn property_order(&self) -> Vec<String> {
        self.auxiliary
            .with_cold(|cold| cold.map_or_else(Vec::new, |cold| cold.property_order.clone()))
    }
}

fn bound_function_name(target: &Value) -> String {
    let target_name = match target {
        Value::Function(function) => function
            .properties
            .borrow()
            .get("name")
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
    seen_functions: &mut Vec<Function>,
    seen_objects: &mut Vec<ObjectRef>,
) -> bool {
    match prototype {
        Prototype::Function(function) => {
            if function.ptr_eq(target) {
                return true;
            }
            if seen_functions.iter().any(|seen| seen.ptr_eq(function)) {
                return false;
            }
            seen_functions.push(function.clone());
            function
                .effective_internal_prototype()
                .is_some_and(|prototype| {
                    prototype_chain_contains_function_inner(
                        &prototype,
                        target,
                        seen_functions,
                        seen_objects,
                    )
                })
        }
        Prototype::Object(object) => {
            if seen_objects.iter().any(|seen| seen.ptr_eq(object)) {
                return false;
            }
            seen_objects.push(object.clone());
            object.prototype_slot().is_some_and(|prototype| {
                prototype_chain_contains_function_inner(
                    &prototype,
                    target,
                    seen_functions,
                    seen_objects,
                )
            })
        }
        Prototype::Proxy(_) => false,
    }
}

fn array_index_property_key(key: &str) -> Option<u32> {
    key.parse::<u32>()
        .ok()
        .filter(|index| *index < u32::MAX && index.to_string() == key)
}

#[cfg(test)]
mod tests {
    use std::{mem, rc::Rc};

    use super::{Function, FunctionData};
    use crate::{NativeFunction, Value, eval};

    #[test]
    fn cloning_function_reuses_the_backing_allocation() {
        let function = Function::new_native(
            Some("shared"),
            0,
            NativeFunction::UninitializedLexical,
            false,
        );
        let cloned = function.clone();

        assert!(Rc::ptr_eq(&function.0, &cloned.0));
        assert!(function.ptr_eq(&cloned));
        assert_eq!(
            mem::size_of::<Function>(),
            mem::size_of::<Rc<FunctionData>>()
        );
        assert!(function.auxiliary.cold.borrow().is_none());
    }

    #[test]
    fn ordinary_function_header_keeps_cold_identity_state_out_of_line() {
        let auxiliary_size = mem::size_of::<super::FunctionAuxiliaryState>();
        let function_data_size = mem::size_of::<FunctionData>();
        assert_eq!(
            mem::size_of::<super::NativeContext>(),
            mem::size_of::<Rc<()>>(),
            "lazy native context must remain pointer-sized"
        );
        assert_eq!(
            mem::size_of::<super::ModuleImports>(),
            mem::size_of::<Rc<()>>(),
            "shared module imports must remain pointer-sized"
        );
        assert!(
            mem::size_of::<super::LazyFunctionProperties>() <= 16,
            "lazy function property header must stay within two machine words"
        );
        assert!(
            auxiliary_size <= 48,
            "function auxiliary header grew to {auxiliary_size} bytes"
        );
        assert!(
            function_data_size <= 296,
            "function object grew to {function_data_size} bytes"
        );
    }

    #[test]
    fn user_functions_keep_empty_function_maps_unallocated() {
        let value = eval("function make() { return function value() {}; } make();")
            .expect("function expression should evaluate");
        let Value::Function(function) = value else {
            panic!("expected function value");
        };
        assert!(!function.native_context.is_allocated());
        assert!(!function.properties.is_allocated());

        let mut native = Function::new_native(
            Some("capturing"),
            0,
            NativeFunction::UninitializedLexical,
            false,
        );
        assert!(!native.native_context.is_allocated());
        native.insert_native_context("captured".to_owned(), Value::Number(1.0));
        assert!(native.native_context.is_allocated());
        assert_eq!(
            native.native_context.get("captured"),
            Some(&Value::Number(1.0))
        );
        native.define_property(
            "explicit".to_owned(),
            crate::Property::enumerable(Value::Number(2.0)),
        );
        assert!(native.properties.is_allocated());
    }

    #[test]
    fn cloned_function_handles_keep_identity_state_and_source_shared() {
        let function = Function::new_native(
            Some("shared"),
            0,
            NativeFunction::UninitializedLexical,
            false,
        );
        let cloned = function.clone();
        cloned.define_property(
            "shared".to_owned(),
            crate::Property::enumerable(Value::Number(1.0)),
        );
        assert!(Rc::ptr_eq(&function.0, &cloned.0));
        assert!(function.ptr_eq(&cloned));
        assert_eq!(
            function
                .own_property("shared")
                .map(|property| property.value),
            Some(Value::Number(1.0))
        );

        let Value::Function(function) = eval("(function shared() {})")
            .expect("function expression with retained source should evaluate")
        else {
            panic!("expected function value");
        };
        let cloned = function.clone();
        assert!(Rc::ptr_eq(&function.0, &cloned.0));
        assert_eq!(
            cloned.source_text().as_deref(),
            Some("function shared() {}")
        );
    }

    #[test]
    fn class_constructions_share_instance_element_metadata() {
        let Value::Function(constructor) =
            eval("(class Point { x = 1; y = 2; })").expect("class expression should evaluate")
        else {
            panic!("expected class constructor");
        };

        let first = constructor
            .instance_elements()
            .expect("class should retain its fields");
        let second = constructor
            .instance_elements()
            .expect("class should retain its fields");
        assert_eq!(first.len(), 2);
        assert!(Rc::ptr_eq(&first, &second));

        let Value::Function(without_fields) =
            eval("(class Empty { method() {} })").expect("class expression should evaluate")
        else {
            panic!("expected class constructor");
        };
        assert!(without_fields.instance_elements().is_none());
    }

    #[test]
    fn default_name_and_length_stay_implicit_until_descriptor_mutation() {
        let function = Function::new_native(
            Some("shared"),
            2,
            NativeFunction::UninitializedLexical,
            false,
        );

        assert!(function.properties.borrow().is_empty());
        assert_eq!(
            function
                .own_property("length")
                .map(|property| property.value),
            Some(Value::Number(2.0))
        );
        assert_eq!(
            function.own_property("name").map(|property| property.value),
            Some(Value::String("shared".to_owned().into()))
        );
        assert_eq!(function.own_property_names(), ["length", "name"]);

        function.define_property(
            "name".to_owned(),
            crate::Property::data(
                Value::String("renamed".to_owned().into()),
                false,
                false,
                true,
            ),
        );
        assert_eq!(function.properties.borrow().len(), 2);
        assert_eq!(function.own_property_names(), ["length", "name"]);
    }

    #[test]
    fn deleting_and_redefining_an_implicit_default_moves_it_to_the_end() {
        assert_eq!(
            eval(
                "var f = function value(a) {}; delete f.name; Object.defineProperty(f, 'name', { value: 'again', configurable: true }); Object.getOwnPropertyNames(f).join('|');"
            ),
            Ok(Value::String("length|prototype|name".to_owned().into()))
        );
    }

    #[test]
    fn install_time_override_and_implicit_default_are_one_property_slot() {
        let function = Function::new_native(
            Some("initial"),
            0,
            NativeFunction::UninitializedLexical,
            false,
        );
        function.properties.borrow_mut().insert(
            "name".to_owned(),
            crate::Property::data(
                Value::String("installed".to_owned().into()),
                false,
                false,
                true,
            ),
        );

        assert_eq!(
            function.own_property("name").map(|property| property.value),
            Some(Value::String("installed".to_owned().into()))
        );
        assert_eq!(
            function
                .own_property_names()
                .into_iter()
                .filter(|key| key == "name")
                .count(),
            1
        );
        assert!(function.delete_own_property("name"));
        assert!(function.own_property("name").is_none());
    }

    #[test]
    fn compiled_function_materializes_default_prototype_on_observation() {
        assert_eq!(
            eval(
                "var f = function (value) { return value; }; f.prototype.constructor === f && Object.getOwnPropertyNames(f).includes('prototype') && !delete f.prototype;"
            ),
            Ok(Value::Boolean(true))
        );
    }

    #[test]
    fn compiled_function_keeps_default_prototype_order_implicit_until_observation() {
        let function = eval("function make() { return function value() {}; } make();")
            .expect("function expression should evaluate");
        let Value::Function(function) = function else {
            panic!("expected function value");
        };

        assert!(function.auxiliary.lazy_default_prototype.get());
        assert!(function.property_order().is_empty());
        assert_eq!(
            function.own_property_names(),
            ["length", "name", "prototype"]
        );
        assert_eq!(function.property_order(), ["prototype"]);
    }

    #[test]
    fn later_properties_follow_an_unobserved_default_prototype_slot() {
        assert_eq!(
            eval(
                "var f = function value() {}; f.assigned = 1; Object.defineProperty(f, 'defined', { value: 2, configurable: true }); Object.getOwnPropertyNames(f).join('|');"
            ),
            Ok(Value::String(
                "length|name|prototype|assigned|defined".to_owned().into()
            ))
        );
    }
}
