use std::collections::HashMap;

use crate::reflect::target::ensure_reflect_object_target;
use crate::{Property, RuntimeError, Value, object, to_length};

pub(crate) fn native_reflect_apply(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(target, Value::Function(_)) {
        return Err(RuntimeError {
            message: "Reflect.apply target is not callable".to_owned(),
        });
    }

    let this_value = crate::function::function_call_this(argument_values.get(1).cloned(), env);
    let arguments = match argument_values.get(2).cloned().unwrap_or(Value::Undefined) {
        Value::Array(elements) => elements.to_vec(),
        value => {
            return Err(RuntimeError {
                message: format!("Reflect.apply argument list is not array-like: {value:?}"),
            });
        }
    };

    crate::call_function(target, this_value, arguments, env, false)
}

pub(crate) fn native_reflect_define_property(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.defineProperty")?;
    let key = crate::to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    let descriptor = object::to_property_descriptor(
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
    )?;

    Ok(Value::Boolean(object::define_property_on_value(
        target, key, descriptor,
    )?))
}

pub(crate) fn native_reflect_delete_property(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.deleteProperty")?;
    let key = crate::to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;

    let success = match target {
        Value::Object(object) => object.delete_own_property(&key),
        Value::Function(function) => crate::function_delete_own_property(&function, &key),
        Value::Array(elements) => {
            key != "length"
                && crate::array_own_property_descriptor(&elements, &key)
                    .is_none_or(|property| property.configurable)
        }
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("target was validated before property deletion"),
    };

    Ok(Value::Boolean(success))
}

pub(crate) fn native_reflect_get(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.get")?;
    let key = crate::to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;

    Ok(match target {
        Value::Object(object) => object.get(&key).unwrap_or(Value::Undefined),
        Value::Array(elements) => {
            if key == "length" {
                Value::Number(elements.len() as f64)
            } else {
                key.parse::<usize>()
                    .ok()
                    .and_then(|index| elements.get(index))
                    .or_else(|| crate::array_prototype_property(&elements, env, &key))
                    .unwrap_or(Value::Undefined)
            }
        }
        Value::Function(function) => crate::function_own_property_descriptor(&function, &key)
            .map(|property| property.value)
            .or_else(|| crate::function_prototype_property(&function, env, &key))
            .unwrap_or(Value::Undefined),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("target was validated before property get"),
    })
}

pub(crate) fn native_reflect_get_prototype_of(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    object::native_object_get_prototype_of(argument_values, env)
}

pub(crate) fn native_reflect_get_own_property_descriptor(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.getOwnPropertyDescriptor")?;
    object::native_object_get_own_property_descriptor(argument_values, env)
}

pub(crate) fn native_reflect_has(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = crate::to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    match target {
        Value::Object(_) | Value::Array(_) | Value::Function(_) => {
            Ok(Value::Boolean(crate::has_property(target, env, &key)?))
        }
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            message: "Reflect.has target must be an object".to_owned(),
        }),
    }
}

pub(crate) fn native_reflect_is_extensible(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.isExtensible")?;
    Ok(Value::Boolean(match target {
        Value::Object(object) => object.is_extensible(),
        Value::Array(elements) => elements.is_extensible(),
        Value::Function(function) => function.is_extensible(),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("target was validated before extensibility check"),
    }))
}

pub(crate) fn native_reflect_own_keys(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.ownKeys")?;
    let keys = match target {
        Value::Object(object) => object.own_property_names(),
        Value::Array(elements) => crate::array_own_property_names(&elements),
        Value::Function(function) => crate::function_own_property_names(&function),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => {
            unreachable!("target was validated before own key enumeration")
        }
    };

    Ok(Value::Array(crate::ArrayRef::new(
        keys.into_iter().map(Value::String).collect(),
    )))
}

pub(crate) fn native_reflect_prevent_extensions(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.preventExtensions")?;
    match target {
        Value::Object(object) => object.prevent_extensions(),
        Value::Array(elements) => elements.prevent_extensions(),
        Value::Function(function) => function.prevent_extensions(),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("target was validated before preventing extensions"),
    }
    Ok(Value::Boolean(true))
}

pub(crate) fn native_reflect_set(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.set")?;
    let key = crate::to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    let value = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let receiver = argument_values
        .get(3)
        .cloned()
        .unwrap_or_else(|| target.clone());

    Ok(Value::Boolean(ordinary_set(
        target, &key, value, receiver, env,
    )?))
}

pub(crate) fn native_reflect_set_prototype_of(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let prototype = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Object(prototype) => Some(prototype),
        Value::Null => None,
        _ => {
            return Err(RuntimeError {
                message: "Reflect.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
    };

    let success = match target {
        Value::Object(object) => object.set_prototype(prototype).is_ok(),
        Value::Array(elements) => elements.set_prototype(prototype).is_ok(),
        Value::Function(function) => function.set_internal_prototype(prototype).is_ok(),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => {
            return Err(RuntimeError {
                message: "Reflect.setPrototypeOf target must be an object".to_owned(),
            });
        }
    };

    Ok(Value::Boolean(success))
}

fn ordinary_set(
    target: Value,
    key: &str,
    value: Value,
    receiver: Value,
    env: &HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    if let Some(property) = own_property_descriptor(&target, key) {
        return set_with_data_descriptor(property, key, value, receiver);
    }

    if let Some(prototype) = crate::value_prototype(target, env) {
        return ordinary_set(Value::Object(prototype), key, value, receiver, env);
    }

    set_with_data_descriptor(Property::enumerable(Value::Undefined), key, value, receiver)
}

fn own_property_descriptor(target: &Value, key: &str) -> Option<Property> {
    match target {
        Value::Object(object) => object.own_property(key),
        Value::Array(elements) => crate::array_own_property_descriptor(elements, key),
        Value::Function(function) => crate::function_own_property_descriptor(function, key),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => None,
    }
}

fn set_with_data_descriptor(
    property: Property,
    key: &str,
    value: Value,
    receiver: Value,
) -> Result<bool, RuntimeError> {
    if !property.writable {
        return Ok(false);
    }
    set_receiver_data_property(receiver, key, value)
}

fn set_receiver_data_property(
    receiver: Value,
    key: &str,
    value: Value,
) -> Result<bool, RuntimeError> {
    match receiver {
        Value::Object(object) => {
            let descriptor = match object.own_property(key) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !object.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            object.define_property(key.to_owned(), descriptor);
            Ok(true)
        }
        Value::Array(elements) => {
            if key == "length" {
                if !crate::array_own_property_descriptor(&elements, key)
                    .is_some_and(|property| property.writable)
                {
                    return Ok(false);
                }
                let length = to_length(value)?;
                if length > elements.len() && !elements.is_extensible() {
                    return Ok(false);
                }
                elements.set_len(length);
                Ok(true)
            } else {
                let Some(index) = key.parse::<usize>().ok() else {
                    return Ok(false);
                };
                if index >= elements.len() && !elements.is_extensible() {
                    return Ok(false);
                }
                if elements.is_frozen() {
                    return Ok(false);
                }
                elements.set(index, value);
                Ok(true)
            }
        }
        Value::Function(function) => {
            let descriptor = match crate::function_own_property_descriptor(&function, key) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !function.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            function
                .properties
                .borrow_mut()
                .insert(key.to_owned(), descriptor);
            Ok(true)
        }
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Ok(false),
    }
}
