use std::collections::HashMap;

use crate::{
    ArrayRef, GLOBAL_THIS_BINDING, RuntimeError, Value, bytecode::eval_function_bytecode,
    native::call_native_function,
};

use super::function_call_this;

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
    local_env.insert("this".to_owned(), function_call_this(Some(this_value), env));
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

    if let Some(bytecode) = &function.bytecode {
        let (value, final_env) = eval_function_bytecode(bytecode, local_env)?;
        propagate_caller_bindings(env, &caller_names, &function.local_names, &final_env);
        return Ok(value);
    }

    Err(RuntimeError {
        message: "user function has no bytecode body".to_owned(),
    })
}

fn propagate_caller_bindings(
    env: &mut HashMap<String, Value>,
    caller_names: &[String],
    function_local_names: &[String],
    final_env: &HashMap<String, Value>,
) {
    for name in caller_names {
        if name != GLOBAL_THIS_BINDING && function_local_names.binary_search(name).is_err() {
            if let Some(value) = final_env.get(name) {
                env.insert(name.clone(), value.clone());
            }
        }
    }
}
