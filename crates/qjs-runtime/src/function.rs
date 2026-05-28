use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use qjs_ast::Stmt;

use crate::{ObjectRef, Property, Value, object_prototype};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeFunction {
    Array,
    ArrayIsArray,
    ArrayPrototypeAt,
    ArrayPrototypeConcat,
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
    /// Function object properties.
    pub(crate) properties: Rc<RefCell<HashMap<String, Property>>>,
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
