use crate::{RuntimeError, Value};

pub(crate) fn native_object_is_extensible(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(match argument_values.first() {
        Some(Value::Object(object)) => object.is_extensible(),
        Some(Value::Array(elements)) => elements.is_extensible(),
        Some(Value::Function(function)) => function.is_extensible(),
        Some(Value::String(_) | Value::Number(_) | Value::Boolean(_) | Value::Null)
        | Some(Value::Undefined)
        | None => false,
    }))
}

pub(crate) fn native_object_prevent_extensions(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    match &target {
        Value::Object(object) => object.prevent_extensions(),
        Value::Array(elements) => elements.prevent_extensions(),
        Value::Function(function) => function.prevent_extensions(),
        _ => {}
    }
    Ok(target)
}

pub(crate) fn native_object_is_sealed(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(match argument_values.first() {
        Some(Value::Object(object)) => object.is_sealed(),
        Some(Value::Array(elements)) => elements.is_sealed(),
        Some(Value::Function(function)) => function.is_sealed(),
        Some(Value::String(_) | Value::Number(_) | Value::Boolean(_) | Value::Null)
        | Some(Value::Undefined)
        | None => true,
    }))
}

pub(crate) fn native_object_seal(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    match &target {
        Value::Object(object) => object.seal(),
        Value::Array(elements) => elements.seal(),
        Value::Function(function) => function.seal(),
        _ => {}
    }
    Ok(target)
}
