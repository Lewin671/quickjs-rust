use crate::{PropertyKey, RuntimeError, Value, has_property, property_value};

use super::{
    array_like::array_like_length,
    indexing::{array_slice_end, array_slice_start},
};
use crate::CallEnv;

const MAX_SAFE_INTEGER_LENGTH: usize = 9_007_199_254_740_991;

pub(crate) fn native_array_prototype_fill(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.fill", env)?;
    let receiver = source.receiver;
    let length = source.length;
    let start = array_slice_start(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    for index in start..end {
        set_array_like_property(receiver.clone(), index.to_string(), value.clone(), env)?;
    }
    Ok(receiver)
}

pub(crate) fn native_array_prototype_copy_within(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.copyWithin", env)?;
    let receiver = source.receiver;
    let length = source.length;
    let target = array_slice_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let start = array_slice_start(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let count = (end.saturating_sub(start)).min(length.saturating_sub(target));
    if count == 0 {
        return Ok(receiver);
    }

    let backwards = start < target && target < start + count;
    for offset in 0..count {
        let index = if backwards {
            count - 1 - offset
        } else {
            offset
        };
        let source_key = (start + index).to_string();
        let target_key = (target + index).to_string();
        if has_property(receiver.clone(), env, &source_key)? {
            let value = property_value(receiver.clone(), &source_key, env)?;
            set_array_like_property(receiver.clone(), target_key, value, env)?;
        } else {
            delete_array_like_property(receiver.clone(), &target_key, env)?;
        }
    }
    Ok(receiver)
}

pub(super) fn set_array_like_property(
    receiver: Value,
    key: String,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    set_array_like_property_with_error(receiver, key, value, env, copy_within_set_error)
}

pub(super) fn delete_array_like_property(
    receiver: Value,
    key: &str,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    delete_array_like_property_with_error(receiver, key, env, copy_within_delete_error)
}

pub(super) fn set_array_like_property_with_error(
    receiver: Value,
    key: String,
    value: Value,
    env: &mut CallEnv,
    error: fn() -> RuntimeError,
) -> Result<(), RuntimeError> {
    if crate::bytecode::set_object_property(receiver, key, value, env)? {
        Ok(())
    } else {
        Err(error())
    }
}

pub(super) fn delete_array_like_property_with_error(
    receiver: Value,
    key: &str,
    env: &mut CallEnv,
    error: fn() -> RuntimeError,
) -> Result<(), RuntimeError> {
    if crate::bytecode::delete_object_property(receiver, &PropertyKey::String(key.to_owned()), env)?
    {
        Ok(())
    } else {
        Err(error())
    }
}

fn copy_within_set_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.copyWithin cannot set target property".to_owned(),
    }
}

fn copy_within_delete_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.copyWithin cannot delete target property".to_owned(),
    }
}

pub(crate) fn native_array_prototype_push(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::String(_)) {
        return Err(push_length_error());
    }

    let source = array_like_length(this_value, "Array.prototype.push", env)?;
    let receiver = source.receiver;
    let length = source.length;
    let new_length = length
        .checked_add(argument_values.len())
        .filter(|length| *length <= MAX_SAFE_INTEGER_LENGTH)
        .ok_or_else(push_length_error)?;
    for (offset, value) in argument_values.iter().cloned().enumerate() {
        push_set_property(receiver.clone(), length + offset, value, env)?;
    }
    push_set_length(receiver, new_length, env)?;
    Ok(Value::Number(new_length as f64))
}

fn push_set_property(
    receiver: Value,
    index: usize,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    set_array_like_property_with_error(receiver, index.to_string(), value, env, push_property_error)
}

fn push_set_length(receiver: Value, length: usize, env: &mut CallEnv) -> Result<(), RuntimeError> {
    set_array_like_property_with_error(
        receiver,
        "length".to_owned(),
        Value::Number(length as f64),
        env,
        push_length_error,
    )
}

fn push_property_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.push cannot set property".to_owned(),
    }
}

fn push_length_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.push cannot set length".to_owned(),
    }
}

pub(crate) fn native_array_prototype_pop(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::String(_)) {
        return Err(pop_length_error());
    }

    let source = array_like_length(this_value, "Array.prototype.pop", env)?;
    let receiver = source.receiver;
    let length = source.length;
    if length == 0 {
        if let Value::Array(elements) = receiver.clone() {
            let _ = elements.pop();
        }
        pop_set_length(receiver, 0, env)?;
        return Ok(Value::Undefined);
    }

    let new_length = length - 1;
    let key = new_length.to_string();
    let element = property_value(receiver.clone(), &key, env)?;
    pop_delete_property(receiver.clone(), &key, env)?;
    pop_set_length(receiver, new_length, env)?;
    Ok(element)
}

fn pop_delete_property(receiver: Value, key: &str, env: &mut CallEnv) -> Result<(), RuntimeError> {
    delete_array_like_property_with_error(receiver, key, env, pop_delete_error)
}

fn pop_set_length(receiver: Value, length: usize, env: &mut CallEnv) -> Result<(), RuntimeError> {
    set_array_like_property_with_error(
        receiver,
        "length".to_owned(),
        Value::Number(length as f64),
        env,
        pop_length_error,
    )
}

fn pop_delete_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.pop cannot delete property".to_owned(),
    }
}

fn pop_length_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.pop cannot set length".to_owned(),
    }
}

pub(crate) fn native_array_prototype_reverse(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.reverse", env)?;
    let receiver = source.receiver;
    let length = source.length;
    if length < 2 {
        return Ok(receiver);
    }

    for lower in 0..(length / 2) {
        let upper = length - lower - 1;
        let lower_key = lower.to_string();
        let upper_key = upper.to_string();
        let lower_exists = has_property(receiver.clone(), env, &lower_key)?;
        let lower_value = if lower_exists {
            Some(property_value(receiver.clone(), &lower_key, env)?)
        } else {
            None
        };
        let upper_exists = has_property(receiver.clone(), env, &upper_key)?;
        let upper_value = if upper_exists {
            Some(property_value(receiver.clone(), &upper_key, env)?)
        } else {
            None
        };

        match (lower_value, upper_value) {
            (Some(lower_value), Some(upper_value)) => {
                set_array_like_property(receiver.clone(), lower_key, upper_value, env)?;
                set_array_like_property(receiver.clone(), upper_key, lower_value, env)?;
            }
            (Some(lower_value), None) => {
                delete_array_like_property(receiver.clone(), &lower_key, env)?;
                set_array_like_property(receiver.clone(), upper_key, lower_value, env)?;
            }
            (None, Some(upper_value)) => {
                set_array_like_property(receiver.clone(), lower_key, upper_value, env)?;
                delete_array_like_property(receiver.clone(), &upper_key, env)?;
            }
            (None, None) => {}
        }
    }
    Ok(receiver)
}
