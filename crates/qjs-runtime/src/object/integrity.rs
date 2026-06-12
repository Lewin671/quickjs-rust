use crate::{CallEnv, RuntimeError, Value};

/// `[[IsExtensible]]` over any value: ordinary objects report their slot, an
/// exotic Proxy consults its `isExtensible` trap (with the target-identity
/// invariant), and primitives are never extensible.
pub(crate) fn value_is_extensible(value: &Value, env: &mut CallEnv) -> Result<bool, RuntimeError> {
    Ok(match value {
        Value::Object(object) => object.is_extensible(),
        Value::Map(map) => map.object().is_extensible(),
        Value::Set(set) => set.object().is_extensible(),
        Value::Array(elements) => elements.is_extensible(),
        Value::Function(function) => function.is_extensible(),
        Value::Proxy(proxy) => crate::proxy::proxy_is_extensible(proxy.clone(), env)?,
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => false,
    })
}

/// Ordinary `[[IsExtensible]]` that never invokes traps; used as the Proxy
/// `isExtensible` trap forward and for invariant checks against the target.
pub(crate) fn ordinary_value_is_extensible(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.is_extensible(),
        Value::Map(map) => map.object().is_extensible(),
        Value::Set(set) => set.object().is_extensible(),
        Value::Array(elements) => elements.is_extensible(),
        Value::Function(function) => function.is_extensible(),
        Value::Proxy(proxy) => ordinary_value_is_extensible(&proxy.target()),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => false,
    }
}

/// Ordinary `[[PreventExtensions]]` over a value without trap dispatch.
pub(crate) fn ordinary_prevent_extensions(value: &Value) {
    match value {
        Value::Object(object) => object.prevent_extensions(),
        Value::Map(map) => map.object().prevent_extensions(),
        Value::Set(set) => set.object().prevent_extensions(),
        Value::Array(elements) => elements.prevent_extensions(),
        Value::Function(function) => function.prevent_extensions(),
        Value::Proxy(proxy) => ordinary_prevent_extensions(&proxy.target()),
        _ => {}
    }
}

pub(crate) fn native_object_is_extensible(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(match argument_values.first() {
        Some(value) => value_is_extensible(value, env)?,
        None => false,
    }))
}

pub(crate) fn native_object_prevent_extensions(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if let Value::Proxy(proxy) = &target {
        if !crate::proxy::proxy_prevent_extensions(proxy.clone(), env)? {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.preventExtensions failed".to_owned(),
            });
        }
        return Ok(target);
    }
    ordinary_prevent_extensions(&target);
    Ok(target)
}

pub(crate) fn native_object_is_sealed(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(match argument_values.first() {
        Some(Value::Object(object)) => object.is_sealed(),
        Some(Value::Map(map)) => map.object().is_sealed(),
        Some(Value::Set(set)) => set.object().is_sealed(),
        Some(Value::Proxy(proxy)) => match proxy.target() {
            Value::Object(object) => object.is_sealed(),
            Value::Map(map) => map.object().is_sealed(),
            Value::Set(set) => set.object().is_sealed(),
            Value::Array(elements) => elements.is_sealed(),
            Value::Function(function) => function.is_sealed(),
            _ => true,
        },
        Some(Value::Array(elements)) => elements.is_sealed(),
        Some(Value::Function(function)) => function.is_sealed(),
        Some(
            Value::String(_)
            | Value::Number(_)
            | Value::BigInt(_)
            | Value::Boolean(_)
            | Value::Null,
        )
        | Some(Value::Undefined)
        | None => true,
    }))
}

pub(crate) fn native_object_is_frozen(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(match argument_values.first() {
        Some(Value::Object(object)) => object.is_frozen(),
        Some(Value::Map(map)) => map.object().is_frozen(),
        Some(Value::Set(set)) => set.object().is_frozen(),
        Some(Value::Proxy(proxy)) => match proxy.target() {
            Value::Object(object) => object.is_frozen(),
            Value::Map(map) => map.object().is_frozen(),
            Value::Set(set) => set.object().is_frozen(),
            Value::Array(elements) => elements.is_frozen(),
            Value::Function(function) => function.is_frozen(),
            _ => true,
        },
        Some(Value::Array(elements)) => elements.is_frozen(),
        Some(Value::Function(function)) => function.is_frozen(),
        Some(
            Value::String(_)
            | Value::Number(_)
            | Value::BigInt(_)
            | Value::Boolean(_)
            | Value::Null,
        )
        | Some(Value::Undefined)
        | None => true,
    }))
}

pub(crate) fn native_object_seal(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    match &target {
        Value::Object(object) => object.seal(),
        Value::Map(map) => map.object().seal(),
        Value::Set(set) => set.object().seal(),
        Value::Proxy(proxy) => match proxy.target() {
            Value::Object(object) => object.seal(),
            Value::Map(map) => map.object().seal(),
            Value::Set(set) => set.object().seal(),
            Value::Array(elements) => elements.seal(),
            Value::Function(function) => function.seal(),
            _ => {}
        },
        Value::Array(elements) => elements.seal(),
        Value::Function(function) => function.seal(),
        _ => {}
    }
    Ok(target)
}

pub(crate) fn native_object_freeze(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    match &target {
        Value::Object(object) => object.freeze(),
        Value::Map(map) => map.object().freeze(),
        Value::Set(set) => set.object().freeze(),
        Value::Proxy(proxy) => match proxy.target() {
            Value::Object(object) => object.freeze(),
            Value::Map(map) => map.object().freeze(),
            Value::Set(set) => set.object().freeze(),
            Value::Array(elements) => elements.freeze(),
            Value::Function(function) => function.freeze(),
            _ => {}
        },
        Value::Array(elements) => elements.freeze(),
        Value::Function(function) => function.freeze(),
        _ => {}
    }
    Ok(target)
}
