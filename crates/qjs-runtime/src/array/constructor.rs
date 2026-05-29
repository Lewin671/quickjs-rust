use std::collections::HashMap;

use crate::{ArrayRef, RuntimeError, Value, call_function, to_length};

pub(crate) fn native_array(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    if argument_values.len() == 1 && matches!(argument_values[0], Value::Number(_)) {
        return Err(RuntimeError {
            message: "Array length construction requires sparse array support".to_owned(),
        });
    }

    Ok(Value::Array(ArrayRef::new(argument_values.to_vec())))
}

pub(crate) fn native_array_is_array(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Array(_))
    )))
}

pub(crate) fn native_array_from(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let items = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let map_fn = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let this_arg = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let mapping = match map_fn {
        Value::Undefined => None,
        Value::Function(_) => Some(map_fn),
        _ => {
            return Err(RuntimeError {
                message: "Array.from map function is not callable".to_owned(),
            });
        }
    };

    let values = array_from_values(items)?;
    let mut result = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        let value = if let Some(callback) = &mapping {
            call_function(
                callback.clone(),
                this_arg.clone(),
                vec![value, Value::Number(index as f64)],
                env,
                false,
            )?
        } else {
            value
        };
        result.push(value);
    }

    Ok(Value::Array(ArrayRef::new(result)))
}

pub(crate) fn native_array_of(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Array(ArrayRef::new(argument_values.to_vec())))
}

fn array_from_values(items: Value) -> Result<Vec<Value>, RuntimeError> {
    match items {
        Value::Array(array) => Ok(array.to_vec()),
        Value::String(value) => Ok(value
            .chars()
            .map(|character| Value::String(character.to_string()))
            .collect()),
        Value::Object(object) => {
            let length = to_length(object.get("length").unwrap_or(Value::Undefined))?;
            Ok((0..length)
                .map(|index| object.get(&index.to_string()).unwrap_or(Value::Undefined))
                .collect())
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "Array.from requires an array-like value".to_owned(),
        }),
        _ => Err(RuntimeError {
            message: "Array.from unsupported source value".to_owned(),
        }),
    }
}
