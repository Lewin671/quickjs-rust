use std::collections::HashMap;

use crate::reflect::target::ensure_reflect_object_target;
use crate::{RuntimeError, Value, property_value};

pub(crate) fn native_reflect_get(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.get")?;
    let key = crate::to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;

    Ok(match target {
        Value::Object(_) => property_value(target, &key, env)?,
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
            thrown: None,
            message: "Reflect.has target must be an object".to_owned(),
        }),
    }
}
