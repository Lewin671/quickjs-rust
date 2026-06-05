use std::collections::HashMap;

use qjs_parser::parse_script;

use crate::{
    Function, NativeFunction, Property, RuntimeError, Value,
    bytecode::{compile_eval_script, eval_bytecode_with_env},
    to_js_string_with_env, to_number,
};

pub(super) fn install_globals(env: &mut HashMap<String, Value>, global_this: &Value) {
    env.insert("undefined".to_owned(), Value::Undefined);
    env.insert("NaN".to_owned(), Value::Number(f64::NAN));
    env.insert("Infinity".to_owned(), Value::Number(f64::INFINITY));
    if let Value::Object(global_object) = global_this {
        global_object.define_property(
            "undefined".to_owned(),
            Property::data(Value::Undefined, false, false, false),
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
    define_global_function(env, global_this, "eval", 1, NativeFunction::Eval);
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
        global_object.define_property(key.to_owned(), Property::data(value, false, true, true));
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
    let bytecode = compile_eval_script(&script)?;
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

pub(super) fn native_encode_uri(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    encode_uri(argument_values, env, is_encode_uri_unescaped)
}

pub(super) fn native_encode_uri_component(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    encode_uri(argument_values, env, is_uri_unescaped)
}

pub(super) fn native_decode_uri(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    decode_uri(argument_values, env, true)
}

pub(super) fn native_decode_uri_component(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    decode_uri(argument_values, env, false)
}

fn encode_uri(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
    is_unescaped: fn(char) -> bool,
) -> Result<Value, RuntimeError> {
    let input = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let mut output = String::new();
    for character in input.chars() {
        if is_unescaped(character) {
            output.push(character);
        } else {
            let mut bytes = [0; 4];
            for byte in character.encode_utf8(&mut bytes).as_bytes() {
                output.push('%');
                output.push(hex_digit(byte >> 4));
                output.push(hex_digit(byte & 0x0f));
            }
        }
    }
    Ok(Value::String(output))
}

fn decode_uri(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
    preserve_reserved: bool,
) -> Result<Value, RuntimeError> {
    let input = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let bytes = input.as_bytes();
    let mut output = String::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            let character = input[index..]
                .chars()
                .next()
                .expect("byte index should point at a UTF-8 character boundary");
            output.push(character);
            index += character.len_utf8();
            continue;
        }

        let start = index;
        let first = decode_percent_byte(bytes, index)?;
        let expected_len = utf8_sequence_len(first)?;
        let mut decoded = vec![first];
        index += 3;
        for _ in 1..expected_len {
            if index >= bytes.len() || bytes[index] != b'%' {
                return uri_error();
            }
            decoded.push(decode_percent_byte(bytes, index)?);
            index += 3;
        }
        let decoded = std::str::from_utf8(&decoded).map_err(|_| RuntimeError {
            thrown: None,
            message: "URIError: malformed URI sequence".to_owned(),
        })?;
        if preserve_reserved && decoded.chars().any(is_uri_reserved) {
            output.push_str(&input[start..index]);
        } else {
            output.push_str(decoded);
        }
    }
    Ok(Value::String(output))
}

fn uri_error<T>() -> Result<T, RuntimeError> {
    Err(RuntimeError {
        thrown: None,
        message: "URIError: malformed URI sequence".to_owned(),
    })
}

fn is_encode_uri_unescaped(character: char) -> bool {
    is_uri_unescaped(character) || is_uri_reserved(character) || character == '#'
}

fn is_uri_unescaped(character: char) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')'
        )
}

fn is_uri_reserved(character: char) -> bool {
    matches!(
        character,
        ';' | ',' | '/' | '?' | ':' | '@' | '&' | '=' | '+' | '$' | '#'
    )
}

fn decode_percent_byte(bytes: &[u8], index: usize) -> Result<u8, RuntimeError> {
    if index + 2 >= bytes.len() {
        return uri_error();
    }
    let Some(high) = hex_value(bytes[index + 1]) else {
        return uri_error();
    };
    let Some(low) = hex_value(bytes[index + 2]) else {
        return uri_error();
    };
    Ok((high << 4) | low)
}

fn utf8_sequence_len(first: u8) -> Result<usize, RuntimeError> {
    match first {
        0x00..=0x7f => Ok(1),
        0xc2..=0xdf => Ok(2),
        0xe0..=0xef => Ok(3),
        0xf0..=0xf4 => Ok(4),
        _ => uri_error(),
    }
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + value - 10) as char,
        _ => unreachable!("hex digit nibble must be in range"),
    }
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}
