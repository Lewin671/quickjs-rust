//! Runtime for the Rust QuickJS rewrite.

use qjs_parser::parse_script;

mod array;
mod boolean;
mod builtins;
mod bytecode;
mod conversion;
mod date;
mod error;
mod function;
mod global;
mod json;
mod map;
mod math;
mod native;
mod number;
mod object;
mod operations;
mod promise;
mod property;
mod reflect;
mod regexp;
mod set;
mod string;
mod symbol;
mod value;
mod weak_map;
mod weak_set;

use builtins::initialize_builtins;
pub(crate) use conversion::{
    error_value, is_truthy, to_int32, to_int32_number, to_js_string, to_js_string_with_env,
    to_length, to_length_with_env, to_number, to_number_with_env, to_primitive_with_env, to_uint16,
    to_uint32, to_uint32_number,
};
use function::{Function, NativeFunction};
pub(crate) use function::{call_function, construct_function, ensure_constructor};
pub(crate) use property::{
    PropertyKey, array_has_own_property, array_own_property_descriptor, array_own_property_keys,
    array_own_property_names, array_prototype, array_prototype_property, constructor_prototype,
    function_delete_own_property, function_intrinsic_prototype, function_own_property_descriptor,
    function_own_property_keys, function_own_property_names, function_prototype,
    function_prototype_property, has_property, has_property_key,
    inherited_object_prototype_property, inherited_string_prototype_property, object_prototype,
    property_value, property_value_key, string_prototype, to_property_key, to_property_key_value,
    value_prototype,
};
pub(crate) use string::string_object_value;
pub use value::Value;
use value::{ArrayRef, MapRef, ObjectRef, Property, SetRef};

pub use bytecode::{Bytecode, compile_script, eval_bytecode, eval_bytecode_source};

pub(crate) const GLOBAL_THIS_BINDING: &str = "\0global_this";
pub(crate) const RUNTIME_INTRINSIC_NAMES: &[&str] = &[
    GLOBAL_THIS_BINDING,
    "globalThis",
    "undefined",
    "Object",
    "Function",
    "Array",
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

/// Evaluates source text through the bytecode VM and returns the last statement value.
///
/// # Errors
///
/// Returns parser or runtime failures.
pub fn eval(source: &str) -> Result<Value, RuntimeError> {
    let script = parse_script(source).map_err(|error| RuntimeError {
        thrown: None,
        message: error.message,
    })?;
    let bytecode = compile_script(&script)?;
    eval_bytecode(&bytecode)
}

#[cfg(test)]
mod tests;
