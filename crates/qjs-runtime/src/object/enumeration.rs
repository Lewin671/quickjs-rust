use crate::{
    ArrayRef, Property, RuntimeError, Value, array_own_property_keys, array_own_property_names,
    function_own_property_keys, function_own_property_names, to_property_key,
};

use super::descriptor::own_property_descriptor;

pub(crate) fn native_object_keys(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let keys = own_property_keys(argument_values.first().cloned().unwrap_or(Value::Undefined));
    Ok(Value::Array(ArrayRef::new(
        keys.into_iter().map(Value::String).collect(),
    )))
}

pub(crate) fn native_object_values(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            message: "Object.values target must not be null or undefined".to_owned(),
        });
    }

    Ok(Value::Array(ArrayRef::new(
        enumerable_property_entries(target)?
            .into_iter()
            .map(|(_, value)| value)
            .collect(),
    )))
}

pub(crate) fn native_object_entries(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            message: "Object.entries target must not be null or undefined".to_owned(),
        });
    }

    Ok(Value::Array(ArrayRef::new(
        enumerable_property_entries(target)?
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

pub(crate) fn native_object_has_own(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            message: "Object.hasOwn target must not be null or undefined".to_owned(),
        });
    }

    let key = to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::Boolean(
        own_property_descriptor(target, &key)?.is_some(),
    ))
}

pub(super) fn enumerable_property_entries(
    value: Value,
) -> Result<Vec<(String, Value)>, RuntimeError> {
    let keys = own_property_keys(value.clone());
    let mut entries = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(Property { value, .. }) = own_property_descriptor(value.clone(), &key)? {
            entries.push((key, value));
        }
    }
    Ok(entries)
}

fn own_property_keys(value: Value) -> Vec<String> {
    match value {
        Value::Object(object) => object.own_property_keys(),
        Value::Array(elements) => array_own_property_keys(&elements),
        Value::Function(function) => function_own_property_keys(&function),
        Value::String(value) => crate::string::string_own_property_keys(&value),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Vec::new(),
    }
}

fn own_property_names(value: Value) -> Vec<String> {
    match value {
        Value::Object(object) => object.own_property_names(),
        Value::Array(elements) => array_own_property_names(&elements),
        Value::Function(function) => function_own_property_names(&function),
        Value::String(value) => crate::string::string_own_property_names(&value),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Vec::new(),
    }
}
