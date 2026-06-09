use std::collections::HashMap;

use crate::{
    ArrayRef, Property, PropertyKey, RuntimeError, Value, construct_function, ensure_constructor,
    property_value, property_value_key, symbol,
};

pub(super) fn array_species_create(
    receiver: Value,
    length: usize,
    method: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_array_species_receiver(&receiver)? {
        return Ok(Value::Array(ArrayRef::new(vec![Value::Undefined; length])));
    }

    let mut constructor = property_value(receiver, "constructor", env)?;
    if is_cross_realm_array_constructor(constructor.clone(), env)? {
        constructor = Value::Undefined;
    } else if is_object_like(&constructor) {
        if let Some(species_symbol) = symbol::species_symbol(env) {
            constructor =
                property_value_key(constructor, &PropertyKey::Symbol(species_symbol), env)?;
        }
        if matches!(constructor, Value::Null) {
            constructor = Value::Undefined;
        }
    }
    if matches!(constructor, Value::Undefined) {
        return Ok(Value::Array(ArrayRef::new(vec![Value::Undefined; length])));
    }
    ensure_constructor(&constructor).map_err(|_| RuntimeError {
        thrown: None,
        message: format!("TypeError: Array.prototype.{method} constructor is not a constructor"),
    })?;
    construct_function(
        constructor.clone(),
        constructor,
        vec![Value::Number(length as f64)],
        env,
    )
}

pub(super) fn validate_array_species_constructor(
    receiver: Value,
    method: &str,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    if !matches!(receiver, Value::Array(_)) {
        return Ok(());
    }

    match property_value(receiver, "constructor", env)? {
        Value::Undefined | Value::Function(_) | Value::Object(_) => Ok(()),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!(
                "TypeError: Array.prototype.{method} constructor is not a constructor"
            ),
        }),
    }
}

pub(super) fn create_data_property_or_throw(
    target: Value,
    key: String,
    value: Value,
) -> Result<(), RuntimeError> {
    if crate::object::define_property_on_value_key(
        target,
        PropertyKey::String(key),
        Property::data(value, true, true, true),
    )? {
        return Ok(());
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.concat cannot create result property".to_owned(),
    })
}

pub(super) fn set_array_like_length(target: Value, length: usize) -> Result<(), RuntimeError> {
    if crate::object::define_property_on_value_key(
        target,
        PropertyKey::String("length".to_owned()),
        Property::data(Value::Number(length as f64), false, true, false),
    )? {
        return Ok(());
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.concat cannot set result length".to_owned(),
    })
}

fn is_array_species_receiver(value: &Value) -> Result<bool, RuntimeError> {
    if matches!(value, Value::Array(_)) {
        return Ok(true);
    }
    match value {
        Value::Proxy(proxy) => crate::proxy::proxy_target_is_array_result(proxy),
        _ => Ok(false),
    }
}

fn is_object_like(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_)
    ) || matches!(value, Value::Proxy(_))
}

fn is_cross_realm_array_constructor(
    constructor: Value,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    if !is_object_like(&constructor) {
        return Ok(false);
    }
    Ok(matches!(
        property_value(constructor, "__quickjsRustCrossRealmArray", env)?,
        Value::Boolean(true)
    ))
}
