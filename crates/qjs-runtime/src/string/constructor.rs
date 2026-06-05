use std::collections::HashMap;

use crate::{
    Function, ObjectRef, Property, RuntimeError, Value, function_prototype, property_value,
    to_js_string, to_js_string_with_env, to_length_with_env, to_number, to_uint16,
};

use super::{STRING_DATA_PROPERTY, string_from_code_unit};

pub(crate) fn native_string(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    let value = match argument_values.first().cloned() {
        Some(value) => to_js_string(value)?,
        None => String::new(),
    };
    if !is_construct {
        return Ok(Value::String(value));
    }

    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    define_string_data(&object, &value);
    Ok(Value::Object(object))
}

pub(crate) fn native_string_from_char_code(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let mut result = String::new();
    for value in argument_values.iter().cloned() {
        let code_unit = to_uint16(value)?;
        result.push_str(&string_from_code_unit(code_unit));
    }
    Ok(Value::String(result))
}

pub(crate) fn native_string_from_code_point(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let mut result = String::new();
    for value in argument_values.iter().cloned() {
        let code_point = to_code_point(value)?;
        match char::from_u32(code_point) {
            Some(character) => result.push(character),
            None => result.push(char::REPLACEMENT_CHARACTER),
        }
    }
    Ok(Value::String(result))
}

pub(crate) fn native_string_raw(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let template = require_object_coercible(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        "String.raw template",
    )?;
    let raw = require_object_coercible(property_value(template, "raw", env)?, "String.raw raw")?;
    let raw_length = to_length_with_env(property_value(raw.clone(), "length", env)?, env)?;
    if raw_length == 0 {
        return Ok(Value::String(String::new()));
    }

    let mut result = String::new();
    for index in 0..raw_length {
        let raw_segment = property_value(raw.clone(), &index.to_string(), env)?;
        result.push_str(&to_js_string_with_env(raw_segment, env)?);
        if index + 1 < raw_length {
            match argument_values.get(index + 1).cloned() {
                Some(substitution) => result.push_str(&to_js_string_with_env(substitution, env)?),
                None => result.push_str(""),
            }
        }
    }
    Ok(Value::String(result))
}

fn to_code_point(value: Value) -> Result<u32, RuntimeError> {
    let number = to_number(value)?;
    if !number.is_finite() || number < 0.0 || number > 0x10FFFF as f64 || number.trunc() != number {
        return Err(RuntimeError {
            thrown: None,
            message: "String.fromCodePoint code point must be an integer in [0, 0x10FFFF]"
                .to_owned(),
        });
    }
    Ok(number as u32)
}

fn require_object_coercible(value: Value, context: &str) -> Result<Value, RuntimeError> {
    match value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: {context} must be object coercible"),
        }),
        value => Ok(value),
    }
}

fn define_string_data(object: &ObjectRef, value: &str) {
    object.define_non_enumerable(
        STRING_DATA_PROPERTY.to_owned(),
        Value::String(value.to_owned()),
    );
    object.define_property(
        "length".to_owned(),
        Property::data(
            Value::Number(value.chars().count() as f64),
            false,
            false,
            false,
        ),
    );
    for (index, character) in value.chars().enumerate() {
        object.define_property(
            index.to_string(),
            Property::data(Value::String(character.to_string()), true, false, false),
        );
    }
}

pub(crate) fn string_object_value(object: &ObjectRef) -> Option<String> {
    match object.own_property(STRING_DATA_PROPERTY) {
        Some(Property {
            value: Value::String(value),
            ..
        }) => Some(value),
        _ => None,
    }
}

pub(crate) fn is_string_object(object: &ObjectRef) -> bool {
    string_object_value(object).is_some()
}
