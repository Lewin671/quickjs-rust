use crate::{
    RuntimeError, Value,
    string::{string_code_units, string_from_code_unit},
};

pub(crate) fn native_regexp_escape(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let Some(Value::String(source)) = argument_values.first() else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp.escape argument must be a string".to_owned(),
        });
    };
    Ok(Value::String(regexp_escape(source)))
}

fn regexp_escape(source: &str) -> String {
    let mut escaped = String::new();
    for (index, code_unit) in string_code_units(source).into_iter().enumerate() {
        if code_unit < 33 {
            match code_unit {
                9 => escaped.push_str("\\t"),
                10 => escaped.push_str("\\n"),
                11 => escaped.push_str("\\v"),
                12 => escaped.push_str("\\f"),
                13 => escaped.push_str("\\r"),
                _ => push_hex_escape(&mut escaped, code_unit),
            }
        } else if code_unit < 128 {
            if is_ascii_alphanumeric_code_unit(code_unit) {
                if index == 0 {
                    push_hex_escape(&mut escaped, code_unit);
                } else {
                    escaped.push_str(&string_from_code_unit(code_unit));
                }
            } else if is_regexp_escape_other_punctuator(code_unit) {
                push_hex_escape(&mut escaped, code_unit);
            } else {
                if code_unit != u16::from(b'_') {
                    escaped.push('\\');
                }
                escaped.push_str(&string_from_code_unit(code_unit));
            }
        } else if code_unit < 256 {
            push_hex_escape(&mut escaped, code_unit);
        } else if is_surrogate(code_unit)
            || is_regexp_escape_whitespace(code_unit)
            || code_unit == 0xFEFF
        {
            push_unicode_escape(&mut escaped, code_unit);
        } else {
            escaped.push_str(&string_from_code_unit(code_unit));
        }
    }
    escaped
}

fn is_regexp_escape_other_punctuator(code_unit: u16) -> bool {
    matches!(
        code_unit,
        0x002c
            | 0x002d
            | 0x003d
            | 0x003c
            | 0x003e
            | 0x0023
            | 0x0026
            | 0x0021
            | 0x0025
            | 0x003a
            | 0x003b
            | 0x0040
            | 0x007e
            | 0x0027
            | 0x0060
            | 0x0022
    )
}

fn is_ascii_alphanumeric_code_unit(code_unit: u16) -> bool {
    (u16::from(b'0')..=u16::from(b'9')).contains(&code_unit)
        || (u16::from(b'A')..=u16::from(b'Z')).contains(&code_unit)
        || (u16::from(b'a')..=u16::from(b'z')).contains(&code_unit)
}

fn is_regexp_escape_whitespace(code_unit: u16) -> bool {
    matches!(
        code_unit,
        0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000
    )
}

fn is_surrogate(code_unit: u16) -> bool {
    (0xD800..=0xDFFF).contains(&code_unit)
}

fn push_hex_escape(output: &mut String, code_unit: u16) {
    output.push_str(&format!("\\x{code_unit:02x}"));
}

fn push_unicode_escape(output: &mut String, code_unit: u16) {
    output.push_str(&format!("\\u{code_unit:04x}"));
}
