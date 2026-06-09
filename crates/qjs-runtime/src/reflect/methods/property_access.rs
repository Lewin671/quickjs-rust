use std::collections::HashMap;

use crate::reflect::target::ensure_reflect_object_target;
use crate::{RuntimeError, Value, has_property_key, property_value_key_with_receiver};

pub(crate) fn native_reflect_get(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.get")?;
    let key = crate::to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    let receiver = argument_values
        .get(2)
        .cloned()
        .unwrap_or_else(|| target.clone());

    Ok(match target {
        Value::Object(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Array(_)
        | Value::Function(_)
        | Value::Proxy(_) => property_value_key_with_receiver(target, &key, receiver, env)?,
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("target was validated before property get"),
    })
}

pub(crate) fn native_reflect_has(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.has")?;
    let key = crate::to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    match target {
        Value::Object(_)
        | Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_) => Ok(Value::Boolean(has_property_key(target, env, &key)?)),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("target was validated before property lookup"),
    }
}
