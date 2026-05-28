use std::collections::HashMap;

use crate::{ArrayRef, RuntimeError, Value, to_js_string, to_number, to_uint32};

use super::super::indexing::{string_slice_index, string_substring_index, this_string_value};

pub(crate) fn native_string_prototype_concat(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let mut result = this_string_value(this_value, env)?;
    for value in argument_values.iter().cloned() {
        result.push_str(&to_js_string(value)?);
    }
    Ok(Value::String(result))
}

pub(crate) fn native_string_prototype_repeat(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let count = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    if count.is_infinite() || count < 0.0 {
        return Err(RuntimeError {
            message: "repeat count must be a finite non-negative number".to_owned(),
        });
    }
    if count.is_nan() || count == 0.0 {
        return Ok(Value::String(String::new()));
    }

    let count = count.trunc() as usize;
    Ok(Value::String(value.repeat(count)))
}

pub(crate) fn native_string_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let chars: Vec<_> = value.chars().collect();
    let length = chars.len();
    let start = string_slice_index(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        0,
    )?;
    let end = string_slice_index(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
    )?;
    if end <= start {
        return Ok(Value::String(String::new()));
    }
    Ok(Value::String(chars[start..end].iter().collect()))
}

pub(crate) fn native_string_prototype_split(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let limit = string_split_limit(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    if limit == 0 {
        return Ok(Value::Array(ArrayRef::new(Vec::new())));
    }
    if matches!(argument_values.first(), None | Some(Value::Undefined)) {
        return Ok(Value::Array(ArrayRef::new(vec![Value::String(value)])));
    }

    let separator = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let parts = if separator.is_empty() {
        value
            .chars()
            .take(limit)
            .map(|character| Value::String(character.to_string()))
            .collect()
    } else {
        value
            .split(&separator)
            .take(limit)
            .map(|part| Value::String(part.to_owned()))
            .collect()
    };
    Ok(Value::Array(ArrayRef::new(parts)))
}

fn string_split_limit(value: Value) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(u32::MAX as usize);
    }
    Ok(to_uint32(value)? as usize)
}

pub(crate) fn native_string_prototype_substring(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let chars: Vec<_> = value.chars().collect();
    let length = chars.len();
    let start = string_substring_index(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        0,
    )?;
    let end = string_substring_index(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
    )?;
    let (from, to) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    Ok(Value::String(chars[from..to].iter().collect()))
}
