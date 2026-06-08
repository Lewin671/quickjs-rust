use std::collections::HashMap;

use crate::{
    ArrayRef, RuntimeError, Value, call_function, has_property, property_value, to_number,
};

use super::array_like::array_like_length;

pub(crate) fn native_array_prototype_flat(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.flat", env)?;
    let depth = flat_depth(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let mut result = Vec::new();
    flatten_source_into(&mut result, source.receiver, source.length, depth, env)?;
    Ok(Value::Array(ArrayRef::new(result)))
}

pub(crate) fn native_array_prototype_flat_map(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.flatMap", env)?;
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.flatMap callback is not callable".to_owned(),
        });
    }

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

fn flat_depth(value: Value) -> Result<usize, RuntimeError> {
    let number = match value {
        Value::Undefined => return Ok(1),
        value => to_number(value)?,
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
    env: &mut HashMap<String, Value>,
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

fn flatten_value_into(
    result: &mut Vec<Value>,
    value: Value,
    depth: usize,
    env: &mut HashMap<String, Value>,
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
