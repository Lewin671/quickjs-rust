use std::collections::HashMap;

use qjs_parser::parse_script;

use crate::{
    Function, NativeFunction, Property, RuntimeError, Value,
    bytecode::{compile_script, eval_bytecode_with_env},
    string::{string_code_units, string_from_code_unit},
    to_js_string_with_env, to_number,
};

pub(super) fn install_globals(env: &mut HashMap<String, Value>, global_this: &Value) {
    env.insert("NaN".to_owned(), Value::Number(f64::NAN));
    env.insert("Infinity".to_owned(), Value::Number(f64::INFINITY));
    env.insert("globalThis".to_owned(), global_this.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_property(
            "globalThis".to_owned(),
            Property::data(global_this.clone(), false, true, true),
        );
        global_object.define_property(
            "NaN".to_owned(),
            Property::data(Value::Number(f64::NAN), false, false, false),
        );
        global_object.define_property(
            "Infinity".to_owned(),
            Property::data(Value::Number(f64::INFINITY), false, false, false),
        );
    }

    define_global_function(
        env,
        global_this,
        "isFinite",
        1,
        NativeFunction::GlobalIsFinite,
    );
    define_global_function(env, global_this, "isNaN", 1, NativeFunction::GlobalIsNaN);
    define_global_function(env, global_this, "decodeURI", 1, NativeFunction::DecodeUri);
    define_global_function(
        env,
        global_this,
        "decodeURIComponent",
        1,
        NativeFunction::DecodeUriComponent,
    );
    define_global_function(env, global_this, "encodeURI", 1, NativeFunction::EncodeUri);
    define_global_function(
        env,
        global_this,
        "encodeURIComponent",
        1,
        NativeFunction::EncodeUriComponent,
    );
    define_global_function(env, global_this, "eval", 1, NativeFunction::Eval);
    define_global_function(env, global_this, "escape", 1, NativeFunction::Escape);
    define_global_function(env, global_this, "unescape", 1, NativeFunction::Unescape);
}

fn define_global_function(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    let value = Value::Function(Function::new_native(Some(key), length, native, false));
    env.insert(key.to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set(key.to_owned(), value);
    }
}

pub(super) fn native_global_is_finite(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(to_number(value)?.is_finite()))
}

pub(super) fn native_global_is_nan(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(to_number(value)?.is_nan()))
}

pub(super) fn native_global_encode_uri(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    encode_uri(&source, UriEncodeKind::Uri).map(Value::String)
}

pub(super) fn native_global_encode_uri_component(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    encode_uri(&source, UriEncodeKind::Component).map(Value::String)
}

pub(super) fn native_global_decode_uri(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    decode_uri(&source, UriDecodeKind::Uri).map(Value::String)
}

pub(super) fn native_global_decode_uri_component(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    decode_uri(&source, UriDecodeKind::Component).map(Value::String)
}

pub(super) fn native_global_eval(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Value::String(source) = value else {
        return Ok(value);
    };
    let script = parse_script(&source).map_err(|error| RuntimeError {
        thrown: None,
        message: error.message,
    })?;
    let bytecode = compile_script(&script)?;
    let result = eval_bytecode_with_env(&bytecode, env.clone());
    for name in bytecode
        .local_names()
        .chain(bytecode.global_names().iter().map(String::as_str))
    {
        if let Some(value) = result.binding(name) {
            env.insert(name.to_owned(), value.clone());
        }
    }
    result.value
}

pub(super) fn native_global_escape(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    let mut escaped = String::new();
    for code_unit in string_code_units(&source) {
        if is_escape_unescaped(code_unit) {
            escaped.push_str(&string_from_code_unit(code_unit));
        } else if code_unit <= 0xFF {
            escaped.push_str(&format!("%{code_unit:02X}"));
        } else {
            escaped.push_str(&format!("%u{code_unit:04X}"));
        }
    }
    Ok(Value::String(escaped))
}

fn is_escape_unescaped(code_unit: u16) -> bool {
    matches!(code_unit, 0x41..=0x5A | 0x61..=0x7A | 0x30..=0x39)
        || matches!(code_unit, 0x40 | 0x2A | 0x5F | 0x2B | 0x2D | 0x2E | 0x2F)
}

pub(super) fn native_global_unescape(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    let mut output = String::new();
    let code_units = string_code_units(&source);
    let mut index = 0;
    while index < code_units.len() {
        if code_units[index] == b'%' as u16 {
            if let Some(code_unit) = parse_hex_escape(&code_units, index) {
                output.push_str(&string_from_code_unit(code_unit));
                index += if code_units.get(index + 1) == Some(&(b'u' as u16)) {
                    6
                } else {
                    3
                };
                continue;
            }
        }
        output.push_str(&string_from_code_unit(code_units[index]));
        index += 1;
    }
    Ok(Value::String(output))
}

