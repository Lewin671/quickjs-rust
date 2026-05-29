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

pub(crate) fn native_reflect_get_own_property_descriptor(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.getOwnPropertyDescriptor")?;
    object::native_object_get_own_property_descriptor(argument_values, env)
}
