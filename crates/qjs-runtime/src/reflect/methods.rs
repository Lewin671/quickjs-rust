use std::collections::HashMap;

use crate::reflect::target::ensure_reflect_object_target;
use crate::{RuntimeError, Value, object};

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
