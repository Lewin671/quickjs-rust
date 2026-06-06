use std::collections::HashMap;

use crate::reflect::target::ensure_reflect_object_target;
use crate::{RuntimeError, Value, has_property_key, property_value_key};

pub(crate) fn native_reflect_get(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.get")?;
    let key =
        crate::to_property_key_value(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;

    Ok(match target {
        Value::Object(_) | Value::Map(_) | Value::Set(_) | Value::Array(_) => {
            property_value_key(target, &key, env)?
        }
        Value::Function(function) => match key {
            crate::PropertyKey::String(key) => {
                crate::function_own_property_descriptor(&function, &key)
                    .map(|property| property.value)
                    .or_else(|| crate::function_prototype_property(&function, env, &key))
                    .unwrap_or(Value::Undefined)
            }
            crate::PropertyKey::Symbol(_) => Value::Undefined,
        },
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("target was validated before property get"),
    })
}

pub(crate) fn native_reflect_has(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key =
        crate::to_property_key_value(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    match target {
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) => {
            Ok(Value::Boolean(has_property_key(target, env, &key)?))
        }
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "Reflect.has target must be an object".to_owned(),
        }),
    }
}
