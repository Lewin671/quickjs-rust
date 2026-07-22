use std::rc::Rc;

use crate::{
    ArrayRef, PropertyKey, RuntimeError, Value, call_function, has_property_key, property_value,
    property_value_key, regexp, symbol, to_js_string_with_env, to_number_with_env,
    to_uint32_number, to_uint32_with_env,
};

use super::super::{
    indexing::{string_slice_index, string_substring_index, this_string_value},
    push_code_unit, string_code_unit_len, surrogate_escape_code_unit,
};
use super::MAX_STRING_LENGTH;
use crate::CallEnv;

pub(crate) fn native_string_prototype_concat(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let mut result = this_string_value(this_value, env)?;
    for value in argument_values.iter().cloned() {
        result.push_str(&to_js_string_with_env(value, env)?);
    }
    Ok(Value::String(result.into()))
}

pub(crate) fn native_string_prototype_repeat(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let count = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if count.is_infinite() || count < 0.0 {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: repeat count must be a finite non-negative number".to_owned(),
        });
    }
    if count.is_nan() || count == 0.0 {
        return Ok(Value::String(::std::rc::Rc::new(String::new())));
    }

    let count = count.trunc() as usize;
    // The result must not exceed the maximum string length, matching QuickJS-NG
    // (2^30 - 1); otherwise repeat would attempt a multi-gigabyte allocation.
    let too_long = value
        .chars()
        .count()
        .checked_mul(count)
        .is_none_or(|len| len > MAX_STRING_LENGTH);
    if too_long {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid string length".to_owned(),
        });
    }
    Ok(Value::String(value.repeat(count).into()))
}

pub(crate) fn native_string_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = string_sequence_value(this_value, env)?;
    let length = string_code_unit_len(value.as_str());
    let start = string_slice_index(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        0,
        env,
    )?;
    let end = string_slice_index(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        env,
    )?;
    if end <= start {
        return Ok(Value::String(::std::rc::Rc::new(String::new())));
    }
    Ok(Value::String(string_slice_code_units(
        &value, start, end, length,
    )))
}

/// Returns the UTF-16 length of a numeric `String.prototype.slice` result.
///
/// The numeric-loop caller has already proved that the receiver is a stable
/// primitive string, both arguments are numbers, and the intrinsic `slice`
/// method is unchanged. Computing the selected code-unit range directly keeps
/// that general admission contract while avoiding a temporary substring that
/// would otherwise be allocated, copied, measured, and immediately dropped.
pub(crate) fn numeric_string_slice_code_unit_len(
    value: &Rc<String>,
    start: f64,
    end: f64,
) -> usize {
    let length = string_code_unit_len(value);
    let start = numeric_string_slice_index(length, start);
    let end = numeric_string_slice_index(length, end);
    end.saturating_sub(start)
}

pub(crate) fn native_string_prototype_split(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let separator_value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let limit_value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);

    if matches!(this_value, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "String.prototype method called on null or undefined".to_owned(),
        });
    }
    if !matches!(separator_value, Value::Null | Value::Undefined) {
        if let Some(splitter) = symbol_split_method(separator_value.clone(), env)? {
            return call_function(
                splitter,
                separator_value,
                vec![this_value, limit_value],
                env,
                false,
            );
        }
    }

    let value = this_string_value(this_value, env)?;
    let limit = string_split_limit(limit_value, env)?;

    if matches!(separator_value, Value::Undefined) {
        if limit == 0 {
            return Ok(Value::Array(ArrayRef::new(Vec::new())));
        }
        return Ok(Value::Array(ArrayRef::new(vec![Value::String(
            value.into(),
        )])));
    }

    if regexp::regexp_is_regexp(&separator_value) {
        if limit == 0 {
            return Ok(Value::Array(ArrayRef::new(Vec::new())));
        }
        return string_split_regexp(value, separator_value, limit, env);
    }

    let separator = to_js_string_with_env(separator_value, env)?;
    if limit == 0 {
        return Ok(Value::Array(ArrayRef::new(Vec::new())));
    }
    let parts = if separator.is_empty() {
        value
            .chars()
            .take(limit)
            .map(|character| Value::String(character.to_string().into()))
            .collect()
    } else {
        value
            .split(&separator)
            .take(limit)
            .map(|part| Value::String(part.to_owned().into()))
            .collect()
    };
    Ok(Value::Array(ArrayRef::new(parts)))
}

