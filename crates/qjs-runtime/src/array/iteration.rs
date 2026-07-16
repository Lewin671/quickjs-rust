use super::{
    array_like::array_like_length,
    species::{
        array_species_create, create_data_property_or_throw, validate_array_species_constructor,
    },
};
use crate::CallEnv;
use crate::{
    RuntimeError, Value, array_prototype, call_function, has_property, is_truthy, property_value,
};

const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;
const DYNAMIC_MAP_INDEX_LIMIT: usize = 100_000;

struct ArrayIteration {
    receiver: Value,
    callback: Value,
    callback_this: Value,
    source_len: usize,
}

struct ArrayReduction {
    receiver: Value,
    callback: Value,
    source_len: usize,
}

fn prepare_array_iteration(
    method: &str,
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
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
    if matches!(method, "filter" | "map") {
        validate_array_species_constructor(source.receiver.clone(), method, env)?;
    }
    Ok(ArrayIteration {
        receiver: source.receiver,
        source_len: source.length,
        callback,
        callback_this: argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    })
}

fn prepare_array_reduction(
    method: &str,
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<ArrayReduction, RuntimeError> {
    let source = array_like_length(this_value, &format!("Array.prototype.{method}"), env)?;
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("Array.prototype.{method} callback is not callable"),
        });
    }
    Ok(ArrayReduction {
        receiver: source.receiver,
        source_len: source.length,
        callback,
    })
}

fn call_iteration_callback(
    iteration: &ArrayIteration,
    value: Value,
    index: usize,
    env: &mut CallEnv,
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
    env: &mut CallEnv,
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("map", this_value, argument_values, env)?;
    let result =
        array_species_create(iteration.receiver.clone(), iteration.source_len, "map", env)?;
    for index in dynamic_iteration_indices(&iteration, env) {
        let key = index.to_string();
        if has_property(iteration.receiver.clone(), env, &key)? {
            let value = property_value(iteration.receiver.clone(), &key, env)?;
            let mapped = call_iteration_callback(&iteration, value, index, env)?;
            create_data_property_or_throw(result.clone(), key, mapped, env)?;
        }
    }

    Ok(result)
}

fn dynamic_iteration_indices(iteration: &ArrayIteration, env: &CallEnv) -> Vec<usize> {
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
        Value::Object(object) => {
            numeric_indices_from_names(object.own_property_names(), iteration.source_len)
        }
        Value::Function(function) => {
            numeric_indices_from_names(function.own_property_names(), iteration.source_len)
        }
        _ => (0..iteration.source_len).collect(),
    }
}

fn numeric_indices_from_names(names: Vec<String>, length: usize) -> Vec<usize> {
    let mut indices: Vec<_> = names
        .into_iter()
        .filter_map(|key| key.parse::<usize>().ok())
        .filter(|index| *index < length)
        .collect();
    indices.sort_unstable();
    indices.dedup();
    indices
}

pub(crate) fn native_array_prototype_filter(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("filter", this_value, argument_values, env)?;
    let result = array_species_create(iteration.receiver.clone(), 0, "filter", env)?;
    let mut target_index = 0;
    for index in dynamic_iteration_indices(&iteration, env) {
        let key = index.to_string();
        if has_property(iteration.receiver.clone(), env, &key)? {
            let value = property_value(iteration.receiver.clone(), &key, env)?;
            let selected = call_iteration_callback(&iteration, value.clone(), index, env)?;
            if is_truthy(&selected) {
                create_data_property_or_throw(
                    result.clone(),
                    target_index.to_string(),
                    value,
                    env,
                )?;
                target_index += 1;
            }
        }
    }

    Ok(result)
}

