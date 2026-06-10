use std::collections::HashMap;

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
pub(crate) use key::{PropertyKey, to_property_key_value};
pub(crate) use prototype::{
    array_as_object_prototype, array_prototype, array_prototype_property, constructor_prototype,
    constructor_prototype_slot, function_intrinsic_prototype, function_prototype,
    function_prototype_chain_descriptor, function_prototype_property,
    inherited_object_prototype_property, inherited_primitive_prototype_descriptor,
    inherited_primitive_prototype_symbol_descriptor, inherited_string_prototype_property,
    object_prototype, string_prototype, value_prototype, value_prototype_slot,
};

pub(crate) fn has_property(
    value: Value,
    env: &HashMap<String, Value>,
    key: &str,
) -> Result<bool, RuntimeError> {
    has_property_key(value, env, &PropertyKey::String(key.to_owned()))
}

pub(crate) fn has_property_key(
    value: Value,
    env: &HashMap<String, Value>,
    key: &PropertyKey,
) -> Result<bool, RuntimeError> {
    let PropertyKey::String(key) = key else {
        return has_symbol_property(value, env, key);
    };
    match value {
        Value::Object(object) => Ok(object.contains_property(key)),
        Value::Map(map) => Ok(map.object().contains_property(key)),
        Value::Set(set) => Ok(set.object().contains_property(key)),
        Value::Proxy(proxy) => {
            let mut proxy_env = env.clone();
            crate::proxy::proxy_has(proxy, &PropertyKey::String(key.to_owned()), &mut proxy_env)
        }
        Value::Array(elements) => Ok(array_has_own_property(&elements, key)
            || array_prototype_property(&elements, env, key).is_some()),
        Value::Function(function) => Ok(function_own_property_descriptor(&function, key).is_some()
            || native_error_constructor_parent_descriptor(&function, env, key).is_some()
            || function_prototype_property(&function, env, key).is_some()),
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

fn has_symbol_property(
    value: Value,
    env: &HashMap<String, Value>,
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
                .prototype_override()
                .unwrap_or_else(|| array_prototype(env))
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
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    property_value_key(receiver, &PropertyKey::String(key.to_owned()), env)
}

pub(crate) fn property_value_key(
    receiver: Value,
    key: &PropertyKey,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    property_value_key_with_receiver(receiver.clone(), key, receiver, env)
}

pub(crate) fn property_value_key_with_receiver(
    target: Value,
    key: &PropertyKey,
    receiver: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let PropertyKey::String(key) = key else {
        return symbol_property_value_with_receiver(target, key, receiver, env);
    };
    match target {
        Value::Object(object) => property_descriptor_value(object.property(key), receiver, env),
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
                            .prototype_override()
                            .unwrap_or_else(|| array_prototype(env))
                            .and_then(|prototype| prototype.property(key))
                    });
                property_descriptor_value(descriptor, receiver, env)
            }
        }
        Value::String(value) => {
            if key == "length" {
                Ok(Value::Number(
                    crate::string::string_code_units(&value).len() as f64,
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
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let PropertyKey::Symbol(symbol) = key else {
        unreachable!("symbol property helper should only receive symbol keys");
    };
    match target {
        Value::Object(object) => {
            property_descriptor_value(object.symbol_property(symbol), receiver, env)
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
                    .prototype_override()
                    .unwrap_or_else(|| array_prototype(env))
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
    env: &HashMap<String, Value>,
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
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Some(property) = property else {
        return Ok(Value::Undefined);
    };
    if let Some(getter) = property.get {
        return call_function(getter, receiver, Vec::new(), env, false);
    }
    Ok(property.value)
}
