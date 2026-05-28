use crate::{RuntimeError, Value};

use super::indexing::{array_at_index, array_search_end_index, array_search_start_index};

pub(crate) fn native_array_prototype_at(
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

pub(crate) fn native_array_prototype_includes(
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

pub(crate) fn native_array_prototype_index_of(
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

pub(crate) fn native_array_prototype_last_index_of(
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

fn same_value_zero(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => {
            left == right || (left.is_nan() && right.is_nan())
        }
        _ => left == right,
    }
}
