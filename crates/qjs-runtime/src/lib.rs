//! Runtime for the Rust QuickJS rewrite.

use qjs_parser::parse_script;

mod array;
mod array_buffer;
mod async_function;
mod bigint;
mod boolean;
mod builtins;
mod bytecode;
mod conversion;
mod date;
mod error;
mod function;
mod generator;
mod global;
mod html_dda;
mod json;
mod map;
mod math;
mod native;
mod number;
mod object;
mod operations;
mod private;
mod promise;
mod property;
mod proxy;
mod reflect;
mod regexp;
mod set;
mod string;
mod symbol;
mod typed_array;
mod value;
mod weak_map;
mod weak_set;

use builtins::initialize_builtins;
pub(crate) use conversion::{
    PreferredType, error_value, is_truthy, ordinary_to_primitive, to_int32, to_int32_number,
    to_js_string, to_js_string_with_env, to_length, to_length_with_env, to_number,
    to_number_with_env, to_primitive_with_env, to_primitive_with_hint, to_uint16, to_uint32,
    to_uint32_number,
};
use function::{Function, NativeFunction};
pub(crate) use function::{call_function, construct_function, ensure_constructor};
pub(crate) use property::{
    PropertyKey, array_as_object_prototype, array_has_own_property, array_own_property_descriptor,
    array_own_property_keys, array_own_property_names, array_prototype, array_prototype_property,
    constructor_prototype, constructor_prototype_slot, function_delete_own_property,
    function_delete_own_symbol_property, function_intrinsic_prototype,
    function_own_property_descriptor, function_own_property_keys, function_own_property_names,
    function_own_property_symbols, function_own_symbol_property_descriptor, function_prototype,
    function_prototype_property, has_property, has_property_key,
    inherited_object_prototype_property, inherited_string_prototype_property, object_prototype,
    property_value, property_value_key, property_value_key_with_receiver, string_prototype,
    to_property_key_value, value_prototype, value_prototype_slot,
};
pub(crate) use string::string_object_value;
pub use value::Value;
use value::{ArrayRef, MapRef, ObjectRef, Property, Prototype, SetRef};

pub use bytecode::{
    Bytecode, EvalOutcome, compile_script, eval_bytecode, eval_bytecode_keep_jobs,
    eval_bytecode_source,
};

pub(crate) const GLOBAL_THIS_BINDING: &str = "\0global_this";
/// Per-frame `[[HomeObject]]` for resolving `super.x`. Single-null prefixed so
/// it never collides with user identifiers but is not treated as an internal
/// destructuring binding (which use a double-null prefix).
pub(crate) const HOME_OBJECT_BINDING: &str = "\0home_object";
/// Per-frame `new.target`, propagated through `super(...)`.
pub(crate) const NEW_TARGET_BINDING: &str = "\0new_target";
/// Per-frame parent constructor invoked by `super(...)` in a derived
/// constructor.
pub(crate) const SUPER_CONSTRUCTOR_BINDING: &str = "\0super_constructor";
/// Per-frame class constructor currently executing, used so a derived
/// constructor can initialize its instance fields once `super(...)` binds
/// `this`.
pub(crate) const ACTIVE_CONSTRUCTOR_BINDING: &str = "\0active_constructor";
pub(crate) const RUNTIME_INTRINSIC_NAMES: &[&str] = &[
    GLOBAL_THIS_BINDING,
    symbol::SYMBOL_REGISTRY_BINDING,
    generator::GENERATOR_PROTOTYPE_BINDING,
    "globalThis",
    "__quickjsRustIsHTMLDDA",
    "undefined",
    "Object",
    "Function",
    "Array",
    "ArrayBuffer",
    "Uint8Array",
    "Uint16Array",
    "Uint32Array",
    "Float32Array",
    "Float64Array",
    "BigInt",
    "Number",
    "String",
    "Symbol",
    "Boolean",
    "Date",
    "RegExp",
    "Error",
    "AggregateError",
    "EvalError",
    "RangeError",
    "ReferenceError",
    "SyntaxError",
    "TypeError",
    "URIError",
    "JSON",
    "Promise",
    "Proxy",
    "Map",
    "WeakMap",
    "WeakSet",
    "Set",
    "Math",
    "Reflect",
    "NaN",
    "Infinity",
    "isFinite",
    "isNaN",
    "escape",
    "unescape",
    "parseFloat",
    "parseInt",
];

/// Runtime error.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeError {
    /// Original JavaScript value thrown by a `throw` statement, when available.
    pub(crate) thrown: Option<Box<Value>>,
    /// Human-readable message.
    pub message: String,
}

/// Evaluation failure stage used by conformance harnesses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvalErrorKind {
    /// The parser rejected the source text.
    Parse,
    /// The bytecode compiler rejected a syntactically valid script as an early error.
    Early,
    /// The VM or JavaScript execution raised an error.
    Runtime,
}

impl EvalErrorKind {
    /// Stable lowercase name for script harness output.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Early => "early",
            Self::Runtime => "runtime",
        }
    }
}

/// Evaluation error with the stage that produced it.
#[derive(Clone, Debug, PartialEq)]
pub struct EvalError {
    /// Failure stage.
    pub kind: EvalErrorKind,
    /// Human-readable message.
    pub message: String,
}

/// Evaluates source text through the bytecode VM and returns the last statement value.
///
/// # Errors
///
/// Returns parser or runtime failures.
pub fn eval(source: &str) -> Result<Value, RuntimeError> {
    eval_classified(source).map_err(|error| RuntimeError {
        thrown: None,
        message: error.message,
    })
}

/// Evaluates source text and preserves the failure stage for harnesses.
///
/// # Errors
///
/// Returns parser, bytecode compiler, or runtime failures with their stage.
pub fn eval_classified(source: &str) -> Result<Value, EvalError> {
    let script = parse_script(source).map_err(|error| EvalError {
        kind: EvalErrorKind::Parse,
        message: error.message,
    })?;
    let bytecode = compile_script(&script).map_err(|error| EvalError {
        kind: EvalErrorKind::Early,
        message: error.message,
    })?;
    eval_bytecode(&bytecode).map_err(|error| EvalError {
        kind: EvalErrorKind::Runtime,
        message: error.message,
    })
}

/// Evaluates source text without draining the promise job queue.
///
/// Returns an [`EvalOutcome`] holding the script completion value and the
/// realm's pending microtasks, so harnesses (for example the Test262 async
/// `$DONE` channel) can evaluate additional code and then drain reactions
/// explicitly via [`EvalOutcome::run_jobs`].
///
/// # Errors
///
/// Returns parser, bytecode compiler, or runtime failures with their stage.
pub fn eval_keep_jobs(source: &str) -> Result<EvalOutcome, EvalError> {
    let script = parse_script(source).map_err(|error| EvalError {
        kind: EvalErrorKind::Parse,
        message: error.message,
    })?;
    let bytecode = compile_script(&script).map_err(|error| EvalError {
        kind: EvalErrorKind::Early,
        message: error.message,
    })?;
    eval_bytecode_keep_jobs(&bytecode).map_err(|error| EvalError {
        kind: EvalErrorKind::Runtime,
        message: error.message,
    })
}

#[cfg(test)]
mod tests;
