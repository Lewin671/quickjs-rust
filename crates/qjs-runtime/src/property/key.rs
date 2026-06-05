use std::collections::HashMap;

use crate::{RuntimeError, Value, number::number_to_js_string, to_js_string_with_env};

pub(crate) fn to_property_key(value: Value) -> Result<String, RuntimeError> {
    let mut env = HashMap::new();
    to_property_key_with_env(value, &mut env)
}

pub(crate) fn to_property_key_with_env(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    match value {
        Value::String(value) => Ok(value),
        Value::Number(number) => Ok(number_to_js_string(number)),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Function(_) | Value::Array(_) | Value::Object(_) => {
            to_js_string_with_env(value, env)
        }
    }
}
