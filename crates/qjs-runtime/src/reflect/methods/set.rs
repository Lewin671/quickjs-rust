use std::collections::HashMap;

use crate::reflect::target::ensure_reflect_object_target;
use crate::{ObjectRef, Property, PropertyKey, RuntimeError, Value, call_function, to_length};

pub(crate) fn native_reflect_set(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.set")?;
    let key = crate::to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let value = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let receiver = argument_values
        .get(3)
        .cloned()
        .unwrap_or_else(|| target.clone());

    Ok(Value::Boolean(ordinary_set(
        target, &key, value, receiver, env,
    )?))
}

pub(crate) fn ordinary_set(
    target: Value,
    key: &PropertyKey,
    value: Value,
    receiver: Value,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    if let Some(property) = own_property_descriptor_key(&target, key) {
        return ordinary_set_with_descriptor(property, key, value, receiver, env);
    }

    if let Some(prototype) = crate::value_prototype(target, env) {
        return ordinary_set(Value::Object(prototype), key, value, receiver, env);
    }

    set_receiver_data_property(receiver, key, value)
}

fn own_property_descriptor_key(target: &Value, key: &PropertyKey) -> Option<Property> {
    let PropertyKey::String(key) = key else {
        return own_symbol_property_descriptor(target, key);
    };
    match target {
        Value::Object(object) => object.own_property(key),
        Value::Map(map) => map.object().own_property(key),
        Value::Set(set) => set.object().own_property(key),
        Value::Array(elements) => crate::array_own_property_descriptor(elements, key),
        Value::Function(function) => crate::function_own_property_descriptor(function, key),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => None,
    }
}

fn own_symbol_property_descriptor(target: &Value, key: &PropertyKey) -> Option<Property> {
    let PropertyKey::Symbol(symbol) = key else {
        unreachable!("symbol descriptor helper should only receive symbol keys");
    };
    match target {
        Value::Object(object) => object.own_symbol_property(symbol),
        Value::Map(map) => map.object().own_symbol_property(symbol),
        Value::Set(set) => set.object().own_symbol_property(symbol),
        Value::Function(function) => {
            crate::function_own_symbol_property_descriptor(function, symbol)
        }
        Value::Array(elements) => elements.own_symbol_property(symbol),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => None,
    }
}

fn ordinary_set_with_descriptor(
    property: Property,
    key: &PropertyKey,
    value: Value,
    receiver: Value,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    if property.is_accessor() {
        let Some(setter) = property.set else {
            return Ok(false);
        };
        call_function(setter, receiver, vec![value], env, false)?;
        return Ok(true);
    }
    if !property.writable {
        return Ok(false);
    }
    set_receiver_data_property(receiver, key, value)
}

fn set_receiver_data_property(
    receiver: Value,
    key: &PropertyKey,
    value: Value,
) -> Result<bool, RuntimeError> {
    let PropertyKey::String(key) = key else {
        return set_receiver_symbol_data_property(receiver, key, value);
    };
    match receiver {
        Value::Object(object) => {
            let descriptor = match object.own_property(key) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !object.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            object.define_property(key.to_owned(), descriptor);
            Ok(true)
        }
        Value::Array(elements) => {
            if key == "length" {
                if !crate::array_own_property_descriptor(&elements, key)
                    .is_some_and(|property| property.writable)
                {
                    return Ok(false);
                }
                let length = to_length(value)?;
                if length > elements.len() && !elements.is_extensible() {
                    return Ok(false);
                }
                elements.set_len(length);
                Ok(true)
            } else {
                match key.parse::<usize>() {
                    Ok(index) => {
                        if index >= elements.len() && !elements.is_extensible() {
                            return Ok(false);
                        }
                        if elements.is_frozen() {
                            return Ok(false);
                        }
                        elements.set(index, value);
                    }
                    Err(_) => elements.set_property(key.to_owned(), value),
                }
                Ok(true)
            }
        }
        Value::Function(function) => {
            let descriptor = match crate::function_own_property_descriptor(&function, key) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !function.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            function
                .properties
                .borrow_mut()
                .insert(key.to_owned(), descriptor);
            Ok(true)
        }
        Value::Map(map) => {
            let object = map.object();
            let descriptor = match object.own_property(key) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !object.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            object.define_property(key.to_owned(), descriptor);
            Ok(true)
        }
        Value::Set(set) => {
            let object = set.object();
            let descriptor = match object.own_property(key) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !object.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            object.define_property(key.to_owned(), descriptor);
            Ok(true)
        }
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Ok(false),
    }
}

fn set_receiver_symbol_data_property(
    receiver: Value,
    key: &PropertyKey,
    value: Value,
) -> Result<bool, RuntimeError> {
    let PropertyKey::Symbol(symbol) = key else {
        unreachable!("symbol set helper should only receive symbol keys");
    };
    match receiver {
        Value::Object(object) => set_object_symbol_data_property(object, symbol.clone(), value),
        Value::Map(map) => set_object_symbol_data_property(map.object(), symbol.clone(), value),
        Value::Set(set) => set_object_symbol_data_property(set.object(), symbol.clone(), value),
        Value::Function(function) => {
            let descriptor = match function.own_symbol_property(symbol) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !function.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            function.define_symbol_property(symbol.clone(), descriptor);
            Ok(true)
        }
        Value::Array(elements) => {
            let descriptor = match elements.own_symbol_property(symbol) {
                Some(existing) if !existing.writable => return Ok(false),
                Some(existing) => Property::data(
                    value,
                    existing.enumerable,
                    existing.writable,
                    existing.configurable,
                ),
                None if !elements.is_extensible() => return Ok(false),
                None => Property::enumerable(value),
            };
            elements.define_symbol_property(symbol.clone(), descriptor);
            Ok(true)
        }
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Ok(false),
    }
}

fn set_object_symbol_data_property(
    object: ObjectRef,
    symbol: ObjectRef,
    value: Value,
) -> Result<bool, RuntimeError> {
    let descriptor = match object.own_symbol_property(&symbol) {
        Some(existing) if !existing.writable => return Ok(false),
        Some(existing) => Property::data(
            value,
            existing.enumerable,
            existing.writable,
            existing.configurable,
        ),
        None if !object.is_extensible() => return Ok(false),
        None => Property::enumerable(value),
    };
    object.define_symbol_property(symbol, descriptor);
    Ok(true)
}
