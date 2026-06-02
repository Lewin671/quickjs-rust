use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::super::indexing::{
    relative_string_code_unit_index, this_string_value, to_char_code_position,
};

pub(crate) fn native_string_prototype_at(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let code_units: Vec<u16> = value.encode_utf16().collect();
    let Some(index) = relative_string_code_unit_index(
        code_units.len(),
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
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
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let position = to_char_code_position(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if position < 0.0 {
        return Ok(Value::String(String::new()));
    }
    let index = position as usize;
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
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let position = to_char_code_position(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
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
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let position = to_char_code_position(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
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
