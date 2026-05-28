use crate::{ArrayRef, RuntimeError, Value, to_js_string, to_number};

pub(super) fn native_array(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    if argument_values.len() == 1 && matches!(argument_values[0], Value::Number(_)) {
        return Err(RuntimeError {
            message: "Array length construction requires sparse array support".to_owned(),
        });
    }

    Ok(Value::Array(ArrayRef::new(argument_values.to_vec())))
}

pub(super) fn native_array_is_array(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Array(_))
    )))
}

pub(super) fn native_array_prototype_concat(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let mut result = Vec::new();
    concat_array_item(&mut result, this_value);
    for value in argument_values.iter().cloned() {
        concat_array_item(&mut result, value);
    }
    Ok(Value::Array(ArrayRef::new(result)))
}

fn concat_array_item(result: &mut Vec<Value>, value: Value) {
    match value {
        Value::Array(elements) => result.extend(elements.to_vec()),
        value => result.push(value),
    }
}

pub(super) fn native_array_prototype_at(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
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

fn array_at_index(length: usize, index: Value) -> Result<Option<usize>, RuntimeError> {
    let number = match index {
        Value::Undefined => 0.0,
        value => to_number(value)?,
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

pub(super) fn native_array_prototype_includes(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
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

fn same_value_zero(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            left == right || (left.is_nan() && right.is_nan())
        }
        _ => left == right,
    }
}

pub(super) fn native_array_prototype_index_of(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            message: "Array.prototype.indexOf called on non-array".to_owned(),
        });
    };
    if elements.is_empty() {
        return Ok(Value::Number(-1.0));
    }

    let search_element = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let start = array_search_start_index(
        elements.len(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    for (index, element) in elements.to_vec().iter().enumerate().skip(start) {
        if *element == search_element {
            return Ok(Value::Number(index as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

pub(super) fn native_array_prototype_last_index_of(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            message: "Array.prototype.lastIndexOf called on non-array".to_owned(),
        });
    };
    if elements.is_empty() {
        return Ok(Value::Number(-1.0));
    }

    let search_element = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Some(start) = array_search_end_index(
        elements.len(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?
    else {
        return Ok(Value::Number(-1.0));
    };
    let elements = elements.to_vec();
    for index in (0..=start).rev() {
        if elements[index] == search_element {
            return Ok(Value::Number(index as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

fn array_search_start_index(length: usize, from_index: Value) -> Result<usize, RuntimeError> {
    let number = match from_index {
        Value::Undefined => 0.0,
        value => to_number(value)?,
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

fn array_search_end_index(length: usize, from_index: Value) -> Result<Option<usize>, RuntimeError> {
    let number = match from_index {
        Value::Undefined => return Ok(Some(length - 1)),
        value => to_number(value)?,
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

pub(super) fn native_array_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            message: "Array.prototype.slice called on non-array".to_owned(),
        });
    };

    let length = elements.len();
    let start = array_slice_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;

    if end <= start {
        return Ok(Value::Array(ArrayRef::new(Vec::new())));
    }
    Ok(Value::Array(ArrayRef::new(
        elements.to_vec()[start..end].to_vec(),
    )))
}

fn array_slice_start(length: usize, start: Value) -> Result<usize, RuntimeError> {
    let number = match start {
        Value::Undefined => 0.0,
        value => to_number(value)?,
    };
    Ok(relative_array_index(length, number))
}

fn array_slice_end(length: usize, end: Value) -> Result<usize, RuntimeError> {
    let number = match end {
        Value::Undefined => return Ok(length),
        value => to_number(value)?,
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

pub(super) fn native_array_prototype_fill(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            message: "Array.prototype.fill called on non-array".to_owned(),
        });
    };

    let length = elements.len();
    let start = array_slice_start(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
    )?;
    if start < end {
        elements.fill(
            start,
            end,
            argument_values.first().cloned().unwrap_or(Value::Undefined),
        );
    }
    Ok(this_value)
}

pub(super) fn native_array_prototype_copy_within(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            message: "Array.prototype.copyWithin called on non-array".to_owned(),
        });
    };

    let length = elements.len();
    let target = array_slice_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?;
    let start = array_slice_start(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
    )?;
    let count = (end.saturating_sub(start)).min(length.saturating_sub(target));
    if count == 0 {
        return Ok(this_value);
    }

    let snapshot = elements.to_vec();
    for offset in 0..count {
        elements.set(target + offset, snapshot[start + offset].clone());
    }
    Ok(this_value)
}

pub(super) fn native_array_prototype_join(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let separator = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => ",".to_owned(),
        value => to_js_string(value)?,
    };
    Ok(Value::String(array_join(this_value, &separator)?))
}

pub(super) fn native_array_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::String(array_join(this_value, ",")?))
}

pub(super) fn native_array_prototype_push(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            message: "Array.prototype.push called on non-array".to_owned(),
        });
    };
    for value in argument_values.iter().cloned() {
        elements.push(value);
    }
    Ok(Value::Number(elements.len() as f64))
}

pub(super) fn native_array_prototype_pop(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            message: "Array.prototype.pop called on non-array".to_owned(),
        });
    };
    Ok(elements.pop().unwrap_or(Value::Undefined))
}

pub(super) fn native_array_prototype_shift(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            message: "Array.prototype.shift called on non-array".to_owned(),
        });
    };
    Ok(elements.shift().unwrap_or(Value::Undefined))
}

pub(super) fn native_array_prototype_unshift(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            message: "Array.prototype.unshift called on non-array".to_owned(),
        });
    };
    Ok(Value::Number(elements.unshift(argument_values) as f64))
}

pub(super) fn native_array_prototype_reverse(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            message: "Array.prototype.reverse called on non-array".to_owned(),
        });
    };
    elements.reverse();
    Ok(this_value)
}

fn array_join(value: Value, separator: &str) -> Result<String, RuntimeError> {
    let Value::Array(elements) = value else {
        return Err(RuntimeError {
            message: "Array.prototype.join called on non-array".to_owned(),
        });
    };

    let elements = elements.to_vec();
    let mut parts = Vec::with_capacity(elements.len());
    for element in elements {
        let part = match element {
            Value::Null | Value::Undefined => String::new(),
            Value::Array(_) => array_join(element, ",")?,
            value => to_js_string(value)?,
        };
        parts.push(part);
    }
    Ok(parts.join(separator))
}
