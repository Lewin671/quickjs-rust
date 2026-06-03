use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
};

use qjs_ast::Stmt;

use crate::{
    Bytecode, NativeFunction, ObjectRef, Property, Value,
    bytecode::compile_function_body,
    function::{collect_function_local_names, is_strict_function_body},
    function_intrinsic_prototype, object_prototype,
};

/// User-defined or native function value.
#[derive(Clone)]
pub struct Function {
    /// Optional internal function name.
    pub name: Option<String>,
    /// Parameter names.
    pub params: Vec<String>,
    /// Environment captured when the function was created.
    pub env: HashMap<String, Value>,
    pub(crate) local_names: Vec<String>,
    pub(crate) bytecode: Option<Rc<Bytecode>>,
    pub(crate) native: Option<NativeFunction>,
    pub(crate) constructable: bool,
    pub(crate) is_strict: bool,
    pub(crate) bound: Option<Box<BoundFunction>>,
    /// Function object properties.
    pub(crate) properties: Rc<RefCell<HashMap<String, Property>>>,
    extensible: Rc<Cell<bool>>,
    sealed: Rc<Cell<bool>>,
    frozen: Rc<Cell<bool>>,
    internal_prototype: Rc<RefCell<Option<Option<ObjectRef>>>>,
}

/// Bound function internal slots.
#[derive(Clone)]
pub(crate) struct BoundFunction {
    pub(crate) target: Value,
    pub(crate) this_value: Value,
    pub(crate) arguments: Vec<Value>,
}

impl fmt::Debug for Function {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Function")
            .field("name", &self.name)
            .field("length", &self.params.len())
            .field("native", &self.native)
            .field("local_names", &self.local_names.len())
            .field("bytecode", &self.bytecode.is_some())
            .field("constructable", &self.constructable)
            .field("is_strict", &self.is_strict)
            .field("bound", &self.bound.is_some())
            .finish()
    }
}

impl Function {
    pub(crate) fn new_user(
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Stmt>,
        env: HashMap<String, Value>,
    ) -> Result<Self, crate::RuntimeError> {
        Self::new_user_with_constructable(name, params, body, env, true)
    }

    pub(crate) fn new_user_with_constructable(
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Stmt>,
        env: HashMap<String, Value>,
        constructable: bool,
    ) -> Result<Self, crate::RuntimeError> {
        Self::new_user_with_bytecode(name, params, body, env, None, constructable)
    }

    pub(crate) fn new_user_with_bytecode(
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Stmt>,
        env: HashMap<String, Value>,
        bytecode: Option<Rc<Bytecode>>,
        constructable: bool,
    ) -> Result<Self, crate::RuntimeError> {
        let prototype = ObjectRef::with_prototype(HashMap::new(), object_prototype(&env));
        let local_names = collect_function_local_names(name.as_ref(), &params, &body);
        let is_strict = is_strict_function_body(&body);
        let bytecode = match bytecode {
            Some(bytecode) => bytecode,
            None => Rc::new(compile_function_body(&params, &body)?),
        };
        let function = Self {
            name,
            params,
            env,
            local_names,
            bytecode: Some(bytecode),
            native: None,
            constructable,
            is_strict,
            bound: None,
            properties: Rc::new(RefCell::new(HashMap::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
        };
        function.define_length_property();
        if constructable {
            prototype
                .define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
            function.properties.borrow_mut().insert(
                "prototype".to_owned(),
                Property::non_enumerable(Value::Object(prototype)),
            );
        }
        Ok(function)
    }

    pub(crate) fn new_user_compiled(
        name: Option<String>,
        params: Vec<String>,
        env: HashMap<String, Value>,
        bytecode: Rc<Bytecode>,
        local_names: Vec<String>,
        constructable: bool,
        is_strict: bool,
    ) -> Self {
        let prototype = ObjectRef::with_prototype(HashMap::new(), object_prototype(&env));
        let function = Self {
            name,
            params,
            env,
            local_names,
            bytecode: Some(bytecode),
            native: None,
            constructable,
            is_strict,
            bound: None,
            properties: Rc::new(RefCell::new(HashMap::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
        };
        function.define_length_property();
        if constructable {
            prototype
                .define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
            function.properties.borrow_mut().insert(
                "prototype".to_owned(),
                Property::non_enumerable(Value::Object(prototype)),
            );
        }
        function
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
        let function = Self {
            name: Some("bound".to_owned()),
            params: vec![String::new(); length],
            env: HashMap::new(),
            local_names: Vec::new(),
            bytecode: None,
            native: None,
            constructable,
            is_strict: false,
            bound: Some(Box::new(BoundFunction {
                target,
                this_value,
                arguments,
            })),
            properties: Rc::new(RefCell::new(HashMap::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
        };
        function.define_length_property();
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
        let function = Self {
            name,
            params,
            env,
            local_names: Vec::new(),
            bytecode: None,
            native,
            constructable,
            is_strict: false,
            bound: None,
            properties: Rc::new(RefCell::new(HashMap::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
        };
        function.define_length_property();
        if constructable {
            prototype
                .define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
            function.properties.borrow_mut().insert(
                "prototype".to_owned(),
                Property::non_enumerable(Value::Object(prototype)),
            );
        }
        function
    }

    fn define_length_property(&self) {
        self.properties.borrow_mut().insert(
            "length".to_owned(),
            Property::data(Value::Number(self.params.len() as f64), false, false, true),
        );
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.extensible.get()
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
    }

    pub(crate) fn is_sealed(&self) -> bool {
        !self.extensible.get()
            && self.sealed.get()
            && self
                .properties
                .borrow()
                .values()
                .all(|property| !property.configurable)
    }

    pub(crate) fn freeze(&self) {
        self.prevent_extensions();
        self.sealed.set(true);
        self.frozen.set(true);
        for property in self.properties.borrow_mut().values_mut() {
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
    }

    pub(crate) fn set_property(&self, key: String, value: Value) {
        let value = constructor_prototype_property_value(&key, value);
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
        properties.insert(key, Property::enumerable(value));
    }

    pub(crate) fn internal_prototype_override(&self) -> Option<Option<ObjectRef>> {
        self.internal_prototype.borrow().clone()
    }

    pub(crate) fn set_internal_prototype(&self, prototype: Option<ObjectRef>) -> Result<(), ()> {
        if matches!(
            self.internal_prototype.borrow().as_ref(),
            Some(current) if same_prototype(current, &prototype)
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

fn constructor_prototype_property_value(key: &str, value: Value) -> Value {
    match (key, value) {
        ("prototype", Value::Function(function)) => {
            Value::Object(function_as_object_prototype(&function))
        }
        (_, value) => value,
    }
}

fn function_as_object_prototype(function: &Function) -> ObjectRef {
    let prototype = function
        .internal_prototype_override()
        .unwrap_or_else(|| function_intrinsic_prototype(&function.env));
    let object = ObjectRef::with_prototype(HashMap::new(), prototype);
    for (key, property) in function.properties.borrow().iter() {
        object.define_property(key.clone(), property.clone());
    }
    object
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.params == other.params
            && self.native == other.native
            && self.bound.is_some() == other.bound.is_some()
    }
}

fn same_prototype(left: &Option<ObjectRef>, right: &Option<ObjectRef>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.ptr_eq(right),
        _ => false,
    }
}
