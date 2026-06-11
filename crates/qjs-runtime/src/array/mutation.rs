use std::collections::HashMap;

use crate::{
    Property, RuntimeError, Value, array_own_property_descriptor, array_prototype, call_function,
    function_delete_own_property, function_own_property_descriptor, has_property, property_value,
    to_length,
};

use super::{
    array_like::array_like_length,
    indexing::{array_slice_end, array_slice_start},
};
use crate::CallEnv;

const MAX_SAFE_INTEGER_LENGTH: usize = 9_007_199_254_740_991;
const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;

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
    match receiver.clone() {
        Value::Object(object) => {
            validate_copy_within_set(object.property(&key), receiver, value.clone(), env)?;
            object.set(key, value);
            Ok(())
        }
        Value::Array(elements) if key == "length" => {
            if array_own_property_descriptor(&elements, &key)
                .is_some_and(|property| !property.writable)
            {
                return Err(copy_within_set_error());
            }
            elements.set_len(to_length(value)?);
            Ok(())
        }
        Value::Array(elements) => {
            validate_copy_within_set(
                array_own_property_descriptor(&elements, &key).or_else(|| elements.property(&key)),
                receiver,
                value.clone(),
                env,
            )?;
            match key.parse::<usize>() {
                Ok(index) => elements.set(index, value),
                Err(_) => elements.set_property(key, value),
            }
            Ok(())
        }
        Value::Function(function) => {
            validate_copy_within_set(
                function_own_property_descriptor(&function, &key),
                receiver,
                value.clone(),
                env,
            )?;
            function.set_property(key, value);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_copy_within_set(
    property: Option<Property>,
    receiver: Value,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let Some(property) = property else {
        return Ok(());
    };
    if let Some(setter) = property.set {
        call_function(setter, receiver, vec![value], env, false)?;
        return Ok(());
    }
    if property.is_accessor() || !property.writable {
        return Err(copy_within_set_error());
    }
    Ok(())
}

pub(super) fn delete_array_like_property(
    receiver: Value,
    key: &str,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    match receiver {
        Value::Object(object) if !object.delete_own_property(key) => {
            return Err(copy_within_delete_error());
        }
        Value::Object(_) => {}
        Value::Proxy(proxy)
            if !crate::proxy::proxy_delete_property(
                proxy.clone(),
                &crate::PropertyKey::String(key.to_owned()),
                env,
            )? =>
        {
            return Err(copy_within_delete_error());
        }
        Value::Proxy(_) => {}
        Value::Array(elements) => match key.parse::<usize>() {
            Ok(index) => {
                if !elements.delete_index(index) {
                    return Err(copy_within_delete_error());
                }
            }
            Err(_) => {
                if !elements.delete_property(key) {
                    return Err(copy_within_delete_error());
                }
            }
        },
        Value::Function(function) if !function_delete_own_property(&function, key) => {
            return Err(copy_within_delete_error());
        }
        Value::Function(_) => {}
        _ => {}
    }
    Ok(())
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
    let key = index.to_string();
    match receiver.clone() {
        Value::Object(object) => {
            if apply_push_setter(object.property(&key), receiver, value.clone(), env)? {
                return Ok(());
            }
            validate_push_data_set(object.property(&key))?;
            if object.own_property(&key).is_none() && !object.is_extensible() {
                return Err(push_property_error());
            }
            object.set(key, value);
            Ok(())
        }
        Value::Array(elements) => {
            let property = array_own_property_descriptor(&elements, &key)
                .or_else(|| elements.property(&key))
                .or_else(|| array_prototype(env).and_then(|prototype| prototype.property(&key)));
            if apply_push_setter(property.clone(), receiver, value.clone(), env)? {
                return Ok(());
            }
            validate_push_data_set(property)?;
            if array_own_property_descriptor(&elements, &key)
                .is_some_and(|property| !property.writable)
                || !elements.is_extensible() && index >= elements.len()
            {
                return Err(push_property_error());
            }
            elements.set(index, value);
            Ok(())
        }
        Value::Function(function) => {
            if apply_push_setter(
                function_own_property_descriptor(&function, &key),
                receiver,
                value.clone(),
                env,
            )? {
                return Ok(());
            }
            validate_push_data_set(function_own_property_descriptor(&function, &key))?;
            function.set_property(key, value);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn push_set_length(receiver: Value, length: usize, env: &mut CallEnv) -> Result<(), RuntimeError> {
    let value = Value::Number(length as f64);
    match receiver.clone() {
        Value::Object(object) => {
            if apply_push_setter(object.property("length"), receiver, value.clone(), env)? {
                return Ok(());
            }
            validate_push_data_set(object.property("length"))?;
            if object.own_property("length").is_none() && !object.is_extensible() {
                return Err(push_length_error());
            }
            object.set("length".to_owned(), value);
            Ok(())
        }
        Value::Array(elements) => {
            if length > MAX_ARRAY_LENGTH {
                return Err(RuntimeError {
                    thrown: None,
                    message: "RangeError: invalid array length".to_owned(),
                });
            }
            if array_own_property_descriptor(&elements, "length")
                .is_some_and(|property| !property.writable)
            {
                return Err(push_length_error());
            }
            elements.set_len(length);
            if elements.len() == length {
                Ok(())
            } else {
                Err(push_length_error())
            }
        }
        Value::Function(function) => {
            if apply_push_setter(
                function_own_property_descriptor(&function, "length"),
                receiver,
                value.clone(),
                env,
            )? {
                return Ok(());
            }
            validate_push_data_set(function_own_property_descriptor(&function, "length"))?;
            function.set_property("length".to_owned(), value);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn apply_push_setter(
    property: Option<Property>,
    receiver: Value,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let Some(property) = property else {
        return Ok(false);
    };
    if let Some(setter) = property.set {
        call_function(setter, receiver, vec![value], env, false)?;
        return Ok(true);
    }
    if property.is_accessor() {
        Err(push_length_error())
    } else {
        Ok(false)
    }
}

fn validate_push_data_set(property: Option<Property>) -> Result<(), RuntimeError> {
    if property.is_some_and(|property| !property.writable || property.is_accessor()) {
        Err(push_property_error())
    } else {
        Ok(())
    }
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
    pop_delete_property(receiver.clone(), &key)?;
    pop_set_length(receiver, new_length, env)?;
    Ok(element)
}

fn pop_delete_property(receiver: Value, key: &str) -> Result<(), RuntimeError> {
    let deleted = match receiver {
        Value::Object(object) => object.delete_own_property(key),
        Value::Array(elements) => {
            if array_own_property_descriptor(&elements, key)
                .is_some_and(|property| !property.configurable)
            {
                false
            } else {
                match key.parse::<usize>() {
                    Ok(index) => elements.delete_index(index),
                    Err(_) => elements.delete_property(key),
                }
            }
        }
        Value::Function(function) => function_delete_own_property(&function, key),
        _ => true,
    };
    if deleted {
        Ok(())
    } else {
        Err(pop_delete_error())
    }
}

fn pop_set_length(receiver: Value, length: usize, env: &mut CallEnv) -> Result<(), RuntimeError> {
    let value = Value::Number(length as f64);
    match receiver.clone() {
        Value::Object(object) => {
            if apply_pop_setter(object.property("length"), receiver, value.clone(), env)? {
                return Ok(());
            }
            validate_pop_data_set(object.property("length"))?;
            if object.own_property("length").is_none() && !object.is_extensible() {
                return Err(pop_length_error());
            }
            object.set("length".to_owned(), value);
            Ok(())
        }
        Value::Array(elements) => {
            if array_own_property_descriptor(&elements, "length")
                .is_some_and(|property| !property.writable)
            {
                return Err(pop_length_error());
            }
            elements.set_len(length);
            if elements.len() == length {
                Ok(())
            } else {
                Err(pop_length_error())
            }
        }
        Value::Function(function) => {
            if apply_pop_setter(
                function_own_property_descriptor(&function, "length"),
                receiver,
                value.clone(),
                env,
            )? {
                return Ok(());
            }
            validate_pop_data_set(function_own_property_descriptor(&function, "length"))?;
            function.set_property("length".to_owned(), value);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn apply_pop_setter(
    property: Option<Property>,
    receiver: Value,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let Some(property) = property else {
        return Ok(false);
    };
    if let Some(setter) = property.set {
        call_function(setter, receiver, vec![value], env, false)?;
        return Ok(true);
    }
    if property.is_accessor() {
        Err(pop_length_error())
    } else {
        Ok(false)
    }
}

fn validate_pop_data_set(property: Option<Property>) -> Result<(), RuntimeError> {
    if property.is_some_and(|property| !property.writable || property.is_accessor()) {
        Err(pop_length_error())
    } else {
        Ok(())
    }
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
