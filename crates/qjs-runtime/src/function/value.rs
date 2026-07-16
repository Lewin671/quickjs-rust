use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    collections::HashMap,
    fmt,
    ops::{Deref, DerefMut},
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
    match realm.borrow().get(DYNAMIC_FUNCTION_REALM_GLOBAL) {
        Some(Value::Object(global)) => Some(global.clone()),
        _ => None,
    }
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
/// cloning the function's vectors and maps. Mutable construction still uses
/// `DerefMut`; the existing shared object-property cells preserve identity when
/// a freshly built function has already installed its prototype back-reference.
#[derive(Clone)]
pub struct Function(Rc<FunctionData>);

/// Storage behind [`Function`]. Public only because it is the target of the
/// handle's public `Deref` implementation; the runtime does not re-export it.
#[doc(hidden)]
#[derive(Clone)]
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
    pub(crate) native_context: Rc<HashMap<String, Value>>,
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
    pub(crate) local_names: Rc<Vec<String>>,
    pub(crate) bytecode: Option<Rc<Bytecode>>,
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
    pub(crate) auxiliary: Rc<FunctionAuxiliaryState>,
    pub(crate) bound: Option<Box<BoundFunction>>,
    /// Function object properties.
    pub(crate) properties: FunctionProperties,
}

/// Identity-bearing mutable state that is cold for ordinary calls. Keeping
/// these cells behind one shared allocation preserves function identity across
/// cloned handles without paying for ten independent `Rc` allocations for
/// every closure.
#[doc(hidden)]
pub struct FunctionAuxiliaryState {
    properties: RefCell<HashMap<String, Property>>,
    pub(crate) home_object: RefCell<Option<Value>>,
    /// For a derived constructor, the parent constructor invoked by `super()`.
    pub(crate) super_constructor: RefCell<Option<Value>>,
    /// For a class constructor, the instance-field initializers run when a new
    /// instance is constructed (base class: at construction start; derived
    /// class: immediately after `super()` returns).
    instance_elements: RefCell<Vec<InstanceElementInitializer>>,
    property_order: RefCell<Vec<String>>,
    symbol_properties: RefCell<Vec<(ObjectRef, Property)>>,
    extensible: Cell<bool>,
    sealed: Cell<bool>,
    frozen: Cell<bool>,
    /// Explicit [[Prototype]] override. `None` means "use the default
    /// %Function.prototype% intrinsic"; `Some(None)` means it is null.
    internal_prototype: RefCell<Option<Option<Prototype>>>,
    private_state: RefCell<crate::private::PrivateState>,
    source_text: RefCell<Option<Rc<str>>>,
    lazy_default_prototype: Cell<bool>,
}

impl FunctionAuxiliaryState {
    fn new(home_object: Option<Value>, super_constructor: Option<Value>) -> Rc<Self> {
        Rc::new(Self {
            properties: RefCell::new(HashMap::new()),
            home_object: RefCell::new(home_object),
            super_constructor: RefCell::new(super_constructor),
            instance_elements: RefCell::new(Vec::new()),
            property_order: RefCell::new(Vec::new()),
            symbol_properties: RefCell::new(Vec::new()),
            extensible: Cell::new(true),
            sealed: Cell::new(false),
            frozen: Cell::new(false),
            internal_prototype: RefCell::new(None),
            private_state: RefCell::new(crate::private::PrivateState::default()),
            source_text: RefCell::new(None),
            lazy_default_prototype: Cell::new(false),
        })
    }
}

/// Compatibility handle for the function's identity-bearing property table.
/// It shares the existing auxiliary allocation instead of allocating a second
/// `Rc<RefCell<_>>` for every function object.
#[derive(Clone)]
pub(crate) struct FunctionProperties(Rc<FunctionAuxiliaryState>);

impl FunctionProperties {
    fn new(auxiliary: &Rc<FunctionAuxiliaryState>) -> Self {
        Self(Rc::clone(auxiliary))
    }

