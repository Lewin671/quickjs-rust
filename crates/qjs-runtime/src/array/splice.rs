use std::collections::HashMap;

use crate::{ArrayRef, RuntimeError, Value, to_number, to_number_with_env};

pub(crate) fn native_array_prototype_splice(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(array) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.splice called on non-array".to_owned(),
        });
    };

    let length = array.len();
    let start = splice_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?;
    let delete_count = splice_delete_count(length, start, argument_values)?;
    let items = if argument_values.len() > 2 {
        &argument_values[2..]
    } else {
        &[]
    };

    let removed = array.splice(start, delete_count, items);
    Ok(Value::Array(ArrayRef::new(removed)))
}

pub(super) fn splice_start(length: usize, start: Value) -> Result<usize, RuntimeError> {
    let mut env = HashMap::new();
    splice_start_with_env(length, start, &mut env)
}

pub(super) fn splice_start_with_env(
    length: usize,
    start: Value,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let number = match start {
        Value::Undefined => 0.0,
        value => to_number_with_env(value, env)?,
    };
    if number.is_nan() {
        return Ok(0);
    }
    if number.is_infinite() {
        return Ok(if number.is_sign_negative() { 0 } else { length });
    }

    let integer = number.trunc();
    if integer < 0.0 {
        Ok((length as f64 + integer).max(0.0) as usize)
    } else {
        Ok(integer.min(length as f64) as usize)
    }
}

pub(super) fn splice_delete_count(
    length: usize,
    start: usize,
    argument_values: &[Value],
) -> Result<usize, RuntimeError> {
    if argument_values.len() < 2 {
        return Ok(length.saturating_sub(start));
    }

    let number = to_number(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    Ok((number.trunc() as usize).min(length.saturating_sub(start)))
}

pub(super) fn to_spliced_delete_count(
    length: usize,
    start: usize,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    if argument_values.is_empty() {
        return Ok(0);
    }
    if argument_values.len() < 2 {
        return Ok(length.saturating_sub(start));
    }

    let number = to_number_with_env(argument_values[1].clone(), env)?;
    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    if number.is_infinite() {
        return Ok(length.saturating_sub(start));
    }
    Ok((number.trunc() as usize).min(length.saturating_sub(start)))
}
