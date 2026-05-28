//! Early interpreter for the Rust QuickJS rewrite.

use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use qjs_ast::{
    AssignmentOp, AssignmentTarget, BinaryOp, CatchClause, Expr, ForInLeft, ForInit, Literal,
    MemberProperty, Script, Stmt, SwitchCase, UnaryOp, UpdateOp, VarKind,
};
use qjs_parser::parse_script;

mod array;
mod value;

pub use value::Value;
use value::{ArrayRef, ObjectRef, Property};

const GLOBAL_THIS_BINDING: &str = "\0global_this";
const BOOLEAN_DATA_PROPERTY: &str = "\0BooleanData";
const NUMBER_DATA_PROPERTY: &str = "\0NumberData";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeFunction {
    Array,
    ArrayIsArray,
    ArrayPrototypeAt,
    ArrayPrototypeConcat,
    ArrayPrototypeIncludes,
    ArrayPrototypeIndexOf,
    ArrayPrototypeLastIndexOf,
    ArrayPrototypeJoin,
    ArrayPrototypePop,
    ArrayPrototypePush,
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

/// User-defined function value.
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
    native: Option<NativeFunction>,
    constructable: bool,
    /// Function object properties.
    properties: Rc<RefCell<HashMap<String, Property>>>,
}

impl fmt::Debug for Function {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Function")
            .field("name", &self.name)
            .field("length", &self.params.len())
            .field("native", &self.native)
            .field("constructable", &self.constructable)
            .finish()
    }
}

impl Function {
    fn new_user(
        name: Option<String>,
        params: Vec<String>,
        body: Vec<Stmt>,
        env: HashMap<String, Value>,
    ) -> Self {
        let prototype = ObjectRef::with_prototype(HashMap::new(), object_prototype(&env));
        let function = Self {
            name,
            params,
            body,
            env,
            native: None,
            constructable: true,
            properties: Rc::new(RefCell::new(HashMap::new())),
        };
        prototype
            .define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
        function.properties.borrow_mut().insert(
            "prototype".to_owned(),
            Property::non_enumerable(Value::Object(prototype)),
        );
        function
    }

    fn new_native(
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
    }
}

/// Runtime error.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeError {
    /// Human-readable message.
    pub message: String,
}

/// Evaluates source text and returns the last statement value.
///
/// # Errors
///
/// Returns parser or runtime failures.
pub fn eval(source: &str) -> Result<Value, RuntimeError> {
    let script = parse_script(source).map_err(|error| RuntimeError {
        message: error.message,
    })?;
    eval_script(&script)
}

/// Evaluates an AST script.
///
/// # Errors
///
/// Returns runtime failures for unsupported operations.
pub fn eval_script(script: &Script) -> Result<Value, RuntimeError> {
    let mut env = HashMap::new();
    let global_this = Value::Object(ObjectRef::new(HashMap::new()));
    env.insert("this".to_owned(), global_this.clone());
    env.insert(GLOBAL_THIS_BINDING.to_owned(), global_this.clone());
    env.insert("undefined".to_owned(), Value::Undefined);
    initialize_builtins(&mut env, &global_this);
    hoist_declarations(&script.body, &mut env);
    let mut last = Value::Undefined;
    for stmt in &script.body {
        match eval_stmt(stmt, &mut env)? {
            Completion::Normal(value) => last = value,
            Completion::Return(value) => return Ok(value),
            Completion::Break | Completion::Continue => {
                return Err(RuntimeError {
                    message: "break or continue outside loop".to_owned(),
                });
            }
            Completion::Throw(value) => {
                return Err(RuntimeError {
                    message: format!("throw statement executed: {}", error_value(value)),
                });
            }
        }
    }
    Ok(last)
}