    pub(crate) fn borrow(&self) -> Ref<'_, HashMap<String, Property>> {
        self.0.properties.borrow()
    }

    pub(crate) fn borrow_mut(&self) -> RefMut<'_, HashMap<String, Property>> {
        self.0.properties.borrow_mut()
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for Function {
    type Target = FunctionData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Function {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Rc::make_mut(&mut self.0)
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
    pub(crate) params: Rc<FunctionParams>,
    pub(crate) realm: Realm,
    pub(crate) module_host: Option<ModuleHostRef>,
    pub(crate) module_imports: ModuleImports,
    pub(crate) bytecode: Rc<Bytecode>,
    pub(crate) local_names: Rc<Vec<String>>,
    pub(crate) constructable: bool,
    pub(crate) is_strict: bool,
    pub(crate) lexical_this: bool,
    pub(crate) lexical_arguments: bool,
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
        Rc::make_mut(&mut self.native_context).insert(key, value);
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
        let properties = FunctionProperties::new(&auxiliary);
        let function = Self(Rc::new(FunctionData {
            has_name_binding: name.is_some(),
            immutable_name_binding: false,
            immutable_env_binding: None,
            immutable_env_value: None,
            name,
            params: Rc::new(params),
            native_context: Rc::new(HashMap::new()),
            realm: Some(realm),
            dynamic_function_realm_global,
            has_dynamic_function_realm_override: Cell::new(false),
            deopt_bindings: None,
            module_host: None,
            module_imports: HashMap::new(),
            with_stack: Vec::new(),
            upvalues: Vec::new(),
            local_names: Rc::new(local_names),
            bytecode: Some(bytecode),
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
            properties,
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
            params,
            realm,
            module_host,
            module_imports,
            bytecode,
            local_names,
            constructable,
            is_strict,
            lexical_this,
            lexical_arguments,
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
        let dynamic_function_realm_global = dynamic_function_realm_global(&realm);
        let auxiliary = FunctionAuxiliaryState::new(home_object, super_constructor);
        let properties = FunctionProperties::new(&auxiliary);
        let function = Self(Rc::new(FunctionData {
            has_name_binding,
            immutable_name_binding,
            immutable_env_binding,
            immutable_env_value: None,
            name,
            params,
            native_context: Rc::new(HashMap::new()),
            realm: Some(realm),
            dynamic_function_realm_global,
            has_dynamic_function_realm_override: Cell::new(false),
            deopt_bindings,
            module_host,
            module_imports,
            with_stack,
            upvalues,
            local_names,
            bytecode: Some(bytecode),
            native: None,
            constructable,
            is_strict,
            lexical_this,
            lexical_arguments,
            lexical_new_target: None,
            is_generator,
            is_async,
            is_class_constructor,
            is_derived_constructor,
            is_field_initializer,
            auxiliary,
            bound: None,
            properties,
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
        let properties = FunctionProperties::new(&auxiliary);
        let function = Self(Rc::new(FunctionData {
            name: Some(name),
            has_name_binding: false,
            immutable_name_binding: false,
            immutable_env_binding: None,
            immutable_env_value: None,
            params: Rc::new(FunctionParams::positional(vec![String::new(); length])),
            native_context: Rc::new(HashMap::new()),
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
            module_imports: HashMap::new(),
            with_stack: Vec::new(),
            upvalues: Vec::new(),
            local_names: Rc::new(Vec::new()),
            bytecode: None,
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
            properties,
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
        let properties = FunctionProperties::new(&auxiliary);
        let function = Self(Rc::new(FunctionData {
            has_name_binding: false,
            immutable_name_binding: false,
            immutable_env_binding: None,
            immutable_env_value: None,
            name,
            params: Rc::new(FunctionParams::positional(params)),
            native_context: Rc::new(env),
            realm: None,
            dynamic_function_realm_global: None,
            has_dynamic_function_realm_override: Cell::new(false),
            deopt_bindings: None,
            module_host: None,
            module_imports: HashMap::new(),
            with_stack: Vec::new(),
            upvalues: Vec::new(),
            local_names: Rc::new(Vec::new()),
            bytecode: None,
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
            properties,
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
        self.define_property(
            "length".to_owned(),
            Property::data(
                Value::Number(self.params.length() as f64),
                false,
                false,
                true,
            ),
        );
    }

    fn define_name_property(&self) {
        self.define_property(
            "name".to_owned(),
            Property::data(
                Value::String(self.name.clone().unwrap_or_default().into()),
                false,
                false,
                true,
            ),
        );
    }

    fn mark_lazy_default_prototype(&self) {
        self.auxiliary.lazy_default_prototype.set(true);
        self.auxiliary
            .property_order
            .borrow_mut()
            .push("prototype".to_owned());
    }

    fn ensure_default_prototype(&self) {
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
        self.properties.ptr_eq(&other.properties)
    }

    pub(crate) fn prevent_extensions(&self) {
        self.ensure_default_prototype();
        self.auxiliary.extensible.set(false);
    }

    pub(crate) fn seal(&self) {
        self.ensure_default_prototype();
        self.prevent_extensions();
        self.auxiliary.sealed.set(true);
        for property in self.properties.borrow_mut().values_mut() {
            property.make_non_configurable();
        }
        for (_, property) in self.auxiliary.symbol_properties.borrow_mut().iter_mut() {
            property.make_non_configurable();
        }
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
            && self
                .auxiliary
                .symbol_properties
                .borrow()
                .iter()
                .all(|(_, property)| !property.configurable)
    }

    pub(crate) fn freeze(&self) {
        self.ensure_default_prototype();
        self.prevent_extensions();
        self.auxiliary.sealed.set(true);
        self.auxiliary.frozen.set(true);
        for property in self.properties.borrow_mut().values_mut() {
            property.freeze_data();
        }
        for (_, property) in self.auxiliary.symbol_properties.borrow_mut().iter_mut() {
            property.freeze_data();
        }
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
            && self
                .auxiliary
                .symbol_properties
                .borrow()
                .iter()
                .all(|(_, property)| !property.configurable && !property.writable)
    }

    pub(crate) fn set_property(&self, key: String, value: Value) {
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
        self.auxiliary.property_order.borrow_mut().push(key.clone());
        properties.insert(key.clone(), Property::enumerable(value));
        drop(properties);
        self.refresh_dynamic_function_realm_override(&key);
    }

    pub(crate) fn define_property(&self, key: String, property: Property) {
        if key == "prototype" {
            self.ensure_default_prototype();
        }
        let mut properties = self.properties.borrow_mut();
        if !properties.contains_key(&key) {
            self.auxiliary.property_order.borrow_mut().push(key.clone());
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
        let property_order = self.auxiliary.property_order.borrow().clone();
        let mut indices = Vec::new();
        let mut strings = Vec::new();
        let mut fallback_strings = Vec::new();

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
            if property_order.iter().any(|ordered| ordered == key) || !include(property) {
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
            .property_order
            .borrow_mut()
            .retain(|existing| existing != key);
        true
    }

    /// Removes an own property regardless of its `[[Configurable]]` attribute,
    /// for install-time setup of native objects that must not expose a property
    /// the generic builder added (e.g. `Proxy` has no own `prototype`).
    pub(crate) fn remove_own_property_unchecked(&self, key: &str) {
        if key == "prototype" {
            self.ensure_default_prototype();
        }
        self.properties.borrow_mut().remove(key);
        if key == DYNAMIC_FUNCTION_REALM_GLOBAL {
            self.has_dynamic_function_realm_override.set(false);
        }
        self.auxiliary
            .property_order
            .borrow_mut()
            .retain(|existing| existing != key);
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
        match self.auxiliary.internal_prototype.borrow().clone() {
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
            || crate::CallEnv::from_map((*self.native_context).clone()),
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
        let mut properties = self.auxiliary.symbol_properties.borrow_mut();
        if let Some((_, existing)) = properties
            .iter_mut()
            .find(|(existing_symbol, _)| existing_symbol.ptr_eq(&symbol))
        {
            *existing = property;
            return;
        }
        properties.push((symbol, property));
    }

    pub(crate) fn has_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.auxiliary
            .symbol_properties
            .borrow()
            .iter()
            .any(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
    }

    pub(crate) fn own_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.auxiliary
            .symbol_properties
            .borrow()
            .iter()
            .find(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
            .map(|(_, property)| property.clone())
    }

    pub(crate) fn delete_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        let mut properties = self.auxiliary.symbol_properties.borrow_mut();
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
        self.auxiliary
            .symbol_properties
            .borrow()
            .iter()
            .map(|(symbol, _)| symbol.clone())
            .collect()
    }

    /// Returns the function's private-name storage, creating it on first use.
    pub(crate) fn private_storage(&self) -> crate::private::PrivateStorage {
        self.auxiliary
            .private_state
            .borrow_mut()
            .storage
            .get_or_insert_with(crate::private::PrivateStorage::new)
            .clone()
    }

    /// Sets the private environment carried by a class constructor.
    pub(crate) fn set_private_environment(&self, environment: crate::private::PrivateEnvironment) {
        self.auxiliary.private_state.borrow_mut().environment = Some(environment);
    }

    /// Returns the private environment carried by this constructor, if any.
    pub(crate) fn private_environment(&self) -> Option<crate::private::PrivateEnvironment> {
        self.auxiliary.private_state.borrow().environment.clone()
    }

    /// Records an instance private element (a field initializer or a
    /// method/accessor brand) applied to each instance at construction time.
    pub(crate) fn push_instance_private_element(&self, element: InstancePrivateElement) {
        self.auxiliary
            .instance_elements
            .borrow_mut()
            .push(InstanceElementInitializer::PrivateElement(element));
    }

    /// Records a public instance field applied at construction time.
    pub(crate) fn push_instance_public_field(&self, field: InstanceFieldInitializer) {
        self.auxiliary
            .instance_elements
            .borrow_mut()
            .push(InstanceElementInitializer::PublicField(field));
    }

    /// Returns a snapshot of this constructor's instance elements.
    pub(crate) fn instance_elements(&self) -> Vec<InstanceElementInitializer> {
        self.auxiliary.instance_elements.borrow().clone()
    }

    /// The explicit [[Prototype]] override as an object slot. A function-valued
    /// override collapses to `Some(None)` here; callers that must observe the
    /// function use [`Function::internal_prototype_slot`].
    pub(crate) fn internal_prototype_override(&self) -> Option<Option<ObjectRef>> {
        self.auxiliary
            .internal_prototype
            .borrow()
            .clone()
            .map(|slot| slot.and_then(|prototype| prototype.as_object()))
    }

    /// The raw [[Prototype]] override slot, preserving a function prototype.
    pub(crate) fn internal_prototype_slot(&self) -> Option<Option<Prototype>> {
        self.auxiliary.internal_prototype.borrow().clone()
    }

    /// Records the function's original source text for `Function.prototype
    /// .toString`.
    pub(crate) fn set_source_text(&self, source: Option<Rc<str>>) {
        *self.auxiliary.source_text.borrow_mut() = source;
    }

    /// The function's original source text, when retained.
    pub(crate) fn source_text(&self) -> Option<Rc<str>> {
        self.auxiliary.source_text.borrow().clone()
    }

    pub(crate) fn set_internal_prototype_slot(
        &self,
        prototype: Option<Prototype>,
    ) -> Result<(), ()> {
        if matches!(
            self.auxiliary.internal_prototype.borrow().as_ref(),
            Some(current) if same_prototype_slot(current, &prototype)
        ) {
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
        *self.auxiliary.internal_prototype.borrow_mut() = Some(prototype);
        Ok(())
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
    }

    #[test]
    fn detached_function_data_keeps_identity_state_shared() {
        let function = Function::new_native(
            Some("shared"),
            0,
            NativeFunction::UninitializedLexical,
            false,
        );
        let mut detached = function.clone();
        Rc::make_mut(&mut detached.0).name = Some("detached handle".to_owned());

        assert!(!Rc::ptr_eq(&function.0, &detached.0));
        assert!(Rc::ptr_eq(&function.auxiliary, &detached.auxiliary));
        detached.define_property(
            "shared".to_owned(),
            crate::Property::enumerable(Value::Number(1.0)),
        );
        assert!(function.ptr_eq(&detached));
        assert_eq!(
            function
                .own_property("shared")
                .map(|property| property.value),
            Some(Value::Number(1.0))
        );
        detached.set_source_text(Some(Rc::from("function shared() {}")));
        assert_eq!(
            function.source_text().as_deref(),
            Some("function shared() {}")
        );
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
}
