use std::collections::HashMap;

use crate::{
    CallEnv, NativeFunction, ObjectRef, Property, RuntimeError, Value, array_buffer, is_truthy,
    object_prototype, property_value,
};

use super::element::{read_view_elements, set_view_elements};
use super::{
    MAX_TYPED_ARRAY_LENGTH, construct, typed_array_buffer, typed_array_is_out_of_bounds,
    typed_array_kind, typed_array_length, typed_array_receiver,
};

pub(crate) fn native_uint8_array_from_base64(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = base64_source(argument_values.first(), "Uint8Array.fromBase64")?;
    let options = from_base64_options(argument_values.get(1).cloned(), env)?;
    let decoded = decode_base64(
        &source,
        options.alphabet,
        options.last_chunk_handling,
        MAX_TYPED_ARRAY_LENGTH,
    );
    if let Some(error) = decoded.error {
        return Err(error);
    }
    let values = decoded.bytes.into_iter().map(number_byte).collect();
    Ok(Value::Object(construct::create_with_values(
        NativeFunction::Uint8Array,
        values,
        env,
    )))
}

pub(crate) fn native_uint8_array_prototype_to_base64(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = uint8_array_receiver(&this_value, "toBase64")?;
    let options = to_base64_options(argument_values.first().cloned(), env)?;
    if super::typed_array_buffer_detached(&object) {
        return Err(array_buffer::detached_error());
    }
    if typed_array_is_out_of_bounds(&object) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: TypedArray is out of bounds".to_owned(),
        });
    }
    let length = typed_array_length(&object);
    let bytes: Vec<u8> = read_view_elements(&object, 0, length)
        .into_iter()
        .map(|value| match value {
            Value::Number(number) => number as u8,
            _ => 0,
        })
        .collect();
    Ok(Value::String(encode_base64(
        &bytes,
        options.alphabet,
        options.omit_padding,
    )))
}

pub(crate) fn native_uint8_array_prototype_set_from_base64(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = uint8_array_receiver(&this_value, "setFromBase64")?;
    if super::typed_array_buffer_detached(&object) {
        return Err(array_buffer::detached_error());
    }
    if typed_array_buffer(&object).is_some_and(|buffer| array_buffer::is_immutable(&buffer)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: ArrayBuffer is immutable".to_owned(),
        });
    }
    let source = base64_source(
        argument_values.first(),
        "Uint8Array.prototype.setFromBase64",
    )?;
    let options = set_from_base64_options(argument_values.get(1).cloned(), env)?;
    if super::typed_array_buffer_detached(&object) {
        return Err(array_buffer::detached_error());
    }
    if typed_array_is_out_of_bounds(&object) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: TypedArray is out of bounds".to_owned(),
        });
    }
    let decoded = decode_base64(
        &source,
        options.alphabet,
        options.last_chunk_handling,
        typed_array_length(&object),
    );
    set_view_elements(&object, 0, decoded.bytes.iter().copied().map(number_byte));
    match decoded.error {
        Some(error) => Err(error),
        None => Ok(set_from_base64_result(
            decoded.read,
            decoded.bytes.len(),
            env,
        )),
    }
}

fn base64_source(value: Option<&Value>, method: &str) -> Result<String, RuntimeError> {
    match value {
        Some(Value::String(source)) => Ok(source.clone()),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: {method} requires a string"),
        }),
    }
}

fn uint8_array_receiver(value: &Value, method: &str) -> Result<ObjectRef, RuntimeError> {
    let object = typed_array_receiver(value)?;
    if typed_array_kind(&object) != NativeFunction::Uint8Array {
        return Err(RuntimeError {
            thrown: None,
            message: format!(
                "TypeError: Uint8Array.prototype.{method} requires a Uint8Array receiver"
            ),
        });
    }
    Ok(object)
}

#[derive(Clone, Copy)]
enum Base64Alphabet {
    Base64,
    Base64Url,
}

struct ToBase64Options {
    alphabet: Base64Alphabet,
    omit_padding: bool,
}

