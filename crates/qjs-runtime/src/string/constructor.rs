use crate::{RuntimeError, Value, to_js_string, to_uint16};

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
