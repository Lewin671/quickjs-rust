use std::collections::HashMap;

use crate::{RuntimeError, Value, has_property, property_value};

use super::array_like::array_like_length;
use super::indexing::{array_at_index, array_search_end_index, array_search_start_index};

pub(crate) fn native_array_prototype_at(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.at", env)?;
    let Some(index) = array_at_index(
        source.length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?
    else {
        return Ok(Value::Undefined);
    };
    property_value(source.receiver, &index.to_string(), env)
}

pub(crate) fn native_array_prototype_includes(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.includes", env)?;
    if source.length == 0 {
        return Ok(Value::Boolean(false));
    }

    let search_element = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let start = array_search_start_index(
        source.length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    for index in start..source.length {
        let element = property_value(source.receiver.clone(), &index.to_string(), env)?;
        if same_value_zero(&element, &search_element) {
            return Ok(Value::Boolean(true));
        }
    }
    Ok(Value::Boolean(false))
}

pub(crate) fn native_array_prototype_index_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.indexOf", env)?;
    if source.length == 0 {
        return Ok(Value::Number(-1.0));
    }

    let search_element = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let start = array_search_start_index(
        source.length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    for index in start..source.length {
        let key = index.to_string();
        if !has_property(source.receiver.clone(), env, &key)? {
            continue;
        }
        if property_value(source.receiver.clone(), &key, env)? == search_element {
            return Ok(Value::Number(index as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

pub(crate) fn native_array_prototype_last_index_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.lastIndexOf", env)?;
    if source.length == 0 {
        return Ok(Value::Number(-1.0));
    }

    let search_element = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Some(start) = array_search_end_index(source.length, argument_values.get(1).cloned(), env)?
    else {
        return Ok(Value::Number(-1.0));
    };
    for index in (0..=start).rev() {
        let key = index.to_string();
        if !has_property(source.receiver.clone(), env, &key)? {
            continue;
        }
        if property_value(source.receiver.clone(), &key, env)? == search_element {
            return Ok(Value::Number(index as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

fn same_value_zero(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            left == right || (left.is_nan() && right.is_nan())
        }
        _ => left == right,
    }
}
