use std::collections::HashMap;

use crate::reflect::target::ensure_reflect_object_target;
use crate::{RuntimeError, Value, object};

pub(crate) fn native_reflect_define_property(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.defineProperty")?;
    let key = crate::to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let descriptor = object::to_property_descriptor_record(
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    Ok(Value::Boolean(
        object::define_property_descriptor_on_value_key(target, key, descriptor, env)?,
    ))
}

pub(crate) fn native_reflect_delete_property(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.deleteProperty")?;
    let key = crate::to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    let success = match target {
        Value::Object(object) => delete_object_property(object, &key),
        Value::Map(map) => delete_object_property(map.object(), &key),
        Value::Set(set) => delete_object_property(set.object(), &key),
        Value::Function(function) => match key {
            crate::PropertyKey::String(key) => crate::function_delete_own_property(&function, &key),
            crate::PropertyKey::Symbol(_) => true,
        },
        Value::Array(elements) => match key {
            crate::PropertyKey::String(key) => {
                key != "length"
                    && crate::array_own_property_descriptor(&elements, &key)
                        .is_none_or(|property| property.configurable)
            }
            crate::PropertyKey::Symbol(symbol) => elements.delete_own_symbol_property(&symbol),
        },
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("target was validated before property deletion"),
    };

    Ok(Value::Boolean(success))
}

fn delete_object_property(object: crate::ObjectRef, key: &crate::PropertyKey) -> bool {
    match key {
        crate::PropertyKey::String(key) => object.delete_own_property(key),
        crate::PropertyKey::Symbol(symbol) => object.delete_own_symbol_property(symbol),
    }
}

pub(crate) fn native_reflect_get_own_property_descriptor(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.getOwnPropertyDescriptor")?;
    object::native_object_get_own_property_descriptor(argument_values, env)
}
