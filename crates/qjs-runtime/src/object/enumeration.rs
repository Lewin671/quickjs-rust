use std::collections::HashMap;

use crate::{
    ArrayRef, Property, PropertyKey, RuntimeError, Value, array_own_property_keys,
    array_own_property_names, function_own_property_keys, function_own_property_names,
    function_own_property_symbols, property_value, property_value_key, to_property_key_value,
};

use super::descriptor::{own_property_descriptor, own_property_descriptor_key};

pub(crate) fn native_object_keys(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.keys target must not be null or undefined".to_owned(),
        });
    }

    let keys = own_property_keys(target);
    Ok(Value::Array(ArrayRef::new(
        keys.into_iter().map(Value::String).collect(),
    )))
}

pub(crate) fn native_object_values(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
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
    env: &mut HashMap<String, Value>,
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
) -> Result<Value, RuntimeError> {
    let names = own_property_names(argument_values.first().cloned().unwrap_or(Value::Undefined));
    Ok(Value::Array(ArrayRef::new(
        names.into_iter().map(Value::String).collect(),
    )))
}

pub(crate) fn native_object_get_own_property_symbols(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.getOwnPropertySymbols target must not be null or undefined".to_owned(),
        });
    }
    Ok(Value::Array(ArrayRef::new(
        own_property_symbols(target)
            .into_iter()
            .map(Value::Object)
            .collect(),
    )))
}

pub(crate) fn native_object_has_own(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.hasOwn target must not be null or undefined".to_owned(),
        });
    }

    let key = to_property_key_value(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::Boolean(
        own_property_descriptor_key(target, &key)?.is_some(),
    ))
}

pub(super) fn enumerable_property_entries(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<(String, Value)>, RuntimeError> {
    let keys = own_property_keys(value.clone());
    let mut entries = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(Property { enumerable, .. }) = own_property_descriptor(value.clone(), &key)?
            && enumerable
        {
            let property = property_value(value.clone(), &key, env)?;
            entries.push((key, property));
        }
    }
    Ok(entries)
}

pub(super) fn enumerable_property_entries_with_symbols(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<(PropertyKey, Value)>, RuntimeError> {
    let string_keys = own_property_keys(value.clone())
        .into_iter()
        .map(PropertyKey::String);
    let symbol_keys = own_property_symbols(value.clone())
        .into_iter()
        .map(PropertyKey::Symbol);
    let keys: Vec<_> = string_keys.chain(symbol_keys).collect();
    let mut entries = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(Property { enumerable, .. }) = own_property_descriptor_key(value.clone(), &key)?
            && enumerable
        {
            let property = property_value_key(value.clone(), &key, env)?;
            entries.push((key, property));
        }
    }
    Ok(entries)
}

fn own_property_keys(value: Value) -> Vec<String> {
    match value {
        Value::Object(object) => object.own_property_keys(),
        Value::Map(map) => map.object().own_property_keys(),
        Value::Set(set) => set.object().own_property_keys(),
        Value::Array(elements) => array_own_property_keys(&elements),
        Value::Function(function) => function_own_property_keys(&function),
        Value::String(value) => crate::string::string_own_property_keys(&value),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Vec::new(),
    }
}

pub(super) fn own_property_names(value: Value) -> Vec<String> {
    match value {
        Value::Object(object) => object.own_property_names(),
        Value::Map(map) => map.object().own_property_names(),
        Value::Set(set) => set.object().own_property_names(),
        Value::Array(elements) => array_own_property_names(&elements),
        Value::Function(function) => function_own_property_names(&function),
        Value::String(value) => crate::string::string_own_property_names(&value),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Vec::new(),
    }
}

fn own_property_symbols(value: Value) -> Vec<crate::ObjectRef> {
    match value {
        Value::Object(object) => object.own_property_symbols(),
        Value::Map(map) => map.object().own_property_symbols(),
        Value::Set(set) => set.object().own_property_symbols(),
        Value::Function(function) => function_own_property_symbols(&function),
        Value::Array(_)
        | Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Vec::new(),
    }
}
