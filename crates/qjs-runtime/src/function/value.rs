use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use qjs_ast::Stmt;

use crate::{NativeFunction, ObjectRef, Property, Value, object_prototype};

/// User-defined or native function value.
#[derive(Clone)]
pub struct Function {
    /// Optional internal function name.
    pub name: Option<String>,
    /// Parameter names.
    pub params: Vec<String>,
    /// Function body statements.
    pub body: Vec<Stmt>,
    /// Environment captured when the function was created.
    pub env: HashMap<String, Value>,
    pub(crate) native: Option<NativeFunction>,
    pub(crate) constructable: bool,
    pub(crate) bound: Option<Box<BoundFunction>>,
    /// Function object properties.
    pub(crate) properties: Rc<RefCell<HashMap<String, Property>>>,
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
            .field("constructable", &self.constructable)
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
    ) -> Self {
        Self::new_user_with_constructable(name, params, body, env, true)
    }

    pub(crate) fn new_user_with_constructable(
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Stmt>,
        env: HashMap<String, Value>,
        constructable: bool,
    ) -> Self {
        let prototype = ObjectRef::with_prototype(HashMap::new(), object_prototype(&env));
        let function = Self {
            name,
            params,
            body,
            env,
            native: None,
            constructable,
            bound: None,
            properties: Rc::new(RefCell::new(HashMap::new())),
        };
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
            Vec::new(),
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
        Self {
            name: Some("bound".to_owned()),
            params: vec![String::new(); length],
            body: Vec::new(),
            env: HashMap::new(),
            native: None,
            constructable,
            bound: Some(Box::new(BoundFunction {
                target,
                this_value,
                arguments,
            })),
            properties: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn new(
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Stmt>,
        env: HashMap<String, Value>,
        native: Option<NativeFunction>,
        constructable: bool,
    ) -> Self {
        let prototype = ObjectRef::new(HashMap::new());
        let function = Self {
            name,
            params,
            body,
            env,
            native,
            constructable,
            bound: None,
            properties: Rc::new(RefCell::new(HashMap::new())),
        };
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
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.params == other.params
            && self.body == other.body
            && self.native == other.native
            && self.bound.is_some() == other.bound.is_some()
    }
}
