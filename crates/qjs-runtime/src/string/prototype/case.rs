use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::super::indexing::this_string_value;
use super::super::{string_code_units, string_from_code_unit};

pub(crate) fn native_string_prototype_to_lower_case(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(case_convert(
        &this_string_value(this_value, env)?,
        str::to_lowercase,
    )))
}

pub(crate) fn native_string_prototype_to_upper_case(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::String(case_convert(
        &this_string_value(this_value, env)?,
        str::to_uppercase,
    )))
}

fn case_convert(value: &str, convert: impl FnOnce(&str) -> String) -> String {
    let code_points = decode_utf16_code_points(value);
    encode_utf16_code_units(&convert(&code_points))
}

fn decode_utf16_code_points(value: &str) -> String {
    let code_units = string_code_units(value);
    let mut result = String::new();
    let mut index = 0;
    while index < code_units.len() {
        let first = code_units[index];
        if (0xD800..=0xDBFF).contains(&first) && index + 1 < code_units.len() {
            let second = code_units[index + 1];
            if (0xDC00..=0xDFFF).contains(&second) {
                let code_point =
                    (u32::from(first) - 0xD800) * 1024 + (u32::from(second) - 0xDC00) + 0x10000;
                if let Some(character) = char::from_u32(code_point) {
                    result.push(character);
                    index += 2;
                    continue;
                }
            }
        }

        result.push_str(&string_from_code_unit(first));
        index += 1;
    }
    result
}

fn encode_utf16_code_units(value: &str) -> String {
    string_code_units(value)
        .into_iter()
        .map(string_from_code_unit)
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{Value, eval};

    #[test]
    fn case_conversion_iterates_over_utf16_code_points() {
        assert_eq!(
            eval("'\\uD801\\uDC00'.toLowerCase() === '\\uD801\\uDC28';"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("'\\uD801\\uDC28'.toUpperCase() === '\\uD801\\uDC00';"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("'\\uD835\\uDCA2\\u03A3'.toLowerCase() === '\\uD835\\uDCA2\\u03C2';"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("'A\\u03A3\\uD835\\uDCA2'.toLowerCase() === 'a\\u03C3\\uD835\\uDCA2';"),
            Ok(Value::Boolean(true))
        );
    }
}