pub(crate) fn native_array_prototype_find(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("find", this_value, argument_values, env)?;
    for index in 0..iteration.source_len {
        let value = property_value(iteration.receiver.clone(), &index.to_string(), env)?;
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("findLast", this_value, argument_values, env)?;
    for index in (0..iteration.source_len).rev() {
        let value = property_value(iteration.receiver.clone(), &index.to_string(), env)?;
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("findLastIndex", this_value, argument_values, env)?;
    for index in (0..iteration.source_len).rev() {
        let value = property_value(iteration.receiver.clone(), &index.to_string(), env)?;
        let selected = call_iteration_callback(&iteration, value, index, env)?;
        if is_truthy(&selected) {
            return Ok(Value::Number(index as f64));
        }
    }

    Ok(Value::Number(-1.0))
}

pub(crate) fn native_array_prototype_find_index(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("findIndex", this_value, argument_values, env)?;
    for index in 0..iteration.source_len {
        let value = property_value(iteration.receiver.clone(), &index.to_string(), env)?;
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("forEach", this_value, argument_values, env)?;
    for index in dynamic_iteration_indices(&iteration, env) {
        let key = index.to_string();
        if has_property(iteration.receiver.clone(), env, &key)? {
            let value = property_value(iteration.receiver.clone(), &key, env)?;
            call_iteration_callback(&iteration, value, index, env)?;
        }
    }

    Ok(Value::Undefined)
}

pub(crate) fn native_array_prototype_some(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("some", this_value, argument_values, env)?;
    for index in dynamic_iteration_indices(&iteration, env) {
        let key = index.to_string();
        if has_property(iteration.receiver.clone(), env, &key)? {
            let value = property_value(iteration.receiver.clone(), &key, env)?;
            let selected = call_iteration_callback(&iteration, value, index, env)?;
            if is_truthy(&selected) {
                return Ok(Value::Boolean(true));
            }
        }
    }

    Ok(Value::Boolean(false))
}

pub(crate) fn native_array_prototype_every(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iteration = prepare_array_iteration("every", this_value, argument_values, env)?;
    for index in dynamic_iteration_indices(&iteration, env) {
        let key = index.to_string();
        if has_property(iteration.receiver.clone(), env, &key)? {
            let value = property_value(iteration.receiver.clone(), &key, env)?;
            let selected = call_iteration_callback(&iteration, value, index, env)?;
            if !is_truthy(&selected) {
                return Ok(Value::Boolean(false));
            }
        }
    }

    Ok(Value::Boolean(true))
}

pub(crate) fn native_array_prototype_reduce(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let reduction = prepare_array_reduction("reduce", this_value, argument_values, env)?;
    let (mut accumulator, start_index) = if argument_values.len() >= 2 {
        (argument_values[1].clone(), 0)
    } else {
        let Some((index, value)) = first_reduction_value(&reduction, env)? else {
            return Err(RuntimeError {
                thrown: None,
                message: "Reduce of empty array with no initial value".to_owned(),
            });
        };
        (value, index + 1)
    };

    for index in start_index..reduction.source_len {
        let key = index.to_string();
        if has_property(reduction.receiver.clone(), env, &key)? {
            let value = property_value(reduction.receiver.clone(), &key, env)?;
            accumulator = call_reduction_callback(&reduction, accumulator, value, index, env)?;
        }
    }

    Ok(accumulator)
}

pub(crate) fn native_array_prototype_reduce_right(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let reduction = prepare_array_reduction("reduceRight", this_value, argument_values, env)?;
    let (mut accumulator, next_index) = if argument_values.len() >= 2 {
        (argument_values[1].clone(), reduction.source_len)
    } else {
        let Some((index, value)) = last_reduction_value(&reduction, env)? else {
            return Err(RuntimeError {
                thrown: None,
                message: "Reduce of empty array with no initial value".to_owned(),
            });
        };
        (value, index)
    };

    for index in (0..next_index).rev() {
        let key = index.to_string();
        if has_property(reduction.receiver.clone(), env, &key)? {
            let value = property_value(reduction.receiver.clone(), &key, env)?;
            accumulator = call_reduction_callback(&reduction, accumulator, value, index, env)?;
        }
    }

    Ok(accumulator)
}

fn first_reduction_value(
    reduction: &ArrayReduction,
    env: &mut CallEnv,
) -> Result<Option<(usize, Value)>, RuntimeError> {
    for index in 0..reduction.source_len {
        let key = index.to_string();
        if has_property(reduction.receiver.clone(), env, &key)? {
            return Ok(Some((
                index,
                property_value(reduction.receiver.clone(), &key, env)?,
            )));
        }
    }
    Ok(None)
}

fn last_reduction_value(
    reduction: &ArrayReduction,
    env: &mut CallEnv,
) -> Result<Option<(usize, Value)>, RuntimeError> {
    for index in (0..reduction.source_len).rev() {
        let key = index.to_string();
        if has_property(reduction.receiver.clone(), env, &key)? {
            return Ok(Some((
                index,
                property_value(reduction.receiver.clone(), &key, env)?,
            )));
        }
    }
    Ok(None)
}
