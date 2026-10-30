//! URI code-unit conversion and validation; no property or callback state.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum UriEncodeKind {
    Uri,
    Component,
}

#[derive(Clone, Copy)]
pub(super) enum UriDecodeKind {
    Uri,
    Component,
}

pub(super) fn encode_uri(source: &str, kind: UriEncodeKind) -> Result<String, RuntimeError> {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(character) = chars.next() {
        let Some(code_unit) = surrogate_escape_code_unit(character) else {
            encode_uri_code_point(&mut output, character as u32, kind)?;
            continue;
        };
        let code_point = if is_high_surrogate(code_unit) {
            let Some(low) = chars.next().and_then(surrogate_escape_code_unit) else {
                return malformed_uri();
            };
            if !is_low_surrogate(low) {
                return malformed_uri();
            }
            0x10000 + ((u32::from(code_unit) - 0xD800) << 10) + u32::from(low) - 0xDC00
        } else if is_low_surrogate(code_unit) {
            return malformed_uri();
        } else {
            u32::from(code_unit)
        };
        encode_uri_code_point(&mut output, code_point, kind)?;
    }
    Ok(output)
}

fn encode_uri_code_point(
    output: &mut String,
    code_point: u32,
    kind: UriEncodeKind,
) -> Result<(), RuntimeError> {
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
    Ok(())
}

pub(super) fn decode_uri(source: &str, kind: UriDecodeKind) -> Result<String, RuntimeError> {
    if !source.contains('%') {
        return Ok(source.to_owned());
    }
    if source.is_ascii() {
        return decode_ascii_uri(source, kind);
    }

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
            for character in decoded.chars() {
                push_code_point(&mut output, character as u32);
            }
        }
    }
    Ok(output)
}

fn decode_ascii_uri(source: &str, kind: UriDecodeKind) -> Result<String, RuntimeError> {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            output.push(char::from(bytes[index]));
            index += 1;
            continue;
        }

        let escape_start = index;
        let first_byte = ascii_percent_byte(bytes, index)?;
        index += 3;

        let expected_len = utf8_sequence_len(first_byte)?;
        let mut decoded_bytes = [0u8; 4];
        decoded_bytes[0] = first_byte;
        for slot in decoded_bytes.iter_mut().take(expected_len).skip(1) {
            if index >= bytes.len() || bytes[index] != b'%' {
                return malformed_uri();
            }
            *slot = ascii_percent_byte(bytes, index)?;
            index += 3;
        }

        let decoded =
            std::str::from_utf8(&decoded_bytes[..expected_len]).map_err(|_| uri_error())?;
        if matches!(kind, UriDecodeKind::Uri) && decoded.chars().all(is_uri_reserved) {
            crate::string::push_js_str(&mut output, &source[escape_start..index]);
        } else {
            for character in decoded.chars() {
                push_code_point(&mut output, character as u32);
            }
        }
    }
    Ok(output)
}

fn ascii_percent_byte(bytes: &[u8], index: usize) -> Result<u8, RuntimeError> {
    let Some(high) = bytes.get(index + 1).and_then(|byte| ascii_hex_digit(*byte)) else {
        return malformed_uri();
    };
    let Some(low) = bytes.get(index + 2).and_then(|byte| ascii_hex_digit(*byte)) else {
        return malformed_uri();
    };
    Ok((high << 4) | low)
}

fn ascii_hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
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
