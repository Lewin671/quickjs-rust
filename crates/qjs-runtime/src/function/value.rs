use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
};

use qjs_ast::{FunctionParams, Stmt};

use crate::CallEnv;
use crate::{
    Bytecode, NativeFunction, ObjectRef, Property, PropertyKey, Prototype, Value,
    bytecode::{CaptureWriteback, compile_function_body},
    function::{collect_function_local_names, is_strict_function_body},
    function_intrinsic_prototype, object_prototype,
};

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

/// A private element applied to each instance at construction time. Methods and
/// accessors only brand the instance (the function is shared); a field both
/// brands and installs a per-instance value.
#[derive(Clone)]
pub(crate) struct InstancePrivateElement {
    /// The private-name identity to brand/install on the instance.
    pub(crate) id: crate::private::PrivateName,
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

/// User-defined or native function value.
#[derive(Clone)]
pub struct Function {
    /// Optional internal function name.
    pub name: Option<String>,
    /// Parameter names.
    pub params: FunctionParams,
    /// Environment captured when the function was created.
    pub env: HashMap<String, Value>,
    pub(crate) captured_env: Rc<RefCell<HashMap<String, Value>>>,
    pub(crate) capture_writeback: Option<CaptureWriteback>,
    pub(crate) local_names: Vec<String>,
    pub(crate) bytecode: Option<Rc<Bytecode>>,
    pub(crate) native: Option<NativeFunction>,
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
    /// The method/constructor [[HomeObject]] used to resolve `super.x`. For an
    /// instance method this is the class prototype; for a static method it is
    /// the constructor; for a derived constructor it is the prototype.
    pub(crate) home_object: Rc<RefCell<Option<Value>>>,
    /// For a derived constructor, the parent constructor invoked by `super()`.
    pub(crate) super_constructor: Rc<RefCell<Option<Value>>>,
    /// For a class constructor, the instance-field initializers run when a new
    /// instance is constructed (base class: at construction start; derived
    /// class: immediately after `super()` returns).
    pub(crate) instance_fields: Rc<RefCell<Vec<InstanceFieldInitializer>>>,
    pub(crate) bound: Option<Box<BoundFunction>>,
    /// Function object properties.
    pub(crate) properties: Rc<RefCell<HashMap<String, Property>>>,
    property_order: Rc<RefCell<Vec<String>>>,
    symbol_properties: Rc<RefCell<Vec<(ObjectRef, Property)>>>,
    extensible: Rc<Cell<bool>>,
    sealed: Rc<Cell<bool>>,
    frozen: Rc<Cell<bool>>,
    /// Explicit [[Prototype]] override. `None` means "use the default
    /// %Function.prototype% intrinsic"; `Some(None)` means the prototype was
    /// set to `null`; `Some(Some(p))` means it points at an object or another
    /// function (for example a subclass constructor whose [[Prototype]] is its
    /// superclass).
    internal_prototype: Rc<RefCell<Option<Option<Prototype>>>>,
    /// Private-name state: per-function storage (static fields and brands on the
    /// constructor) and the private environment a class constructor carries.
    /// Lazily populated; combined behind one allocation to keep `Function`
    /// small.
    private_state: Rc<RefCell<crate::private::PrivateState>>,
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
    pub(crate) params: FunctionParams,
    pub(crate) env: HashMap<String, Value>,
    pub(crate) bytecode: Rc<Bytecode>,
    pub(crate) local_names: Vec<String>,
    pub(crate) constructable: bool,
    pub(crate) is_strict: bool,
    pub(crate) lexical_this: bool,
    pub(crate) lexical_arguments: bool,
    pub(crate) is_generator: bool,
    pub(crate) is_async: bool,
    pub(crate) is_class_constructor: bool,
    pub(crate) is_derived_constructor: bool,
    pub(crate) home_object: Option<Value>,
    pub(crate) super_constructor: Option<Value>,
    pub(crate) captured_env: Rc<RefCell<HashMap<String, Value>>>,
    pub(crate) capture_writeback: Option<CaptureWriteback>,
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
        let prototype = ObjectRef::with_prototype(
            HashMap::new(),
            object_prototype(&crate::CallEnv::from_map(env.clone())),
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
        let captured_env = Rc::new(RefCell::new(env.clone()));
        let function = Self {
            name,
            params,
            env,
            captured_env,
            capture_writeback: None,
            local_names,
            bytecode: Some(bytecode),
            native: None,
            constructable,
            is_strict,
            lexical_this: lexical_bindings.this,
            lexical_arguments: lexical_bindings.arguments,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            home_object: Rc::new(RefCell::new(None)),
            super_constructor: Rc::new(RefCell::new(None)),
            instance_fields: Rc::new(RefCell::new(Vec::new())),
            bound: None,
            properties: Rc::new(RefCell::new(HashMap::new())),
            property_order: Rc::new(RefCell::new(Vec::new())),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
            private_state: Rc::new(RefCell::new(crate::private::PrivateState::default())),
        };
        function.define_length_property();
        function.define_name_property();
        if constructable {
            prototype
                .define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
            function.define_property(
                "prototype".to_owned(),
                Property::non_enumerable(Value::Object(prototype)),
            );
        }
        Ok(function)
    }

