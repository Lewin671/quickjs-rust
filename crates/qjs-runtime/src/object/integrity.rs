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

pub(crate) fn native_object_is_sealed(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(match argument_values.first() {
        Some(Value::Object(object)) => object.is_sealed(),
        Some(Value::Map(map)) => map.object().is_sealed(),
        Some(Value::Set(set)) => set.object().is_sealed(),
        Some(Value::Proxy(proxy)) => {
            test_integrity_level_on_proxy(proxy.clone(), IntegrityLevel::Sealed, env)?
        }
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

pub(crate) fn native_object_is_frozen(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(match argument_values.first() {
        Some(Value::Object(object))
            if object.is_module_namespace_exotic() && !object.own_property_names().is_empty() =>
        {
            false
        }
        Some(Value::Object(object)) => object.is_frozen(),
        Some(Value::Map(map)) => map.object().is_frozen(),
        Some(Value::Set(set)) => set.object().is_frozen(),
        Some(Value::Proxy(proxy)) => {
            test_integrity_level_on_proxy(proxy.clone(), IntegrityLevel::Frozen, env)?
        }
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

#[derive(Clone, Copy)]
enum IntegrityLevel {
    Sealed,
    Frozen,
}

pub(crate) fn native_object_seal(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if let Value::Proxy(proxy) = &target {
        if !set_integrity_level_on_proxy(proxy.clone(), &target, IntegrityLevel::Sealed, env)? {
            return Err(integrity_failed_error("Object.seal"));
        }
        return Ok(target);
    }
    match &target {
        Value::Object(object) => object.seal(),
        Value::Map(map) => map.object().seal(),
        Value::Set(set) => set.object().seal(),
        Value::Array(elements) => elements.seal(),
        Value::Function(function) => function.seal(),
        _ => {}
    }
    Ok(target)
}

pub(crate) fn native_object_freeze(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if let Value::Proxy(proxy) = &target {
        if !set_integrity_level_on_proxy(proxy.clone(), &target, IntegrityLevel::Frozen, env)? {
            return Err(integrity_failed_error("Object.freeze"));
        }
        return Ok(target);
    }
    match &target {
        Value::Object(object)
            if object.is_module_namespace_exotic() && !object.own_property_names().is_empty() =>
        {
            return Err(integrity_failed_error("Object.freeze"));
        }
        Value::Object(object) if crate::typed_array::is_typed_array_object(object) => {
            freeze_typed_array_object(object.clone(), &target, env)?;
        }
        Value::Object(object) => object.freeze(),
        Value::Map(map) => map.object().freeze(),
        Value::Set(set) => set.object().freeze(),
        Value::Array(elements) => elements.freeze(),
        Value::Function(function) => function.freeze(),
        _ => {}
    }
    Ok(target)
}

/// SetIntegrityLevel(O, level) for an exotic Proxy: it runs through the
/// `preventExtensions`, `ownKeys`, `getOwnPropertyDescriptor`, and
/// `defineProperty` traps instead of mutating the proxy target directly.
fn set_integrity_level_on_proxy(
    proxy: crate::proxy::ProxyRef,
    proxy_value: &Value,
    level: IntegrityLevel,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    use crate::object::{PropertyDescriptor, define_property_descriptor_on_value_key};

    // 1. If ? O.[[PreventExtensions]]() is false, SetIntegrityLevel returns false.
    if !crate::proxy::proxy_prevent_extensions(proxy.clone(), env)? {
        return Ok(false);
    }
    // 2. keys = ? O.[[OwnPropertyKeys]]().
    let keys = crate::proxy::proxy_own_keys(proxy.clone(), env)?;
    // 3. For each key, DefinePropertyOrThrow with the integrity descriptor.
    for key in keys {
        let descriptor = match level {
            IntegrityLevel::Sealed => PropertyDescriptor::integrity_non_configurable(),
            IntegrityLevel::Frozen => {
                let current = crate::proxy::proxy_get_own_property_descriptor(
                    proxy.clone(),
                    &key,
                    env,
                    |target, env| crate::object::own_property_descriptor_key(target, &key, env),
                )?;
                let Some(property) = current else {
                    continue;
                };
                if property.is_accessor() {
                    PropertyDescriptor::integrity_non_configurable()
                } else {
                    PropertyDescriptor::integrity_frozen_data()
                }
            }
        };
        if !define_property_descriptor_on_value_key(proxy_value.clone(), key, descriptor, env)? {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Cannot redefine property during integrity level change"
                    .to_owned(),
            });
        }
    }
    Ok(true)
}

fn test_integrity_level_on_proxy(
    proxy: crate::proxy::ProxyRef,
    level: IntegrityLevel,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    if crate::proxy::proxy_is_extensible(proxy.clone(), env)? {
        return Ok(false);
    }
    for key in crate::proxy::proxy_own_keys(proxy.clone(), env)? {
        let current = crate::proxy::proxy_get_own_property_descriptor(
            proxy.clone(),
            &key,
            env,
            |target, env| crate::object::own_property_descriptor_key(target, &key, env),
        )?;
        let Some(property) = current else {
            continue;
        };
        if property.configurable {
            return Ok(false);
        }
        if matches!(level, IntegrityLevel::Frozen) && !property.is_accessor() && property.writable {
            return Ok(false);
        }
    }
    Ok(true)
}

fn freeze_typed_array_object(
    object: crate::ObjectRef,
    target: &Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    use crate::object::{PropertyDescriptor, define_property_descriptor_on_value_key};

    object.prevent_extensions();
    if crate::typed_array::typed_array_buffer(&object)
        .is_some_and(|buffer| crate::array_buffer::is_resizable(&buffer))
        || crate::typed_array::typed_array_length(&object) > 0
    {
        return Err(integrity_failed_error("Object.freeze"));
    }

    let string_keys = crate::typed_array::typed_array_own_property_names(&object)
        .into_iter()
        .map(crate::PropertyKey::String);
    let symbol_keys = object
        .own_property_symbols()
        .into_iter()
        .map(crate::PropertyKey::Symbol);
    for key in string_keys.chain(symbol_keys) {
        let current = crate::object::own_property_descriptor_key(target.clone(), &key, env)?;
        let Some(property) = current else {
            continue;
        };
        let descriptor = if property.is_accessor() {
            PropertyDescriptor::integrity_non_configurable()
        } else {
            PropertyDescriptor::integrity_frozen_data()
        };
        if !define_property_descriptor_on_value_key(target.clone(), key, descriptor, env)? {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Cannot freeze typed array property".to_owned(),
            });
        }
    }
    Ok(())
}

fn integrity_failed_error(method: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("TypeError: {method} could not prevent extensions on the object"),
    }
}
