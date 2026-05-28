//! Early interpreter for the Rust QuickJS rewrite.

use std::collections::HashMap;

use qjs_ast::Script;
use qjs_parser::parse_script;

mod array;
mod boolean;
mod builtins;
mod conversion;
mod expression;
mod function;
mod global;
mod math;
mod native;
mod number;
mod object;
mod operations;
mod property;
mod statement;
mod string;
mod value;

use builtins::initialize_builtins;
pub(crate) use conversion::{
    error_value, is_truthy, to_int32, to_int32_number, to_js_string, to_length, to_number,
    to_uint16, to_uint32, to_uint32_number,
};
pub(crate) use expression::{assign_target, eval_expr};
pub(crate) use function::call_function;
use function::{Function, NativeFunction};
pub(crate) use property::{
    array_has_own_property, array_own_property_descriptor, array_own_property_keys,
    array_own_property_names, array_prototype, constructor_prototype, function_intrinsic_prototype,
    function_own_property_descriptor, function_own_property_keys, function_own_property_names,
    function_prototype, inherited_array_prototype_property, inherited_function_prototype_property,
    inherited_object_prototype_property, inherited_string_prototype_property, object_prototype,
    string_prototype, to_array_index, to_property_key, value_prototype,
};
use statement::{Completion, eval_stmt, hoist_declarations};
pub use value::Value;
use value::{ArrayRef, ObjectRef, Property};

pub(crate) const GLOBAL_THIS_BINDING: &str = "\0global_this";

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

#[cfg(test)]
mod tests;
