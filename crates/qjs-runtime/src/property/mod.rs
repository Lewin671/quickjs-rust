use crate::CallEnv;
use crate::{Property, RuntimeError, Value, call_function, error};

mod array;
mod function;
mod key;
mod prototype;

pub(crate) use array::{
    array_has_own_property, array_own_property_descriptor, array_own_property_keys,
    array_own_property_names,
};
pub(crate) use function::{
    function_delete_own_property, function_delete_own_symbol_property,
    function_own_property_descriptor, function_own_property_keys, function_own_property_names,
    function_own_property_symbols, function_own_symbol_property_descriptor,
};
pub(crate) use key::{PropertyKey, to_property_key_value, try_to_property_key_without_coercion};
pub(crate) use prototype::{
    array_as_prototype_slot, array_prototype, constructor_named_prototype, constructor_prototype,
    constructor_prototype_slot, function_constructor_as_prototype_slot,
    function_intrinsic_prototype_slot, function_prototype, function_prototype_chain_descriptor,
    inherited_primitive_prototype_descriptor, inherited_primitive_prototype_symbol_descriptor,
    native_construct_prototype_slot, object_prototype, string_prototype, value_prototype,
    value_prototype_slot,
};

pub(crate) fn has_property(value: Value, env: &CallEnv, key: &str) -> Result<bool, RuntimeError> {
    has_property_key(value, env, &PropertyKey::String(key.to_owned()))
}

pub(crate) fn has_property_key(
    value: Value,
    env: &CallEnv,
    key: &PropertyKey,
) -> Result<bool, RuntimeError> {
    let PropertyKey::String(key) = key else {
        return has_symbol_property(value, env, key);
    };
    match value {
        Value::Object(object) => {
            if crate::typed_array::is_typed_array_object(&object) {
                return match crate::typed_array::indexed_element_value(&object, key) {
                    crate::typed_array::IndexedRead::Present(_) => Ok(true),
                    crate::typed_array::IndexedRead::Missing => Ok(false),
                    crate::typed_array::IndexedRead::NotIndexed => {
                        object_has_property(&object, env, key)
                    }
                };
            }
            object_has_property(&object, env, key)
        }
        Value::Map(map) => object_has_property(&map.object(), env, key),
        Value::Set(set) => object_has_property(&set.object(), env, key),
        Value::Proxy(proxy) => {
            let mut proxy_env = env.clone();
            crate::proxy::proxy_has(proxy, &PropertyKey::String(key.to_owned()), &mut proxy_env)
        }
        Value::Array(elements) => {
            if array_has_own_property(&elements, key) {
                return Ok(true);
            }
            match elements.prototype_slot_override() {
                Some(prototype) => prototype_has_property(prototype, env, key),
                None => prototype_has_property(
                    array_prototype(env).map(crate::Prototype::Object),
                    env,
                    key,
                ),
            }
        }
        Value::Function(function) => function_has_property(&function, env, key),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "property target must be an object".to_owned(),
        }),
    }
}

fn object_has_property(
    object: &crate::ObjectRef,
    env: &CallEnv,
    key: &str,
) -> Result<bool, RuntimeError> {
    if object.has_own_property(key) {
        return Ok(true);
    }
    prototype_has_property(object.prototype_slot(), env, key)
}

fn prototype_has_property(
    prototype: Option<crate::Prototype>,
    env: &CallEnv,
    key: &str,
) -> Result<bool, RuntimeError> {
    match prototype {
        Some(crate::Prototype::Object(object)) => object_has_property(&object, env, key),
        Some(crate::Prototype::Array(array)) => has_property(Value::Array(array.array()), env, key),
        Some(crate::Prototype::Function(function)) => function_has_property(&function, env, key),
        Some(crate::Prototype::Proxy(proxy)) => {
            let mut proxy_env = env.clone();
            crate::proxy::proxy_has(proxy, &PropertyKey::String(key.to_owned()), &mut proxy_env)
        }
        None => Ok(false),
    }
}

fn function_has_property(
    function: &crate::Function,
    env: &CallEnv,
    key: &str,
) -> Result<bool, RuntimeError> {
    if function_own_property_descriptor(function, key).is_some()
        || native_error_constructor_parent_descriptor(function, env, key).is_some()
    {
        return Ok(true);
    }
    match function.internal_prototype_slot() {
        Some(slot) => prototype_has_property(slot, env, key),
        None => prototype_has_property(function_intrinsic_prototype_slot(env), env, key),
    }
}

pub(crate) fn own_or_inherited_descriptor(value: Value, key: &str) -> Option<Property> {
    match value {
        Value::Object(object) if crate::typed_array::is_typed_array_object(&object) => {
            match crate::typed_array::typed_array_own_property_descriptor(&object, key) {
                Some(property) => Some(property),
                None if crate::typed_array::canonical_numeric_index(key).is_some() => None,
                None => object.property(key),
            }
        }
        Value::Object(object) => object.property(key),
        Value::Map(map) => map.object().property(key),
        Value::Set(set) => set.object().property(key),
        Value::Array(elements) => {
            crate::array_own_property_descriptor(&elements, key).or_else(|| {
                elements
                    .prototype_slot_override()
                    .and_then(|slot| slot)
                    .and_then(|prototype| prototype_descriptor_without_traps(prototype, key))
            })
        }
        Value::Function(function) => function.chain_property(key),
        Value::Proxy(proxy) => proxy
            .target_result()
            .ok()
            .and_then(|target| own_or_inherited_descriptor(target, key)),
        Value::String(value) => crate::string::string_own_property_descriptor(&value, key),
        Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => None,
    }
}

