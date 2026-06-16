use crate::{
    ArrayRef, Property, PropertyKey, RuntimeError, Value, array_own_property_keys,
    array_own_property_names, function_own_property_keys, function_own_property_names,
    function_own_property_symbols, property_value, property_value_key, to_property_key_value,
};

use super::descriptor::own_property_descriptor_key;
use crate::CallEnv;

pub(crate) fn native_object_keys(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.keys target must not be null or undefined".to_owned(),
        });
    }

    let keys = enumerable_property_keys(target, env)?;
    Ok(Value::Array(ArrayRef::new(
        keys.into_iter().map(Value::String).collect(),
    )))
}

pub(crate) fn native_object_values(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.values target must not be null or undefined".to_owned(),
        });
    }

    Ok(Value::Array(ArrayRef::new(
        enumerable_property_entries(target, env)?
            .into_iter()
            .map(|(_, value)| value)
            .collect(),
    )))
}

pub(crate) fn native_object_entries(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.entries target must not be null or undefined".to_owned(),
        });
    }

    Ok(Value::Array(ArrayRef::new(
        enumerable_property_entries(target, env)?
            .into_iter()
            .map(|(key, value)| Value::Array(ArrayRef::new(vec![Value::String(key), value])))
            .collect(),
    )))
}

pub(crate) fn native_object_get_own_property_names(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.getOwnPropertyNames target must not be null or undefined".to_owned(),
        });
    }
    // GetOwnPropertyKeys runs O.[[OwnPropertyKeys]] (with the Proxy ownKeys
    // invariants over both string and symbol keys) and then filters to strings.
    let names = if let Value::Proxy(proxy) = &target {
        crate::proxy::proxy_own_keys(proxy.clone(), env)?
            .into_iter()
            .filter_map(|key| match key {
                PropertyKey::String(name) => Some(Value::String(name)),
                PropertyKey::Symbol(_) => None,
            })
            .collect()
    } else {
        own_property_names(target)
            .into_iter()
            .map(Value::String)
            .collect()
    };
    Ok(Value::Array(ArrayRef::new(names)))
}

pub(crate) fn native_object_get_own_property_symbols(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.getOwnPropertySymbols target must not be null or undefined".to_owned(),
        });
    }
    let symbols = if let Value::Proxy(proxy) = &target {
        crate::proxy::proxy_own_keys(proxy.clone(), env)?
            .into_iter()
            .filter_map(|key| match key {
                PropertyKey::Symbol(symbol) => Some(Value::Object(symbol)),
                PropertyKey::String(_) => None,
            })
            .collect()
    } else {
        own_property_symbols(target)
            .into_iter()
            .map(Value::Object)
            .collect()
    };
    Ok(Value::Array(ArrayRef::new(symbols)))
}

pub(crate) fn native_object_has_own(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.hasOwn target must not be null or undefined".to_owned(),
        });
    }

    let key = to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::Boolean(
        own_property_descriptor_key(target, &key)?.is_some(),
    ))
}

pub(crate) fn enumerable_property_entries(
    value: Value,
    env: &mut CallEnv,
) -> Result<Vec<(String, Value)>, RuntimeError> {
    let keys = own_property_keys_for_enumeration(value.clone(), false, env)?;
    let mut entries = Vec::with_capacity(keys.len());
    for key in keys {
        let PropertyKey::String(name) = key else {
            continue;
        };
        if let Some(Property {
            enumerable: true, ..
        }) = observable_own_property_descriptor(
            value.clone(),
            &PropertyKey::String(name.clone()),
            env,
        )? {
            let property = property_value(value.clone(), &name, env)?;
            entries.push((name, property));
        }
    }
    Ok(entries)
}

