use std::collections::HashMap;

use crate::{ArrayRef, RuntimeError, Value, call_function, is_truthy};

pub(crate) fn native_array_prototype_map(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            message: "Array.prototype.map called on non-array".to_owned(),
        });
    };
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            message: "Array.prototype.map callback is not callable".to_owned(),
        });
    }

    let callback_this = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let source = elements.to_vec();
    let mut mapped = Vec::with_capacity(source.len());
    for (index, value) in source.into_iter().enumerate() {
        mapped.push(call_function(
            callback.clone(),
            callback_this.clone(),
            vec![
                value,
                Value::Number(index as f64),
                Value::Array(elements.clone()),
            ],
            env,
            false,
        )?);
    }

    Ok(Value::Array(ArrayRef::new(mapped)))
}

pub(crate) fn native_array_prototype_filter(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            message: "Array.prototype.filter called on non-array".to_owned(),
        });
    };
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            message: "Array.prototype.filter callback is not callable".to_owned(),
        });
    }

    let callback_this = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let source = elements.to_vec();
    let mut filtered = Vec::new();
    for (index, value) in source.into_iter().enumerate() {
        let selected = call_function(
            callback.clone(),
            callback_this.clone(),
            vec![
                value.clone(),
                Value::Number(index as f64),
                Value::Array(elements.clone()),
            ],
            env,
            false,
        )?;
        if is_truthy(&selected) {
            filtered.push(value);
        }
    }

    Ok(Value::Array(ArrayRef::new(filtered)))
}

pub(crate) fn native_array_prototype_for_each(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            message: "Array.prototype.forEach called on non-array".to_owned(),
        });
    };
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            message: "Array.prototype.forEach callback is not callable".to_owned(),
        });
    }

    let callback_this = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let source = elements.to_vec();
    for (index, value) in source.into_iter().enumerate() {
        call_function(
            callback.clone(),
            callback_this.clone(),
            vec![
                value,
                Value::Number(index as f64),
                Value::Array(elements.clone()),
            ],
            env,
            false,
        )?;
    }

    Ok(Value::Undefined)
}