    pub(crate) fn new_user_compiled(compiled: CompiledUserFunction) -> Self {
        let CompiledUserFunction {
            name,
            params,
            env,
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
            home_object,
            super_constructor,
            captured_env,
            capture_writeback,
        } = compiled;
        let prototype = ObjectRef::with_prototype(
            HashMap::new(),
            object_prototype(&crate::CallEnv::from_map(env.clone())),
        );
        let function = Self {
            name,
            params,
            env,
            captured_env,
            capture_writeback,
            local_names,
            bytecode: Some(bytecode),
            native: None,
            constructable,
            is_strict,
            lexical_this,
            lexical_arguments,
            is_generator,
            is_async,
            is_class_constructor,
            is_derived_constructor,
            home_object: Rc::new(RefCell::new(home_object)),
            super_constructor: Rc::new(RefCell::new(super_constructor)),
            instance_fields: Rc::new(RefCell::new(Vec::new())),
            bound: None,
            properties: Rc::new(RefCell::new(HashMap::new())),
            property_order: Rc::new(RefCell::new(Vec::new())),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
            private_state: Rc::new(RefCell::new(crate::private::PrivateState::default())),
        };
        function.define_length_property();
        function.define_name_property();
        // Class constructors receive their `prototype` wiring from the class
        // builder so the property attributes and prototype object can match the
        // class semantics; ordinary functions get the default prototype here.
        if constructable && !is_class_constructor {
            prototype
                .define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
            function.define_property(
                "prototype".to_owned(),
                Property::non_enumerable(Value::Object(prototype)),
            );
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
        let function = Self {
            name: Some(name),
            params: FunctionParams::positional(vec![String::new(); length]),
            env: HashMap::new(),
            captured_env: Rc::new(RefCell::new(HashMap::new())),
            capture_writeback: None,
            local_names: Vec::new(),
            bytecode: None,
            native: None,
            constructable,
            is_strict: false,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            home_object: Rc::new(RefCell::new(None)),
            super_constructor: Rc::new(RefCell::new(None)),
            instance_fields: Rc::new(RefCell::new(Vec::new())),
            bound: Some(Box::new(BoundFunction {
                target,
                this_value,
                arguments,
            })),
            properties: Rc::new(RefCell::new(HashMap::new())),
            property_order: Rc::new(RefCell::new(Vec::new())),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
            private_state: Rc::new(RefCell::new(crate::private::PrivateState::default())),
        };
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
        let captured_env = Rc::new(RefCell::new(env.clone()));
        let function = Self {
            name,
            params: FunctionParams::positional(params),
            env,
            captured_env,
            capture_writeback: None,
            local_names: Vec::new(),
            bytecode: None,
            native,
            constructable,
            is_strict: false,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: false,
            is_async: false,
            is_class_constructor: false,
            is_derived_constructor: false,
            home_object: Rc::new(RefCell::new(None)),
            super_constructor: Rc::new(RefCell::new(None)),
            instance_fields: Rc::new(RefCell::new(Vec::new())),
            bound: None,
            properties: Rc::new(RefCell::new(HashMap::new())),
            property_order: Rc::new(RefCell::new(Vec::new())),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
            private_state: Rc::new(RefCell::new(crate::private::PrivateState::default())),
        };
        function.define_length_property();
        function.define_name_property();
        if constructable {
            prototype
                .define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
            function.define_property(
                "prototype".to_owned(),
                Property::non_enumerable(Value::Object(prototype)),
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
                Value::String(self.name.clone().unwrap_or_default()),
                false,
                false,
                true,
            ),
        );
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.extensible.get()
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.properties, &other.properties)
    }