fn symbol_split_method(value: Value, env: &mut CallEnv) -> Result<Option<Value>, RuntimeError> {
    if !is_object_value(&value) {
        return Ok(None);
    }
    let Some(split_symbol) = symbol::split_symbol(env) else {
        return Ok(None);
    };
    let key = PropertyKey::Symbol(split_symbol);
    if !has_property_key(value.clone(), env, &key)? {
        return Ok(None);
    }
    let method = property_value_key(value, &key, env)?;
    if matches!(method, Value::Null | Value::Undefined) {
        return Ok(None);
    }
    if !matches!(method, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Symbol.split method is not callable".to_owned(),
        });
    }
    Ok(Some(method))
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}

fn string_split_regexp(
    input: String,
    separator: Value,
    limit: usize,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let regexp = string_split_regexp_clone(separator, env)?;
    let mut parts = Vec::new();
    let input_len = input.chars().count();
    let mut next_search = 0usize;
    let mut segment_start = 0usize;
    let mut trailing_empty = true;

    while next_search <= input_len {
        regexp::regexp_set_last_index(&regexp, next_search);
        let exec = property_value(regexp.clone(), "exec", env)?;
        let result = call_function(
            exec,
            regexp.clone(),
            vec![Value::String(input.clone().into())],
            env,
            false,
        )?;
        let Value::Array(match_array) = result else {
            break;
        };
        let Some(Value::String(matched)) = match_array.get(0) else {
            break;
        };
        let match_start = regexp_match_index(&Value::Array(match_array.clone()), env)?;
        let match_len = matched.chars().count();
        let match_end = match_start + match_len;
        if match_start < next_search {
            next_search += 1;
            trailing_empty = false;
            continue;
        }

        if match_start == match_end && match_start == segment_start {
            next_search += 1;
            continue;
        }

        parts.push(Value::String(
            input_char_slice(&input, segment_start, match_start).into(),
        ));
        if parts.len() == limit {
            return Ok(Value::Array(ArrayRef::new(parts)));
        }

        segment_start = match_end;
        next_search = if match_start == match_end {
            match_end + 1
        } else {
            match_end
        };
        trailing_empty = match_start == match_end && match_end == input_len;
    }

    if segment_start < input_len || !trailing_empty {
        parts.push(Value::String(
            input_char_slice(&input, segment_start, input_len).into(),
        ));
    }
    Ok(Value::Array(ArrayRef::new(
        parts.into_iter().take(limit).collect(),
    )))
}

fn string_split_regexp_clone(separator: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let constructor = env.get("RegExp").ok_or_else(|| RuntimeError {
        thrown: None,
        message: "RegExp constructor is not available".to_owned(),
    })?;
    call_function(
        constructor,
        Value::Undefined,
        vec![separator, Value::String("g".to_owned().into())],
        env,
        false,
    )
}

fn regexp_match_index(match_value: &Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    let index = property_value(match_value.clone(), "index", env)?;
    let number = to_number_with_env(index, env)?;
    if number.is_nan() || number <= 0.0 {
        Ok(0)
    } else {
        Ok(number.trunc() as usize)
    }
}

fn input_char_slice(input: &str, start: usize, end: usize) -> String {
    input.chars().skip(start).take(end - start).collect()
}

fn string_split_limit(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(u32::MAX as usize);
    }
    match value {
        Value::Object(_) | Value::Function(_) | Value::Array(_) => {
            Ok(to_uint32_number(to_number_with_env(value, env)?) as usize)
        }
        value => Ok(to_uint32_with_env(value, env)? as usize),
    }
}

pub(crate) fn native_string_prototype_substr(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = string_sequence_value(this_value, env)?;
    let length = string_code_unit_len(value.as_str());
    let start = string_substr_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let count = string_substr_count(
        length,
        start,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::String(string_slice_code_units(
        &value,
        start,
        start + count,
        length,
    )))
}

pub(crate) fn native_string_prototype_substring(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = string_sequence_value(this_value, env)?;
    let length = string_code_unit_len(value.as_str());
    let start = string_substring_index(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        0,
        env,
    )?;
    let end = string_substring_index(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        env,
    )?;
    let (from, to) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    Ok(Value::String(string_slice_code_units(
        &value, from, to, length,
    )))
}

enum StringSequenceValue {
    Shared(Rc<String>),
    Owned(String),
}

impl StringSequenceValue {
    fn as_str(&self) -> &str {
        match self {
            Self::Shared(value) => value.as_str(),
            Self::Owned(value) => value.as_str(),
        }
    }
}

