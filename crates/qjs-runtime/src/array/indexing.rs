use std::collections::HashMap;

use crate::{RuntimeError, Value, to_number_with_env};

pub(super) fn array_at_index(
    length: usize,
    index: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Option<usize>, RuntimeError> {
    let number = match index {
        Value::Undefined => 0.0,
        value => to_number_with_env(value, env)?,
    };
    if number.is_nan() {
        return Ok(Some(0));
    }

    let integer = number.trunc();
    let resolved = if integer < 0.0 {
        length as f64 + integer
    } else {
        integer
    };
    if resolved < 0.0 || resolved >= length as f64 {
        Ok(None)
    } else {
        Ok(Some(resolved as usize))
    }
}

pub(super) fn array_search_start_index(
    length: usize,
    from_index: Value,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let number = match from_index {
        Value::Undefined => 0.0,
        value => to_number_with_env(value, env)?,
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

pub(super) fn array_search_end_index(
    length: usize,
    from_index: Option<Value>,
    env: &mut HashMap<String, Value>,
) -> Result<Option<usize>, RuntimeError> {
    let number = match from_index {
        None => return Ok(Some(length - 1)),
        Some(value) => to_number_with_env(value, env)?,
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

pub(super) fn array_slice_start(
    length: usize,
    start: Value,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let number = match start {
        Value::Undefined => 0.0,
        value => to_number_with_env(value, env)?,
    };
    Ok(relative_array_index(length, number))
}

pub(super) fn array_slice_end(
    length: usize,
    end: Value,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let number = match end {
        Value::Undefined => return Ok(length),
        value => to_number_with_env(value, env)?,
    };
    Ok(relative_array_index(length, number))
}

fn relative_array_index(length: usize, number: f64) -> usize {
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
