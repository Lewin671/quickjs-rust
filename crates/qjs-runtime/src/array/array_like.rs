use std::collections::HashMap;

use crate::{
    ObjectRef, RuntimeError, Value, object, property_value, string_prototype, to_length_with_env,
};

pub(crate) struct ArrayLikeLength {
    pub(crate) receiver: Value,
    pub(crate) length: usize,
}

pub(crate) fn array_like_length(
    value: Value,
    context: &str,
    env: &mut HashMap<String, Value>,
) -> Result<ArrayLikeLength, RuntimeError> {
    let receiver = array_like_receiver(value, env);
    let length = match receiver.clone() {
        Value::Array(array) => array.len(),
        Value::String(value) => value.chars().count(),
        Value::Object(_) => {
            to_length_with_env(property_value(receiver.clone(), "length", env)?, env)?
        }
        Value::Function(function) => function.params.len(),
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
    let mut env = HashMap::new();
    array_like_values_with_env(value, context, &mut env)
}

pub(crate) fn array_like_values_with_env(
    value: Value,
    context: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    match value {
        Value::Array(array) => Ok(array.to_vec()),
        Value::String(value) => Ok(value
            .chars()
            .map(|character| Value::String(character.to_string()))
            .collect()),
        Value::Object(object) => {
            let receiver = Value::Object(object);
            let length = to_length_with_env(property_value(receiver.clone(), "length", env)?, env)?;
            array_like_values_from_receiver(receiver, length, env)
        }
        Value::Function(function) => {
            let length = function.params.len();
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

pub(crate) fn array_like_values_from_receiver(
    receiver: Value,
    length: usize,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    match receiver {
        Value::Object(_) => (0..length)
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

pub(super) fn array_like_receiver(value: Value, env: &HashMap<String, Value>) -> Value {
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
