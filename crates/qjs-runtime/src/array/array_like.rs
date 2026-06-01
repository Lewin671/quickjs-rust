use crate::{RuntimeError, Value, to_length};

pub(crate) struct ArrayLike {
    pub(crate) receiver: Value,
    pub(crate) values: Vec<Value>,
}

pub(crate) fn array_like(value: Value, context: &str) -> Result<ArrayLike, RuntimeError> {
    let values = array_like_values(value.clone(), context)?;
    Ok(ArrayLike {
        receiver: value,
        values,
    })
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
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: format!("{context} called on null or undefined"),
        }),
        _ => Ok(Vec::new()),
    }
}
