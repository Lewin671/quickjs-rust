use crate::CallEnv;
use crate::{RuntimeError, Value, array_prototype, error, function_intrinsic_prototype, symbol};

pub(crate) fn native_reflect_get_prototype_of(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if let Some(Value::Proxy(proxy)) = argument_values.first() {
        return crate::proxy::proxy_get_prototype_of(proxy.clone(), env);
    }
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let prototype_value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    match &prototype_value {
        Value::Object(prototype) if symbol::is_symbol_primitive(prototype) => {
            return Err(RuntimeError {
                thrown: None,
                message: "Reflect.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Null => {}
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Reflect.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
    }

    let success = match &target {
        Value::Object(object) if symbol::is_symbol_primitive(object) => {
            return Err(RuntimeError {
                thrown: None,
                message: "Reflect.setPrototypeOf target must be an object".to_owned(),
            });
        }
        Value::Proxy(proxy) => {
            crate::proxy::proxy_set_prototype_of(proxy.clone(), prototype_value, env)?
        }
        Value::Object(_) | Value::Map(_) | Value::Set(_) | Value::Array(_) | Value::Function(_) => {
            crate::object::ordinary_set_prototype_of(&target, prototype_value, env)?
        }
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
