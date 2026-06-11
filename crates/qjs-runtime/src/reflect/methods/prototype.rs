use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    RuntimeError, Value, array_as_object_prototype, array_prototype, error,
    function_intrinsic_prototype, symbol,
};

pub(crate) fn native_reflect_get_prototype_of(
    argument_values: &[Value],
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::Object(object)) if symbol::is_symbol_primitive(object) => Err(RuntimeError {
            thrown: None,
            message: "Reflect.getPrototypeOf target must be an object".to_owned(),
        }),
        Some(Value::Object(object)) => Ok(object
            .prototype_slot()
            .map(|prototype| prototype.to_value())
            .unwrap_or(Value::Null)),
        Some(Value::Map(map)) => Ok(map
            .object()
            .prototype_slot()
            .map(|prototype| prototype.to_value())
            .unwrap_or(Value::Null)),
        Some(Value::Set(set)) => Ok(set
            .object()
            .prototype_slot()
            .map(|prototype| prototype.to_value())
            .unwrap_or(Value::Null)),
        Some(Value::Proxy(proxy)) => native_reflect_get_prototype_of(&[proxy.target()], env),
        Some(Value::Array(elements)) => Ok(elements
            .prototype_override()
            .unwrap_or_else(|| array_prototype(env))
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Function(function)) => {
            Ok(error::native_error_constructor_parent(function, env)
                .or_else(|| match function.internal_prototype_slot() {
                    Some(slot) => slot.map(|prototype| prototype.to_value()),
                    None => function_intrinsic_prototype(env).map(Value::Object),
                })
                .unwrap_or(Value::Null))
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "Reflect.getPrototypeOf target must be an object".to_owned(),
        }),
    }
}

pub(crate) fn native_reflect_set_prototype_of(
    argument_values: &[Value],
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let prototype = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Object(prototype) if symbol::is_symbol_primitive(&prototype) => {
            return Err(RuntimeError {
                thrown: None,
                message: "Reflect.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
        Value::Object(prototype) => Some(crate::Prototype::Object(prototype)),
        Value::Array(array) => Some(crate::Prototype::Object(array_as_object_prototype(
            &array, env,
        ))),
        Value::Function(function) => Some(crate::Prototype::Function(function)),
        Value::Null => None,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Reflect.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
    };

    let as_object = |prototype: Option<crate::Prototype>| prototype.and_then(|p| p.as_object());
    let success = match target {
        Value::Object(object) if symbol::is_symbol_primitive(&object) => {
            return Err(RuntimeError {
                thrown: None,
                message: "Reflect.setPrototypeOf target must be an object".to_owned(),
            });
        }
        Value::Object(object) => object.set_prototype_slot(prototype).is_ok(),
        Value::Map(map) => map.object().set_prototype_slot(prototype).is_ok(),
        Value::Set(set) => set.object().set_prototype_slot(prototype).is_ok(),
        Value::Proxy(proxy) => match proxy.target() {
            Value::Object(object) => object.set_prototype_slot(prototype).is_ok(),
            Value::Map(map) => map.object().set_prototype_slot(prototype).is_ok(),
            Value::Set(set) => set.object().set_prototype_slot(prototype).is_ok(),
            Value::Array(elements) => elements.set_prototype(as_object(prototype)).is_ok(),
            Value::Function(function) => function.set_internal_prototype_slot(prototype).is_ok(),
            _ => false,
        },
        Value::Array(elements) => elements.set_prototype(as_object(prototype)).is_ok(),
        Value::Function(function) => function.set_internal_prototype_slot(prototype).is_ok(),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => {
            return Err(RuntimeError {
                thrown: None,
                message: "Reflect.setPrototypeOf target must be an object".to_owned(),
            });
        }
    };

    Ok(Value::Boolean(success))
}
