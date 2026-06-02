use std::collections::HashMap;

use crate::{
    Property, RuntimeError, Value, array_own_property_descriptor, call_function,
    function_delete_own_property, function_own_property_descriptor, has_property, property_value,
    to_length,
};

use super::{
    array_like::array_like_length,
    indexing::{array_slice_end, array_slice_start},
};

pub(crate) fn native_array_prototype_fill(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.fill called on non-array".to_owned(),
        });
    };

    let length = elements.len();
    let start = array_slice_start(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
    )?;
    if start < end {
        elements.fill(
            start,
            end,
            argument_values.first().cloned().unwrap_or(Value::Undefined),
        );
    }
    Ok(this_value)
}

pub(crate) fn native_array_prototype_copy_within(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let source = array_like_length(this_value, "Array.prototype.copyWithin", env)?;
    let receiver = source.receiver;
    let length = source.length;
    let target = array_slice_start(
        length,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
    )?;
    let start = array_slice_start(
        length,
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
    )?;
    let end = array_slice_end(
        length,
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
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
            delete_array_like_property(receiver.clone(), &target_key)?;
        }
    }
    Ok(receiver)
}

fn set_array_like_property(
    receiver: Value,
    key: String,
    value: Value,
    env: &mut HashMap<String, Value>,
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
    env: &mut HashMap<String, Value>,
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

fn delete_array_like_property(receiver: Value, key: &str) -> Result<(), RuntimeError> {
    match receiver {
        Value::Object(object) if !object.delete_own_property(key) => {
            return Err(copy_within_delete_error());
        }
        Value::Object(_) => {}
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
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.push called on non-array".to_owned(),
        });
    };
    for value in argument_values.iter().cloned() {
        elements.push(value);
    }
    Ok(Value::Number(elements.len() as f64))
}

pub(crate) fn native_array_prototype_pop(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.pop called on non-array".to_owned(),
        });
    };
    Ok(elements.pop().unwrap_or(Value::Undefined))
}

pub(crate) fn native_array_prototype_shift(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.shift called on non-array".to_owned(),
        });
    };
    Ok(elements.shift().unwrap_or(Value::Undefined))
}

pub(crate) fn native_array_prototype_unshift(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.unshift called on non-array".to_owned(),
        });
    };
    Ok(Value::Number(elements.unshift(argument_values) as f64))
}

pub(crate) fn native_array_prototype_reverse(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.reverse called on non-array".to_owned(),
        });
    };
    elements.reverse();
    Ok(this_value)
}