fn prototype_descriptor_without_traps(prototype: crate::Prototype, key: &str) -> Option<Property> {
    match prototype {
        crate::Prototype::Object(object) => object.property(key),
        crate::Prototype::Array(array) => array.property(key),
        crate::Prototype::Function(function) => function.chain_property(key),
        crate::Prototype::Proxy(proxy) => proxy
            .target_result()
            .ok()
            .and_then(|target| own_or_inherited_descriptor(target, key)),
    }
}

pub(crate) fn own_or_inherited_symbol_descriptor(
    value: Value,
    symbol: &crate::ObjectRef,
) -> Option<Property> {
    match value {
        Value::Object(object) => object.symbol_property(symbol),
        Value::Map(map) => map.object().symbol_property(symbol),
        Value::Set(set) => set.object().symbol_property(symbol),
        Value::Array(elements) => elements.symbol_property(symbol).or_else(|| {
            elements
                .prototype_slot_override()
                .and_then(|slot| slot)
                .and_then(|prototype| prototype_symbol_descriptor_without_traps(prototype, symbol))
        }),
        Value::Function(function) => function.chain_symbol_property(symbol),
        Value::Proxy(proxy) => proxy
            .target_result()
            .ok()
            .and_then(|target| own_or_inherited_symbol_descriptor(target, symbol)),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => None,
    }
}

fn prototype_symbol_descriptor_without_traps(
    prototype: crate::Prototype,
    symbol: &crate::ObjectRef,
) -> Option<Property> {
    match prototype {
        crate::Prototype::Object(object) => object.symbol_property(symbol),
        crate::Prototype::Array(array) => array.symbol_property(symbol),
        crate::Prototype::Function(function) => function.chain_symbol_property(symbol),
        crate::Prototype::Proxy(proxy) => proxy
            .target_result()
            .ok()
            .and_then(|target| own_or_inherited_symbol_descriptor(target, symbol)),
    }
}

fn has_symbol_property(
    value: Value,
    env: &CallEnv,
    key: &PropertyKey,
) -> Result<bool, RuntimeError> {
    let PropertyKey::Symbol(symbol) = key else {
        unreachable!("symbol property helper should only receive symbol keys");
    };
    match value {
        Value::Object(object) => Ok(object.symbol_property(symbol).is_some()),
        Value::Map(map) => Ok(map.object().symbol_property(symbol).is_some()),
        Value::Set(set) => Ok(set.object().symbol_property(symbol).is_some()),
        Value::Proxy(proxy) => {
            let mut proxy_env = env.clone();
            crate::proxy::proxy_has(proxy, key, &mut proxy_env)
        }
        Value::Function(function) => Ok(function.symbol_property(symbol, env).is_some()),
        Value::Array(elements) => Ok(elements.symbol_property(symbol).is_some()
            || elements
                .effective_prototype_slot(env)
                .is_some_and(|prototype| prototype.symbol_property(symbol).is_some())),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "property target must be an object".to_owned(),
        }),
    }
}

pub(crate) fn property_value(
    receiver: Value,
    key: &str,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    property_value_key(receiver, &PropertyKey::String(key.to_owned()), env)
}

pub(crate) fn property_value_key(
    receiver: Value,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    property_value_key_with_receiver(receiver.clone(), key, receiver, env)
}