fn initialize_builtins(env: &mut HashMap<String, Value>, global_this: &Value) {
    let object_prototype = ObjectRef::new(HashMap::new());
    let object_function = Function::new_native(Some("Object"), 1, NativeFunction::Object, true);
    object_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(object_function.clone()),
    );
    object_prototype.define_non_enumerable(
        "hasOwnProperty".to_owned(),
        Value::Function(Function::new_native(
            Some("hasOwnProperty"),
            1,
            NativeFunction::ObjectPrototypeHasOwnProperty,
            false,
        )),
    );
    object_prototype.define_non_enumerable(
        "propertyIsEnumerable".to_owned(),
        Value::Function(Function::new_native(
            Some("propertyIsEnumerable"),
            1,
            NativeFunction::ObjectPrototypePropertyIsEnumerable,
            false,
        )),
    );
    object_prototype.define_non_enumerable(
        "isPrototypeOf".to_owned(),
        Value::Function(Function::new_native(
            Some("isPrototypeOf"),
            1,
            NativeFunction::ObjectPrototypeIsPrototypeOf,
            false,
        )),
    );
    object_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::ObjectPrototypeToString,
            false,
        )),
    );
    object_prototype.define_non_enumerable(
        "valueOf".to_owned(),
        Value::Function(Function::new_native(
            Some("valueOf"),
            0,
            NativeFunction::ObjectPrototypeValueOf,
            false,
        )),
    );
    object_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(object_prototype.clone())),
    );
    object_function.properties.borrow_mut().insert(
        "assign".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("assign"),
            2,
            NativeFunction::ObjectAssign,
            false,
        ))),
    );
    object_function.properties.borrow_mut().insert(
        "create".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("create"),
            1,
            NativeFunction::ObjectCreate,
            false,
        ))),
    );
    object_function.properties.borrow_mut().insert(
        "defineProperty".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("defineProperty"),
            3,
            NativeFunction::ObjectDefineProperty,
            false,
        ))),
    );
    object_function.properties.borrow_mut().insert(
        "defineProperties".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("defineProperties"),
            2,
            NativeFunction::ObjectDefineProperties,
            false,
        ))),
    );
    object_function.properties.borrow_mut().insert(
        "getPrototypeOf".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("getPrototypeOf"),
            1,
            NativeFunction::ObjectGetPrototypeOf,
            false,
        ))),
    );
    object_function.properties.borrow_mut().insert(
        "getOwnPropertyDescriptor".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("getOwnPropertyDescriptor"),
            2,
            NativeFunction::ObjectGetOwnPropertyDescriptor,
            false,
        ))),
    );
    object_function.properties.borrow_mut().insert(
        "getOwnPropertyNames".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("getOwnPropertyNames"),
            1,
            NativeFunction::ObjectGetOwnPropertyNames,
            false,
        ))),
    );
    object_function.properties.borrow_mut().insert(
        "hasOwn".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("hasOwn"),
            2,
            NativeFunction::ObjectHasOwn,
            false,
        ))),
    );
    object_function.properties.borrow_mut().insert(
        "keys".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("keys"),
            1,
            NativeFunction::ObjectKeys,
            false,
        ))),
    );

    let object_value = Value::Function(object_function);
    env.insert("Object".to_owned(), object_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Object".to_owned(), object_value);
    }

    env.insert("NaN".to_owned(), Value::Number(f64::NAN));
    env.insert("Infinity".to_owned(), Value::Number(f64::INFINITY));
    if let Value::Object(global_object) = global_this {
        global_object.define_property(
            "NaN".to_owned(),
            Property::data(Value::Number(f64::NAN), false, false, false),
        );
        global_object.define_property(
            "Infinity".to_owned(),
            Property::data(Value::Number(f64::INFINITY), false, false, false),
        );
    }

    let is_finite_value = Value::Function(Function::new_native(
        Some("isFinite"),
        1,
        NativeFunction::GlobalIsFinite,
        false,
    ));
    env.insert("isFinite".to_owned(), is_finite_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("isFinite".to_owned(), is_finite_value);
    }

    let is_nan_value = Value::Function(Function::new_native(
        Some("isNaN"),
        1,
        NativeFunction::GlobalIsNaN,
        false,
    ));
    env.insert("isNaN".to_owned(), is_nan_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("isNaN".to_owned(), is_nan_value);
    }

    let number_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    let number_function = Function::new_native(Some("Number"), 1, NativeFunction::Number, true);
    number_prototype.define_non_enumerable(NUMBER_DATA_PROPERTY.to_owned(), Value::Number(0.0));
    number_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(number_function.clone()),
    );
    number_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            1,
            NativeFunction::NumberPrototypeToString,
            false,
        )),
    );
    number_prototype.define_non_enumerable(
        "valueOf".to_owned(),
        Value::Function(Function::new_native(
            Some("valueOf"),
            0,
            NativeFunction::NumberPrototypeValueOf,
            false,
        )),
    );
    number_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(number_prototype)),
    );
    define_number_constant(&number_function, "EPSILON", f64::EPSILON);
    define_number_constant(
        &number_function,
        "MAX_SAFE_INTEGER",
        9_007_199_254_740_991.0,
    );
    define_number_constant(&number_function, "MAX_VALUE", f64::MAX);
    define_number_constant(
        &number_function,
        "MIN_SAFE_INTEGER",
        -9_007_199_254_740_991.0,
    );
    define_number_constant(&number_function, "MIN_VALUE", f64::MIN_POSITIVE);
    define_number_constant(&number_function, "NaN", f64::NAN);
    define_number_constant(&number_function, "NEGATIVE_INFINITY", f64::NEG_INFINITY);
    define_number_constant(&number_function, "POSITIVE_INFINITY", f64::INFINITY);
    define_function_property(
        &number_function,
        "isFinite",
        1,
        NativeFunction::NumberIsFinite,
    );
    define_function_property(
        &number_function,
        "isInteger",
        1,
        NativeFunction::NumberIsInteger,
    );
    define_function_property(&number_function, "isNaN", 1, NativeFunction::NumberIsNaN);
    define_function_property(
        &number_function,
        "isSafeInteger",
        1,
        NativeFunction::NumberIsSafeInteger,
    );
    let parse_float_value = Value::Function(Function::new_native(
        Some("parseFloat"),
        1,
        NativeFunction::ParseFloat,
        false,
    ));
    let parse_int_value = Value::Function(Function::new_native(
        Some("parseInt"),
        2,
        NativeFunction::ParseInt,
        false,
    ));
    number_function.properties.borrow_mut().insert(
        "parseFloat".to_owned(),
        Property::non_enumerable(parse_float_value.clone()),
    );
    number_function.properties.borrow_mut().insert(
        "parseInt".to_owned(),
        Property::non_enumerable(parse_int_value.clone()),
    );
    let number_value = Value::Function(number_function);
    env.insert("Number".to_owned(), number_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Number".to_owned(), number_value);
    }

    env.insert("parseFloat".to_owned(), parse_float_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("parseFloat".to_owned(), parse_float_value);
    }

    env.insert("parseInt".to_owned(), parse_int_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("parseInt".to_owned(), parse_int_value);
    }

    let string_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    let string_function = Function::new_native(Some("String"), 1, NativeFunction::String, true);
    string_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(string_function.clone()),
    );
    string_prototype.define_non_enumerable(
        "charAt".to_owned(),
        Value::Function(Function::new_native(
            Some("charAt"),
            1,
            NativeFunction::StringPrototypeCharAt,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "charCodeAt".to_owned(),
        Value::Function(Function::new_native(
            Some("charCodeAt"),
            1,
            NativeFunction::StringPrototypeCharCodeAt,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "codePointAt".to_owned(),
        Value::Function(Function::new_native(
            Some("codePointAt"),
            1,
            NativeFunction::StringPrototypeCodePointAt,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "concat".to_owned(),
        Value::Function(Function::new_native(
            Some("concat"),
            1,
            NativeFunction::StringPrototypeConcat,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "endsWith".to_owned(),
        Value::Function(Function::new_native(
            Some("endsWith"),
            1,
            NativeFunction::StringPrototypeEndsWith,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "includes".to_owned(),
        Value::Function(Function::new_native(
            Some("includes"),
            1,
            NativeFunction::StringPrototypeIncludes,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "indexOf".to_owned(),
        Value::Function(Function::new_native(
            Some("indexOf"),
            1,
            NativeFunction::StringPrototypeIndexOf,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "lastIndexOf".to_owned(),
        Value::Function(Function::new_native(
            Some("lastIndexOf"),
            1,
            NativeFunction::StringPrototypeLastIndexOf,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "padEnd".to_owned(),
        Value::Function(Function::new_native(
            Some("padEnd"),
            1,
            NativeFunction::StringPrototypePadEnd,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "padStart".to_owned(),
        Value::Function(Function::new_native(
            Some("padStart"),
            1,
            NativeFunction::StringPrototypePadStart,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "repeat".to_owned(),
        Value::Function(Function::new_native(
            Some("repeat"),
            1,
            NativeFunction::StringPrototypeRepeat,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "slice".to_owned(),
        Value::Function(Function::new_native(
            Some("slice"),
            2,
            NativeFunction::StringPrototypeSlice,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "split".to_owned(),
        Value::Function(Function::new_native(
            Some("split"),
            2,
            NativeFunction::StringPrototypeSplit,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "startsWith".to_owned(),
        Value::Function(Function::new_native(
            Some("startsWith"),
            1,
            NativeFunction::StringPrototypeStartsWith,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "substring".to_owned(),
        Value::Function(Function::new_native(
            Some("substring"),
            2,
            NativeFunction::StringPrototypeSubstring,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "toLowerCase".to_owned(),
        Value::Function(Function::new_native(
            Some("toLowerCase"),
            0,
            NativeFunction::StringPrototypeToLowerCase,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "trim".to_owned(),
        Value::Function(Function::new_native(
            Some("trim"),
            0,
            NativeFunction::StringPrototypeTrim,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "trimEnd".to_owned(),
        Value::Function(Function::new_native(
            Some("trimEnd"),
            0,
            NativeFunction::StringPrototypeTrimEnd,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "trimStart".to_owned(),
        Value::Function(Function::new_native(
            Some("trimStart"),
            0,
            NativeFunction::StringPrototypeTrimStart,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::StringPrototypeToString,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "toUpperCase".to_owned(),
        Value::Function(Function::new_native(
            Some("toUpperCase"),
            0,
            NativeFunction::StringPrototypeToUpperCase,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "valueOf".to_owned(),
        Value::Function(Function::new_native(
            Some("valueOf"),
            0,
            NativeFunction::StringPrototypeValueOf,
            false,
        )),
    );
    string_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(string_prototype)),
    );
    define_function_property(
        &string_function,
        "fromCharCode",
        1,
        NativeFunction::StringFromCharCode,
    );
    let string_value = Value::Function(string_function);
    env.insert("String".to_owned(), string_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("String".to_owned(), string_value);
    }

    let boolean_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    let boolean_function = Function::new_native(Some("Boolean"), 1, NativeFunction::Boolean, true);
    boolean_prototype
        .define_non_enumerable(BOOLEAN_DATA_PROPERTY.to_owned(), Value::Boolean(false));
    boolean_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(boolean_function.clone()),
    );
    boolean_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::BooleanPrototypeToString,
            false,
        )),
    );
    boolean_prototype.define_non_enumerable(
        "valueOf".to_owned(),
        Value::Function(Function::new_native(
            Some("valueOf"),
            0,
            NativeFunction::BooleanPrototypeValueOf,
            false,
        )),
    );
    boolean_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(boolean_prototype)),
    );
    let boolean_value = Value::Function(boolean_function);
    env.insert("Boolean".to_owned(), boolean_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Boolean".to_owned(), boolean_value);
    }

    let math_object = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    define_math_constant(&math_object, "E", std::f64::consts::E);
    define_math_constant(&math_object, "LN10", std::f64::consts::LN_10);
    define_math_constant(&math_object, "LN2", std::f64::consts::LN_2);
    define_math_constant(&math_object, "LOG10E", std::f64::consts::LOG10_E);
    define_math_constant(&math_object, "LOG2E", std::f64::consts::LOG2_E);
    define_math_constant(&math_object, "PI", std::f64::consts::PI);
    define_math_constant(&math_object, "SQRT1_2", std::f64::consts::FRAC_1_SQRT_2);
    define_math_constant(&math_object, "SQRT2", std::f64::consts::SQRT_2);
    define_math_function(&math_object, "abs", 1, NativeFunction::MathAbs);
    define_math_function(&math_object, "acos", 1, NativeFunction::MathAcos);
    define_math_function(&math_object, "acosh", 1, NativeFunction::MathAcosh);
    define_math_function(&math_object, "asin", 1, NativeFunction::MathAsin);
    define_math_function(&math_object, "asinh", 1, NativeFunction::MathAsinh);
    define_math_function(&math_object, "atan", 1, NativeFunction::MathAtan);
    define_math_function(&math_object, "atan2", 2, NativeFunction::MathAtan2);
    define_math_function(&math_object, "atanh", 1, NativeFunction::MathAtanh);
    define_math_function(&math_object, "cbrt", 1, NativeFunction::MathCbrt);
    define_math_function(&math_object, "ceil", 1, NativeFunction::MathCeil);
    define_math_function(&math_object, "clz32", 1, NativeFunction::MathClz32);
    define_math_function(&math_object, "cos", 1, NativeFunction::MathCos);
    define_math_function(&math_object, "cosh", 1, NativeFunction::MathCosh);
    define_math_function(&math_object, "exp", 1, NativeFunction::MathExp);
    define_math_function(&math_object, "expm1", 1, NativeFunction::MathExpm1);
    define_math_function(&math_object, "floor", 1, NativeFunction::MathFloor);
    define_math_function(&math_object, "fround", 1, NativeFunction::MathFround);
    define_math_function(&math_object, "hypot", 2, NativeFunction::MathHypot);
    define_math_function(&math_object, "imul", 2, NativeFunction::MathImul);
    define_math_function(&math_object, "log", 1, NativeFunction::MathLog);
    define_math_function(&math_object, "log1p", 1, NativeFunction::MathLog1p);
    define_math_function(&math_object, "log10", 1, NativeFunction::MathLog10);
    define_math_function(&math_object, "log2", 1, NativeFunction::MathLog2);
    define_math_function(&math_object, "max", 2, NativeFunction::MathMax);
    define_math_function(&math_object, "min", 2, NativeFunction::MathMin);
    define_math_function(&math_object, "pow", 2, NativeFunction::MathPow);
    define_math_function(&math_object, "round", 1, NativeFunction::MathRound);
    define_math_function(&math_object, "sign", 1, NativeFunction::MathSign);
    define_math_function(&math_object, "sin", 1, NativeFunction::MathSin);
    define_math_function(&math_object, "sinh", 1, NativeFunction::MathSinh);
    define_math_function(&math_object, "sqrt", 1, NativeFunction::MathSqrt);
    define_math_function(&math_object, "tan", 1, NativeFunction::MathTan);
    define_math_function(&math_object, "tanh", 1, NativeFunction::MathTanh);
    define_math_function(&math_object, "trunc", 1, NativeFunction::MathTrunc);
    let math_value = Value::Object(math_object);
    env.insert("Math".to_owned(), math_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Math".to_owned(), math_value);
    }

    let array_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    let array_function = Function::new_native(Some("Array"), 1, NativeFunction::Array, true);
    array_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(array_function.clone()),
    );
    array_prototype.define_non_enumerable(
        "at".to_owned(),
        Value::Function(Function::new_native(
            Some("at"),
            1,
            NativeFunction::ArrayPrototypeAt,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "concat".to_owned(),
        Value::Function(Function::new_native(
            Some("concat"),
            1,
            NativeFunction::ArrayPrototypeConcat,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "includes".to_owned(),
        Value::Function(Function::new_native(
            Some("includes"),
            1,
            NativeFunction::ArrayPrototypeIncludes,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "join".to_owned(),
        Value::Function(Function::new_native(
            Some("join"),
            1,
            NativeFunction::ArrayPrototypeJoin,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "indexOf".to_owned(),
        Value::Function(Function::new_native(
            Some("indexOf"),
            1,
            NativeFunction::ArrayPrototypeIndexOf,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "lastIndexOf".to_owned(),
        Value::Function(Function::new_native(
            Some("lastIndexOf"),
            1,
            NativeFunction::ArrayPrototypeLastIndexOf,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "pop".to_owned(),
        Value::Function(Function::new_native(
            Some("pop"),
            0,
            NativeFunction::ArrayPrototypePop,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "push".to_owned(),
        Value::Function(Function::new_native(
            Some("push"),
            1,
            NativeFunction::ArrayPrototypePush,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "shift".to_owned(),
        Value::Function(Function::new_native(
            Some("shift"),
            0,
            NativeFunction::ArrayPrototypeShift,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "slice".to_owned(),
        Value::Function(Function::new_native(
            Some("slice"),
            2,
            NativeFunction::ArrayPrototypeSlice,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::ArrayPrototypeToString,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "unshift".to_owned(),
        Value::Function(Function::new_native(
            Some("unshift"),
            1,
            NativeFunction::ArrayPrototypeUnshift,
            false,
        )),
    );
    array_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(array_prototype)),
    );
    array_function.properties.borrow_mut().insert(
        "isArray".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("isArray"),
            1,
            NativeFunction::ArrayIsArray,
            false,
        ))),
    );

    let array_value = Value::Function(array_function);
    env.insert("Array".to_owned(), array_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Array".to_owned(), array_value);
    }
}

fn define_math_constant(object: &ObjectRef, key: &str, value: f64) {
    object.define_property(
        key.to_owned(),
        Property::data(Value::Number(value), false, false, false),
    );
}

fn define_math_function(object: &ObjectRef, key: &str, length: usize, native: NativeFunction) {
    object.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}

fn define_number_constant(function: &Function, key: &str, value: f64) {
    function.properties.borrow_mut().insert(
        key.to_owned(),
        Property::data(Value::Number(value), false, false, false),
    );
}

fn define_function_property(function: &Function, key: &str, length: usize, native: NativeFunction) {
    function.properties.borrow_mut().insert(
        key.to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some(key),
            length,
            native,
            false,
        ))),
    );
}

enum Completion {
    Normal(Value),
    Return(Value),
    Break,
    Continue,
    Throw(Value),
}

fn eval_stmt(stmt: &Stmt, env: &mut HashMap<String, Value>) -> Result<Completion, RuntimeError> {
    match stmt {
        Stmt::Expr(expr) => eval_expr(expr, env).map(Completion::Normal),
        Stmt::Block { body, .. } => eval_statement_list(body, env),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            let test = eval_expr(test, env)?;
            if is_truthy(&test) {
                eval_stmt(consequent, env)
            } else if let Some(alternate) = alternate {
                eval_stmt(alternate, env)
            } else {
                Ok(Completion::Normal(Value::Undefined))
            }
        }
        Stmt::While { test, body, .. } => {
            let mut last = Value::Undefined;
            while is_truthy(&eval_expr(test, env)?) {
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::DoWhile { body, test, .. } => {
            let mut last = Value::Undefined;
            loop {
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
                if !is_truthy(&eval_expr(test, env)?) {
                    break;
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            if let Some(init) = init {
                eval_for_init(init, env)?;
            }
            let mut last = Value::Undefined;
            while test.as_ref().map_or(Ok(true), |test| {
                eval_expr(test, env).map(|value| is_truthy(&value))
            })? {
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
                if let Some(update) = update {
                    eval_expr(update, env)?;
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::ForIn {
            left, right, body, ..
        } => {
            let keys = enumerable_keys(eval_expr(right, env)?)?;
            let mut last = Value::Undefined;
            for key in keys {
                assign_for_in_left(left, Value::String(key), env)?;
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => eval_switch(discriminant, cases, env),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => eval_try(block, handler.as_ref(), finalizer.as_deref(), env),
        Stmt::FunctionDecl {
            name, params, body, ..
        } => {
            env.insert(
                name.clone(),
                Value::Function(Function::new_user(
                    Some(name.clone()),
                    params.clone(),
                    body.clone(),
                    env.clone(),
                )),
            );
            Ok(Completion::Normal(Value::Undefined))
        }
        Stmt::Return { argument, .. } => {
            let value = if let Some(argument) = argument {
                eval_expr(argument, env)?
            } else {
                Value::Undefined
            };
            Ok(Completion::Return(value))
        }
        Stmt::Throw { argument, .. } => {
            let value = if let Some(argument) = argument {
                eval_expr(argument, env)?
            } else {
                Value::Undefined
            };
            Ok(Completion::Throw(value))
        }
        Stmt::Debugger { .. } => Ok(Completion::Normal(Value::Undefined)),
        Stmt::Break { .. } => Ok(Completion::Break),
        Stmt::Continue { .. } => Ok(Completion::Continue),
        Stmt::VarDecl { declarations, .. } => {
            for declaration in declarations {
                let value = if let Some(init) = &declaration.init {
                    eval_expr(init, env)?
                } else {
                    Value::Undefined
                };
                env.insert(declaration.name.clone(), value);
            }
            Ok(Completion::Normal(Value::Undefined))
        }
        Stmt::Empty => Ok(Completion::Normal(Value::Undefined)),
    }
}

fn eval_statement_list(
    body: &[Stmt],
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    hoist_declarations(body, env);
    let mut last = Value::Undefined;
    for stmt in body {
        match eval_stmt(stmt, env)? {
            Completion::Normal(value) => last = value,
            Completion::Return(value) => return Ok(Completion::Return(value)),
            Completion::Break => return Ok(Completion::Break),
            Completion::Continue => return Ok(Completion::Continue),
            Completion::Throw(value) => return Ok(Completion::Throw(value)),
        }
    }
    Ok(Completion::Normal(last))
}

fn hoist_declarations(body: &[Stmt], env: &mut HashMap<String, Value>) {
    hoist_var_declarations(body, env);
    hoist_function_declarations(body, env);
}

fn hoist_var_declarations(body: &[Stmt], env: &mut HashMap<String, Value>) {
    for stmt in body {
        match stmt {
            Stmt::VarDecl {
                kind: VarKind::Var,
                declarations,
                ..
            } => {
                for declaration in declarations {
                    env.entry(declaration.name.clone())
                        .or_insert(Value::Undefined);
                }
            }
            Stmt::Block { body, .. } => hoist_var_declarations(body, env),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                hoist_var_declarations(std::slice::from_ref(consequent.as_ref()), env);
                if let Some(alternate) = alternate {
                    hoist_var_declarations(std::slice::from_ref(alternate.as_ref()), env);
                }
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                hoist_var_declarations(std::slice::from_ref(body.as_ref()), env);
            }
            Stmt::For { init, body, .. } => {
                if let Some(ForInit::VarDecl {
                    kind: VarKind::Var,
                    declarations,
                    ..
                }) = init
                {
                    for declaration in declarations {
                        env.entry(declaration.name.clone())
                            .or_insert(Value::Undefined);
                    }
                }
                hoist_var_declarations(std::slice::from_ref(body.as_ref()), env);
            }
            Stmt::ForIn { left, body, .. } => {
                if let ForInLeft::VarDecl {
                    kind: VarKind::Var,
                    name,
                    ..
                } = left
                {
                    env.entry(name.clone()).or_insert(Value::Undefined);
                }
                hoist_var_declarations(std::slice::from_ref(body.as_ref()), env);
            }
            Stmt::Switch { cases, .. } => {
                for case in cases {
                    hoist_var_declarations(&case.consequent, env);
                }
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                hoist_var_declarations(block, env);
                if let Some(handler) = handler {
                    hoist_var_declarations(&handler.body, env);
                }
                if let Some(finalizer) = finalizer {
                    hoist_var_declarations(finalizer, env);
                }
            }
            Stmt::FunctionDecl { .. } => {}
            _ => {}
        }
    }
}

fn hoist_function_declarations(body: &[Stmt], env: &mut HashMap<String, Value>) {
    for stmt in body {
        if let Stmt::FunctionDecl {
            name, params, body, ..
        } = stmt
        {
            env.insert(
                name.clone(),
                Value::Function(Function::new_user(
                    Some(name.clone()),
                    params.clone(),
                    body.clone(),
                    env.clone(),
                )),
            );
        }
    }
}

fn eval_switch(
    discriminant: &Expr,
    cases: &[SwitchCase],
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    let discriminant = eval_expr(discriminant, env)?;
    let mut default_index = None;
    let mut selected_index = None;

    for (index, case) in cases.iter().enumerate() {
        if let Some(test) = &case.test {
            if eval_expr(test, env)? == discriminant {
                selected_index = Some(index);
                break;
            }
        } else {
            default_index = Some(index);
        }
    }

    let Some(start_index) = selected_index.or(default_index) else {
        return Ok(Completion::Normal(Value::Undefined));
    };

    let mut last = Value::Undefined;
    for case in &cases[start_index..] {
        for stmt in &case.consequent {
            match eval_stmt(stmt, env)? {
                Completion::Normal(value) => last = value,
                Completion::Break => return Ok(Completion::Normal(last)),
                Completion::Return(value) => return Ok(Completion::Return(value)),
                Completion::Continue => return Ok(Completion::Continue),
                Completion::Throw(value) => return Ok(Completion::Throw(value)),
            }
        }
    }
    Ok(Completion::Normal(last))
}

fn eval_try(
    block: &[Stmt],
    handler: Option<&CatchClause>,
    finalizer: Option<&[Stmt]>,
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    let mut completion = match eval_statement_list(block, env)? {
        Completion::Throw(value) => {
            if let Some(handler) = handler {
                eval_catch(handler, value, env)?
            } else {
                Completion::Throw(value)
            }
        }
        other => other,
    };

    if let Some(finalizer) = finalizer {
        let final_completion = eval_statement_list(finalizer, env)?;
        completion = match final_completion {
            Completion::Normal(_) => completion,
            abrupt => abrupt,
        };
    }

    Ok(completion)
}

fn eval_catch(
    handler: &CatchClause,
    thrown: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    let previous = if let Some(param) = &handler.param {
        env.insert(param.clone(), thrown)
    } else {
        None
    };
    let completion = eval_statement_list(&handler.body, env);
    if let Some(param) = &handler.param {
        if let Some(value) = previous {
            env.insert(param.clone(), value);
        } else {
            env.remove(param);
        }
    }
    completion
}

fn eval_for_init(init: &ForInit, env: &mut HashMap<String, Value>) -> Result<(), RuntimeError> {
    match init {
        ForInit::VarDecl { declarations, .. } => {
            for declaration in declarations {
                let value = if let Some(init) = &declaration.init {
                    eval_expr(init, env)?
                } else {
                    Value::Undefined
                };
                env.insert(declaration.name.clone(), value);
            }
            Ok(())
        }
        ForInit::Expr(expr) => eval_expr(expr, env).map(|_| ()),
    }
}

fn assign_for_in_left(
    left: &ForInLeft,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    match left {
        ForInLeft::VarDecl { name, .. } => {
            env.insert(name.clone(), value);
            Ok(())
        }
        ForInLeft::Target(target) => assign_target(target, value, env),
    }
}

fn enumerable_keys(value: Value) -> Result<Vec<String>, RuntimeError> {
    match value {
        Value::Object(object) => Ok(object.own_property_keys()),
        Value::Array(elements) => Ok((0..elements.len()).map(|index| index.to_string()).collect()),
        Value::Null | Value::Undefined => Ok(Vec::new()),
        _ => Err(RuntimeError {
            message: "for-in target is not enumerable".to_owned(),
        }),
    }
}

fn eval_call(
    callee: &Expr,
    arguments: &[Expr],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (callee, this_value) = match callee {
        Expr::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            let callee = eval_member(object.clone(), property, env)?;
            (callee, object)
        }
        _ => {
            let callee = eval_expr(callee, env)?;
            let this_value = env
                .get(GLOBAL_THIS_BINDING)
                .cloned()
                .unwrap_or(Value::Undefined);
            (callee, this_value)
        }
    };

    let argument_values = eval_arguments(arguments, env)?;
    call_function(callee, this_value, argument_values, env, false)
}

fn eval_new(
    callee: &Expr,
    arguments: &[Expr],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let callee = eval_expr(callee, env)?;
    let Value::Function(function) = &callee else {
        return Err(RuntimeError {
            message: "value is not a constructor".to_owned(),
        });
    };
    if !function.constructable {
        return Err(RuntimeError {
            message: "value is not a constructor".to_owned(),
        });
    }
    let argument_values = eval_arguments(arguments, env)?;
    let prototype = constructor_prototype(&callee);
    let this_value = Value::Object(ObjectRef::with_prototype(HashMap::new(), prototype));
    let result = call_function(callee, this_value.clone(), argument_values, env, true)?;
    match result {
        Value::Array(_) | Value::Function(_) | Value::Object(_) => Ok(result),
        _ => Ok(this_value),
    }
}

fn constructor_prototype(callee: &Value) -> Option<ObjectRef> {
    let Value::Function(function) = callee else {
        return None;
    };
    function_prototype(function)
}

fn object_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(object_function)) = env.get("Object") else {
        return None;
    };
    function_prototype(object_function)
}

fn array_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(array_function)) = env.get("Array") else {
        return None;
    };
    function_prototype(array_function)
}

fn string_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(string_function)) = env.get("String") else {
        return None;
    };
    function_prototype(string_function)
}

fn boolean_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(boolean_function)) = env.get("Boolean") else {
        return None;
    };
    function_prototype(boolean_function)
}

fn number_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(number_function)) = env.get("Number") else {
        return None;
    };
    function_prototype(number_function)
}

fn eval_arguments(
    arguments: &[Expr],
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let mut argument_values = Vec::with_capacity(arguments.len());
    for argument in arguments {
        argument_values.push(eval_expr(argument, env)?);
    }
    Ok(argument_values)
}

fn call_function(
    callee: Value,
    this_value: Value,
    argument_values: Vec<Value>,
    env: &mut HashMap<String, Value>,
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    let Value::Function(function) = callee.clone() else {
        return Err(RuntimeError {
            message: "value is not callable".to_owned(),
        });
    };
    if let Some(native) = function.native {
        return call_native_function(
            &function,
            native,
            this_value,
            argument_values,
            is_construct,
            env,
        );
    }
    let mut local_env = env.clone();
    for (name, value) in &function.env {
        local_env
            .entry(name.clone())
            .or_insert_with(|| value.clone());
    }
    if let Some(global_this) = env.get(GLOBAL_THIS_BINDING).cloned() {
        local_env.insert(GLOBAL_THIS_BINDING.to_owned(), global_this);
    }
    if let Some(name) = &function.name {
        local_env.insert(name.clone(), callee);
    }
    local_env.insert("this".to_owned(), this_value);
    local_env.insert(
        "arguments".to_owned(),
        Value::Array(ArrayRef::new(argument_values.clone())),
    );
    for (index, param) in function.params.iter().enumerate() {
        let value = argument_values
            .get(index)
            .cloned()
            .unwrap_or(Value::Undefined);
        local_env.insert(param.clone(), value);
    }

    match eval_statement_list(&function.body, &mut local_env)? {
        Completion::Normal(value) => Ok(value),
        Completion::Return(value) => Ok(value),
        Completion::Break | Completion::Continue => Err(RuntimeError {
            message: "break or continue outside loop".to_owned(),
        }),
        Completion::Throw(value) => Err(RuntimeError {
            message: format!("throw statement executed: {}", error_value(value)),
        }),
    }
}

fn call_native_function(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: Vec<Value>,
    is_construct: bool,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match native {
        NativeFunction::Array => array::native_array(&argument_values),
        NativeFunction::ArrayIsArray => array::native_array_is_array(&argument_values),
        NativeFunction::ArrayPrototypeAt => {
            array::native_array_prototype_at(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeConcat => {
            array::native_array_prototype_concat(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeIncludes => {
            array::native_array_prototype_includes(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeIndexOf => {
            array::native_array_prototype_index_of(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeLastIndexOf => {
            array::native_array_prototype_last_index_of(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeJoin => {
            array::native_array_prototype_join(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypePop => array::native_array_prototype_pop(this_value),
        NativeFunction::ArrayPrototypePush => {
            array::native_array_prototype_push(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeShift => array::native_array_prototype_shift(this_value),
        NativeFunction::ArrayPrototypeSlice => {
            array::native_array_prototype_slice(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeToString => {
            array::native_array_prototype_to_string(this_value)
        }
        NativeFunction::ArrayPrototypeUnshift => {
            array::native_array_prototype_unshift(this_value, &argument_values)
        }
        NativeFunction::Boolean => {
            native_boolean(function, this_value, &argument_values, is_construct)
        }
        NativeFunction::BooleanPrototypeToString => native_boolean_prototype_to_string(this_value),
        NativeFunction::BooleanPrototypeValueOf => native_boolean_prototype_value_of(this_value),
        NativeFunction::MathAbs => native_math_unary(&argument_values, f64::abs),
        NativeFunction::MathAcos => native_math_unary(&argument_values, f64::acos),
        NativeFunction::MathAcosh => native_math_unary(&argument_values, f64::acosh),
        NativeFunction::MathAsin => native_math_unary(&argument_values, f64::asin),
        NativeFunction::MathAsinh => native_math_unary(&argument_values, f64::asinh),
        NativeFunction::MathAtan => native_math_unary(&argument_values, f64::atan),
        NativeFunction::MathAtan2 => native_math_atan2(&argument_values),
        NativeFunction::MathAtanh => native_math_unary(&argument_values, f64::atanh),
        NativeFunction::MathCbrt => native_math_unary(&argument_values, f64::cbrt),
        NativeFunction::MathCeil => native_math_unary(&argument_values, f64::ceil),
        NativeFunction::MathClz32 => native_math_clz32(&argument_values),
        NativeFunction::MathCos => native_math_unary(&argument_values, f64::cos),
        NativeFunction::MathCosh => native_math_unary(&argument_values, f64::cosh),
        NativeFunction::MathExp => native_math_unary(&argument_values, f64::exp),
        NativeFunction::MathExpm1 => native_math_unary(&argument_values, f64::exp_m1),
        NativeFunction::MathFloor => native_math_unary(&argument_values, f64::floor),
        NativeFunction::MathFround => native_math_fround(&argument_values),
        NativeFunction::MathHypot => native_math_hypot(&argument_values),
        NativeFunction::MathImul => native_math_imul(&argument_values),
        NativeFunction::MathLog => native_math_unary(&argument_values, f64::ln),
        NativeFunction::MathLog1p => native_math_unary(&argument_values, f64::ln_1p),
        NativeFunction::MathLog10 => native_math_unary(&argument_values, f64::log10),
        NativeFunction::MathLog2 => native_math_unary(&argument_values, f64::log2),
        NativeFunction::MathMax => native_math_max(&argument_values),
        NativeFunction::MathMin => native_math_min(&argument_values),
        NativeFunction::MathPow => native_math_pow(&argument_values),
        NativeFunction::MathRound => native_math_round(&argument_values),
        NativeFunction::MathSign => native_math_sign(&argument_values),
        NativeFunction::MathSin => native_math_unary(&argument_values, f64::sin),
        NativeFunction::MathSinh => native_math_unary(&argument_values, f64::sinh),
        NativeFunction::MathSqrt => native_math_unary(&argument_values, f64::sqrt),
        NativeFunction::MathTan => native_math_unary(&argument_values, f64::tan),
        NativeFunction::MathTanh => native_math_unary(&argument_values, f64::tanh),
        NativeFunction::MathTrunc => native_math_unary(&argument_values, f64::trunc),
        NativeFunction::GlobalIsFinite => native_global_is_finite(&argument_values),
        NativeFunction::GlobalIsNaN => native_global_is_nan(&argument_values),
        NativeFunction::Number => {
            native_number(function, this_value, &argument_values, is_construct)
        }
        NativeFunction::NumberIsFinite => native_number_is_finite(&argument_values),
        NativeFunction::NumberIsInteger => native_number_is_integer(&argument_values),
        NativeFunction::NumberIsNaN => native_number_is_nan(&argument_values),
        NativeFunction::NumberIsSafeInteger => native_number_is_safe_integer(&argument_values),
        NativeFunction::NumberPrototypeToString => {
            native_number_prototype_to_string(this_value, &argument_values)
        }
        NativeFunction::NumberPrototypeValueOf => native_number_prototype_value_of(this_value),
        NativeFunction::ParseFloat => native_parse_float(&argument_values),
        NativeFunction::ParseInt => native_parse_int(&argument_values),
        NativeFunction::Object => {
            native_object(function, this_value, &argument_values, is_construct)
        }
        NativeFunction::ObjectAssign => native_object_assign(&argument_values),
        NativeFunction::ObjectCreate => native_object_create(&argument_values),
        NativeFunction::ObjectDefineProperties => native_object_define_properties(&argument_values),
        NativeFunction::ObjectDefineProperty => native_object_define_property(&argument_values),
        NativeFunction::ObjectGetOwnPropertyDescriptor => {
            native_object_get_own_property_descriptor(&argument_values, env)
        }
        NativeFunction::ObjectGetPrototypeOf => {
            native_object_get_prototype_of(&argument_values, env)
        }
        NativeFunction::ObjectGetOwnPropertyNames => {
            native_object_get_own_property_names(&argument_values)
        }
        NativeFunction::ObjectHasOwn => native_object_has_own(&argument_values),
        NativeFunction::ObjectKeys => native_object_keys(&argument_values),
        NativeFunction::ObjectPrototypeHasOwnProperty => {
            native_object_prototype_has_own_property(this_value, &argument_values)
        }
        NativeFunction::ObjectPrototypeIsPrototypeOf => {
            native_object_prototype_is_prototype_of(this_value, &argument_values, env)
        }
        NativeFunction::ObjectPrototypePropertyIsEnumerable => {
            native_object_prototype_property_is_enumerable(this_value, &argument_values)
        }
        NativeFunction::ObjectPrototypeToString => native_object_prototype_to_string(this_value),
        NativeFunction::ObjectPrototypeValueOf => native_object_prototype_value_of(this_value),
        NativeFunction::String => native_string(&argument_values),
        NativeFunction::StringFromCharCode => native_string_from_char_code(&argument_values),
        NativeFunction::StringPrototypeCharAt => {
            native_string_prototype_char_at(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeCharCodeAt => {
            native_string_prototype_char_code_at(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeCodePointAt => {
            native_string_prototype_code_point_at(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeConcat => {
            native_string_prototype_concat(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeEndsWith => {
            native_string_prototype_ends_with(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeIncludes => {
            native_string_prototype_includes(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeIndexOf => {
            native_string_prototype_index_of(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeLastIndexOf => {
            native_string_prototype_last_index_of(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypePadEnd => {
            native_string_prototype_pad(this_value, &argument_values, env, StringPadKind::End)
        }
        NativeFunction::StringPrototypePadStart => {
            native_string_prototype_pad(this_value, &argument_values, env, StringPadKind::Start)
        }
        NativeFunction::StringPrototypeRepeat => {
            native_string_prototype_repeat(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeSlice => {
            native_string_prototype_slice(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeSplit => {
            native_string_prototype_split(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeStartsWith => {
            native_string_prototype_starts_with(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeSubstring => {
            native_string_prototype_substring(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeToLowerCase => {
            native_string_prototype_to_lower_case(this_value, env)
        }
        NativeFunction::StringPrototypeTrim => native_string_prototype_trim(this_value, env),
        NativeFunction::StringPrototypeTrimEnd => native_string_prototype_trim_end(this_value, env),
        NativeFunction::StringPrototypeTrimStart => {
            native_string_prototype_trim_start(this_value, env)
        }
        NativeFunction::StringPrototypeToString | NativeFunction::StringPrototypeValueOf => {
            native_string_prototype_to_string(this_value, env)
        }
        NativeFunction::StringPrototypeToUpperCase => {
            native_string_prototype_to_upper_case(this_value, env)
        }
    }
}

fn native_boolean(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().is_some_and(is_truthy);
    if !is_construct {
        return Ok(Value::Boolean(value));
    }

    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    object.define_non_enumerable(BOOLEAN_DATA_PROPERTY.to_owned(), Value::Boolean(value));
    Ok(Value::Object(object))
}

fn native_boolean_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::String(if this_boolean_value(this_value)? {
        "true".to_owned()
    } else {
        "false".to_owned()
    }))
}

fn native_boolean_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(this_boolean_value(this_value)?))
}

fn this_boolean_value(value: Value) -> Result<bool, RuntimeError> {
    match value {
        Value::Boolean(value) => Ok(value),
        Value::Object(object) => match object.own_property(BOOLEAN_DATA_PROPERTY) {
            Some(Property {
                value: Value::Boolean(value),
                ..
            }) => Ok(value),
            _ => Err(RuntimeError {
                message: "Boolean.prototype method called on non-boolean object".to_owned(),
            }),
        },
        _ => Err(RuntimeError {
            message: "Boolean.prototype method called on non-boolean".to_owned(),
        }),
    }
}

fn native_math_unary(
    argument_values: &[Value],
    operation: fn(f64) -> f64,
) -> Result<Value, RuntimeError> {
    let argument = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Number(operation(to_number(argument)?)))
}

fn native_math_atan2(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let y = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let x = to_number(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::Number(y.atan2(x)))
}

fn native_math_fround(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let number = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::Number(f64::from(number as f32)))
}

fn native_math_hypot(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let mut sum = 0.0;
    for value in argument_values.iter().cloned() {
        let number = to_number(value)?;
        if number.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        if number.is_infinite() {
            return Ok(Value::Number(f64::INFINITY));
        }
        sum += number * number;
    }
    Ok(Value::Number(sum.sqrt()))
}

fn native_math_max(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    if argument_values.is_empty() {
        return Ok(Value::Number(f64::NEG_INFINITY));
    }

    let mut maximum = f64::NEG_INFINITY;
    for value in argument_values.iter().cloned() {
        let number = to_number(value)?;
        if number.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        if number > maximum || (number == 0.0 && maximum == 0.0 && number.is_sign_positive()) {
            maximum = number;
        }
    }
    Ok(Value::Number(maximum))
}

fn native_math_min(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    if argument_values.is_empty() {
        return Ok(Value::Number(f64::INFINITY));
    }

    let mut minimum = f64::INFINITY;
    for value in argument_values.iter().cloned() {
        let number = to_number(value)?;
        if number.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        if number < minimum || (number == 0.0 && minimum == 0.0 && number.is_sign_negative()) {
            minimum = number;
        }
    }
    Ok(Value::Number(minimum))
}

fn native_math_pow(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let base = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let exponent = to_number(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::Number(base.powf(exponent)))
}

fn native_math_round(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let number = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    if number.is_nan() || number.is_infinite() || number == 0.0 {
        return Ok(Value::Number(number));
    }

    let rounded = (number + 0.5).floor();
    if rounded == 0.0 && number < 0.0 {
        Ok(Value::Number(-0.0))
    } else {
        Ok(Value::Number(rounded))
    }
}

fn native_math_sign(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let number = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    if number.is_nan() || number == 0.0 {
        Ok(Value::Number(number))
    } else if number.is_sign_negative() {
        Ok(Value::Number(-1.0))
    } else {
        Ok(Value::Number(1.0))
    }
}

fn native_math_clz32(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let number = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::Number(f64::from(
        to_uint32_number(number).leading_zeros(),
    )))
}

fn native_math_imul(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let left = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let right = to_number(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    let product = to_uint32_number(left).wrapping_mul(to_uint32_number(right));
    Ok(Value::Number(f64::from(product as i32)))
}

fn native_number(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    let number = match argument_values.first() {
        Some(value) => to_number(value.clone())?,
        None => 0.0,
    };
    if !is_construct {
        return Ok(Value::Number(number));
    }

    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    object.define_non_enumerable(NUMBER_DATA_PROPERTY.to_owned(), Value::Number(number));
    Ok(Value::Object(object))
}

fn native_number_prototype_to_string(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    let radix =
        number_to_string_radix(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::String(number_to_radix_string(number, radix)?))
}

fn native_number_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Number(this_number_value(this_value)?))
}

fn this_number_value(value: Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(value) => Ok(value),
        Value::Object(object) => match object.own_property(NUMBER_DATA_PROPERTY) {
            Some(Property {
                value: Value::Number(value),
                ..
            }) => Ok(value),
            _ => Err(RuntimeError {
                message: "Number.prototype method called on non-number object".to_owned(),
            }),
        },
        _ => Err(RuntimeError {
            message: "Number.prototype method called on non-number".to_owned(),
        }),
    }
}

fn number_to_string_radix(value: Value) -> Result<u32, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(10);
    }
    let radix = to_int32(value)?;
    if !(2..=36).contains(&radix) {
        return Err(RuntimeError {
            message: "radix must be between 2 and 36".to_owned(),
        });
    }
    Ok(radix as u32)
}

fn number_to_radix_string(number: f64, radix: u32) -> Result<String, RuntimeError> {
    if radix == 10 || !number.is_finite() {
        return Ok(number_to_js_string(number));
    }
    if number.fract() != 0.0 {
        return Err(RuntimeError {
            message: "non-decimal number formatting supports integers only".to_owned(),
        });
    }

    let sign = if number < 0.0 { "-" } else { "" };
    let mut integer = number.abs() as u128;
    if integer == 0 {
        return Ok("0".to_owned());
    }

    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut output = Vec::new();
    while integer > 0 {
        let digit = (integer % u128::from(radix)) as usize;
        output.push(DIGITS[digit] as char);
        integer /= u128::from(radix);
    }
    output.reverse();
    Ok(format!("{sign}{}", output.into_iter().collect::<String>()))
}

fn native_number_is_finite(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number)) if number.is_finite()
    )))
}

fn native_number_is_integer(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number)) if number.is_finite() && number.fract() == 0.0
    )))
}

fn native_number_is_nan(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number)) if number.is_nan()
    )))
}

fn native_number_is_safe_integer(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number))
            if number.is_finite() && number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER
    )))
}

fn native_global_is_finite(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(to_number(value)?.is_finite()))
}

fn native_global_is_nan(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(to_number(value)?.is_nan()))
}

fn native_parse_float(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let input = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::Number(parse_float_string(&input)))
}

fn parse_float_string(input: &str) -> f64 {
    let input = input.trim_start();
    if input.starts_with("Infinity") {
        return f64::INFINITY;
    }
    if input.starts_with("+Infinity") {
        return f64::INFINITY;
    }
    if input.starts_with("-Infinity") {
        return f64::NEG_INFINITY;
    }

    let bytes = input.as_bytes();
    let mut end = 0;
    if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
        end = 1;
    }

    let mut digits_before_dot = 0usize;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        digits_before_dot += 1;
        end += 1;
    }

    let mut digits_after_dot = 0usize;
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            digits_after_dot += 1;
            end += 1;
        }
    }

    if digits_before_dot + digits_after_dot == 0 {
        return f64::NAN;
    }

    let exponent_marker = end;
    if matches!(bytes.get(end), Some(b'e') | Some(b'E')) {
        let mut exponent_end = end + 1;
        if matches!(bytes.get(exponent_end), Some(b'+') | Some(b'-')) {
            exponent_end += 1;
        }
        let exponent_digits_start = exponent_end;
        while bytes.get(exponent_end).is_some_and(u8::is_ascii_digit) {
            exponent_end += 1;
        }
        if exponent_end > exponent_digits_start {
            end = exponent_end;
        } else {
            end = exponent_marker;
        }
    }

    input[..end].parse::<f64>().unwrap_or(f64::NAN)
}

fn native_parse_int(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let input = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let radix = argument_values
        .get(1)
        .cloned()
        .map(to_int32)
        .transpose()?
        .unwrap_or(0);
    Ok(Value::Number(parse_int_string(&input, radix)))
}

fn parse_int_string(input: &str, radix: i32) -> f64 {
    let mut input = input.trim_start();
    let mut sign = 1.0;
    if let Some(rest) = input.strip_prefix('-') {
        sign = -1.0;
        input = rest;
    } else if let Some(rest) = input.strip_prefix('+') {
        input = rest;
    }

    let mut radix = radix;
    if radix != 0 && !(2..=36).contains(&radix) {
        return f64::NAN;
    }

    if radix == 0 {
        if let Some(rest) = input
            .strip_prefix("0x")
            .or_else(|| input.strip_prefix("0X"))
        {
            input = rest;
            radix = 16;
        } else {
            radix = 10;
        }
    } else if radix == 16 {
        if let Some(rest) = input
            .strip_prefix("0x")
            .or_else(|| input.strip_prefix("0X"))
        {
            input = rest;
        }
    }

    let radix = radix as u32;
    let mut value = 0.0;
    let mut digits = 0usize;
    for character in input.chars() {
        let Some(digit) = character.to_digit(36) else {
            break;
        };
        if digit >= radix {
            break;
        }
        value = value * f64::from(radix) + f64::from(digit);
        digits += 1;
    }

    if digits == 0 { f64::NAN } else { sign * value }
}

fn native_string(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    match argument_values.first().cloned() {
        Some(value) => Ok(Value::String(to_js_string(value)?)),
        None => Ok(Value::String(String::new())),
    }
}

fn native_string_from_char_code(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let mut result = String::new();
    for value in argument_values.iter().cloned() {
        let code_unit = to_uint16(value)?;
        match char::from_u32(u32::from(code_unit)) {
            Some(character) => result.push(character),
            None => result.push(char::REPLACEMENT_CHARACTER),
        }
    }
    Ok(Value::String(result))
}

fn native_string_prototype_char_at(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let index = to_string_position(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::String(
        value
            .chars()
            .nth(index)
            .map(|character| character.to_string())
            .unwrap_or_default(),
    ))
}

fn native_string_prototype_char_code_at(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let position =
        to_char_code_position(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    if position < 0.0 {
        return Ok(Value::Number(f64::NAN));
    }

    let code_units: Vec<u16> = value.encode_utf16().collect();
    let index = position as usize;
    Ok(code_units
        .get(index)
        .map(|code_unit| Value::Number(f64::from(*code_unit)))
        .unwrap_or(Value::Number(f64::NAN)))
}

fn native_string_prototype_code_point_at(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let position =
        to_char_code_position(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    if position < 0.0 || !position.is_finite() {
        return Ok(Value::Undefined);
    }

    let code_units: Vec<u16> = value.encode_utf16().collect();
    let index = position as usize;
    let Some(first) = code_units.get(index).copied() else {
        return Ok(Value::Undefined);
    };
    if !(0xD800..=0xDBFF).contains(&first) || index + 1 == code_units.len() {
        return Ok(Value::Number(f64::from(first)));
    }

    let second = code_units[index + 1];
    if !(0xDC00..=0xDFFF).contains(&second) {
        return Ok(Value::Number(f64::from(first)));
    }

    let code_point = (u32::from(first) - 0xD800) * 1024 + (u32::from(second) - 0xDC00) + 0x10000;
    Ok(Value::Number(f64::from(code_point)))
}

fn native_string_prototype_concat(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let mut result = this_string_value(this_value, env)?;
    for value in argument_values.iter().cloned() {
        result.push_str(&to_js_string(value)?);
    }
    Ok(Value::String(result))
}

fn native_string_prototype_ends_with(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let end = string_end_position(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    let prefix = value.chars().take(end).collect::<String>();
    Ok(Value::Boolean(prefix.ends_with(&search)))
}

fn native_string_prototype_includes(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let start = string_search_start(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    Ok(Value::Boolean(
        value
            .chars()
            .skip(start)
            .collect::<String>()
            .contains(&search),
    ))
}

fn native_string_prototype_index_of(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let start = string_search_start(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    let haystack = value.chars().skip(start).collect::<String>();
    let Some(byte_index) = haystack.find(&search) else {
        return Ok(Value::Number(-1.0));
    };
    let char_offset = haystack[..byte_index].chars().count();
    Ok(Value::Number((start + char_offset) as f64))
}

fn native_string_prototype_last_index_of(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let chars: Vec<_> = value.chars().collect();
    let search_chars: Vec<_> = search.chars().collect();
    let position = string_last_search_position(
        chars.len(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;

    if search_chars.is_empty() {
        return Ok(Value::Number(position as f64));
    }
    if search_chars.len() > chars.len() {
        return Ok(Value::Number(-1.0));
    }

    let start = position.min(chars.len() - search_chars.len());
    for candidate in (0..=start).rev() {
        if chars[candidate..candidate + search_chars.len()] == search_chars {
            return Ok(Value::Number(candidate as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

#[derive(Clone, Copy)]
enum StringPadKind {
    Start,
    End,
}

fn native_string_prototype_pad(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
    kind: StringPadKind,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let max_length = to_length(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let string_length = value.chars().count();
    if max_length <= string_length {
        return Ok(Value::String(value));
    }

    let fill_string = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => " ".to_owned(),
        value => to_js_string(value)?,
    };
    if fill_string.is_empty() {
        return Ok(Value::String(value));
    }

    let fill_length = max_length - string_length;
    let filler = repeated_prefix(&fill_string, fill_length);
    Ok(Value::String(match kind {
        StringPadKind::Start => format!("{filler}{value}"),
        StringPadKind::End => format!("{value}{filler}"),
    }))
}

fn repeated_prefix(pattern: &str, length: usize) -> String {
    pattern.chars().cycle().take(length).collect()
}

fn native_string_prototype_repeat(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let count = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    if count.is_infinite() || count < 0.0 {
        return Err(RuntimeError {
            message: "repeat count must be a finite non-negative number".to_owned(),
        });
    }
    if count.is_nan() || count == 0.0 {
        return Ok(Value::String(String::new()));
    }

    let count = count.trunc() as usize;
    Ok(Value::String(value.repeat(count)))
}

fn native_string_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let chars: Vec<_> = value.chars().collect();
    let length = chars.len();
    let start = string_slice_index(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        0,
    )?;
    let end = string_slice_index(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
    )?;
    if end <= start {
        return Ok(Value::String(String::new()));
    }
    Ok(Value::String(chars[start..end].iter().collect()))
}

fn native_string_prototype_split(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let limit = string_split_limit(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    if limit == 0 {
        return Ok(Value::Array(ArrayRef::new(Vec::new())));
    }
    if matches!(argument_values.first(), None | Some(Value::Undefined)) {
        return Ok(Value::Array(ArrayRef::new(vec![Value::String(value)])));
    }

    let separator = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let parts = if separator.is_empty() {
        value
            .chars()
            .take(limit)
            .map(|character| Value::String(character.to_string()))
            .collect()
    } else {
        value
            .split(&separator)
            .take(limit)
            .map(|part| Value::String(part.to_owned()))
            .collect()
    };
    Ok(Value::Array(ArrayRef::new(parts)))
}

fn string_split_limit(value: Value) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(u32::MAX as usize);
    }
    Ok(to_uint32(value)? as usize)
}

fn native_string_prototype_starts_with(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let start = string_search_start(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    Ok(Value::Boolean(
        value
            .chars()
            .skip(start)
            .collect::<String>()
            .starts_with(&search),
    ))
}

fn native_string_prototype_substring(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let chars: Vec<_> = value.chars().collect();
    let length = chars.len();
    let start = string_substring_index(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        0,
    )?;
    let end = string_substring_index(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
    )?;
    let (from, to) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    Ok(Value::String(chars[from..to].iter().collect()))
}

fn native_string_prototype_to_lower_case(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.to_lowercase(),
    ))
}

fn native_string_prototype_trim(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.trim().to_owned(),
    ))
}

fn native_string_prototype_trim_end(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.trim_end().to_owned(),
    ))
}

fn native_string_prototype_trim_start(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.trim_start().to_owned(),
    ))
}

fn native_string_prototype_to_string(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(this_string_value(this_value, env)?))
}

fn native_string_prototype_to_upper_case(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.to_uppercase(),
    ))
}

fn this_string_value(value: Value, env: &HashMap<String, Value>) -> Result<String, RuntimeError> {
    match value {
        Value::String(value) => Ok(value),
        Value::Object(object) => {
            if string_prototype(env).is_some_and(|prototype| object.ptr_eq(&prototype)) {
                Ok(String::new())
            } else {
                Err(RuntimeError {
                    message: "String.prototype method called on non-string object".to_owned(),
                })
            }
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "String.prototype method called on null or undefined".to_owned(),
        }),
        value => to_js_string(value),
    }
}

fn to_string_position(value: Value) -> Result<usize, RuntimeError> {
    let number = to_number(value)?;
    if !number.is_finite() || number <= 0.0 {
        Ok(0)
    } else {
        Ok(number.trunc() as usize)
    }
}

fn to_char_code_position(value: Value) -> Result<f64, RuntimeError> {
    let number = to_number(value)?;
    if number.is_nan() {
        Ok(0.0)
    } else {
        Ok(number.trunc())
    }
}

fn string_search_start(length: usize, value: Value) -> Result<usize, RuntimeError> {
    Ok(to_string_position(value)?.min(length))
}

fn string_last_search_position(length: usize, value: Value) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(length);
    }
    let number = to_number(value)?;
    if number.is_nan() || number <= 0.0 {
        Ok(0)
    } else if number.is_infinite() {
        Ok(length)
    } else {
        Ok((number.trunc() as usize).min(length))
    }
}

fn string_end_position(length: usize, value: Value) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(length);
    }
    Ok(to_string_position(value)?.min(length))
}

fn string_slice_index(length: usize, value: Value, default: usize) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(default);
    }
    let number = to_number(value)?;
    if number.is_nan() {
        return Ok(0);
    }
    let integer = number.trunc();
    if integer < 0.0 {
        Ok((length as f64 + integer).max(0.0) as usize)
    } else {
        Ok(integer.min(length as f64) as usize)
    }
}

fn string_substring_index(
    length: usize,
    value: Value,
    default: usize,
) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(default);
    }
    let number = to_number(value)?;
    if number.is_nan() || number <= 0.0 {
        Ok(0)
    } else {
        Ok(number.trunc().min(length as f64) as usize)
    }
}

fn native_object_assign(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    match target {
        Value::Object(_) | Value::Function(_) => {}
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                message: "Object.assign target must not be null or undefined".to_owned(),
            });
        }
        Value::Array(_) | Value::String(_) | Value::Number(_) | Value::Boolean(_) => {
            return Err(RuntimeError {
                message: "Object.assign primitive targets are not implemented".to_owned(),
            });
        }
    }

    for source in argument_values.iter().skip(1).cloned() {
        if matches!(source, Value::Null | Value::Undefined) {
            continue;
        }
        for (key, value) in enumerable_property_entries(source)? {
            set_property(target.clone(), key, value)?;
        }
    }
    Ok(target)
}

fn enumerable_property_entries(value: Value) -> Result<Vec<(String, Value)>, RuntimeError> {
    let keys = match value.clone() {
        Value::Object(object) => object.own_property_keys(),
        Value::Array(elements) => array_own_property_keys(&elements),
        Value::Function(function) => function_own_property_keys(&function),
        Value::String(value) => string_own_property_keys(&value),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Vec::new(),
    };
    let mut entries = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(property) = own_property_descriptor(value.clone(), &key)? {
            entries.push((key, property.value));
        }
    }
    Ok(entries)
}

fn set_property(target: Value, key: String, value: Value) -> Result<(), RuntimeError> {
    match target {
        Value::Object(object) => {
            object.set(key, value);
            Ok(())
        }
        Value::Function(function) => {
            function
                .properties
                .borrow_mut()
                .insert(key, Property::enumerable(value));
            Ok(())
        }
        _ => Err(RuntimeError {
            message: "property target is not mutable".to_owned(),
        }),
    }
}

fn native_object(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::Array(_) | Value::Function(_) | Value::Object(_)) => {
            Ok(argument_values[0].clone())
        }
        _ if is_construct => Ok(this_value),
        _ => Ok(Value::Object(ObjectRef::with_prototype(
            HashMap::new(),
            function_prototype(function),
        ))),
    }
}

fn native_object_create(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let object = match argument_values.first() {
        Some(Value::Object(prototype)) => Value::Object(ObjectRef::with_prototype(
            HashMap::new(),
            Some(prototype.clone()),
        )),
        Some(Value::Null) => Value::Object(ObjectRef::new(HashMap::new())),
        _ => {
            return Err(RuntimeError {
                message: "Object.create prototype must be an object or null".to_owned(),
            });
        }
    };

    if !matches!(argument_values.get(1), None | Some(Value::Undefined)) {
        native_object_define_properties(&[
            object.clone(),
            argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        ])?;
    }
    Ok(object)
}

fn native_object_define_property(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    let descriptor =
        to_property_descriptor(argument_values.get(2).cloned().unwrap_or(Value::Undefined))?;

    define_property_on_value(target.clone(), key, descriptor)?;
    Ok(target)
}

fn native_object_define_properties(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_define_property_target(&target)?;

    let descriptors = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !matches!(descriptors, Value::Object(_) | Value::Function(_)) {
        return Err(RuntimeError {
            message: "property descriptors must be an object".to_owned(),
        });
    }

    for (key, descriptor_value) in enumerable_property_entries(descriptors)? {
        let descriptor = to_property_descriptor(descriptor_value)?;
        define_property_on_value(target.clone(), key, descriptor)?;
    }
    Ok(target)
}

fn define_property_on_value(
    target: Value,
    key: String,
    descriptor: Property,
) -> Result<(), RuntimeError> {
    match &target {
        Value::Object(object) => {
            object.define_property(key, descriptor);
            Ok(())
        }
        Value::Function(function) => {
            function.properties.borrow_mut().insert(key, descriptor);
            Ok(())
        }
        _ => ensure_define_property_target(&target),
    }
}

fn ensure_define_property_target(target: &Value) -> Result<(), RuntimeError> {
    match target {
        Value::Object(_) | Value::Function(_) => Ok(()),
        Value::Array(_) | Value::String(_) | Value::Number(_) | Value::Boolean(_) => {
            Err(RuntimeError {
                message: "Object.defineProperty primitive targets are not implemented".to_owned(),
            })
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "Object.defineProperty target must be an object".to_owned(),
        }),
    }
}

fn to_property_descriptor(value: Value) -> Result<Property, RuntimeError> {
    let Value::Object(descriptor) = value else {
        return Err(RuntimeError {
            message: "property descriptor must be an object".to_owned(),
        });
    };

    if descriptor.contains_property("get") || descriptor.contains_property("set") {
        return Err(RuntimeError {
            message: "accessor property descriptors are not implemented".to_owned(),
        });
    }

    Ok(Property {
        value: descriptor.get("value").unwrap_or(Value::Undefined),
        writable: descriptor
            .get("writable")
            .is_some_and(|value| is_truthy(&value)),
        enumerable: descriptor
            .get("enumerable")
            .is_some_and(|value| is_truthy(&value)),
        configurable: descriptor
            .get("configurable")
            .is_some_and(|value| is_truthy(&value)),
    })
}

fn native_object_get_prototype_of(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::Object(object)) => {
            Ok(object.prototype().map(Value::Object).unwrap_or(Value::Null))
        }
        Some(Value::Array(_)) => Ok(array_prototype(env)
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Function(_)) => Ok(Value::Null),
        _ => Err(RuntimeError {
            message: "Object.getPrototypeOf target must be an object".to_owned(),
        }),
    }
}

fn native_object_get_own_property_descriptor(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    let Some(property) = own_property_descriptor(target, &key)? else {
        return Ok(Value::Undefined);
    };
    Ok(Value::Object(property_descriptor_object(
        property,
        object_prototype(env),
    )))
}

fn own_property_descriptor(value: Value, key: &str) -> Result<Option<Property>, RuntimeError> {
    match value {
        Value::Object(object) => Ok(object.own_property(key)),
        Value::Function(function) => Ok(function_own_property_descriptor(&function, key)),
        Value::Array(elements) => Ok(array_own_property_descriptor(&elements, key)),
        Value::String(value) => Ok(string_own_property_descriptor(&value, key)),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Ok(None),
    }
}

fn property_descriptor_object(property: Property, prototype: Option<ObjectRef>) -> ObjectRef {
    ObjectRef::with_prototype(
        HashMap::from([
            ("value".to_owned(), property.value),
            ("writable".to_owned(), Value::Boolean(property.writable)),
            ("enumerable".to_owned(), Value::Boolean(property.enumerable)),
            (
                "configurable".to_owned(),
                Value::Boolean(property.configurable),
            ),
        ]),
        prototype,
    )
}

fn native_object_keys(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let keys = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Object(object) => object.own_property_keys(),
        Value::Array(elements) => array_own_property_keys(&elements),
        Value::Function(function) => function_own_property_keys(&function),
        Value::String(value) => string_own_property_keys(&value),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Vec::new(),
    };
    Ok(Value::Array(ArrayRef::new(
        keys.into_iter().map(Value::String).collect(),
    )))
}

fn native_object_get_own_property_names(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let names = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Object(object) => object.own_property_names(),
        Value::Array(elements) => array_own_property_names(&elements),
        Value::Function(function) => function_own_property_names(&function),
        Value::String(value) => string_own_property_names(&value),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Vec::new(),
    };
    Ok(Value::Array(ArrayRef::new(
        names.into_iter().map(Value::String).collect(),
    )))
}

fn native_object_has_own(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            message: "Object.hasOwn target must not be null or undefined".to_owned(),
        });
    }

    let key = to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::Boolean(
        own_property_descriptor(target, &key)?.is_some(),
    ))
}

fn native_object_prototype_has_own_property(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let key = to_property_key(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    match this_value {
        Value::Object(object) => Ok(Value::Boolean(object.has_own_property(&key))),
        Value::Function(function) => Ok(Value::Boolean(
            function_own_property_descriptor(&function, &key).is_some(),
        )),
        Value::Array(elements) => Ok(Value::Boolean(array_has_own_property(&elements, &key))),
        Value::String(value) => Ok(Value::Boolean(string_has_own_property(&value, &key))),
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "hasOwnProperty called on null or undefined".to_owned(),
        }),
        Value::Number(_) | Value::Boolean(_) => Ok(Value::Boolean(false)),
    }
}

fn native_object_prototype_property_is_enumerable(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let key = to_property_key(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "propertyIsEnumerable called on null or undefined".to_owned(),
        }),
        value => Ok(Value::Boolean(
            own_property_descriptor(value, &key)?.is_some_and(|property| property.enumerable),
        )),
    }
}

fn native_object_prototype_is_prototype_of(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Some(target_prototype) = value_prototype(target, env) else {
        return Ok(Value::Boolean(false));
    };
    let Value::Object(prototype) = this_value else {
        return Err(RuntimeError {
            message: "isPrototypeOf called on non-object".to_owned(),
        });
    };
    Ok(Value::Boolean(
        target_prototype.ptr_eq(&prototype) || target_prototype.has_prototype(&prototype),
    ))
}

fn value_prototype(value: Value, env: &HashMap<String, Value>) -> Option<ObjectRef> {
    match value {
        Value::Object(object) => object.prototype(),
        Value::Array(_) => array_prototype(env),
        Value::Function(_) => object_prototype(env),
        Value::String(_) | Value::Number(_) | Value::Boolean(_) => None,
        Value::Null | Value::Undefined => None,
    }
}

fn native_object_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    let tag = match this_value {
        Value::Undefined => "Undefined",
        Value::Null => "Null",
        Value::Array(_) => "Array",
        Value::Function(_) => "Function",
        Value::String(_) => "String",
        Value::Number(_) => "Number",
        Value::Boolean(_) => "Boolean",
        Value::Object(object) => {
            if matches!(
                object.own_property(BOOLEAN_DATA_PROPERTY),
                Some(Property {
                    value: Value::Boolean(_),
                    ..
                })
            ) {
                "Boolean"
            } else if matches!(
                object.own_property(NUMBER_DATA_PROPERTY),
                Some(Property {
                    value: Value::Number(_),
                    ..
                })
            ) {
                "Number"
            } else {
                "Object"
            }
        }
    };
    Ok(Value::String(format!("[object {tag}]")))
}

fn native_object_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "valueOf called on null or undefined".to_owned(),
        }),
        _ => Ok(this_value),
    }
}

fn function_prototype(function: &Function) -> Option<ObjectRef> {
    match function.properties.borrow().get("prototype") {
        Some(Property {
            value: Value::Object(prototype),
            ..
        }) => Some(prototype.clone()),
        _ => None,
    }
}

fn eval_expr(expr: &Expr, env: &mut HashMap<String, Value>) -> Result<Value, RuntimeError> {
    match expr {
        Expr::Literal(literal) => eval_literal(literal),
        Expr::Array { elements, .. } => {
            let mut values = Vec::with_capacity(elements.len());
            for element in elements {
                values.push(eval_expr(element, env)?);
            }
            Ok(Value::Array(ArrayRef::new(values)))
        }
        Expr::Object { properties, .. } => {
            let mut values = HashMap::new();
            for property in properties {
                values.insert(property.key.clone(), eval_expr(&property.value, env)?);
            }
            Ok(Value::Object(ObjectRef::with_prototype(
                values,
                object_prototype(env),
            )))
        }
        Expr::Function {
            name, params, body, ..
        } => Ok(Value::Function(Function::new_user(
            name.clone(),
            params.clone(),
            body.clone(),
            env.clone(),
        ))),
        Expr::Sequence { expressions, .. } => {
            let mut last = Value::Undefined;
            for expression in expressions {
                last = eval_expr(expression, env)?;
            }
            Ok(last)
        }
        Expr::This { .. } => env.get("this").cloned().ok_or_else(|| RuntimeError {
            message: "missing this binding".to_owned(),
        }),
        Expr::Identifier { name, .. } => env.get(name).cloned().ok_or_else(|| RuntimeError {
            message: format!("undefined identifier `{name}`"),
        }),
        Expr::Unary {
            op: UnaryOp::Typeof,
            argument,
            ..
        } => eval_typeof(argument, env),
        Expr::Unary {
            op: UnaryOp::Delete,
            argument,
            ..
        } => eval_delete(argument, env),
        Expr::Unary { op, argument, .. } => {
            let argument = eval_expr(argument, env)?;
            eval_unary(*op, argument)
        }
        Expr::Assignment {
            target, op, value, ..
        } => eval_assignment(target, *op, value, env),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            let test = eval_expr(test, env)?;
            if is_truthy(&test) {
                eval_expr(consequent, env)
            } else {
                eval_expr(alternate, env)
            }
        }
        Expr::Update {
            target, op, prefix, ..
        } => eval_update(target, *op, *prefix, env),
        Expr::Call {
            callee, arguments, ..
        } => eval_call(callee, arguments, env),
        Expr::New {
            callee, arguments, ..
        } => eval_new(callee, arguments, env),
        Expr::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            eval_member(object, property, env)
        }
        Expr::Binary {
            left, op, right, ..
        } if *op == BinaryOp::LogicalAnd => {
            let left = eval_expr(left, env)?;
            if is_truthy(&left) {
                eval_expr(right, env)
            } else {
                Ok(left)
            }
        }
        Expr::Binary {
            left, op, right, ..
        } if *op == BinaryOp::LogicalOr => {
            let left = eval_expr(left, env)?;
            if is_truthy(&left) {
                Ok(left)
            } else {
                eval_expr(right, env)
            }
        }
        Expr::Binary {
            left, op, right, ..
        } if *op == BinaryOp::NullishCoalescing => {
            let left = eval_expr(left, env)?;
            if matches!(left, Value::Null | Value::Undefined) {
                eval_expr(right, env)
            } else {
                Ok(left)
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let left = eval_expr(left, env)?;
            let right = eval_expr(right, env)?;
            eval_binary(left, *op, right, env)
        }
    }
}

fn assign_target(
    target: &AssignmentTarget,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    match target {
        AssignmentTarget::Identifier { name, .. } => {
            if !env.contains_key(name) {
                return Err(RuntimeError {
                    message: format!("undefined identifier `{name}`"),
                });
            }
            env.insert(name.clone(), value);
            Ok(())
        }
        AssignmentTarget::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            assign_member(object, property, value, env)
        }
    }
}

fn read_target(
    target: &AssignmentTarget,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match target {
        AssignmentTarget::Identifier { name, .. } => {
            env.get(name).cloned().ok_or_else(|| RuntimeError {
                message: format!("undefined identifier `{name}`"),
            })
        }
        AssignmentTarget::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            eval_member(object, property, env)
        }
    }
}

fn eval_assignment(
    target: &AssignmentTarget,
    op: AssignmentOp,
    right: &Expr,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let old_value = match op {
        AssignmentOp::LogicalAndAssign
        | AssignmentOp::LogicalOrAssign
        | AssignmentOp::NullishAssign => read_target(target, env)?,
        _ => Value::Undefined,
    };

    match op {
        AssignmentOp::LogicalAndAssign if !is_truthy(&old_value) => return Ok(old_value),
        AssignmentOp::LogicalOrAssign if is_truthy(&old_value) => return Ok(old_value),
        AssignmentOp::NullishAssign if !matches!(old_value, Value::Null | Value::Undefined) => {
            return Ok(old_value);
        }
        _ => {}
    }

    let right = eval_expr(right, env)?;
    let value = match op {
        AssignmentOp::Assign => right,
        AssignmentOp::AddAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::Add, right, env)?
        }
        AssignmentOp::SubAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::Sub, right, env)?
        }
        AssignmentOp::MulAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::Mul, right, env)?
        }
        AssignmentOp::PowAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::Pow, right, env)?
        }
        AssignmentOp::DivAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::Div, right, env)?
        }
        AssignmentOp::RemAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::Rem, right, env)?
        }
        AssignmentOp::ShlAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::Shl, right, env)?
        }
        AssignmentOp::ShrAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::Shr, right, env)?
        }
        AssignmentOp::UShrAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::UShr, right, env)?
        }
        AssignmentOp::BitwiseAndAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::BitwiseAnd, right, env)?
        }
        AssignmentOp::BitwiseXorAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::BitwiseXor, right, env)?
        }
        AssignmentOp::BitwiseOrAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::BitwiseOr, right, env)?
        }
        AssignmentOp::LogicalAndAssign
        | AssignmentOp::LogicalOrAssign
        | AssignmentOp::NullishAssign => right,
    };
    assign_target(target, value.clone(), env)?;
    Ok(value)
}

