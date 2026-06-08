use std::collections::HashMap;

use crate::{
    ArrayRef, PropertyKey, RuntimeError, Value, call_function, is_truthy, property_value,
    property_value_key, symbol,
};

use super::array_like::array_like_values_with_env;

pub(crate) fn native_array(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    if let [Value::Number(length)] = argument_values {
        if !length.is_finite() || *length < 0.0 || length.fract() != 0.0 {
            return Err(RuntimeError {
                thrown: None,
                message: "RangeError: invalid array length".to_owned(),
            });
        }
        let length = *length as usize;
        return Ok(Value::Array(ArrayRef::new_sparse(
            vec![Value::Undefined; length],
            (0..length).collect(),
        )));
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
                thrown: None,
                message: "Array.from map function is not callable".to_owned(),
            });
        }
    };

    let values = array_from_values(items, mapping.as_ref(), this_arg, env)?;
    Ok(Value::Array(ArrayRef::new(values)))
}

fn array_from_values(
    items: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    if matches!(items, Value::Null | Value::Undefined) {
        return array_like_values_with_env(items, "Array.from", env);
    }

    let iterator_method = match symbol::iterator_symbol(env) {
        Some(iterator_symbol) => {
            property_value_key(items.clone(), &PropertyKey::Symbol(iterator_symbol), env)?
        }
        None => Value::Undefined,
    };

    match iterator_method {
        Value::Undefined | Value::Null => map_array_like_values(items, mapping, this_arg, env),
        Value::Function(_) => {
            array_from_iterable_values(items, iterator_method, mapping, this_arg, env)
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "Array.from iterator method is not callable".to_owned(),
        }),
    }
}

fn map_array_like_values(
    items: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let values = array_like_values_with_env(items, "Array.from", env)?;
    let mut result = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        result.push(array_from_mapped_value(
            value,
            index,
            mapping,
            this_arg.clone(),
            env,
        )?);
    }
    Ok(result)
}

fn array_from_iterable_values(
    items: Value,
    iterator_method: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let iterator = call_function(iterator_method, items, Vec::new(), env, false)?;
    let next = property_value(iterator.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.from iterator next method is not callable".to_owned(),
        });
    }

    let mut result = Vec::new();
    loop {
        let step = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
        if !is_iterator_result_object(&step) {
            return Err(RuntimeError {
                thrown: None,
                message: "Array.from iterator result is not an object".to_owned(),
            });
        }
        if is_truthy(&property_value(step.clone(), "done", env)?) {
            break;
        }
        let value = property_value(step, "value", env)?;
        let index = result.len();
        result.push(array_from_mapped_value(
            value,
            index,
            mapping,
            this_arg.clone(),
            env,
        )?);
    }
    Ok(result)
}

fn array_from_mapped_value(
    value: Value,
    index: usize,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if let Some(callback) = mapping {
        call_function(
            callback.clone(),
            this_arg,
            vec![value, Value::Number(index as f64)],
            env,
            false,
        )
    } else {
        Ok(value)
    }
}

fn is_iterator_result_object(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}

pub(crate) fn native_array_of(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Array(ArrayRef::new(argument_values.to_vec())))
}
