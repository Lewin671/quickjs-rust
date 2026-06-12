use crate::{
    ArrayRef, RuntimeError, Value, call_function, has_property, property_value, to_length_with_env,
    to_number_with_env,
};

use super::{
    array_like::array_like_length,
    species::{
        array_species_create, create_data_property_or_throw, validate_array_species_constructor,
    },
};
use crate::CallEnv;

const MAX_SAFE_LENGTH: usize = (1usize << 53) - 1;

pub(crate) fn native_array_prototype_flat(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.flat", env)?;
    let depth = flat_depth(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let result = array_species_create(source.receiver.clone(), 0, "flat", env)?;
    flatten_source_into_result(
        result.clone(),
        0,
        source.receiver,
        source.length,
        depth,
        env,
    )?;
    Ok(result)
}

pub(crate) fn native_array_prototype_flat_map(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.flatMap", env)?;
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.flatMap callback is not callable".to_owned(),
        });
    }
    validate_array_species_constructor(source.receiver.clone(), "flatMap", env)?;

    let callback_this = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let mut result = Vec::new();
    for index in 0..source.length {
        let key = index.to_string();
        if !has_property(source.receiver.clone(), env, &key)? {
            continue;
        }
        let value = property_value(source.receiver.clone(), &key, env)?;
        let mapped = call_function(
            callback.clone(),
            callback_this.clone(),
            vec![value, Value::Number(index as f64), source.receiver.clone()],
            env,
            false,
        )?;
        flatten_value_into(&mut result, mapped, 1, env)?;
    }

    Ok(Value::Array(ArrayRef::new(result)))
}

fn flat_depth(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    let number = match value {
        Value::Undefined => return Ok(1),
        value => to_number_with_env(value, env)?,
    };

    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    if number.is_infinite() {
        return Ok(usize::MAX);
    }
    Ok(number.trunc() as usize)
}

fn flatten_source_into(
    result: &mut Vec<Value>,
    receiver: Value,
    length: usize,
    depth: usize,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    for index in 0..length {
        let key = index.to_string();
        if !has_property(receiver.clone(), env, &key)? {
            continue;
        }
        let value = property_value(receiver.clone(), &key, env)?;
        flatten_value_into(result, value, depth, env)?;
    }
    Ok(())
}

fn flatten_source_into_result(
    target: Value,
    mut target_index: usize,
    receiver: Value,
    length: usize,
    depth: usize,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    for source_index in 0..length {
        let key = source_index.to_string();
        if !has_property(receiver.clone(), env, &key)? {
            continue;
        }
        let value = property_value(receiver.clone(), &key, env)?;
        if should_flatten(value.clone(), depth)? {
            let element_length = flattenable_length(value.clone(), env)?;
            target_index = flatten_source_into_result(
                target.clone(),
                target_index,
                value,
                element_length,
                depth.saturating_sub(1),
                env,
            )?;
        } else {
            if target_index >= MAX_SAFE_LENGTH {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: invalid array length".to_owned(),
                });
            }
            create_data_property_or_throw(target.clone(), target_index.to_string(), value, env)?;
            target_index += 1;
        }
    }
    Ok(target_index)
}

fn flatten_value_into(
    result: &mut Vec<Value>,
    value: Value,
    depth: usize,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    match value {
        Value::Array(array) if depth > 0 => {
            flatten_source_into(
                result,
                Value::Array(array.clone()),
                array.len(),
                depth.saturating_sub(1),
                env,
            )?;
        }
        value => result.push(value),
    }
    Ok(())
}

fn should_flatten(value: Value, depth: usize) -> Result<bool, RuntimeError> {
    if depth == 0 {
        return Ok(false);
    }
    match value {
        Value::Array(_) => Ok(true),
        Value::Proxy(proxy) => crate::proxy::proxy_target_is_array_result(&proxy),
        _ => Ok(false),
    }
}

fn flattenable_length(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    match value {
        Value::Array(array) => Ok(array.len()),
        value => to_length_with_env(property_value(value, "length", env)?, env),
    }
}