fn eval_update(
    target: &AssignmentTarget,
    op: UpdateOp,
    prefix: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let old_number = to_number(read_target(target, env)?)?;
    let new = match op {
        UpdateOp::Increment => Value::Number(old_number + 1.0),
        UpdateOp::Decrement => Value::Number(old_number - 1.0),
    };
    assign_target(target, new.clone(), env)?;
    if prefix {
        Ok(new)
    } else {
        Ok(Value::Number(old_number))
    }
}

fn eval_literal(literal: &Literal) -> Result<Value, RuntimeError> {
    match literal {
        Literal::Number { raw, .. } => {
            raw.parse::<f64>()
                .map(Value::Number)
                .map_err(|_| RuntimeError {
                    message: format!("invalid number literal `{raw}`"),
                })
        }
        Literal::String { value, .. } => Ok(Value::String(value.clone())),
        Literal::Boolean { value, .. } => Ok(Value::Boolean(*value)),
        Literal::Null { .. } => Ok(Value::Null),
    }
}

fn eval_member(
    object: Value,
    property: &MemberProperty,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match (object, property) {
        (Value::Array(elements), MemberProperty::Named(name)) if name == "length" => {
            Ok(Value::Number(elements.len() as f64))
        }
        (Value::Array(_), MemberProperty::Named(name)) => {
            Ok(inherited_array_prototype_property(env, name).unwrap_or(Value::Undefined))
        }
        (Value::Function(function), MemberProperty::Named(name)) if name == "length" => {
            Ok(Value::Number(function.params.len() as f64))
        }
        (Value::Function(function), property) => {
            let key = property_key(property, env)?;
            Ok(function
                .properties
                .borrow()
                .get(&key)
                .map(|property| property.value.clone())
                .or_else(|| inherited_object_prototype_property(env, &key))
                .unwrap_or(Value::Undefined))
        }
        (Value::String(value), MemberProperty::Named(name)) if name == "length" => {
            Ok(Value::Number(value.chars().count() as f64))
        }
        (Value::String(value), property) => {
            let key = property_key(property, env)?;
            Ok(string_property(&value, &key)
                .or_else(|| inherited_string_prototype_property(env, &key))
                .unwrap_or(Value::Undefined))
        }
        (Value::Boolean(_), MemberProperty::Named(name)) => {
            Ok(inherited_boolean_prototype_property(env, name).unwrap_or(Value::Undefined))
        }
        (Value::Number(_), MemberProperty::Named(name)) => {
            Ok(inherited_number_prototype_property(env, name).unwrap_or(Value::Undefined))
        }
        (Value::Array(elements), MemberProperty::Computed(index)) => {
            let index = eval_expr(index, env)?;
            let index = to_array_index(index)?;
            Ok(elements.get(index).unwrap_or(Value::Undefined))
        }
        (Value::Object(object), property) => {
            let key = property_key(property, env)?;
            Ok(object.get(&key).unwrap_or(Value::Undefined))
        }
        (_, MemberProperty::Named(name)) => Err(RuntimeError {
            message: format!("unsupported property `{name}`"),
        }),
        (_, MemberProperty::Computed(_)) => Err(RuntimeError {
            message: "unsupported computed member access".to_owned(),
        }),
    }
}

