use std::collections::HashMap;

use crate::{
    ArrayRef, GLOBAL_THIS_BINDING, RuntimeError, Value, error_value,
    native::call_native_function,
    statement::{Completion, collect_function_local_names, eval_statement_list},
};

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
