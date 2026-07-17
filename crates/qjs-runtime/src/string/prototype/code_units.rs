use std::rc::Rc;

use crate::{
    RuntimeError, Value,
    string::{string_code_unit_at, string_code_unit_len, string_from_code_unit},
};

use super::super::indexing::{
    relative_string_code_unit_index, this_string_value, to_char_code_position,
};
use crate::CallEnv;

fn shared_string_value(value: Value, env: &mut CallEnv) -> Result<Rc<String>, RuntimeError> {
    match value {
        Value::String(value) => Ok(value),
        value => this_string_value(value, env).map(Rc::new),
    }
}

pub(crate) fn native_string_prototype_at(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = shared_string_value(this_value, env)?;
    let Some(index) = relative_string_code_unit_index(
        string_code_unit_len(&value),
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?
    else {
        return Ok(Value::Undefined);
    };

    let Some(code_unit) = string_code_unit_at(&value, index) else {
        return Ok(Value::Undefined);
    };
    Ok(Value::String(string_from_code_unit(code_unit).into()))
}

pub(crate) fn native_string_prototype_char_at(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = shared_string_value(this_value, env)?;
    let position = to_char_code_position(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if position < 0.0 {
        return Ok(Value::String(::std::rc::Rc::new(String::new())));
    }
    let index = position as usize;
    Ok(Value::String(
        string_code_unit_at(&value, index)
            .map(string_from_code_unit)
            .unwrap_or_default()
            .into(),
    ))
}

pub(crate) fn native_string_prototype_char_code_at(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = shared_string_value(this_value, env)?;
    let position = to_char_code_position(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if position < 0.0 {
        return Ok(Value::Number(f64::NAN));
    }

    let index = position as usize;
    Ok(string_code_unit_at(&value, index)
        .map(|code_unit| Value::Number(f64::from(code_unit)))
        .unwrap_or(Value::Number(f64::NAN)))
}

pub(crate) fn native_string_prototype_code_point_at(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = shared_string_value(this_value, env)?;
    let position = to_char_code_position(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if position < 0.0 || !position.is_finite() {
        return Ok(Value::Undefined);
    }

    let index = position as usize;
    let Some(first) = string_code_unit_at(&value, index) else {
        return Ok(Value::Undefined);
    };
    if !(0xD800..=0xDBFF).contains(&first) {
        return Ok(Value::Number(f64::from(first)));
    }

    let Some(second) = string_code_unit_at(&value, index + 1) else {
        return Ok(Value::Number(f64::from(first)));
    };
    if !(0xDC00..=0xDFFF).contains(&second) {
        return Ok(Value::Number(f64::from(first)));
    }

    let code_point = (u32::from(first) - 0xD800) * 1024 + (u32::from(second) - 0xDC00) + 0x10000;
    Ok(Value::Number(f64::from(code_point)))
}