fn object_prototype_property(env: &HashMap<String, Value>, key: &str) -> Option<Value> {
    object_prototype(env).and_then(|prototype| prototype.get(key))
}

fn inherited_object_prototype_property(env: &HashMap<String, Value>, key: &str) -> Option<Value> {
    if matches!(
        key,
        "hasOwnProperty" | "isPrototypeOf" | "propertyIsEnumerable"
    ) {
        object_prototype_property(env, key)
    } else {
        None
    }
}

fn inherited_array_prototype_property(env: &HashMap<String, Value>, key: &str) -> Option<Value> {
    array_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}

fn inherited_string_prototype_property(env: &HashMap<String, Value>, key: &str) -> Option<Value> {
    string_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}

fn inherited_boolean_prototype_property(env: &HashMap<String, Value>, key: &str) -> Option<Value> {
    boolean_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}

fn inherited_number_prototype_property(env: &HashMap<String, Value>, key: &str) -> Option<Value> {
    number_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}

fn assign_member(
    object: Value,
    property: &MemberProperty,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let key = property_key(property, env)?;
    match object {
        Value::Object(object) => {
            object.set(key, value);
            Ok(())
        }
        Value::Function(function) => {
            function
                .properties
                .borrow_mut()
                .insert(key, Property::enumerable(value));
            Ok(())
        }
        Value::Array(elements) => {
            if key == "length" {
                elements.set_len(to_length(value)?);
            } else {
                let index = key.parse::<usize>().map_err(|_| RuntimeError {
                    message: "array property assignment requires an array index".to_owned(),
                })?;
                elements.set(index, value);
            }
            Ok(())
        }
        _ => Err(RuntimeError {
            message: "member assignment target is not an object".to_owned(),
        }),
    }
}

