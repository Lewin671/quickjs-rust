use crate::reflect::target::ensure_reflect_object_target;
use crate::{RuntimeError, Value};

pub(crate) fn native_reflect_is_extensible(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.isExtensible")?;
    Ok(Value::Boolean(match target {
        Value::Object(object) => object.is_extensible(),
        Value::Map(map) => map.object().is_extensible(),
        Value::Set(set) => set.object().is_extensible(),
        Value::Array(elements) => elements.is_extensible(),
        Value::Function(function) => function.is_extensible(),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("target was validated before extensibility check"),
    }))
}

pub(crate) fn native_reflect_prevent_extensions(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.preventExtensions")?;
    match target {
        Value::Object(object) => object.prevent_extensions(),
        Value::Map(map) => map.object().prevent_extensions(),
        Value::Set(set) => set.object().prevent_extensions(),
        Value::Array(elements) => elements.prevent_extensions(),
        Value::Function(function) => function.prevent_extensions(),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => unreachable!("target was validated before preventing extensions"),
    }
    Ok(Value::Boolean(true))
}