fn to_base64_options(
    value: Option<Value>,
    env: &mut CallEnv,
) -> Result<ToBase64Options, RuntimeError> {
    let mut options = ToBase64Options {
        alphabet: Base64Alphabet::Base64,
        omit_padding: false,
    };
    let Some(value) = value else {
        return Ok(options);
    };
    if matches!(value, Value::Undefined) {
        return Ok(options);
    }
    match value {
        Value::Object(_)
        | Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Proxy(_)
        | Value::Set(_) => {
            options.alphabet =
                base64_alphabet_option(property_value(value.clone(), "alphabet", env)?)?;
            let omit_padding = property_value(value, "omitPadding", env)?;
            options.omit_padding = is_truthy(&omit_padding);
            Ok(options)
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: Uint8Array.prototype.toBase64 options must be an object"
                .to_owned(),
        }),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LastChunkHandling {
    Loose,
    Strict,
    StopBeforePartial,
}

struct SetFromBase64Options {
    alphabet: Base64Alphabet,
    last_chunk_handling: LastChunkHandling,
}

fn from_base64_options(
    value: Option<Value>,
    env: &mut CallEnv,
) -> Result<SetFromBase64Options, RuntimeError> {
    base64_decode_options(value, env, "Uint8Array.fromBase64")
}

fn set_from_base64_options(
    value: Option<Value>,
    env: &mut CallEnv,
) -> Result<SetFromBase64Options, RuntimeError> {
    base64_decode_options(value, env, "Uint8Array.prototype.setFromBase64")
}

fn base64_decode_options(
    value: Option<Value>,
    env: &mut CallEnv,
    method: &str,
) -> Result<SetFromBase64Options, RuntimeError> {
    let mut options = SetFromBase64Options {
        alphabet: Base64Alphabet::Base64,
        last_chunk_handling: LastChunkHandling::Loose,
    };
    let Some(value) = value else {
        return Ok(options);
    };
    if matches!(value, Value::Undefined) {
        return Ok(options);
    }
    match value {
        Value::Object(_)
        | Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Proxy(_)
        | Value::Set(_) => {
            options.alphabet =
                base64_alphabet_option(property_value(value.clone(), "alphabet", env)?)?;
            let last_chunk_handling = property_value(value, "lastChunkHandling", env)?;
            match last_chunk_handling {
                Value::Undefined => {}
                Value::String(name) if name == "loose" => {
                    options.last_chunk_handling = LastChunkHandling::Loose;
                }
                Value::String(name) if name == "strict" => {
                    options.last_chunk_handling = LastChunkHandling::Strict;
                }
                Value::String(name) if name == "stop-before-partial" => {
                    options.last_chunk_handling = LastChunkHandling::StopBeforePartial;
                }
                _ => {
                    return Err(RuntimeError {
                        thrown: None,
                        message: "TypeError: invalid lastChunkHandling".to_owned(),
                    });
                }
            }
            Ok(options)
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: {method} options must be an object"),
        }),
    }
}

fn base64_alphabet_option(value: Value) -> Result<Base64Alphabet, RuntimeError> {
    match value {
        Value::Undefined => Ok(Base64Alphabet::Base64),
        Value::String(name) if name == "base64" => Ok(Base64Alphabet::Base64),
        Value::String(name) if name == "base64url" => Ok(Base64Alphabet::Base64Url),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: invalid base64 alphabet".to_owned(),
        }),
    }
}

