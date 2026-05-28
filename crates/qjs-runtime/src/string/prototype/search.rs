use std::collections::HashMap;

use crate::{RuntimeError, Value, to_js_string};

use super::super::indexing::{
    string_end_position, string_last_search_position, string_search_start, this_string_value,
};

pub(crate) fn native_string_prototype_ends_with(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let end = string_end_position(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    let prefix = value.chars().take(end).collect::<String>();
    Ok(Value::Boolean(prefix.ends_with(&search)))
}

pub(crate) fn native_string_prototype_includes(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let start = string_search_start(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    Ok(Value::Boolean(
        value
            .chars()
            .skip(start)
            .collect::<String>()
            .contains(&search),
    ))
}

pub(crate) fn native_string_prototype_index_of(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let start = string_search_start(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    let haystack = value.chars().skip(start).collect::<String>();
    let Some(byte_index) = haystack.find(&search) else {
        return Ok(Value::Number(-1.0));
    };
    let char_offset = haystack[..byte_index].chars().count();
    Ok(Value::Number((start + char_offset) as f64))
}

pub(crate) fn native_string_prototype_last_index_of(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let chars: Vec<_> = value.chars().collect();
    let search_chars: Vec<_> = search.chars().collect();
    let position = string_last_search_position(
        chars.len(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;

    if search_chars.is_empty() {
        return Ok(Value::Number(position as f64));
    }
    if search_chars.len() > chars.len() {
        return Ok(Value::Number(-1.0));
    }

    let start = position.min(chars.len() - search_chars.len());
    for candidate in (0..=start).rev() {
        if chars[candidate..candidate + search_chars.len()] == search_chars {
            return Ok(Value::Number(candidate as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

pub(crate) fn native_string_prototype_starts_with(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let start = string_search_start(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    Ok(Value::Boolean(
        value
            .chars()
            .skip(start)
            .collect::<String>()
            .starts_with(&search),
    ))
}
