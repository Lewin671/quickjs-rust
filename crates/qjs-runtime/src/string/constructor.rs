use std::collections::HashMap;

use crate::{
    Function, ObjectRef, Property, RuntimeError, Value, function_prototype, property_value, symbol,
    to_js_string_with_env, to_length_with_env, to_number_with_env, to_uint16_with_env,
};

use super::{
    STRING_DATA_PROPERTY, string_code_units, string_from_code_unit, string_from_code_units,
};
use crate::CallEnv;

pub(crate) fn native_string(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = match argument_values.first().cloned() {
        Some(Value::Object(object)) if symbol::is_symbol_primitive(&object) => {
            symbol::symbol_descriptive_string(&object)
        }
        Some(value) => to_js_string_with_env(value, env)?,
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let mut result = String::new();
    for value in argument_values.iter().cloned() {
        let code_unit = to_uint16_with_env(value, env)?;
        result.push_str(&string_from_code_unit(code_unit));
    }
    Ok(Value::String(result))
}

pub(crate) fn native_string_from_code_point(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    // Reserve roughly one byte per argument up front (BMP code points are one to
    // three UTF-8 bytes) and push each code point directly into the accumulator
    // instead of allocating a throwaway `String` per argument.
    let mut result = String::with_capacity(argument_values.len());
    for value in argument_values.iter().cloned() {
        let code_point = to_code_point(value, env)?;
        push_code_point(&mut result, code_point);
    }
    Ok(Value::String(result))
}

pub(crate) fn native_string_raw(
    argument_values: &[Value],
    env: &mut CallEnv,
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

fn to_code_point(value: Value, env: &mut CallEnv) -> Result<u32, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    if !number.is_finite() || number < 0.0 || number > 0x10FFFF as f64 || number.trunc() != number {
        return Err(RuntimeError {
            thrown: None,
            message:
                "RangeError: String.fromCodePoint code point must be an integer in [0, 0x10FFFF]"
                    .to_owned(),
        });
    }
    Ok(number as u32)
}

/// Appends `code_point` to `result` without an intermediate allocation. A BMP
/// code point routes through the code-unit path so lone surrogates keep their
/// sentinel escaping; anything else is a `char` pushed directly.
fn push_code_point(result: &mut String, code_point: u32) {
    // A non-surrogate code point (BMP or supplementary) is a real scalar value
    // and pushes as a `char`. Lone surrogates have no scalar value, so they keep
    // the sentinel escaping the code-unit helper applies.
    match char::from_u32(code_point) {
        Some(character) => result.push(character),
        // Only lone surrogates (and only values up to 0xFFFF) lack a scalar
        // value, so the code-unit helper can apply its sentinel escaping.
        None => result.push_str(&string_from_code_unit(code_point as u16)),
    }
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
            Value::Number(string_code_units(value).len() as f64),
            false,
            false,
            false,
        ),
    );
    for (index, code_unit) in string_code_units(value).into_iter().enumerate() {
        object.define_property(
            index.to_string(),
            Property::data(
                Value::String(string_from_code_units(&[code_unit])),
                true,
                false,
                false,
            ),
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