fn encode_base64(bytes: &[u8], alphabet: Base64Alphabet, omit_padding: bool) -> String {
    let table = match alphabet {
        Base64Alphabet::Base64 => {
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        }
        Base64Alphabet::Base64Url => {
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
        }
    };
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = *chunk.get(1).unwrap_or(&0);
        let third = *chunk.get(2).unwrap_or(&0);
        output.push(table[(first >> 2) as usize] as char);
        output.push(table[(((first & 0b0000_0011) << 4) | (second >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(table[(((second & 0b0000_1111) << 2) | (third >> 6)) as usize] as char);
        } else if !omit_padding {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(table[(third & 0b0011_1111) as usize] as char);
        } else if !omit_padding {
            output.push('=');
        }
    }
    output
}

struct Base64Token {
    ch: char,
    read_after: usize,
}

struct DecodedBase64 {
    read: usize,
    bytes: Vec<u8>,
    error: Option<RuntimeError>,
}

fn decode_base64(
    source: &str,
    alphabet: Base64Alphabet,
    last_chunk_handling: LastChunkHandling,
    max_len: usize,
) -> DecodedBase64 {
    if max_len == 0 {
        return DecodedBase64 {
            read: 0,
            bytes: Vec::new(),
            error: None,
        };
    }
    let tokens: Vec<Base64Token> = source
        .chars()
        .enumerate()
        .filter_map(|(index, ch)| {
            if is_base64_ascii_whitespace(ch) {
                None
            } else {
                Some(Base64Token {
                    ch,
                    read_after: index + 1,
                })
            }
        })
        .collect();
    let mut bytes = Vec::new();
    let mut index = 0usize;
    let mut read = 0usize;
    while index < tokens.len() {
        let remaining = tokens.len() - index;
        if remaining < 4 {
            if last_chunk_handling == LastChunkHandling::StopBeforePartial {
                if let Err(error) = validate_stop_before_partial_tail(&tokens[index..], alphabet) {
                    return DecodedBase64 {
                        read,
                        bytes,
                        error: Some(error),
                    };
                }
                return DecodedBase64 {
                    read,
                    bytes,
                    error: None,
                };
            }
            match decode_base64_partial(
                &tokens[index..],
                alphabet,
                last_chunk_handling,
                max_len.saturating_sub(bytes.len()),
            ) {
                Ok((chunk, chunk_read)) => {
                    bytes.extend(chunk);
                    read = chunk_read;
                    return DecodedBase64 {
                        read,
                        bytes,
                        error: None,
                    };
                }
                Err(error) => {
                    return DecodedBase64 {
                        read,
                        bytes,
                        error: Some(error),
                    };
                }
            }
        }

        let is_last_quad = remaining == 4;
        let quad = &tokens[index..index + 4];
        let decoded = match decode_base64_quad(quad, alphabet, last_chunk_handling) {
            Ok(decoded) => decoded,
            Err(error) => {
                return DecodedBase64 {
                    read,
                    bytes,
                    error: Some(error),
                };
            }
        };
        if decoded.padding && !is_last_quad {
            return DecodedBase64 {
                read,
                bytes,
                error: Some(syntax_error("unexpected base64 data after padding")),
            };
        }
        if bytes.len() + decoded.bytes.len() > max_len {
            return DecodedBase64 {
                read,
                bytes,
                error: None,
            };
        }
        bytes.extend(decoded.bytes);
        read = quad[3].read_after;
        index += 4;
        if bytes.len() == max_len {
            return DecodedBase64 {
                read,
                bytes,
                error: None,
            };
        }
    }
    DecodedBase64 {
        read,
        bytes,
        error: None,
    }
}

fn validate_stop_before_partial_tail(
    tokens: &[Base64Token],
    alphabet: Base64Alphabet,
) -> Result<(), RuntimeError> {
    if tokens
        .iter()
        .all(|token| token.ch != '=' && base64_value(token.ch, alphabet).is_some())
    {
        return Ok(());
    }
    if tokens.len() == 3
        && base64_value(tokens[0].ch, alphabet).is_some()
        && base64_value(tokens[1].ch, alphabet).is_some()
        && tokens[2].ch == '='
    {
        return Ok(());
    }
    Err(syntax_error("invalid base64 final chunk"))
}

struct DecodedBase64Quad {
    bytes: Vec<u8>,
    padding: bool,
}

fn decode_base64_quad(
    quad: &[Base64Token],
    alphabet: Base64Alphabet,
    last_chunk_handling: LastChunkHandling,
) -> Result<DecodedBase64Quad, RuntimeError> {
    let first = base64_value(quad[0].ch, alphabet)
        .ok_or_else(|| syntax_error("invalid base64 character"))?;
    let second = base64_value(quad[1].ch, alphabet)
        .ok_or_else(|| syntax_error("invalid base64 character"))?;
    match (quad[2].ch, quad[3].ch) {
        ('=', '=') => {
            if last_chunk_handling == LastChunkHandling::Strict && (second & 0b0000_1111) != 0 {
                return Err(syntax_error("non-zero base64 padding bits"));
            }
            Ok(DecodedBase64Quad {
                bytes: vec![(first << 2) | (second >> 4)],
                padding: true,
            })
        }
        ('=', _) => Err(syntax_error("invalid base64 padding")),
        (_, '=') => {
            let third = base64_value(quad[2].ch, alphabet)
                .ok_or_else(|| syntax_error("invalid base64 character"))?;
            if last_chunk_handling == LastChunkHandling::Strict && (third & 0b0000_0011) != 0 {
                return Err(syntax_error("non-zero base64 padding bits"));
            }
            Ok(DecodedBase64Quad {
                bytes: vec![
                    (first << 2) | (second >> 4),
                    ((second & 0b0000_1111) << 4) | (third >> 2),
                ],
                padding: true,
            })
        }
        (_, _) => {
            let third = base64_value(quad[2].ch, alphabet)
                .ok_or_else(|| syntax_error("invalid base64 character"))?;
            let fourth = base64_value(quad[3].ch, alphabet)
                .ok_or_else(|| syntax_error("invalid base64 character"))?;
            Ok(DecodedBase64Quad {
                bytes: vec![
                    (first << 2) | (second >> 4),
                    ((second & 0b0000_1111) << 4) | (third >> 2),
                    ((third & 0b0000_0011) << 6) | fourth,
                ],
                padding: false,
            })
        }
    }
}

fn decode_base64_partial(
    tokens: &[Base64Token],
    alphabet: Base64Alphabet,
    last_chunk_handling: LastChunkHandling,
    capacity: usize,
) -> Result<(Vec<u8>, usize), RuntimeError> {
    if last_chunk_handling == LastChunkHandling::Strict {
        return Err(syntax_error("base64 padding required"));
    }
    match tokens.len() {
        1 => Err(syntax_error("invalid base64 final chunk")),
        2 => {
            let first = base64_value(tokens[0].ch, alphabet)
                .ok_or_else(|| syntax_error("invalid base64 character"))?;
            let second = base64_value(tokens[1].ch, alphabet)
                .ok_or_else(|| syntax_error("invalid base64 character"))?;
            if capacity < 1 {
                return Ok((Vec::new(), 0));
            }
            Ok((vec![(first << 2) | (second >> 4)], tokens[1].read_after))
        }
        3 => {
            let first = base64_value(tokens[0].ch, alphabet)
                .ok_or_else(|| syntax_error("invalid base64 character"))?;
            let second = base64_value(tokens[1].ch, alphabet)
                .ok_or_else(|| syntax_error("invalid base64 character"))?;
            let third = base64_value(tokens[2].ch, alphabet)
                .ok_or_else(|| syntax_error("invalid base64 character"))?;
            if capacity < 2 {
                return Ok((Vec::new(), 0));
            }
            Ok((
                vec![
                    (first << 2) | (second >> 4),
                    ((second & 0b0000_1111) << 4) | (third >> 2),
                ],
                tokens[2].read_after,
            ))
        }
        _ => Ok((Vec::new(), 0)),
    }
}

fn base64_value(ch: char, alphabet: Base64Alphabet) -> Option<u8> {
    match ch {
        'A'..='Z' => Some(ch as u8 - b'A'),
        'a'..='z' => Some(ch as u8 - b'a' + 26),
        '0'..='9' => Some(ch as u8 - b'0' + 52),
        '+' if matches!(alphabet, Base64Alphabet::Base64) => Some(62),
        '/' if matches!(alphabet, Base64Alphabet::Base64) => Some(63),
        '-' if matches!(alphabet, Base64Alphabet::Base64Url) => Some(62),
        '_' if matches!(alphabet, Base64Alphabet::Base64Url) => Some(63),
        _ => None,
    }
}

fn is_base64_ascii_whitespace(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\x0C' | '\r' | ' ')
}

fn number_byte(value: u8) -> Value {
    Value::Number(value as f64)
}

fn set_from_base64_result(read: usize, written: usize, env: &CallEnv) -> Value {
    let result = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));
    result.define_property(
        "read".to_owned(),
        Property::enumerable(Value::Number(read as f64)),
    );
    result.define_property(
        "written".to_owned(),
        Property::enumerable(Value::Number(written as f64)),
    );
    Value::Object(result)
}

fn syntax_error(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("SyntaxError: {message}"),
    }
}
