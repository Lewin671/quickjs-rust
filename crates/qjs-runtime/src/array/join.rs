use std::collections::HashMap;

use crate::{RuntimeError, Value, call_function, object, property_value, to_js_string_with_env};

use super::array_like::{array_like_length, array_like_receiver};

pub(crate) fn native_array_prototype_join(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let separator = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => ",".to_owned(),
        value => to_js_string_with_env(value, env)?,
    };
    Ok(Value::String(array_join(this_value, &separator, env)?))
}

pub(crate) fn native_array_prototype_to_string(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let receiver = array_like_receiver(this_value, env);
    let join = property_value(receiver.clone(), "join", env)?;
    if matches!(join, Value::Function(_)) {
        return call_function(join, receiver, Vec::new(), env, false);
    }
    object::native_object_prototype_to_string(receiver)
}

pub(crate) fn array_join(
    value: Value,
    separator: &str,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    let array_like = array_like_length(value, "Array.prototype.join", env)?;
    let mut parts = Vec::with_capacity(array_like.length);
    for index in 0..array_like.length {
        let element = property_value(array_like.receiver.clone(), &index.to_string(), env)?;
        let part = match element {
            Value::Null | Value::Undefined => String::new(),
            Value::Array(_) => array_join(element, ",", env)?,
            value => to_js_string_with_env(value, env)?,
        };
        parts.push(part);
    }
    Ok(parts.join(separator))
}
