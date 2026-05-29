use std::collections::HashMap;

use crate::{
    Function, ObjectRef, Property, RuntimeError, Value, function_prototype, to_js_string,
    to_number, to_uint16,
};

use super::STRING_DATA_PROPERTY;

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
        match char::from_u32(u32::from(code_unit)) {
            Some(character) => result.push(character),
            None => result.push(char::REPLACEMENT_CHARACTER),
        }
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

fn to_code_point(value: Value) -> Result<u32, RuntimeError> {
    let number = to_number(value)?;
    if !number.is_finite() || number < 0.0 || number > 0x10FFFF as f64 || number.trunc() != number {
        return Err(RuntimeError {
            message: "String.fromCodePoint code point must be an integer in [0, 0x10FFFF]"
                .to_owned(),
        });
    }
    Ok(number as u32)
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