fn property_key(
    property: &MemberProperty,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    match property {
        MemberProperty::Named(name) => Ok(name.clone()),
        MemberProperty::Computed(expr) => to_property_key(eval_expr(expr, env)?),
    }
}

fn to_property_key(value: Value) -> Result<String, RuntimeError> {
    match value {
        Value::String(value) => Ok(value),
        Value::Number(number) if number.fract() == 0.0 => Ok(format!("{number:.0}")),
        Value::Number(number) => Ok(number.to_string()),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Function(_) | Value::Array(_) | Value::Object(_) => Err(RuntimeError {
            message: "unsupported property key".to_owned(),
        }),
    }
}

fn string_property(value: &str, key: &str) -> Option<Value> {
    let index = canonical_string_index(key)?;
    value
        .chars()
        .nth(index)
        .map(|character| Value::String(character.to_string()))
}

fn string_has_own_property(value: &str, key: &str) -> bool {
    key == "length"
        || canonical_string_index(key).is_some_and(|index| index < value.chars().count())
}

fn string_own_property_descriptor(value: &str, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property {
            value: Value::Number(value.chars().count() as f64),
            enumerable: false,
            writable: false,
            configurable: false,
        });
    }
    string_property(value, key).map(Property::enumerable)
}

fn string_own_property_keys(value: &str) -> Vec<String> {
    (0..value.chars().count())
        .map(|index| index.to_string())
        .collect()
}

