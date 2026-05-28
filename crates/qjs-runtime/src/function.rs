use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use qjs_ast::Stmt;

use crate::{GLOBAL_THIS_BINDING, ObjectRef, Property, RuntimeError, Value, object_prototype};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeFunction {
    Array,
    ArrayIsArray,
    ArrayPrototypeAt,
    ArrayPrototypeConcat,
    ArrayPrototypeCopyWithin,
    ArrayPrototypeFill,
    ArrayPrototypeIncludes,
    ArrayPrototypeIndexOf,
    ArrayPrototypeLastIndexOf,
    ArrayPrototypeJoin,
    ArrayPrototypePop,
    ArrayPrototypePush,
    ArrayPrototypeReverse,
    ArrayPrototypeShift,
    ArrayPrototypeSlice,
    ArrayPrototypeToString,
    ArrayPrototypeUnshift,
    Boolean,
    BooleanPrototypeToString,
    BooleanPrototypeValueOf,
    MathAbs,
    MathAcos,
    MathAcosh,
    MathAsin,
    MathAsinh,
    MathAtan,
    MathAtan2,
    MathAtanh,
    MathCbrt,
    MathCeil,
    MathClz32,
    MathCos,
    MathCosh,
    MathExp,
    MathExpm1,
    MathFloor,
    MathFround,
    MathHypot,
    MathImul,
    MathLog,
    MathLog1p,
    MathLog10,
    MathLog2,
    MathMax,
    MathMin,
    MathPow,
    MathRound,
    MathSign,
    MathSin,
    MathSinh,
    MathSqrt,
    MathTan,
    MathTanh,
    MathTrunc,
    GlobalIsFinite,
    GlobalIsNaN,
    Function,
    FunctionPrototypeApply,
    FunctionPrototypeBind,
    FunctionPrototypeCall,
    Number,
    NumberIsFinite,
    NumberIsInteger,
    NumberIsNaN,
    NumberIsSafeInteger,
    NumberPrototypeToString,
    NumberPrototypeValueOf,
    ParseFloat,
    ParseInt,
    Object,
    ObjectAssign,
    ObjectCreate,
    ObjectDefineProperties,
    ObjectDefineProperty,
    ObjectGetOwnPropertyDescriptor,
    ObjectGetPrototypeOf,
    ObjectGetOwnPropertyNames,
    ObjectHasOwn,
    ObjectKeys,
    ObjectPrototypeHasOwnProperty,
    ObjectPrototypeIsPrototypeOf,
    ObjectPrototypePropertyIsEnumerable,
    ObjectPrototypeToString,
    ObjectPrototypeValueOf,
    String,
    StringFromCharCode,
    StringPrototypeAt,
    StringPrototypeCharAt,
    StringPrototypeCharCodeAt,
    StringPrototypeCodePointAt,
    StringPrototypeConcat,
    StringPrototypeEndsWith,
    StringPrototypeIncludes,
    StringPrototypeIndexOf,
    StringPrototypeLastIndexOf,
    StringPrototypePadEnd,
    StringPrototypePadStart,
    StringPrototypeRepeat,
    StringPrototypeSlice,
    StringPrototypeSplit,
    StringPrototypeStartsWith,
    StringPrototypeSubstring,
    StringPrototypeToLowerCase,
    StringPrototypeTrim,
    StringPrototypeTrimEnd,
    StringPrototypeTrimStart,
    StringPrototypeToString,
    StringPrototypeToUpperCase,
    StringPrototypeValueOf,
}

pub(crate) fn install_function(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let function_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    let function_constructor =
        Function::new_native(Some("Function"), 1, NativeFunction::Function, true);
    function_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(function_constructor.clone()),
    );
    function_prototype.define_non_enumerable(
        "apply".to_owned(),
        Value::Function(Function::new_native(
            Some("apply"),
            2,
            NativeFunction::FunctionPrototypeApply,
            false,
        )),
    );
    function_prototype.define_non_enumerable(
        "call".to_owned(),
        Value::Function(Function::new_native(
            Some("call"),
            1,
            NativeFunction::FunctionPrototypeCall,
            false,
        )),
    );
    function_prototype.define_non_enumerable(
        "bind".to_owned(),
        Value::Function(Function::new_native(
            Some("bind"),
            1,
            NativeFunction::FunctionPrototypeBind,
            false,
        )),
    );
    function_constructor.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(function_prototype)),
    );

    let function_value = Value::Function(function_constructor);
    env.insert("Function".to_owned(), function_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Function".to_owned(), function_value);
    }
}

pub(crate) fn native_function(
    _function: &Function,
    _this_value: Value,
    _argument_values: &[Value],
    _is_construct: bool,
) -> Result<Value, RuntimeError> {
    Err(RuntimeError {
        message: "Function constructor is not implemented".to_owned(),
    })
}

pub(crate) fn native_function_prototype_apply(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Function(_) = this_value else {
        return Err(RuntimeError {
            message: "Function.prototype.apply target is not callable".to_owned(),
        });
    };

    let call_this = function_call_this(argument_values.first().cloned(), env);
    let apply_arguments = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Null | Value::Undefined => Vec::new(),
        Value::Array(elements) => elements.to_vec(),
        value => {
            return Err(RuntimeError {
                message: format!(
                    "Function.prototype.apply argument list is not array-like: {value:?}"
                ),
            });
        }
    };

    crate::call_function(this_value, call_this, apply_arguments, env, false)
}

pub(crate) fn native_function_prototype_call(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Function(_) = this_value else {
        return Err(RuntimeError {
            message: "Function.prototype.call target is not callable".to_owned(),
        });
    };

    let call_this = function_call_this(argument_values.first().cloned(), env);
    crate::call_function(
        this_value,
        call_this,
        argument_values.iter().skip(1).cloned().collect(),
        env,
        false,
    )
}

pub(crate) fn native_function_prototype_bind(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Function(target) = this_value.clone() else {
        return Err(RuntimeError {
            message: "Function.prototype.bind target is not callable".to_owned(),
        });
    };

    let bound_this = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let bound_arguments = argument_values.iter().skip(1).cloned().collect::<Vec<_>>();
    let length = target.params.len().saturating_sub(bound_arguments.len());
    let bound = Function::new_bound(this_value, bound_this, bound_arguments, length);
    Ok(Value::Function(bound))
}

fn function_call_this(this_arg: Option<Value>, env: &HashMap<String, Value>) -> Value {
    match this_arg.unwrap_or(Value::Undefined) {
        Value::Null | Value::Undefined => env
            .get(GLOBAL_THIS_BINDING)
            .cloned()
            .unwrap_or(Value::Undefined),
        value => value,
    }
}

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
