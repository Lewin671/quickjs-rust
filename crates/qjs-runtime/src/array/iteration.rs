use std::collections::HashMap;

use crate::{ArrayRef, RuntimeError, Value, call_function, is_truthy};

struct ArrayIteration {
    elements: ArrayRef,
    callback: Value,
    callback_this: Value,
    source: Vec<Value>,
}

fn prepare_array_iteration(
    method: &str,
    this_value: Value,
    argument_values: &[Value],
) -> Result<ArrayIteration, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            message: format!("Array.prototype.{method} called on non-array"),
        });
    };
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            message: format!("Array.prototype.{method} callback is not callable"),
        });
    }

    Ok(ArrayIteration {
        source: elements.to_vec(),
        elements,
        callback,
        callback_this: argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    })
}

fn call_iteration_callback(
    iteration: &ArrayIteration,
    value: Value,
    index: usize,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    call_function(
        iteration.callback.clone(),
        iteration.callback_this.clone(),
        vec![
            value,
            Value::Number(index as f64),
            Value::Array(iteration.elements.clone()),
        ],
        env,
        false,
    )
}

pub(crate) fn native_array_prototype_map(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("map", this_value, argument_values)?;
    let mut mapped = Vec::with_capacity(iteration.source.len());
    for (index, value) in iteration.source.iter().cloned().enumerate() {
        mapped.push(call_iteration_callback(&iteration, value, index, env)?);
    }

    Ok(Value::Array(ArrayRef::new(mapped)))
}

pub(crate) fn native_array_prototype_filter(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("filter", this_value, argument_values)?;
    let mut filtered = Vec::new();
    for (index, value) in iteration.source.iter().cloned().enumerate() {
        let selected = call_iteration_callback(&iteration, value.clone(), index, env)?;
        if is_truthy(&selected) {
            filtered.push(value);
        }
    }

    Ok(Value::Array(ArrayRef::new(filtered)))
}

pub(crate) fn native_array_prototype_find(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("find", this_value, argument_values)?;
    for (index, value) in iteration.source.iter().cloned().enumerate() {
        let selected = call_iteration_callback(&iteration, value.clone(), index, env)?;
        if is_truthy(&selected) {
            return Ok(value);
        }
    }

    Ok(Value::Undefined)
}

pub(crate) fn native_array_prototype_for_each(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("forEach", this_value, argument_values)?;
    for (index, value) in iteration.source.iter().cloned().enumerate() {
        call_iteration_callback(&iteration, value, index, env)?;
    }

    Ok(Value::Undefined)
}

pub(crate) fn native_array_prototype_some(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("some", this_value, argument_values)?;
    for (index, value) in iteration.source.iter().cloned().enumerate() {
        let selected = call_iteration_callback(&iteration, value, index, env)?;
        if is_truthy(&selected) {
            return Ok(Value::Boolean(true));
        }
    }

    Ok(Value::Boolean(false))
}

pub(crate) fn native_array_prototype_every(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("every", this_value, argument_values)?;
    for (index, value) in iteration.source.iter().cloned().enumerate() {
        let selected = call_iteration_callback(&iteration, value, index, env)?;
        if !is_truthy(&selected) {
            return Ok(Value::Boolean(false));
        }
    }

    Ok(Value::Boolean(true))
}
