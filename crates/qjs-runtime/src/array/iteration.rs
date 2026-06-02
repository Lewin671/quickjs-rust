use std::collections::HashMap;

use super::array_like::{array_like_length, array_like_values_from_receiver};
use crate::{
    ArrayRef, RuntimeError, Value, array_prototype, call_function, has_property, is_truthy,
    property_value,
};

const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;
const DYNAMIC_MAP_INDEX_LIMIT: usize = 100_000;

struct ArrayIteration {
    receiver: Value,
    callback: Value,
    callback_this: Value,
    source_len: usize,
    source: Vec<Value>,
}

struct ArrayReduction {
    receiver: Value,
    callback: Value,
    source: Vec<Value>,
}

fn prepare_array_iteration(
    method: &str,
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<ArrayIteration, RuntimeError> {
    let source = array_like_length(this_value, &format!("Array.prototype.{method}"), env)?;
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("Array.prototype.{method} callback is not callable"),
        });
    }
    if method == "map" && source.length > MAX_ARRAY_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    if method == "map" {
        validate_array_map_constructor(source.receiver.clone(), env)?;
    }
    let values = array_like_values_from_receiver(source.receiver.clone(), source.length, env)?;

    Ok(ArrayIteration {
        receiver: source.receiver,
        source_len: source.length,
        source: values,
        callback,
        callback_this: argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    })
}

fn validate_array_map_constructor(
    receiver: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    if !matches!(receiver, Value::Array(_)) {
        return Ok(());
    }

    match property_value(receiver, "constructor", env)? {
        Value::Undefined | Value::Function(_) | Value::Object(_) => Ok(()),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: Array.prototype.map constructor is not a constructor".to_owned(),
        }),
    }
}

fn prepare_array_reduction(
    method: &str,
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<ArrayReduction, RuntimeError> {
    let source = array_like_length(this_value, &format!("Array.prototype.{method}"), env)?;
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("Array.prototype.{method} callback is not callable"),
        });
    }
    let values = array_like_values_from_receiver(source.receiver.clone(), source.length, env)?;

    Ok(ArrayReduction {
        receiver: source.receiver,
        source: values,
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
            iteration.receiver.clone(),
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
            reduction.receiver.clone(),
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
    let iteration = prepare_array_iteration("map", this_value, argument_values, env)?;
    let mut mapped = vec![Value::Undefined; iteration.source_len];
    let mut holes = (0..iteration.source_len).collect::<Vec<_>>();
    for index in map_iteration_indices(&iteration, env) {
        let key = index.to_string();
        if has_property(iteration.receiver.clone(), env, &key)? {
            let value = property_value(iteration.receiver.clone(), &key, env)?;
            mapped[index] = call_iteration_callback(&iteration, value, index, env)?;
            holes.retain(|hole| *hole != index);
        }
    }

    Ok(Value::Array(ArrayRef::new_sparse(mapped, holes)))
}

fn map_iteration_indices(iteration: &ArrayIteration, env: &HashMap<String, Value>) -> Vec<usize> {
    if iteration.source_len <= DYNAMIC_MAP_INDEX_LIMIT {
        return (0..iteration.source_len).collect();
    }

    match &iteration.receiver {
        Value::Array(array) => {
            let mut indices = array.present_indices();
            if let Some(prototype) = array_prototype(env) {
                indices.extend(
                    prototype
                        .own_property_names()
                        .into_iter()
                        .filter_map(|key| key.parse::<usize>().ok()),
                );
            }
            indices.retain(|index| *index < iteration.source_len);
            indices.sort_unstable();
            indices.dedup();
            indices
        }
        _ => (0..iteration.source_len).collect(),
    }
}

pub(crate) fn native_array_prototype_filter(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("filter", this_value, argument_values, env)?;
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
    let iteration = prepare_array_iteration("find", this_value, argument_values, env)?;
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
    let iteration = prepare_array_iteration("findLast", this_value, argument_values, env)?;
    for index in (0..iteration.source.len()).rev() {
        let value = iteration.source[index].clone();
        let selected = call_iteration_callback(&iteration, value.clone(), index, env)?;
        if is_truthy(&selected) {
            return Ok(value);
        }
    }

    Ok(Value::Undefined)
}

pub(crate) fn native_array_prototype_find_last_index(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("findLastIndex", this_value, argument_values, env)?;
    for index in (0..iteration.source.len()).rev() {
        let selected =
            call_iteration_callback(&iteration, iteration.source[index].clone(), index, env)?;
        if is_truthy(&selected) {
            return Ok(Value::Number(index as f64));
        }
    }

    Ok(Value::Number(-1.0))
}

pub(crate) fn native_array_prototype_find_index(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("findIndex", this_value, argument_values, env)?;
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
    let iteration = prepare_array_iteration("forEach", this_value, argument_values, env)?;
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
    let iteration = prepare_array_iteration("some", this_value, argument_values, env)?;
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
    let iteration = prepare_array_iteration("every", this_value, argument_values, env)?;
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
    let reduction = prepare_array_reduction("reduce", this_value, argument_values, env)?;
    if reduction.source.is_empty() && argument_values.len() < 2 {
        return Err(RuntimeError {
            thrown: None,
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
    let reduction = prepare_array_reduction("reduceRight", this_value, argument_values, env)?;
    if reduction.source.is_empty() && argument_values.len() < 2 {
        return Err(RuntimeError {
            thrown: None,
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
