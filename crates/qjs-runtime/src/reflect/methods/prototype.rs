use std::collections::HashMap;

use crate::{RuntimeError, Value, array_prototype, function_intrinsic_prototype, symbol};

pub(crate) fn native_reflect_get_prototype_of(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::Object(object)) if symbol::is_symbol_primitive(object) => Err(RuntimeError {
            thrown: None,
            message: "Reflect.getPrototypeOf target must be an object".to_owned(),
        }),
        Some(Value::Object(object)) => {
            Ok(object.prototype().map(Value::Object).unwrap_or(Value::Null))
        }
        Some(Value::Map(map)) => Ok(map
            .object()
            .prototype()
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Set(set)) => Ok(set
            .object()
            .prototype()
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Array(elements)) => Ok(elements
            .prototype_override()
            .unwrap_or_else(|| array_prototype(env))
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Function(function)) => Ok(function
            .internal_prototype_override()
            .unwrap_or_else(|| function_intrinsic_prototype(env))
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Reflect.getPrototypeOf target must be an object".to_owned(),
        }),
    }
}

pub(crate) fn native_reflect_set_prototype_of(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let prototype = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Object(prototype) if symbol::is_symbol_primitive(&prototype) => {
            return Err(RuntimeError {
                thrown: None,
                message: "Reflect.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
        Value::Object(prototype) => Some(prototype),
        Value::Null => None,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Reflect.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
    };

    let success = match target {
        Value::Object(object) if symbol::is_symbol_primitive(&object) => {
            return Err(RuntimeError {
                thrown: None,
                message: "Reflect.setPrototypeOf target must be an object".to_owned(),
            });
        }
        Value::Object(object) => object.set_prototype(prototype).is_ok(),
        Value::Map(map) => map.object().set_prototype(prototype).is_ok(),
        Value::Set(set) => set.object().set_prototype(prototype).is_ok(),
        Value::Array(elements) => elements.set_prototype(prototype).is_ok(),
        Value::Function(function) => function.set_internal_prototype(prototype).is_ok(),
        Value::String(_)
        | Value::Number(_)
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
