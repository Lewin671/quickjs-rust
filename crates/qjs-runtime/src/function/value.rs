use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
};

use qjs_ast::{FunctionParams, Stmt};

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
    pub params: FunctionParams,
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
    property_order: Rc<RefCell<Vec<String>>>,
    symbol_properties: Rc<RefCell<Vec<(ObjectRef, Property)>>>,
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
            .field("length", &self.params.length())
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
            property_order: Rc::new(RefCell::new(Vec::new())),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
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

    pub(crate) fn new_user_compiled(
        name: Option<String>,
        params: FunctionParams,
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
            property_order: Rc::new(RefCell::new(Vec::new())),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
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
            property_order: Rc::new(RefCell::new(Vec::new())),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
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
        let function = Self {
            name,
            params: FunctionParams::positional(params),
            env,
            local_names: Vec::new(),
            bytecode: None,
            native,
            constructable,
            is_strict: false,
            bound: None,
            properties: Rc::new(RefCell::new(HashMap::new())),
            property_order: Rc::new(RefCell::new(Vec::new())),
            symbol_properties: Rc::new(RefCell::new(Vec::new())),
            extensible: Rc::new(Cell::new(true)),
            sealed: Rc::new(Cell::new(false)),
            frozen: Rc::new(Cell::new(false)),
            internal_prototype: Rc::new(RefCell::new(None)),
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

    pub(crate) fn symbol_property(
        &self,
        symbol: &ObjectRef,
        env: &HashMap<String, Value>,
    ) -> Option<Property> {
        self.own_symbol_property(symbol).or_else(|| {
            self.internal_prototype_override()
                .unwrap_or_else(|| function_intrinsic_prototype(env))
                .and_then(|prototype| prototype.symbol_property(symbol))
        })
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

fn function_as_object_prototype(function: &Function) -> ObjectRef {
    let prototype = function
        .internal_prototype_override()
        .unwrap_or_else(|| function_intrinsic_prototype(&function.env));
    let object = ObjectRef::with_prototype(HashMap::new(), prototype);
    for key in function.own_property_names() {
        if let Some(property) = function.own_property(&key) {
            object.define_property(key, property);
        }
    }
    for symbol in function.own_property_symbols() {
        if let Some(property) = function.own_symbol_property(&symbol) {
            object.define_symbol_property(symbol, property);
        }
    }
    object
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

fn same_prototype(left: &Option<ObjectRef>, right: &Option<ObjectRef>) -> bool {
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
