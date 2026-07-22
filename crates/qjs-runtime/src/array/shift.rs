use crate::{RuntimeError, Value, has_property, property_value};

use super::{
    array_like::array_like_length,
    mutation::{delete_array_like_property_with_error, set_array_like_property_with_error},
};
use crate::CallEnv;

pub(crate) fn native_array_prototype_shift(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::String(_)) {
        return Err(shift_length_error());
    }

    let source = array_like_length(this_value, "Array.prototype.shift", env)?;
    let receiver = source.receiver;
    let length = source.length;
    if length == 0 {
        shift_set_length(receiver, 0, env)?;
        return Ok(Value::Undefined);
    }

    let first = property_value(receiver.clone(), "0", env)?;
    for index in 1..length {
        let from = index.to_string();
        let to = (index - 1).to_string();
        if has_property(receiver.clone(), env, &from)? {
            let value = property_value(receiver.clone(), &from, env)?;
            shift_set_property(receiver.clone(), &to, value, env)?;
        } else {
            shift_delete_property(receiver.clone(), &to, env)?;
        }
    }
    shift_delete_property(receiver.clone(), &(length - 1).to_string(), env)?;
    shift_set_length(receiver, length - 1, env)?;
    Ok(first)
}

fn shift_set_property(
    receiver: Value,
    key: &str,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    set_array_like_property_with_error(receiver, key.to_owned(), value, env, shift_property_error)
}

fn shift_delete_property(
    receiver: Value,
    key: &str,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    delete_array_like_property_with_error(receiver, key, env, shift_delete_error)
}

fn shift_set_length(receiver: Value, length: usize, env: &mut CallEnv) -> Result<(), RuntimeError> {
    set_array_like_property_with_error(
        receiver,
        "length".to_owned(),
        Value::Number(length as f64),
        env,
        shift_length_error,
    )
}

fn shift_property_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.shift cannot set property".to_owned(),
    }
}

fn shift_delete_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.shift cannot delete property".to_owned(),
    }
}

fn shift_length_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.shift cannot set length".to_owned(),
    }
}
