use std::collections::HashMap;

use crate::{ObjectRef, RuntimeError, Value, function_prototype, string_prototype, to_length};

pub(crate) struct ArrayLike {
    pub(crate) receiver: Value,
    pub(crate) values: Vec<Value>,
}

pub(crate) fn array_like(
    value: Value,
    context: &str,
    env: &HashMap<String, Value>,
) -> Result<ArrayLike, RuntimeError> {
    let receiver = array_like_receiver(value, env);
    let values = array_like_values(receiver.clone(), context)?;
    Ok(ArrayLike { receiver, values })
}

pub(crate) fn array_like_values(value: Value, context: &str) -> Result<Vec<Value>, RuntimeError> {
    match value {
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

fn array_like_receiver(value: Value, env: &HashMap<String, Value>) -> Value {
    match value {
        Value::Boolean(_) => boxed_primitive("Boolean", env).unwrap_or(value),
        Value::Number(_) => boxed_primitive("Number", env).unwrap_or(value),
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

fn boxed_primitive(constructor_name: &str, env: &HashMap<String, Value>) -> Option<Value> {
    let Value::Function(function) = env.get(constructor_name)? else {
        return None;
    };
    Some(Value::Object(ObjectRef::with_prototype(
        HashMap::new(),
        function_prototype(function),
    )))
}
