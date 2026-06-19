use crate::{RuntimeError, Value, call_function, object, property_value, to_js_string_with_env};

use super::array_like::{array_like_length, array_like_receiver};
use crate::CallEnv;

pub(crate) fn native_array_prototype_join(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let array_like = array_like_length(this_value, "Array.prototype.join", env)?;
    let separator = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => ",".to_owned(),
        value => to_js_string_with_env(value, env)?,
    };
    Ok(Value::String(
        array_join_array_like(array_like, &separator, env)?.into(),
    ))
}

pub(crate) fn native_array_prototype_to_string(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let receiver = match this_value {
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                thrown: None,
                message: "Array.prototype.toString called on null or undefined".to_owned(),
            });
        }
        value => array_like_receiver(value, env),
    };
    let join = property_value(receiver.clone(), "join", env)?;
    if matches!(join, Value::Function(_)) {
        return call_function(join, receiver, Vec::new(), env, false);
    }
    object::native_object_prototype_to_string(receiver, env)
}

pub(crate) fn native_array_prototype_to_locale_string(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let array_like = array_like_length(this_value, "Array.prototype.toLocaleString", env)?;
    let mut parts = Vec::with_capacity(array_like.length);
    for index in 0..array_like.length {
        let element = property_value(array_like.receiver.clone(), &index.to_string(), env)?;
        let part = match element {
            Value::Null | Value::Undefined => String::new(),
            value => {
                let method = property_value(value.clone(), "toLocaleString", env)?;
                let localized = call_function(method, value, Vec::new(), env, false)?;
                to_js_string_with_env(localized, env)?
            }
        };
        parts.push(part);
    }
    Ok(Value::String(parts.join(",").into()))
}

pub(crate) fn array_join(
    value: Value,
    separator: &str,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
    let array_like = array_like_length(value, "Array.prototype.join", env)?;
    array_join_array_like(array_like, separator, env)
}

fn array_join_array_like(
    array_like: super::array_like::ArrayLikeLength,
    separator: &str,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
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
