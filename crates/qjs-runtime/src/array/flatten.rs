use std::collections::HashMap;

use crate::{ArrayRef, RuntimeError, Value, call_function, to_number};

pub(crate) fn native_array_prototype_flat(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(array) = this_value else {
        return Err(RuntimeError {
            message: "Array.prototype.flat called on non-array".to_owned(),
        });
    };

    let depth = flat_depth(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let mut result = Vec::new();
    flatten_into(&mut result, array.to_vec(), depth);
    Ok(Value::Array(ArrayRef::new(result)))
}

pub(crate) fn native_array_prototype_flat_map(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Array(array) = this_value else {
        return Err(RuntimeError {
            message: "Array.prototype.flatMap called on non-array".to_owned(),
        });
    };
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            message: "Array.prototype.flatMap callback is not callable".to_owned(),
        });
    }

    let callback_this = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let source = array.to_vec();
    let mut result = Vec::new();
    for (index, value) in source.iter().cloned().enumerate() {
        let mapped = call_function(
            callback.clone(),
            callback_this.clone(),
            vec![
                value,
                Value::Number(index as f64),
                Value::Array(array.clone()),
            ],
            env,
            false,
        )?;
        flatten_value_into(&mut result, mapped, 1);
    }

    Ok(Value::Array(ArrayRef::new(result)))
}

fn flat_depth(value: Value) -> Result<usize, RuntimeError> {
    let number = match value {
        Value::Undefined => return Ok(1),
        value => to_number(value)?,
    };

    if number.is_nan() || number <= 0.0 {
        return Ok(0);
    }
    if number.is_infinite() {
        return Ok(usize::MAX);
    }
    Ok(number.trunc() as usize)
}

fn flatten_into(result: &mut Vec<Value>, values: Vec<Value>, depth: usize) {
    for value in values {
        flatten_value_into(result, value, depth);
    }
}

fn flatten_value_into(result: &mut Vec<Value>, value: Value, depth: usize) {
    match value {
        Value::Array(array) if depth > 0 => {
            flatten_into(result, array.to_vec(), depth.saturating_sub(1));
        }
        value => result.push(value),
    }
}
