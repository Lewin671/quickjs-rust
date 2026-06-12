use crate::{
    Property, RuntimeError, Value, array_own_property_descriptor, array_prototype, call_function,
    function_delete_own_property, function_own_property_descriptor, has_property, property_value,
};

use super::array_like::array_like_length;
use crate::CallEnv;

const MAX_SAFE_INTEGER_LENGTH: usize = 9_007_199_254_740_991;
const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;

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
                unshift_delete_property(receiver.clone(), &to)?;
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
    match receiver.clone() {
        Value::Object(object) => {
            if apply_unshift_setter(object.property(key), receiver, value.clone(), env)? {
                return Ok(());
            }
            validate_unshift_data_set(object.property(key), unshift_property_error)?;
            if object.own_property(key).is_none() && !object.is_extensible() {
                return Err(unshift_property_error());
            }
            object.set(key.to_owned(), value);
            Ok(())
        }
        Value::Array(elements) => {
            let property = array_own_property_descriptor(&elements, key)
                .or_else(|| elements.property(key))
                .or_else(|| array_prototype(env).and_then(|prototype| prototype.property(key)));
            if apply_unshift_setter(property.clone(), receiver, value.clone(), env)? {
                return Ok(());
            }
            validate_unshift_data_set(property, unshift_property_error)?;
            let index = key.parse::<usize>().ok();
            if array_own_property_descriptor(&elements, key)
                .is_some_and(|property| !property.writable)
                || index.is_some_and(|index| !elements.is_extensible() && index >= elements.len())
            {
                return Err(unshift_property_error());
            }
            match index {
                Some(index) => elements.set(index, value),
                None => elements.set_property(key.to_owned(), value),
            }
            Ok(())
        }
        Value::Function(function) => {
            if apply_unshift_setter(
                function_own_property_descriptor(&function, key),
                receiver,
                value.clone(),
                env,
            )? {
                return Ok(());
            }
            validate_unshift_data_set(
                function_own_property_descriptor(&function, key),
                unshift_property_error,
            )?;
            function.set_property(key.to_owned(), value);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn unshift_delete_property(receiver: Value, key: &str) -> Result<(), RuntimeError> {
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
        Err(unshift_delete_error())
    }
}

fn unshift_set_length(
    receiver: Value,
    length: usize,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let value = Value::Number(length as f64);
    match receiver.clone() {
        Value::Object(object) => {
            if apply_unshift_setter(object.property("length"), receiver, value.clone(), env)? {
                return Ok(());
            }
            validate_unshift_data_set(object.property("length"), unshift_length_error)?;
            if object.own_property("length").is_none() && !object.is_extensible() {
                return Err(unshift_length_error());
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
                return Err(unshift_length_error());
            }
            elements.set_len(length);
            if elements.len() == length {
                Ok(())
            } else {
                Err(unshift_length_error())
            }
        }
        Value::Function(function) => {
            if apply_unshift_setter(
                function_own_property_descriptor(&function, "length"),
                receiver,
                value.clone(),
                env,
            )? {
                return Ok(());
            }
            validate_unshift_data_set(
                function_own_property_descriptor(&function, "length"),
                unshift_length_error,
            )?;
            function.set_property("length".to_owned(), value);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn apply_unshift_setter(
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
        Err(unshift_property_error())
    } else {
        Ok(false)
    }
}

fn validate_unshift_data_set(
    property: Option<Property>,
    error: fn() -> RuntimeError,
) -> Result<(), RuntimeError> {
    if property.is_some_and(|property| !property.writable || property.is_accessor()) {
        Err(error())
    } else {
        Ok(())
    }
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