fn string_own_property_names(value: &str) -> Vec<String> {
    let mut names = string_own_property_keys(value);
    names.push("length".to_owned());
    names
}

fn canonical_string_index(key: &str) -> Option<usize> {
    if key.is_empty() {
        return None;
    }

    let index = key.parse::<usize>().ok()?;
    if index.to_string() == key {
        Some(index)
    } else {
        None
    }
}

fn array_has_own_property(elements: &ArrayRef, key: &str) -> bool {
    key == "length"
        || key
            .parse::<usize>()
            .is_ok_and(|index| index < elements.len())
}

fn array_own_property_descriptor(elements: &ArrayRef, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property {
            value: Value::Number(elements.len() as f64),
            enumerable: false,
            writable: true,
            configurable: false,
        });
    }
    let index = key.parse::<usize>().ok()?;
    elements.get(index).map(Property::enumerable)
}

fn array_own_property_keys(elements: &ArrayRef) -> Vec<String> {
    (0..elements.len()).map(|index| index.to_string()).collect()
}

fn array_own_property_names(elements: &ArrayRef) -> Vec<String> {
    let mut names = array_own_property_keys(elements);
    names.push("length".to_owned());
    names
}

fn function_own_property_keys(function: &Function) -> Vec<String> {
    let mut keys: Vec<_> = function
        .properties
        .borrow()
        .iter()
        .filter(|(_, property)| property.enumerable)
        .map(|(key, _)| key.clone())
        .collect();
    keys.sort();
    keys
}