pub(crate) fn property_value_key_with_receiver(
    target: Value,
    key: &PropertyKey,
    receiver: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let PropertyKey::String(key) = key else {
        return symbol_property_value_with_receiver(target, key, receiver, env);
    };
    match target {
        Value::Object(object) => {
            if crate::typed_array::is_typed_array_object(&object) {
                match crate::typed_array::indexed_element_value(&object, key) {
                    crate::typed_array::IndexedRead::Present(value) => return Ok(*value),
                    crate::typed_array::IndexedRead::Missing => return Ok(Value::Undefined),
                    crate::typed_array::IndexedRead::NotIndexed => {}
                }
            }
            if object.is_module_namespace_exotic()
                && let Some(property) = object.module_namespace_export_property(key)?
            {
                return property_descriptor_value(Some(property), receiver, env);
            }
            // OrdinaryGet: resolve the own property, otherwise walk the
            // [[Prototype]] chain one slot at a time so a Proxy anywhere in the
            // chain dispatches its `get` trap with the original receiver.
            if let Some(property) = object.own_property(key) {
                return property_descriptor_value(Some(property), receiver, env);
            }
            match object.prototype_slot() {
                Some(slot) => property_value_key_with_receiver(
                    slot.to_value(),
                    &PropertyKey::String(key.to_owned()),
                    receiver,
                    env,
                ),
                None => Ok(Value::Undefined),
            }
        }
        Value::Map(map) => property_descriptor_value(map.object().property(key), receiver, env),
        Value::Set(set) => property_descriptor_value(set.object().property(key), receiver, env),
        Value::Proxy(proxy) => {
            crate::proxy::proxy_get(proxy, &PropertyKey::String(key.to_owned()), receiver, env)
        }
        Value::Function(function) => property_descriptor_value(
            function_own_property_descriptor(&function, key)
                .or_else(|| native_error_constructor_parent_descriptor(&function, env, key))
                .or_else(|| function_prototype_chain_descriptor(&function, env, key)),
            receiver,
            env,
        ),
        Value::Array(elements) => {
            if key == "length" {
                Ok(Value::Number(elements.len() as f64))
            } else {
                let descriptor = key
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| elements.get(index).map(Property::enumerable))
                    .or_else(|| elements.property(key))
                    .or_else(|| {
                        elements
                            .effective_prototype_slot(env)
                            .and_then(|prototype| prototype.property(key))
                    });
                property_descriptor_value(descriptor, receiver, env)
            }
        }
        Value::String(value) => {
            if key == "length" {
                Ok(Value::Number(
                    crate::string::string_code_unit_len(&value) as f64
                ))
            } else {
                let descriptor = crate::string::string_property(&value, key)
                    .map(|value| Property::data(value, true, false, false))
                    .or_else(|| inherited_primitive_prototype_descriptor(env, "String", key));
                property_descriptor_value(descriptor, receiver, env)
            }
        }
        Value::Boolean(_) => property_descriptor_value(
            inherited_primitive_prototype_descriptor(env, "Boolean", key),
            receiver,
            env,
        ),
        Value::Number(_) => property_descriptor_value(
            inherited_primitive_prototype_descriptor(env, "Number", key),
            receiver,
            env,
        ),
        Value::BigInt(_) => property_descriptor_value(
            inherited_primitive_prototype_descriptor(env, "BigInt", key),
            receiver,
            env,
        ),
        Value::Null | Value::Undefined => Ok(Value::Undefined),
    }
}

fn symbol_property_value_with_receiver(
    target: Value,
    key: &PropertyKey,
    receiver: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let PropertyKey::Symbol(symbol) = key else {
        unreachable!("symbol property helper should only receive symbol keys");
    };
    match target {
        Value::Object(object) => {
            // OrdinaryGet for a symbol key, walking the [[Prototype]] chain one
            // slot at a time so a Proxy in the chain dispatches its `get` trap.
            if let Some(property) = object.own_symbol_property(symbol) {
                return property_descriptor_value(Some(property), receiver, env);
            }
            match object.prototype_slot() {
                Some(slot) => {
                    symbol_property_value_with_receiver(slot.to_value(), key, receiver, env)
                }
                None => Ok(Value::Undefined),
            }
        }
        Value::Proxy(proxy) => crate::proxy::proxy_get(proxy, key, receiver, env),
        Value::Map(map) => {
            property_descriptor_value(map.object().symbol_property(symbol), receiver, env)
        }
        Value::Set(set) => {
            property_descriptor_value(set.object().symbol_property(symbol), receiver, env)
        }
        Value::Function(function) => {
            property_descriptor_value(function.symbol_property(symbol, env), receiver, env)
        }
        Value::Array(elements) => property_descriptor_value(
            elements.symbol_property(symbol).or_else(|| {
                elements
                    .effective_prototype_slot(env)
                    .and_then(|prototype| prototype.symbol_property(symbol))
            }),
            receiver,
            env,
        ),
        Value::String(_) => property_descriptor_value(
            inherited_primitive_prototype_symbol_descriptor(env, "String", symbol),
            receiver,
            env,
        ),
        Value::Number(_) => property_descriptor_value(
            inherited_primitive_prototype_symbol_descriptor(env, "Number", symbol),
            receiver,
            env,
        ),
        Value::BigInt(_) => property_descriptor_value(
            inherited_primitive_prototype_symbol_descriptor(env, "BigInt", symbol),
            receiver,
            env,
        ),
        Value::Boolean(_) => property_descriptor_value(
            inherited_primitive_prototype_symbol_descriptor(env, "Boolean", symbol),
            receiver,
            env,
        ),
        Value::Null | Value::Undefined => Ok(Value::Undefined),
    }
}

fn native_error_constructor_parent_descriptor(
    function: &crate::Function,
    env: &CallEnv,
    key: &str,
) -> Option<Property> {
    match error::native_error_constructor_parent(function, env) {
        Some(Value::Function(parent)) => function_own_property_descriptor(&parent, key),
        Some(Value::Object(parent)) => parent.property(key),
        _ => None,
    }
}

fn property_descriptor_value(
    property: Option<Property>,
    receiver: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let Some(property) = property else {
        return Ok(Value::Undefined);
    };
    if let Some(getter) = property.get {
        return call_function(getter, receiver, Vec::new(), env, false);
    }
    Ok(property.value)
}