    pub(crate) fn prevent_extensions(&self) {
        self.extensible.set(false);
    }

    pub(crate) fn seal(&self) {
        self.prevent_extensions();
        self.sealed.set(true);
        for property in self.properties.borrow_mut().values_mut() {
            property.make_non_configurable();
        }
        for (_, property) in self.symbol_properties.borrow_mut().iter_mut() {
            property.make_non_configurable();
        }
    }

    pub(crate) fn is_sealed(&self) -> bool {
        !self.extensible.get()
            && self.sealed.get()
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
        self.sealed.set(true);
        self.frozen.set(true);
        for property in self.properties.borrow_mut().values_mut() {
            property.freeze_data();
        }
        for (_, property) in self.symbol_properties.borrow_mut().iter_mut() {
            property.freeze_data();
        }
    }

    pub(crate) fn is_frozen(&self) -> bool {
        !self.extensible.get()
            && self.sealed.get()
            && self.frozen.get()
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

    pub(crate) fn set_property(&self, key: String, value: Value) {
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
        self.property_order.borrow_mut().push(key.clone());
        properties.insert(key, Property::enumerable(value));
    }

    pub(crate) fn define_property(&self, key: String, property: Property) {
        let mut properties = self.properties.borrow_mut();
        if !properties.contains_key(&key) {
            self.property_order.borrow_mut().push(key.clone());
        }
        properties.insert(key, property);
    }

    pub(crate) fn own_property(&self, key: &str) -> Option<Property> {
        self.properties.borrow().get(key).cloned()
    }

    pub(crate) fn own_property_keys(&self) -> Vec<String> {
        self.ordered_property_names(|property| property.enumerable)
    }

    pub(crate) fn own_property_names(&self) -> Vec<String> {
        self.ordered_property_names(|_| true)
    }

    fn ordered_property_names(&self, include: impl Fn(&Property) -> bool) -> Vec<String> {
        let properties = self.properties.borrow();
        let property_order = self.property_order.borrow().clone();
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
        let mut properties = self.properties.borrow_mut();
        if properties
            .get(key)
            .is_some_and(|property| !property.configurable)
        {
            return false;
        }
        properties.remove(key);
        self.property_order
            .borrow_mut()
            .retain(|existing| existing != key);
        true
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
    fn effective_internal_prototype(&self) -> Option<Prototype> {
        self.effective_internal_prototype_with_env(&crate::CallEnv::from_map(self.env.clone()))
    }

    fn effective_internal_prototype_with_env(&self, env: &CallEnv) -> Option<Prototype> {
        match self.internal_prototype.borrow().clone() {
            Some(slot) => slot,
            None => function_intrinsic_prototype(env).map(Prototype::Object),
        }
    }

    /// Walks this function's own properties, then its [[Prototype]] chain, for a
    /// string-keyed property. Used when a function sits inside another value's
    /// prototype chain.
    pub(crate) fn chain_property(&self, key: &str) -> Option<Property> {
        self.own_property(key)
            .or_else(|| match self.effective_internal_prototype() {
                Some(Prototype::Object(prototype)) => prototype.property(key),
                Some(Prototype::Function(parent)) => parent.chain_property(key),
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

    /// Whether the object `target` appears in this function's [[Prototype]]
    /// chain (used by `isPrototypeOf`/`instanceof` when a function is mid-chain).
    pub(crate) fn chain_contains_object(&self, target: &ObjectRef) -> bool {
        match self.effective_internal_prototype() {
            Some(Prototype::Object(prototype)) => {
                prototype.ptr_eq(target) || prototype.has_prototype(target)
            }
            Some(Prototype::Function(parent)) => parent.chain_contains_object(target),
            Some(Prototype::Proxy(proxy)) => proxy.target_result().is_ok_and(|target_value| {
                crate::property::value_has_prototype_object(target_value, target)
            }),
            None => false,
        }
    }

    /// Whether the function `target` appears in this function's [[Prototype]]
    /// chain (for example a superclass constructor).
    pub(crate) fn chain_contains_function(&self, target: &Self) -> bool {
        match self.effective_internal_prototype() {
            Some(Prototype::Function(parent)) => {
                parent.ptr_eq(target) || parent.chain_contains_function(target)
            }
            Some(Prototype::Proxy(proxy)) => proxy.target_result().is_ok_and(|target_value| {
                crate::property::value_has_prototype_value(
                    target_value,
                    &Value::Function(target.clone()),
                )
            }),
            Some(Prototype::Object(_)) | None => false,
        }
    }

    /// Whether the value `target` (object or function) appears beyond this
    /// function in the [[Prototype]] chain.
    pub(crate) fn chain_contains_value(&self, target: &Value) -> bool {
        match self.effective_internal_prototype() {
            Some(prototype) => prototype.chain_contains_value(target),
            None => false,
        }
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

    pub(crate) fn has_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.symbol_properties
            .borrow()
            .iter()
            .any(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
    }

    pub(crate) fn own_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.symbol_properties
            .borrow()
            .iter()
            .find(|(existing_symbol, _)| existing_symbol.ptr_eq(symbol))
            .map(|(_, property)| property.clone())
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

    pub(crate) fn own_property_symbols(&self) -> Vec<ObjectRef> {
        self.symbol_properties
            .borrow()
            .iter()
            .map(|(symbol, _)| symbol.clone())
            .collect()
    }

    /// Returns the function's private-name storage, creating it on first use.
    pub(crate) fn private_storage(&self) -> crate::private::PrivateStorage {
        self.private_state
            .borrow_mut()
            .storage
            .get_or_insert_with(crate::private::PrivateStorage::new)
            .clone()
    }

    /// Sets the private environment carried by a class constructor.
    pub(crate) fn set_private_environment(&self, environment: crate::private::PrivateEnvironment) {
        self.private_state.borrow_mut().environment = Some(environment);
    }

    /// Returns the private environment carried by this constructor, if any.
    pub(crate) fn private_environment(&self) -> Option<crate::private::PrivateEnvironment> {
        self.private_state.borrow().environment.clone()
    }

    /// Records an instance private element (a field initializer or a
    /// method/accessor brand) applied to each instance at construction time.
    pub(crate) fn push_instance_private_element(&self, element: InstancePrivateElement) {
        self.private_state
            .borrow_mut()
            .instance_elements
            .push(element);
    }

    /// Returns a snapshot of this constructor's instance private elements.
    pub(crate) fn instance_private_elements(&self) -> Vec<InstancePrivateElement> {
        self.private_state.borrow().instance_elements.clone()
    }

    /// The explicit [[Prototype]] override as an object slot. A function-valued
    /// override collapses to `Some(None)` here; callers that must observe the
    /// function use [`Function::internal_prototype_slot`].
    pub(crate) fn internal_prototype_override(&self) -> Option<Option<ObjectRef>> {
        self.internal_prototype
            .borrow()
            .clone()
            .map(|slot| slot.and_then(|prototype| prototype.as_object()))
    }

    /// The raw [[Prototype]] override slot, preserving a function prototype.
    pub(crate) fn internal_prototype_slot(&self) -> Option<Option<Prototype>> {
        self.internal_prototype.borrow().clone()
    }

    pub(crate) fn set_internal_prototype_slot(
        &self,
        prototype: Option<Prototype>,
    ) -> Result<(), ()> {
        if matches!(
            self.internal_prototype.borrow().as_ref(),
            Some(current) if same_prototype_slot(current, &prototype)
        ) {
            return Ok(());
        }
        if !self.extensible.get() {
            return Err(());
        }
        *self.internal_prototype.borrow_mut() = Some(prototype);
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
                Value::String(name) => Some(name.clone()),
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

fn array_index_property_key(key: &str) -> Option<u32> {
    key.parse::<u32>()
        .ok()
        .filter(|index| *index < u32::MAX && index.to_string() == key)
}