fn function_own_property_descriptor(function: &Function, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property {
            value: Value::Number(function.params.len() as f64),
            enumerable: false,
            writable: false,
            configurable: true,
        });
    }
    function.properties.borrow().get(key).cloned()
}

fn function_own_property_names(function: &Function) -> Vec<String> {
    let mut names: Vec<_> = function.properties.borrow().keys().cloned().collect();
    names.push("length".to_owned());
    names.sort();
    names
}

fn to_array_index(value: Value) -> Result<usize, RuntimeError> {
    let number = to_number(value)?;
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
        return Err(RuntimeError {
            message: "array index must be a non-negative integer".to_owned(),
        });
    }
    Ok(number as usize)
}

fn eval_unary(op: UnaryOp, argument: Value) -> Result<Value, RuntimeError> {
    match op {
        UnaryOp::Not => Ok(Value::Boolean(!is_truthy(&argument))),
        UnaryOp::Plus => Ok(Value::Number(to_number(argument)?)),
        UnaryOp::Minus => Ok(Value::Number(-to_number(argument)?)),
        UnaryOp::BitwiseNot => Ok(Value::Number(f64::from(!to_int32(argument)?))),
        UnaryOp::Void => Ok(Value::Undefined),
        UnaryOp::Typeof | UnaryOp::Delete => {
            unreachable!("operator requires unevaluated operand handling")
        }
    }
}