fn parse_hex_escape(code_units: &[u16], index: usize) -> Option<u16> {
    if code_units.get(index + 1) == Some(&(b'u' as u16)) {
        return parse_hex_digits(code_units.get(index + 2..index + 6)?);
    }
    parse_hex_digits(code_units.get(index + 1..index + 3)?)
}

fn parse_hex_digits(digits: &[u16]) -> Option<u16> {
    let mut value = 0u16;
    for digit in digits {
        value = value.checked_mul(16)? + u16::try_from(hex_digit(*digit)?).ok()?;
    }
    Some(value)
}

fn hex_digit(code_unit: u16) -> Option<u32> {
    match code_unit {
        0x30..=0x39 => Some(u32::from(code_unit - 0x30)),
        0x61..=0x66 => Some(u32::from(code_unit - 0x61 + 10)),
        0x41..=0x46 => Some(u32::from(code_unit - 0x41 + 10)),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum UriEncodeKind {
    Uri,
    Component,
}

#[derive(Clone, Copy)]
enum UriDecodeKind {
    Uri,
    Component,
}

fn encode_uri(source: &str, kind: UriEncodeKind) -> Result<String, RuntimeError> {
    let code_units = string_code_units(source);
    let mut output = String::new();
    let mut index = 0;
    while index < code_units.len() {
        let code_unit = code_units[index];
        let code_point = if is_high_surrogate(code_unit) {
            let Some(&low) = code_units.get(index + 1) else {
                return malformed_uri();
            };
            if !is_low_surrogate(low) {
                return malformed_uri();
            }
            index += 1;
            0x10000 + ((u32::from(code_unit) - 0xD800) << 10) + u32::from(low) - 0xDC00
        } else if is_low_surrogate(code_unit) {
            return malformed_uri();
        } else {
            u32::from(code_unit)
        };

        let character = char::from_u32(code_point).ok_or_else(uri_error)?;
        if is_uri_unescaped(character, kind) {
            output.push(character);
        } else {
            let mut buffer = [0; 4];
            for byte in character.encode_utf8(&mut buffer).as_bytes() {
                output.push('%');
                output.push(hex_upper(byte >> 4));
                output.push(hex_upper(byte & 0x0F));
            }
        }
        index += 1;
    }
    Ok(output)
}

fn decode_uri(source: &str, kind: UriDecodeKind) -> Result<String, RuntimeError> {
    let mut output = String::new();
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '%' {
            output.push(chars[index]);
            index += 1;
            continue;
        }

        let escape_start = index;
        let first_byte = percent_byte(&chars, index)?;
        index += 3;

        let expected_len = utf8_sequence_len(first_byte)?;
        let mut bytes = vec![first_byte];
        for _ in 1..expected_len {
            if index >= chars.len() || chars[index] != '%' {
                return malformed_uri();
            }
            bytes.push(percent_byte(&chars, index)?);
            index += 3;
        }

        let decoded = std::str::from_utf8(&bytes).map_err(|_| uri_error())?;
        if matches!(kind, UriDecodeKind::Uri) && decoded.chars().all(is_uri_reserved) {
            output.extend(chars[escape_start..index].iter());
        } else {
            output.push_str(decoded);
        }
    }
    Ok(output)
}

fn is_uri_unescaped(character: char, kind: UriEncodeKind) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')'
        )
        || (matches!(kind, UriEncodeKind::Uri) && is_uri_reserved(character))
}

fn is_uri_reserved(character: char) -> bool {
    matches!(
        character,
        ';' | '/' | '?' | ':' | '@' | '&' | '=' | '+' | '$' | ',' | '#'
    )
}

fn is_high_surrogate(code_unit: u16) -> bool {
    (0xD800..=0xDBFF).contains(&code_unit)
}

fn is_low_surrogate(code_unit: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&code_unit)
}

fn percent_byte(chars: &[char], index: usize) -> Result<u8, RuntimeError> {
    let Some(high) = chars.get(index + 1).and_then(|ch| ch.to_digit(16)) else {
        return malformed_uri();
    };
    let Some(low) = chars.get(index + 2).and_then(|ch| ch.to_digit(16)) else {
        return malformed_uri();
    };
    Ok(((high << 4) | low) as u8)
}

fn utf8_sequence_len(first_byte: u8) -> Result<usize, RuntimeError> {
    match first_byte {
        0x00..=0x7F => Ok(1),
        0xC2..=0xDF => Ok(2),
        0xE0..=0xEF => Ok(3),
        0xF0..=0xF4 => Ok(4),
        _ => malformed_uri(),
    }
}

fn hex_upper(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'A' + nibble - 10),
        _ => unreachable!("nibble must be in 0..16"),
    }
}

fn malformed_uri<T>() -> Result<T, RuntimeError> {
    Err(uri_error())
}

fn uri_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "URIError: malformed URI sequence".to_owned(),
    }
}