fn string_sequence_value(
    value: Value,
    env: &mut CallEnv,
) -> Result<StringSequenceValue, RuntimeError> {
    match value {
        Value::String(value) => Ok(StringSequenceValue::Shared(value)),
        value => this_string_value(value, env).map(StringSequenceValue::Owned),
    }
}

fn string_slice_code_units(
    value: &StringSequenceValue,
    start: usize,
    end: usize,
    length: usize,
) -> Rc<String> {
    if start >= end {
        return Rc::new(String::new());
    }
    if start == 0
        && end == length
        && let StringSequenceValue::Shared(value) = value
    {
        return value.clone();
    }
    let value = value.as_str();
    if value.is_ascii() {
        Rc::new(value[start..end].to_owned())
    } else {
        let mut result = String::with_capacity(value.len().min((end - start).saturating_mul(4)));
        let mut index = 0usize;
        for character in value.chars() {
            let character_units = if surrogate_escape_code_unit(character).is_some() {
                1
            } else {
                character.len_utf16()
            };
            let character_end = index + character_units;
            if character_end <= start {
                index = character_end;
                continue;
            }
            if index >= end {
                break;
            }

            let selected_start = start.saturating_sub(index);
            let selected_end = end.min(character_end) - index;
            if selected_start == 0 && selected_end == character_units {
                result.push(character);
            } else if let Some(code_unit) = surrogate_escape_code_unit(character) {
                push_code_unit(&mut result, code_unit);
            } else {
                let mut buffer = [0; 2];
                let encoded = character.encode_utf16(&mut buffer);
                for code_unit in &encoded[selected_start..selected_end] {
                    push_code_unit(&mut result, *code_unit);
                }
            }
            index = character_end;
        }
        Rc::new(result)
    }
}

fn numeric_string_slice_index(length: usize, number: f64) -> usize {
    if number.is_nan() {
        return 0;
    }
    let integer = number.trunc();
    if integer < 0.0 {
        (length as f64 + integer).max(0.0) as usize
    } else {
        integer.min(length as f64) as usize
    }
}

fn string_substr_start(
    length: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
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

fn string_substr_count(
    length: usize,
    start: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    let remaining = length - start;
    if matches!(value, Value::Undefined) {
        return Ok(remaining);
    }

    let number = to_number_with_env(value, env)?;
    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    if number.is_infinite() {
        return Ok(remaining);
    }
    Ok((number.trunc() as usize).min(remaining))
}

#[cfg(test)]
mod tests {
    use super::numeric_string_slice_code_unit_len;

    #[test]
    fn numeric_slice_length_uses_utf16_code_units_without_materializing() {
        let ascii = std::rc::Rc::new("abcdef".to_owned());
        assert_eq!(numeric_string_slice_code_unit_len(&ascii, 1.0, 4.0), 3);
        assert_eq!(numeric_string_slice_code_unit_len(&ascii, 1.9, 4.9), 3);
        assert_eq!(numeric_string_slice_code_unit_len(&ascii, -4.0, -1.0), 3);
        assert_eq!(numeric_string_slice_code_unit_len(&ascii, 4.0, 1.0), 0);

        let supplementary = std::rc::Rc::new("😀x".to_owned());
        assert_eq!(
            numeric_string_slice_code_unit_len(&supplementary, 0.0, 1.0),
            1
        );
        assert_eq!(
            numeric_string_slice_code_unit_len(&supplementary, 0.0, 2.0),
            2
        );
        assert_eq!(
            numeric_string_slice_code_unit_len(&supplementary, 1.0, 2.0),
            1
        );

        let escaped_surrogates = std::rc::Rc::new(crate::string::string_from_code_units(&[
            0xD800, 0x61, 0xDC00,
        ]));
        assert_eq!(
            numeric_string_slice_code_unit_len(&escaped_surrogates, 0.0, 1.0),
            1
        );
        assert_eq!(
            numeric_string_slice_code_unit_len(&escaped_surrogates, 0.0, 3.0),
            3
        );
    }

    #[test]
    fn numeric_slice_length_normalizes_non_finite_and_out_of_range_indices() {
        let value = std::rc::Rc::new("abcdef".to_owned());
        assert_eq!(
            numeric_string_slice_code_unit_len(&value, f64::NAN, f64::INFINITY),
            6
        );
        assert_eq!(
            numeric_string_slice_code_unit_len(&value, f64::NEG_INFINITY, -1.0),
            5
        );
        assert_eq!(
            numeric_string_slice_code_unit_len(&value, f64::INFINITY, f64::NEG_INFINITY),
            0
        );
        assert_eq!(numeric_string_slice_code_unit_len(&value, -100.0, 100.0), 6);
    }
}
