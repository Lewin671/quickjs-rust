use crate::{RuntimeError, Value, has_property, property_value};

use super::{
    array_like::array_like_length,
    mutation::{delete_array_like_property_with_error, set_array_like_property_with_error},
};
use crate::CallEnv;

const MAX_SAFE_INTEGER_LENGTH: usize = 9_007_199_254_740_991;

pub(crate) fn native_array_prototype_unshift(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::String(_)) {
        return Err(unshift_length_error());
    }

    let source = array_like_length(this_value, "Array.prototype.unshift", env)?;
    let receiver = source.receiver;
    let length = source.length;
    let argument_count = argument_values.len();
    let new_length = length
        .checked_add(argument_count)
        .filter(|length| *length <= MAX_SAFE_INTEGER_LENGTH)
        .ok_or_else(unshift_length_error)?;

    if argument_count > 0 {
        for index in (0..length).rev() {
            let from = index.to_string();
            let to = (index + argument_count).to_string();
            if has_property(receiver.clone(), env, &from)? {
                let value = property_value(receiver.clone(), &from, env)?;
                unshift_set_property(receiver.clone(), &to, value, env)?;
            } else {
                unshift_delete_property(receiver.clone(), &to, env)?;
            }
        }

        for (index, value) in argument_values.iter().cloned().enumerate() {
            unshift_set_property(receiver.clone(), &index.to_string(), value, env)?;
        }
    }

    unshift_set_length(receiver, new_length, env)?;
    Ok(Value::Number(new_length as f64))
}

fn unshift_set_property(
    receiver: Value,
    key: &str,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    set_array_like_property_with_error(receiver, key.to_owned(), value, env, unshift_property_error)
}

fn unshift_delete_property(
    receiver: Value,
    key: &str,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    delete_array_like_property_with_error(receiver, key, env, unshift_delete_error)
}

fn unshift_set_length(
    receiver: Value,
    length: usize,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    set_array_like_property_with_error(
        receiver,
        "length".to_owned(),
        Value::Number(length as f64),
        env,
        unshift_length_error,
    )
}

fn unshift_property_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.unshift cannot set property".to_owned(),
    }
}

fn unshift_delete_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.unshift cannot delete property".to_owned(),
    }
}

fn unshift_length_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.unshift cannot set length".to_owned(),
    }
}