fn eval_delete(expr: &Expr, env: &mut HashMap<String, Value>) -> Result<Value, RuntimeError> {
    let Expr::Member {
        object, property, ..
    } = expr
    else {
        return Ok(Value::Boolean(true));
    };

    let object = eval_expr(object, env)?;
    match object {
        Value::Object(object) => {
            let key = property_key(property, env)?;
            object.delete_own_property(&key);
            Ok(Value::Boolean(true))
        }
        Value::Array(_) => Ok(Value::Boolean(true)),
        _ => Err(RuntimeError {
            message: "delete target is not an object".to_owned(),
        }),
    }
}

fn eval_typeof(expr: &Expr, env: &mut HashMap<String, Value>) -> Result<Value, RuntimeError> {
    let value = match expr {
        Expr::Identifier { name, .. } => env.get(name).cloned().unwrap_or(Value::Undefined),
        _ => eval_expr(expr, env)?,
    };
    let type_name = match value {
        Value::Undefined => "undefined",
        Value::Boolean(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Function(_) => "function",
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
    };
    Ok(Value::String(type_name.to_owned()))
}

fn eval_binary(
    left: Value,
    op: BinaryOp,
    right: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if op == BinaryOp::In {
        return eval_in(left, right);
    }
    if op == BinaryOp::Instanceof {
        return eval_instanceof(left, right, env);
    }

    match op {
        BinaryOp::Eq | BinaryOp::StrictEq => return Ok(Value::Boolean(left == right)),
        BinaryOp::Ne | BinaryOp::StrictNe => return Ok(Value::Boolean(left != right)),
        BinaryOp::Add if matches!(left, Value::String(_)) || matches!(right, Value::String(_)) => {
            return Ok(Value::String(format!(
                "{}{}",
                to_js_string(left)?,
                to_js_string(right)?
            )));
        }
        _ => {}
    }

    let left = to_number(left)?;
    let right = to_number(right)?;

    let value = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Pow => left.powf(right),
        BinaryOp::Div => left / right,
        BinaryOp::Rem => left % right,
        BinaryOp::Shl => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) << (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::Shr => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) >> (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::UShr => {
            return Ok(Value::Number(f64::from(
                to_uint32_number(left) >> (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::BitwiseAnd => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) & to_int32_number(right),
            )));
        }
        BinaryOp::BitwiseXor => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) ^ to_int32_number(right),
            )));
        }
        BinaryOp::BitwiseOr => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) | to_int32_number(right),
            )));
        }
        BinaryOp::Lt => return Ok(Value::Boolean(left < right)),
        BinaryOp::Le => return Ok(Value::Boolean(left <= right)),
        BinaryOp::Gt => return Ok(Value::Boolean(left > right)),
        BinaryOp::Ge => return Ok(Value::Boolean(left >= right)),
        BinaryOp::Eq
        | BinaryOp::StrictEq
        | BinaryOp::Ne
        | BinaryOp::StrictNe
        | BinaryOp::In
        | BinaryOp::Instanceof
        | BinaryOp::LogicalAnd
        | BinaryOp::LogicalOr
        | BinaryOp::NullishCoalescing => unreachable!("handled before numeric binary evaluation"),
    };
    Ok(Value::Number(value))
}

fn eval_instanceof(
    left: Value,
    right: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Function(constructor) = right else {
        return Err(RuntimeError {
            message: "right-hand side of instanceof is not callable".to_owned(),
        });
    };
    let Some(left_prototype) = value_prototype(left, env) else {
        return Ok(Value::Boolean(false));
    };
    let Some(Property {
        value: Value::Object(prototype),
        ..
    }) = constructor.properties.borrow().get("prototype").cloned()
    else {
        return Err(RuntimeError {
            message: "function prototype is not an object".to_owned(),
        });
    };
    Ok(Value::Boolean(
        left_prototype.ptr_eq(&prototype) || left_prototype.has_prototype(&prototype),
    ))
}

fn to_js_string(value: Value) -> Result<String, RuntimeError> {
    match value {
        Value::Number(number) => Ok(number_to_js_string(number)),
        Value::String(value) => Ok(value),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Function(_) | Value::Array(_) | Value::Object(_) => Err(RuntimeError {
            message: "cannot convert object to string".to_owned(),
        }),
    }
}

fn error_value(value: Value) -> String {
    match value {
        Value::Number(number) => number_to_js_string(number),
        Value::String(value) => value,
        Value::Boolean(true) => "true".to_owned(),
        Value::Boolean(false) => "false".to_owned(),
        Value::Null => "null".to_owned(),
        Value::Undefined => "undefined".to_owned(),
        Value::Function(_) => "function".to_owned(),
        Value::Array(_) => "array".to_owned(),
        Value::Object(_) => "object".to_owned(),
    }
}

fn number_to_js_string(number: f64) -> String {
    if number.is_nan() {
        "NaN".to_owned()
    } else if number == f64::INFINITY {
        "Infinity".to_owned()
    } else if number == f64::NEG_INFINITY {
        "-Infinity".to_owned()
    } else if number == 0.0 {
        "0".to_owned()
    } else if number.fract() == 0.0 {
        format!("{number:.0}")
    } else {
        number.to_string()
    }
}

fn eval_in(left: Value, right: Value) -> Result<Value, RuntimeError> {
    let key = to_property_key(left)?;
    match right {
        Value::Object(object) => Ok(Value::Boolean(object.contains_property(&key))),
        Value::Array(elements) => {
            let index = key.parse::<usize>().ok();
            Ok(Value::Boolean(
                index.is_some_and(|index| index < elements.len()) || key == "length",
            ))
        }
        _ => Err(RuntimeError {
            message: "right operand of in is not an object".to_owned(),
        }),
    }
}

fn to_number(value: Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(number) => Ok(number),
        Value::Boolean(true) => Ok(1.0),
        Value::Boolean(false) | Value::Null => Ok(0.0),
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Ok(0.0)
            } else {
                Ok(trimmed.parse::<f64>().unwrap_or(f64::NAN))
            }
        }
        Value::Undefined => Ok(f64::NAN),
        Value::Function(_) => Err(RuntimeError {
            message: "cannot convert function to number".to_owned(),
        }),
        Value::Array(_) | Value::Object(_) => Err(RuntimeError {
            message: "cannot convert object to number".to_owned(),
        }),
    }
}

fn to_int32(value: Value) -> Result<i32, RuntimeError> {
    to_number(value).map(to_int32_number)
}

fn to_uint32(value: Value) -> Result<u32, RuntimeError> {
    to_number(value).map(to_uint32_number)
}

fn to_int32_number(number: f64) -> i32 {
    let int = to_uint32_number(number);
    if int >= 0x8000_0000 {
        (i64::from(int) - 0x1_0000_0000) as i32
    } else {
        int as i32
    }
}

fn to_uint32_number(number: f64) -> u32 {
    if !number.is_finite() || number == 0.0 {
        return 0;
    }
    const TWO_32: f64 = 4_294_967_296.0;
    number.trunc().rem_euclid(TWO_32) as u32
}

fn to_uint16(value: Value) -> Result<u16, RuntimeError> {
    let number = to_number(value)?;
    if !number.is_finite() || number == 0.0 {
        return Ok(0);
    }
    const TWO_16: f64 = 65_536.0;
    Ok(number.trunc().rem_euclid(TWO_16) as u16)
}

fn to_length(value: Value) -> Result<usize, RuntimeError> {
    let number = to_number(value)?;
    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    if number.is_infinite() {
        return Err(RuntimeError {
            message: "string padding length must be finite".to_owned(),
        });
    }
    Ok(number.trunc().min(9_007_199_254_740_991.0) as usize)
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Number(number) => *number != 0.0 && !number.is_nan(),
        Value::String(value) => !value.is_empty(),
        Value::Boolean(value) => *value,
        Value::Null | Value::Undefined => false,
        Value::Function(_) | Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests;
