use std::collections::HashMap;

use crate::{ArrayRef, RuntimeError, Value, call_function, is_truthy};

struct ArrayIteration {
    elements: ArrayRef,
    callback: Value,
    callback_this: Value,
    source: Vec<Value>,
}

struct ArrayReduction {
    elements: ArrayRef,
    callback: Value,
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

fn prepare_array_reduction(
    method: &str,
    this_value: Value,
    argument_values: &[Value],
) -> Result<ArrayReduction, RuntimeError> {
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

    Ok(ArrayReduction {
        source: elements.to_vec(),
        elements,
        callback,
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

fn call_reduction_callback(
    reduction: &ArrayReduction,
    accumulator: Value,
    value: Value,
    index: usize,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    call_function(
        reduction.callback.clone(),
        Value::Undefined,
        vec![
            accumulator,
            value,
            Value::Number(index as f64),
            Value::Array(reduction.elements.clone()),
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

pub(crate) fn native_array_prototype_find_last(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("findLast", this_value, argument_values)?;
    for index in (0..iteration.source.len()).rev() {
        let value = iteration.source[index].clone();
        let selected = call_iteration_callback(&iteration, value.clone(), index, env)?;
        if is_truthy(&selected) {
            return Ok(value);
        }
    }

    Ok(Value::Undefined)
}

pub(crate) fn native_array_prototype_find_index(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("findIndex", this_value, argument_values)?;
    for (index, value) in iteration.source.iter().cloned().enumerate() {
        let selected = call_iteration_callback(&iteration, value, index, env)?;
        if is_truthy(&selected) {
            return Ok(Value::Number(index as f64));
        }
    }

    Ok(Value::Number(-1.0))
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

pub(crate) fn native_array_prototype_reduce(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let reduction = prepare_array_reduction("reduce", this_value, argument_values)?;
    if reduction.source.is_empty() && argument_values.len() < 2 {
        return Err(RuntimeError {
            message: "Reduce of empty array with no initial value".to_owned(),
        });
    }

    let (mut accumulator, start_index) = if argument_values.len() >= 2 {
        (argument_values[1].clone(), 0)
    } else {
        (reduction.source[0].clone(), 1)
    };

    for (index, value) in reduction
        .source
        .iter()
        .cloned()
        .enumerate()
        .skip(start_index)
    {
        accumulator = call_reduction_callback(&reduction, accumulator, value, index, env)?;
    }

    Ok(accumulator)
}

pub(crate) fn native_array_prototype_reduce_right(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let reduction = prepare_array_reduction("reduceRight", this_value, argument_values)?;
    if reduction.source.is_empty() && argument_values.len() < 2 {
        return Err(RuntimeError {
            message: "Reduce of empty array with no initial value".to_owned(),
        });
    }

    let (mut accumulator, next_index) = if argument_values.len() >= 2 {
        (argument_values[1].clone(), reduction.source.len())
    } else {
        let last_index = reduction.source.len() - 1;
        (reduction.source[last_index].clone(), last_index)
    };

    for index in (0..next_index).rev() {
        accumulator = call_reduction_callback(
            &reduction,
            accumulator,
            reduction.source[index].clone(),
            index,
            env,
        )?;
    }

    Ok(accumulator)
}
