use std::collections::HashMap;

use crate::{ArrayRef, RuntimeError, Value, to_js_string, to_length, to_number, to_uint32};

use super::indexing::{
    relative_string_code_unit_index, string_end_position, string_last_search_position,
    string_search_start, string_slice_index, string_substring_index, this_string_value,
    to_char_code_position, to_string_position,
};

pub(crate) fn native_string_prototype_at(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let code_units: Vec<u16> = value.encode_utf16().collect();
    let Some(index) = relative_string_code_unit_index(
        code_units.len(),
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?
    else {
        return Ok(Value::Undefined);
    };

    Ok(String::from_utf16(&[code_units[index]])
        .map(Value::String)
        .unwrap_or_else(|_| Value::String(char::REPLACEMENT_CHARACTER.to_string())))
}

pub(crate) fn native_string_prototype_char_at(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let index = to_string_position(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::String(
        value
            .chars()
            .nth(index)
            .map(|character| character.to_string())
            .unwrap_or_default(),
    ))
}

pub(crate) fn native_string_prototype_char_code_at(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let position =
        to_char_code_position(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    if position < 0.0 {
        return Ok(Value::Number(f64::NAN));
    }

    let code_units: Vec<u16> = value.encode_utf16().collect();
    let index = position as usize;
    Ok(code_units
        .get(index)
        .map(|code_unit| Value::Number(f64::from(*code_unit)))
        .unwrap_or(Value::Number(f64::NAN)))
}

pub(crate) fn native_string_prototype_code_point_at(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let position =
        to_char_code_position(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    if position < 0.0 || !position.is_finite() {
        return Ok(Value::Undefined);
    }

    let code_units: Vec<u16> = value.encode_utf16().collect();
    let index = position as usize;
    let Some(first) = code_units.get(index).copied() else {
        return Ok(Value::Undefined);
    };
    if !(0xD800..=0xDBFF).contains(&first) || index + 1 == code_units.len() {
        return Ok(Value::Number(f64::from(first)));
    }

    let second = code_units[index + 1];
    if !(0xDC00..=0xDFFF).contains(&second) {
        return Ok(Value::Number(f64::from(first)));
    }

    let code_point = (u32::from(first) - 0xD800) * 1024 + (u32::from(second) - 0xDC00) + 0x10000;
    Ok(Value::Number(f64::from(code_point)))
}

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

#[derive(Clone, Copy)]
pub(crate) enum StringPadKind {
    Start,
    End,
}

pub(crate) fn native_string_prototype_pad(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
    kind: StringPadKind,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let max_length = to_length(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let string_length = value.chars().count();
    if max_length <= string_length {
        return Ok(Value::String(value));
    }

    let fill_string = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => " ".to_owned(),
        value => to_js_string(value)?,
    };
    if fill_string.is_empty() {
        return Ok(Value::String(value));
    }

    let fill_length = max_length - string_length;
    let filler = repeated_prefix(&fill_string, fill_length);
    Ok(Value::String(match kind {
        StringPadKind::Start => format!("{filler}{value}"),
        StringPadKind::End => format!("{value}{filler}"),
    }))
}

fn repeated_prefix(pattern: &str, length: usize) -> String {
    pattern.chars().cycle().take(length).collect()
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

pub(crate) fn native_string_prototype_to_lower_case(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.to_lowercase(),
    ))
}

pub(crate) fn native_string_prototype_trim(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.trim().to_owned(),
    ))
}

pub(crate) fn native_string_prototype_trim_end(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.trim_end().to_owned(),
    ))
}

pub(crate) fn native_string_prototype_trim_start(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.trim_start().to_owned(),
    ))
}

pub(crate) fn native_string_prototype_to_string(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(this_string_value(this_value, env)?))
}

pub(crate) fn native_string_prototype_to_upper_case(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        this_string_value(this_value, env)?.to_uppercase(),
    ))
}
