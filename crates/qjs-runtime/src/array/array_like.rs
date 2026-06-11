use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    ObjectRef, PropertyKey, RuntimeError, Value, call_function, is_truthy, object, property_value,
    property_value_key, string_prototype, symbol, to_length_with_env,
};

pub(crate) struct ArrayLikeLength {
    pub(crate) receiver: Value,
    pub(crate) length: usize,
}

pub(crate) fn array_like_length(
    value: Value,
    context: &str,
    env: &mut CallEnv,
) -> Result<ArrayLikeLength, RuntimeError> {
    let receiver = array_like_receiver(value, env);
    let length = match receiver.clone() {
        Value::Array(array) => array.len(),
        Value::String(value) => value.chars().count(),
        Value::Object(_) | Value::Proxy(_) => {
            to_length_with_env(property_value(receiver.clone(), "length", env)?, env)?
        }
        Value::Function(function) => function.params.length(),
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                thrown: None,
                message: format!("{context} called on null or undefined"),
            });
        }
        _ => 0,
    };
    Ok(ArrayLikeLength { receiver, length })
}

pub(crate) fn array_like_values(value: Value, context: &str) -> Result<Vec<Value>, RuntimeError> {
    let mut env = crate::CallEnv::detached();
    array_like_values_with_env(value, context, &mut env)
}

pub(crate) fn array_like_values_with_env(
    value: Value,
    context: &str,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    match value {
        Value::Array(array) => Ok(array.to_vec()),
        Value::String(value) => Ok(value
            .chars()
            .map(|character| Value::String(character.to_string()))
            .collect()),
        Value::Object(_) | Value::Proxy(_) => {
            let receiver = value;
            let length = to_length_with_env(property_value(receiver.clone(), "length", env)?, env)?;
            array_like_values_from_receiver(receiver, length, env)
        }
        Value::Function(function) => {
            let length = function.params.length();
            Ok((0..length)
                .map(|index| {
                    function
                        .properties
                        .borrow()
                        .get(&index.to_string())
                        .map(|property| property.value.clone())
                        .unwrap_or(Value::Undefined)
                })
                .collect())
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: format!("{context} called on null or undefined"),
        }),
        _ => Ok(Vec::new()),
    }
}

pub(crate) fn iterable_values_with_env(
    value: Value,
    context: &str,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: format!("{context} iterator symbol is unavailable"),
        });
    };
    let iterator_method =
        property_value_key(value.clone(), &PropertyKey::Symbol(iterator_symbol), env)?;
    if !matches!(iterator_method, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("{context} argument is not iterable"),
        });
    }
    let iterator = call_function(iterator_method, value, Vec::new(), env, false)?;
    let next = property_value(iterator.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("{context} iterator next method is not callable"),
        });
    }

    let mut values = Vec::new();
    loop {
        let step = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
        if !matches!(
            step,
            Value::Object(_)
                | Value::Array(_)
                | Value::Function(_)
                | Value::Map(_)
                | Value::Set(_)
                | Value::Proxy(_)
        ) {
            return Err(RuntimeError {
                thrown: None,
                message: format!("{context} iterator result is not an object"),
            });
        }
        if is_truthy(&property_value(step.clone(), "done", env)?) {
            break;
        }
        values.push(property_value(step, "value", env)?);
    }
    Ok(values)
}

pub(crate) fn array_like_values_from_receiver(
    receiver: Value,
    length: usize,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    match receiver {
        Value::Object(_) | Value::Proxy(_) => (0..length)
            .map(|index| property_value(receiver.clone(), &index.to_string(), env))
            .collect(),
        Value::Array(array) => Ok(array.to_vec()),
        Value::String(value) => Ok(value
            .chars()
            .map(|character| Value::String(character.to_string()))
            .collect()),
        Value::Function(function) => Ok((0..length)
            .map(|index| {
                function
                    .properties
                    .borrow()
                    .get(&index.to_string())
                    .map(|property| property.value.clone())
                    .unwrap_or(Value::Undefined)
            })
            .collect()),
        _ => Ok(Vec::new()),
    }
}

pub(super) fn array_like_receiver(value: Value, env: &CallEnv) -> Value {
    match value {
        Value::Boolean(_) | Value::Number(_) => {
            object::boxed_primitive(value.clone(), env).unwrap_or(value)
        }
        Value::String(value) => {
            let mut properties = HashMap::new();
            properties.insert(
                "length".to_owned(),
                Value::Number(value.chars().count() as f64),
            );
            for (index, character) in value.chars().enumerate() {
                properties.insert(index.to_string(), Value::String(character.to_string()));
            }
            let object = ObjectRef::with_prototype(properties, string_prototype(env));
            Value::Object(object)
        }
        _ => value,
    }
}
