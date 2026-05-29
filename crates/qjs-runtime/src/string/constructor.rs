use crate::{RuntimeError, Value, to_js_string, to_number, to_uint16};

pub(crate) fn native_string(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    match argument_values.first().cloned() {
        Some(value) => Ok(Value::String(to_js_string(value)?)),
        None => Ok(Value::String(String::new())),
    }
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