pub(crate) fn enumerable_property_entries_with_symbols(
    value: Value,
    env: &mut CallEnv,
) -> Result<Vec<(PropertyKey, Value)>, RuntimeError> {
    let keys = own_property_keys_for_enumeration(value.clone(), true, env)?;
    let mut entries = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(Property { enumerable, .. }) =
            observable_own_property_descriptor(value.clone(), &key, env)?
            && enumerable
        {
            let property = property_value_key(value.clone(), &key, env)?;
            entries.push((key, property));
        }
    }
    Ok(entries)
}

fn enumerable_property_keys(value: Value, env: &mut CallEnv) -> Result<Vec<String>, RuntimeError> {
    let keys = own_property_keys_for_enumeration(value.clone(), false, env)?;
    let mut enumerable = Vec::with_capacity(keys.len());
    for key in keys {
        let PropertyKey::String(name) = key else {
            continue;
        };
        if let Some(Property {
            enumerable: true, ..
        }) = observable_own_property_descriptor(
            value.clone(),
            &PropertyKey::String(name.clone()),
            env,
        )? {
            enumerable.push(name);
        }
    }
    Ok(enumerable)
}

fn own_property_keys_for_enumeration(
    value: Value,
    include_symbols: bool,
    env: &mut CallEnv,
) -> Result<Vec<PropertyKey>, RuntimeError> {
    if let Value::Proxy(proxy) = value {
        return Ok(crate::proxy::proxy_own_keys(proxy, env)?
            .into_iter()
            .filter(|key| include_symbols || matches!(key, PropertyKey::String(_)))
            .collect());
    }

    let string_keys = own_property_keys(value.clone())
        .into_iter()
        .map(PropertyKey::String);
    if !include_symbols {
        return Ok(string_keys.collect());
    }

    Ok(string_keys
        .chain(
            own_property_symbols(value)
                .into_iter()
                .map(PropertyKey::Symbol),
        )
        .collect())
}

fn observable_own_property_descriptor(
    value: Value,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<Option<Property>, RuntimeError> {
    if let Value::Proxy(proxy) = &value {
        return crate::proxy::proxy_get_own_property_descriptor(
            proxy.clone(),
            key,
            env,
            |target, _env| own_property_descriptor_key(target, key),
        );
    }

    own_property_descriptor_key(value, key)
}

pub(crate) fn own_property_keys(value: Value) -> Vec<String> {
    match value {
        Value::Object(object) if crate::typed_array::is_typed_array_object(&object) => {
            crate::typed_array::typed_array_own_property_keys(&object)
        }
        Value::Object(object) => object.own_property_keys(),
        Value::Map(map) => map.object().own_property_keys(),
        Value::Set(set) => set.object().own_property_keys(),
        Value::Proxy(proxy) => own_property_keys(proxy.target()),
        Value::Array(elements) => array_own_property_keys(&elements),
        Value::Function(function) => function_own_property_keys(&function),
        Value::String(value) => crate::string::string_own_property_keys(&value),
        Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Vec::new(),
    }
}

pub(crate) fn own_property_names(value: Value) -> Vec<String> {
    match value {
        Value::Object(object) if crate::typed_array::is_typed_array_object(&object) => {
            crate::typed_array::typed_array_own_property_names(&object)
        }
        Value::Object(object) => object.own_property_names(),
        Value::Map(map) => map.object().own_property_names(),
        Value::Set(set) => set.object().own_property_names(),
        Value::Proxy(proxy) => own_property_names(proxy.target()),
        Value::Array(elements) => array_own_property_names(&elements),
        Value::Function(function) => function_own_property_names(&function),
        Value::String(value) => crate::string::string_own_property_names(&value),
        Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Vec::new(),
    }
}

pub(crate) fn own_property_symbols(value: Value) -> Vec<crate::ObjectRef> {
    match value {
        Value::Object(object) => object.own_property_symbols(),
        Value::Map(map) => map.object().own_property_symbols(),
        Value::Set(set) => set.object().own_property_symbols(),
        Value::Proxy(proxy) => own_property_symbols(proxy.target()),
        Value::Function(function) => function_own_property_symbols(&function),
        Value::Array(elements) => elements.own_property_symbols(),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Vec::new(),
    }
}
