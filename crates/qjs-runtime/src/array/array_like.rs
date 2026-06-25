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
        Value::BigInt(_) | Value::Object(_) | Value::Proxy(_) => {
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

pub(crate) fn array_like_values_with_env(
    value: Value,
    context: &str,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    match value {
        Value::Array(array) => Ok(array.to_vec()),
        Value::String(value) => Ok(value
            .chars()
            .map(|character| Value::String(character.to_string().into()))
            .collect()),
        Value::Boolean(_)
        | Value::BigInt(_)
        | Value::Number(_)
        | Value::Object(_)
        | Value::Proxy(_) => {
            let receiver = array_like_receiver(value, env);
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
    let mut values = Vec::new();
    for_each_iterable_value_with_env(value, context, env, |value, _| {
        values.push(value);
        Ok(())
    })?;
    Ok(values)
}

pub(crate) fn iterable_values_from_method_with_env(
    value: Value,
    iterator_method: Value,
    context: &str,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    let mut values = Vec::new();
    for_each_iterable_value_from_method_with_env(
        value,
        iterator_method,
        context,
        env,
        |value, _| {
            values.push(value);
            Ok(())
        },
    )?;
    Ok(values)
}

pub(crate) fn for_each_iterable_value_with_env<F>(
    value: Value,
    context: &str,
    env: &mut CallEnv,
    visit: F,
) -> Result<(), RuntimeError>
where
    F: FnMut(Value, &mut CallEnv) -> Result<(), RuntimeError>,
{
    let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: format!("{context} iterator symbol is unavailable"),
        });
    };
    let iterator_method =
        property_value_key(value.clone(), &PropertyKey::Symbol(iterator_symbol), env)?;
    for_each_iterable_value_from_method_with_env(value, iterator_method, context, env, visit)
}

fn for_each_iterable_value_from_method_with_env<F>(
    value: Value,
    iterator_method: Value,
    context: &str,
    env: &mut CallEnv,
    mut visit: F,
) -> Result<(), RuntimeError>
where
    F: FnMut(Value, &mut CallEnv) -> Result<(), RuntimeError>,
{
    if !matches!(iterator_method, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("{context} argument is not iterable"),
        });
    }
    let iterator = call_function(iterator_method, value, Vec::new(), env, false)?;
    if !is_object_like(&iterator) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("{context} iterator method must return an object"),
        });
    }
    let next = property_value(iterator.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("{context} iterator next method is not callable"),
        });
    }

    loop {
        let step = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
        if !is_object_like(&step) {
            return Err(RuntimeError {
                thrown: None,
                message: format!("{context} iterator result is not an object"),
            });
        }
        if is_truthy(&property_value(step.clone(), "done", env)?) {
            break;
        }
        let value = match property_value(step, "value", env) {
            Ok(value) => value,
            Err(error) => return Err(iterator_close_on_throw(&iterator, error, env)),
        };
        if let Err(error) = visit(value, env) {
            return Err(iterator_close_on_throw(&iterator, error, env));
        }
    }
    Ok(())
}

pub(crate) fn array_like_values_from_receiver(
    receiver: Value,
    length: usize,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    match receiver {
        Value::Object(object) => object_array_like_values(object, length, env),
        Value::Proxy(_) => (0..length)
            .map(|index| property_value(receiver.clone(), &index.to_string(), env))
            .collect(),
        Value::Array(array) => Ok(array.to_vec()),
        Value::String(value) => Ok(value
            .chars()
            .map(|character| Value::String(character.to_string().into()))
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

fn object_array_like_values(
    object: ObjectRef,
    length: usize,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    let receiver = Value::Object(object.clone());
    let mut values = Vec::with_capacity(length);
    for index in 0..length {
        let key = index.to_string();
        if let Some(property) = object.own_property(&key)
            && !property.accessor
            && property.get.is_none()
        {
            values.push(property.value);
        } else {
            values.push(property_value(receiver.clone(), &key, env)?);
        }
    }
    Ok(values)
}

pub(super) fn array_like_receiver(value: Value, env: &CallEnv) -> Value {
    match value {
        Value::Boolean(_) | Value::BigInt(_) | Value::Number(_) => {
            object::boxed_primitive(value.clone(), env).unwrap_or(value)
        }
        Value::Object(ref object) if symbol::is_symbol_primitive(object) => {
            // A Symbol primitive (represented as `Value::Object`) is boxed to a
            // Symbol wrapper object by ToObject, like the other primitives.
            object::boxed_primitive(value.clone(), env).unwrap_or(value)
        }
        Value::String(value) => {
            let mut properties = HashMap::new();
            properties.insert(
                "length".to_owned(),
                Value::Number(value.chars().count() as f64),
            );
            for (index, character) in value.chars().enumerate() {
                properties.insert(
                    index.to_string(),
                    Value::String(character.to_string().into()),
                );
            }
            let object = ObjectRef::with_prototype(properties, string_prototype(env));
            Value::Object(object)
        }
        _ => value,
    }
}

fn is_object_like(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)
    )
}

fn iterator_close_on_throw(
    iterator: &Value,
    error: RuntimeError,
    env: &mut CallEnv,
) -> RuntimeError {
    if let Ok(return_method) = property_value(iterator.clone(), "return", env)
        && !matches!(return_method, Value::Null | Value::Undefined)
    {
        let _ = call_function(return_method, iterator.clone(), Vec::new(), env, false);
    }
    error
}
