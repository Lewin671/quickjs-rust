use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    RuntimeError, Value, string_object_value, string_prototype, to_js_string_with_env,
    to_number_with_env,
};

pub(super) fn this_string_value(value: Value, env: &mut CallEnv) -> Result<String, RuntimeError> {
    match value {
        Value::String(value) => Ok(value),
        Value::Object(object) => {
            if let Some(value) = string_object_value(&object) {
                Ok(value)
            } else if string_prototype(env).is_some_and(|prototype| object.ptr_eq(&prototype)) {
                Ok(String::new())
            } else {
                to_js_string_with_env(Value::Object(object), env)
            }
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "String.prototype method called on null or undefined".to_owned(),
        }),
        value => to_js_string_with_env(value, env),
    }
}

pub(super) fn to_string_position(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    if number.is_nan() || number <= 0.0 {
        Ok(0)
    } else if number.is_infinite() {
        Ok(usize::MAX)
    } else {
        Ok(number.trunc() as usize)
    }
}

pub(super) fn to_char_code_position(value: Value, env: &mut CallEnv) -> Result<f64, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    if number.is_nan() {
        Ok(0.0)
    } else {
        Ok(number.trunc())
    }
}

pub(super) fn relative_string_code_unit_index(
    length: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<Option<usize>, RuntimeError> {
    let number = match value {
        Value::Undefined => 0.0,
        value => to_number_with_env(value, env)?,
    };
    let integer = if number.is_nan() { 0.0 } else { number.trunc() };
    let index = if integer < 0.0 {
        length as f64 + integer
    } else {
        integer
    };
    if index < 0.0 || index >= length as f64 {
        Ok(None)
    } else {
        Ok(Some(index as usize))
    }
}

pub(super) fn string_search_start(
    length: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    Ok(to_string_position(value, env)?.min(length))
}

pub(super) fn string_last_search_position(
    length: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(length);
    }
    let number = to_number_with_env(value, env)?;
    if number.is_nan() {
        Ok(length)
    } else if number <= 0.0 {
        Ok(0)
    } else if number.is_infinite() {
        Ok(length)
    } else {
        Ok((number.trunc() as usize).min(length))
    }
}

pub(super) fn string_end_position(
    length: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(length);
    }
    Ok(to_string_position(value, env)?.min(length))
}

pub(super) fn string_slice_index(
    length: usize,
    value: Value,
    default: usize,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(default);
    }
    let number = to_number_with_env(value, env)?;
    if number.is_nan() {
        return Ok(0);
    }
    let integer = number.trunc();
    if integer < 0.0 {
        Ok((length as f64 + integer).max(0.0) as usize)
    } else {
        Ok(integer.min(length as f64) as usize)
    }
}

pub(super) fn string_substring_index(
    length: usize,
    value: Value,
    default: usize,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(default);
    }
    let number = to_number_with_env(value, env)?;
    if number.is_nan() || number <= 0.0 {
        Ok(0)
    } else {
        Ok(number.trunc().min(length as f64) as usize)
    }
}

pub(super) fn canonical_string_index(key: &str) -> Option<usize> {
    if key.is_empty() {
        return None;
    }

    let index = key.parse::<usize>().ok()?;
    if index.to_string() == key {
        Some(index)
    } else {
        None
    }
}
