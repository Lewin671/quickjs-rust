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
use function::{Function, NativeFunction};
use native::call_native_function;
pub(crate) use property::{
    array_has_own_property, array_own_property_descriptor, array_own_property_keys,
    array_own_property_names, array_prototype, constructor_prototype, function_intrinsic_prototype,
    function_own_property_descriptor, function_own_property_keys, function_own_property_names,
    function_prototype, inherited_array_prototype_property, inherited_function_prototype_property,
    inherited_object_prototype_property, inherited_string_prototype_property, object_prototype,
    string_prototype, to_array_index, to_property_key, value_prototype,
};
use statement::{
    Completion, collect_function_local_names, eval_statement_list, eval_stmt, hoist_declarations,
};
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

pub(crate) fn call_function(
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
    if let Some(bound) = &function.bound {
        let mut bound_arguments = bound.arguments.clone();
        bound_arguments.extend(argument_values);
        let bound_this = if is_construct {
            this_value
        } else {
            bound.this_value.clone()
        };
        return call_function(
            bound.target.clone(),
            bound_this,
            bound_arguments,
            env,
            is_construct,
        );
    }
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
    let caller_names: Vec<String> = env.keys().cloned().collect();
    let function_local_names = collect_function_local_names(&function);
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

    let completion = eval_statement_list(&function.body, &mut local_env)?;
    for name in caller_names {
        if name != GLOBAL_THIS_BINDING && !function_local_names.contains(&name) {
            if let Some(value) = local_env.get(&name) {
                env.insert(name, value.clone());
            }
        }
    }

    match completion {
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

#[cfg(test)]
mod tests;
