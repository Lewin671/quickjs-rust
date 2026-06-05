use std::collections::HashMap;

use crate::{RuntimeError, Value, has_property, property_value, to_number_with_env};

use super::array_like::array_like_length;

use super::indexing::{array_at_index, array_search_start_index};

pub(crate) fn native_array_prototype_at(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.at called on non-array".to_owned(),
        });
    };
    let Some(index) = array_at_index(
        elements.len(),
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?
    else {
        return Ok(Value::Undefined);
    };
    Ok(elements.get(index).unwrap_or(Value::Undefined))
}

pub(crate) fn native_array_prototype_includes(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.includes called on non-array".to_owned(),
        });
    };
    if elements.is_empty() {
        return Ok(Value::Boolean(false));
    }

    let search_element = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let start = array_search_start_index(
        elements.len(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    Ok(Value::Boolean(
        elements
            .to_vec()
            .iter()
            .skip(start)
            .any(|element| same_value_zero(element, &search_element)),
    ))
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
    let start = array_search_start_index_with_env(source.length, argument_values.get(1), env)?;
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
    let Some(start) = array_search_end_index_with_env(source.length, argument_values.get(1), env)?
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

fn array_search_start_index_with_env(
    length: usize,
    from_index: Option<&Value>,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let number = match from_index {
        None | Some(Value::Undefined) => 0.0,
        Some(value) => to_number_with_env(value.clone(), env)?,
    };
    if number.is_nan() {
        return Ok(0);
    }
    if number >= length as f64 {
        return Ok(length);
    }
    if number >= 0.0 {
        return Ok(number.trunc() as usize);
    }

    let start = length as f64 + number.trunc();
    if start <= 0.0 {
        Ok(0)
    } else {
        Ok(start as usize)
    }
}

fn array_search_end_index_with_env(
    length: usize,
    from_index: Option<&Value>,
    env: &mut HashMap<String, Value>,
) -> Result<Option<usize>, RuntimeError> {
    let number = match from_index {
        None => return Ok(Some(length - 1)),
        Some(Value::Undefined) => 0.0,
        Some(value) => to_number_with_env(value.clone(), env)?,
    };
    if number.is_nan() {
        return Ok(Some(0));
    }
    if number >= 0.0 {
        return Ok(Some(number.trunc().min((length - 1) as f64) as usize));
    }

    let start = length as f64 + number.trunc();
    if start < 0.0 {
        Ok(None)
    } else {
        Ok(Some(start as usize))
    }
}
