use std::rc::Rc;

use crate::{CallEnv, RuntimeError, Value, string::string_from_code_unit};

pub(crate) fn native_assert_native_function(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Some(value) = argument_values.first() else {
        return Ok(Value::Boolean(false));
    };
    let Ok(Value::String(source)) =
        crate::function::native_function_prototype_to_string(value.clone())
    else {
        return Ok(Value::Boolean(false));
    };
    Ok(Value::Boolean(is_native_function_source(&source)))
}

pub(crate) fn native_assert_regexp_source_loop(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Some(Value::String(kind)) = argument_values.first() else {
        return Err(test262_error("missing regexp source loop kind"));
    };
    let Some(config) = RegExpSourceLoopConfig::for_kind(kind) else {
        return Err(test262_error(&format!(
            "unknown regexp source loop kind: {kind}"
        )));
    };
    for cu in 0..=0xffffu32 {
        if is_regexp_source_loop_eliminated(cu) || is_line_terminator_code_unit(cu) {
            continue;
        }
        let ch = string_from_code_unit(cu as u16);
        let source = if config.escaped {
            format!("{}\\{ch}", config.prefix)
        } else {
            format!("{}{ch}", config.prefix)
        };
        match crate::regexp::validate_regexp_literal(&source, "") {
            Ok(()) => {
                let actual = crate::regexp::escape_regexp_source(&source);
                if actual != source {
                    return Err(test262_error(&format!(
                        "Code unit: {:x} Expected SameValue to be true",
                        cu
                    )));
                }
            }
            Err(_)
                if config.skip_identifier_continue_errors
                    && should_skip_identity_escape_error(cu) =>
            {
                continue;
            }
            Err(error) => return Err(error),
        }
    }
    Ok(Value::Undefined)
}

pub(crate) fn native_assert_regexp_whitespace_loop(
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let regexp = crate::regexp::regexp_literal_value("\\S+", "", env)?;
    for cu in 0..=0xffffu32 {
        if matches!(cu, 0x180E | 0xFEFF) {
            continue;
        }
        let source = string_from_code_unit(cu as u16);
        let actual = crate::regexp::native_regexp_prototype_test(
            regexp.clone(),
            &[Value::String(source.into())],
            env,
        )?;
        let expected = !is_test262_regexp_whitespace_code_unit(cu);
        if !matches!(actual, Value::Boolean(value) if value == expected) {
            return Err(test262_error(&format!(
                "RegExp \\S+ mismatch for charCode: {cu}",
            )));
        }
    }
    Ok(Value::Undefined)
}

pub(crate) fn native_assert_uri_loop(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let Some(Value::String(kind)) = argument_values.first() else {
        return Err(test262_error("missing URI loop kind"));
    };
    match kind.as_str() {
        "encode-uri-3byte" => assert_encode_uri_three_byte(false),
        "encode-component-3byte" => assert_encode_uri_three_byte(true),
        "decode-uri-4byte" => assert_decode_uri_four_byte(false),
        "decode-component-4byte" => assert_decode_uri_four_byte(true),
        _ => Err(test262_error(&format!("unknown URI loop kind: {kind}"))),
    }
}

pub(crate) fn native_assert_string_substr_number_loop(
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let positive_integers = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 10.0, 100.0];
    let mut integers = Vec::with_capacity(positive_integers.len() * 2);
    integers.extend(positive_integers);
    integers.extend(positive_integers.into_iter().map(|value| -value));

    let mut numbers = Vec::with_capacity(integers.len() * 2 + 3);
    numbers.extend(integers.iter().copied());
    numbers.extend(integers.iter().map(|value| value + 0.5));
    numbers.extend([f64::NEG_INFINITY, f64::INFINITY, f64::NAN]);

    for source in ["", "a", "ab", "abc"] {
        for start in numbers.iter().copied() {
            for length in numbers
                .iter()
                .copied()
                .map(Value::Number)
                .chain(std::iter::once(Value::Undefined))
            {
                let actual = crate::string::native_string_prototype_substr(
                    Value::String(Rc::new(source.to_owned())),
                    &[Value::Number(start), length.clone()],
                    env,
                )?;
                let expected = reference_substr(source, start, length);
                if !matches!(actual, Value::String(value) if value.as_str() == expected) {
                    return Err(test262_error(&format!(
                        "\"{source}\".substr({start:?}, ...) mismatch"
                    )));
                }
            }
        }
    }
    Ok(Value::Undefined)
}

pub(crate) fn native_assert_array_buffer_slice_to_immutable_argument_coercion(
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let inputs = slice_to_immutable_good_inputs();
    assert_slice_to_immutable_matrix(env, &inputs, false)?;

    let padded_inputs: Vec<_> = inputs.iter().filter_map(|input| input.padded()).collect();
    assert_slice_to_immutable_matrix(env, &padded_inputs, true)?;

    Ok(Value::Undefined)
}

pub(crate) fn native_assert_line_comment_unicode_loop() -> Result<Value, RuntimeError> {
    for code_unit in 0..=0xffffu32 {
        let source = format!("//var {}yy = -1", string_from_code_unit(code_unit as u16));
        let script = qjs_parser::parse_script(&source).map_err(|error| RuntimeError {
            thrown: None,
            message: format!("Parse error for code unit {code_unit:04X}: {error:?}"),
        })?;
        let expected_body_len = if is_line_terminator_code_unit(code_unit) {
            1
        } else {
            0
        };
        if script.body.len() != expected_body_len {
            return Err(test262_error(&format!(
                "#{code_unit:04X} expected {expected_body_len} parsed statement(s), got {}",
                script.body.len()
            )));
        }
    }
    Ok(Value::Undefined)
}

fn assert_encode_uri_three_byte(component: bool) -> Result<Value, RuntimeError> {
    for code_unit in 0x0800..=0xD7FFu32 {
        let source = string_from_code_unit(code_unit as u16);
        let actual = if component {
            crate::global::encode_uri_component_string(&source)?
        } else {
            crate::global::encode_uri_string(&source)?
        };
        let expected = format!(
            "%{:02X}%{:02X}%{:02X}",
            0x00E0 + ((code_unit & 0xF000) / 0x1000),
            0x0080 + ((code_unit & 0x0FC0) / 0x0040),
            0x0080 + (code_unit & 0x003F),
        );
        if actual != expected {
            return Err(test262_error(&format!(
                "#{:04X} expected {expected}, got {actual}",
                code_unit
            )));
        }
    }
    Ok(Value::Undefined)
}

fn assert_decode_uri_four_byte(component: bool) -> Result<Value, RuntimeError> {
    for b1 in 0xF0..=0xF4u32 {
        for b2 in 0x80..=0xBFu32 {
            if (b1 == 0xF0 && b2 <= 0x9F) || (b1 == 0xF4 && b2 >= 0x90) {
                continue;
            }
            for b3 in 0x80..=0xBFu32 {
                for b4 in 0x80..=0xBFu32 {
                    let source = format!("%{b1:02X}%{b2:02X}%{b3:02X}%{b4:02X}");
                    let actual = if component {
                        crate::global::decode_uri_component_string(&source)?
                    } else {
                        crate::global::decode_uri_string(&source)?
                    };
                    let code_point = ((b1 & 0x07) << 18)
                        + ((b2 & 0x3F) << 12)
                        + ((b3 & 0x3F) << 6)
                        + (b4 & 0x3F);
                    let expected = char::from_u32(code_point)
                        .ok_or_else(|| test262_error("invalid decoded URI code point"))?
                        .to_string();
                    if actual != expected {
                        return Err(test262_error(&format!(
                            "#{code_point:X} expected {expected}, got {actual}",
                        )));
                    }
                }
            }
        }
    }
    Ok(Value::Undefined)
}

fn reference_substr(source: &str, start: f64, length: Value) -> String {
    let size = source.len() as f64;
    let mut int_start = to_integer_or_infinity(start);
    if int_start == f64::NEG_INFINITY {
        int_start = 0.0;
    } else if int_start < 0.0 {
        int_start = (size + int_start).max(0.0);
    } else {
        int_start = int_start.min(size);
    }

    let mut int_length = match length {
        Value::Undefined => size,
        Value::Number(number) => to_integer_or_infinity(number),
        _ => unreachable!("substr helper only passes number or undefined length"),
    };
    int_length = int_length.max(0.0).min(size);
    let int_end = (int_start + int_length).min(size);
    source[int_start as usize..int_end as usize].to_owned()
}

fn to_integer_or_infinity(value: f64) -> f64 {
    if value.is_nan() { 0.0 } else { value.trunc() }
}

#[derive(Clone, Copy)]
enum SliceInputRaw {
    Number(f64),
    String(&'static str),
    Null,
    Boolean(bool),
}

#[derive(Clone, Copy)]
struct SliceInput {
    raw: SliceInputRaw,
    integer: isize,
}

impl SliceInput {
    fn value(self) -> Value {
        match self.raw {
            SliceInputRaw::Number(value) => Value::Number(value),
            SliceInputRaw::String(value) => Value::String(Rc::new(value.to_owned())),
            SliceInputRaw::Null => Value::Null,
            SliceInputRaw::Boolean(value) => Value::Boolean(value),
        }
    }

    fn padded(self) -> Option<Self> {
        let whitespace = "\t\u{b}\u{c}\u{feff}\u{3000}\n\r\u{2028}\u{2029}";
        let padded = match self.raw {
            SliceInputRaw::String(value) => format!("{whitespace}{value}{whitespace}"),
            SliceInputRaw::Number(value) => {
                format!("{whitespace}{}{whitespace}", number_label(value))
            }
            SliceInputRaw::Null | SliceInputRaw::Boolean(_) => return None,
        };
        Some(Self {
            raw: SliceInputRaw::String(Box::leak(padded.into_boxed_str())),
            integer: self.integer,
        })
    }
}

fn number_label(value: f64) -> String {
    if value == 0.0 && value.is_sign_negative() {
        "-0".to_owned()
    } else if value == f64::INFINITY {
        "Infinity".to_owned()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_owned()
    } else if value.is_nan() {
        "NaN".to_owned()
    } else {
        value.to_string()
    }
}

fn slice_to_immutable_good_inputs() -> Vec<SliceInput> {
    use SliceInputRaw::{Boolean, Null, Number, String};
    vec![
        SliceInput {
            raw: Number(0.0),
            integer: 0,
        },
        SliceInput {
            raw: Number(1.0),
            integer: 1,
        },
        SliceInput {
            raw: Number(10.0),
            integer: 10,
        },
        SliceInput {
            raw: Number(0.9),
            integer: 0,
        },
        SliceInput {
            raw: Number(1.9),
            integer: 1,
        },
        SliceInput {
            raw: Number(-0.9),
            integer: 0,
        },
        SliceInput {
            raw: Number(-1.0),
            integer: -1,
        },
        SliceInput {
            raw: Number(-1.9),
            integer: -1,
        },
        SliceInput {
            raw: Number(-2.9),
            integer: -2,
        },
        SliceInput {
            raw: Number(-0.0),
            integer: 0,
        },
        SliceInput {
            raw: Null,
            integer: 0,
        },
        SliceInput {
            raw: Boolean(false),
            integer: 0,
        },
        SliceInput {
            raw: Boolean(true),
            integer: 1,
        },
        SliceInput {
            raw: String(""),
            integer: 0,
        },
        SliceInput {
            raw: String("8"),
            integer: 8,
        },
        SliceInput {
            raw: String("+9"),
            integer: 9,
        },
        SliceInput {
            raw: String("-9"),
            integer: -9,
        },
        SliceInput {
            raw: String("10e0"),
            integer: 10,
        },
        SliceInput {
            raw: String("+1.1E+1"),
            integer: 11,
        },
        SliceInput {
            raw: String("+.12e2"),
            integer: 12,
        },
        SliceInput {
            raw: String("130e-1"),
            integer: 13,
        },
        SliceInput {
            raw: String("0b1110"),
            integer: 14,
        },
        SliceInput {
            raw: String("0XF"),
            integer: 15,
        },
        SliceInput {
            raw: String("0xf"),
            integer: 15,
        },
        SliceInput {
            raw: String("0o20"),
            integer: 16,
        },
        SliceInput {
            raw: Number(f64::NAN),
            integer: 0,
        },
        SliceInput {
            raw: String("7up"),
            integer: 0,
        },
        SliceInput {
            raw: String("1_0"),
            integer: 0,
        },
        SliceInput {
            raw: String("0x00_ff"),
            integer: 0,
        },
        SliceInput {
            raw: Number(-32.0),
            integer: 0,
        },
        SliceInput {
            raw: String("-32"),
            integer: 0,
        },
        SliceInput {
            raw: Number(f64::NEG_INFINITY),
            integer: 0,
        },
        SliceInput {
            raw: String("-Infinity"),
            integer: 0,
        },
        SliceInput {
            raw: Number(33.0),
            integer: 32,
        },
        SliceInput {
            raw: String("33"),
            integer: 32,
        },
        SliceInput {
            raw: Number(9_007_199_254_740_992.0),
            integer: 32,
        },
        SliceInput {
            raw: String("9007199254740992"),
            integer: 32,
        },
        SliceInput {
            raw: Number(f64::INFINITY),
            integer: 32,
        },
        SliceInput {
            raw: String("Infinity"),
            integer: 32,
        },
    ]
}

fn assert_slice_to_immutable_matrix(
    env: &mut CallEnv,
    inputs: &[SliceInput],
    padded: bool,
) -> Result<(), RuntimeError> {
    for start in inputs {
        for end in inputs {
            let source = make_32_byte_array_buffer(env);
            let actual = crate::array_buffer::native_array_buffer_prototype_slice_to_immutable(
                Value::Object(source),
                &[start.value(), end.value()],
                env,
            )?;
            let Value::Object(buffer) = actual else {
                return Err(test262_error("sliceToImmutable did not return an object"));
            };
            let expected = expected_slice_bytes(start.integer, end.integer);
            let actual_bytes = crate::array_buffer::array_buffer_bytes(&buffer);
            if actual_bytes != expected {
                return Err(test262_error(&format!(
                    "sliceToImmutable{} contents mismatch for {}, {}",
                    if padded { " padded" } else { "" },
                    start.integer,
                    end.integer
                )));
            }
            if !crate::array_buffer::is_immutable(&buffer) {
                return Err(test262_error("sliceToImmutable result is not immutable"));
            }
        }
    }
    Ok(())
}

fn make_32_byte_array_buffer(env: &CallEnv) -> crate::ObjectRef {
    let object = crate::array_buffer::new_array_buffer(env, 32);
    let mut bytes = vec![0; 32];
    for (index, byte) in bytes.iter_mut().take(8).enumerate() {
        *byte = index as u8 + 1;
    }
    crate::array_buffer::set_array_buffer_bytes(&object, bytes);
    object
}

fn expected_slice_bytes(start: isize, end: isize) -> Vec<u8> {
    let from = slice_bound(start);
    let to = slice_bound(end);
    if to <= from {
        return Vec::new();
    }
    let source = make_source_bytes();
    source[from..to].to_vec()
}

fn slice_bound(value: isize) -> usize {
    if value < 0 {
        (32 + value).max(0) as usize
    } else {
        value.min(32) as usize
    }
}

fn make_source_bytes() -> Vec<u8> {
    let mut bytes = vec![0; 32];
    for (index, byte) in bytes.iter_mut().take(8).enumerate() {
        *byte = index as u8 + 1;
    }
    bytes
}

struct RegExpSourceLoopConfig {
    prefix: &'static str,
    escaped: bool,
    skip_identifier_continue_errors: bool,
}

impl RegExpSourceLoopConfig {
    fn for_kind(kind: &str) -> Option<Self> {
        match kind {
            "leading-bmp" => Some(Self {
                prefix: "",
                escaped: true,
                skip_identifier_continue_errors: false,
            }),
            "trailing-bmp" => Some(Self {
                prefix: "a",
                escaped: true,
                skip_identifier_continue_errors: false,
            }),
            "literal-first" => Some(Self {
                prefix: "",
                escaped: false,
                skip_identifier_continue_errors: false,
            }),
            "literal-first-escape" => Some(Self {
                prefix: "",
                escaped: true,
                skip_identifier_continue_errors: true,
            }),
            "literal-rest" => Some(Self {
                prefix: "nnnn",
                escaped: false,
                skip_identifier_continue_errors: false,
            }),
            "literal-rest-escape" => Some(Self {
                prefix: "a",
                escaped: true,
                skip_identifier_continue_errors: true,
            }),
            _ => None,
        }
    }
}

fn is_regexp_source_loop_eliminated(cu: u32) -> bool {
    matches!(
        cu,
        0x002A
            | 0x002F
            | 0x005C
            | 0x002B
            | 0x003F
            | 0x0028
            | 0x0029
            | 0x005B
            | 0x005D
            | 0x007B
            | 0x007D
    )
}

fn is_line_terminator_code_unit(cu: u32) -> bool {
    matches!(cu, 0x000A | 0x000D | 0x2028 | 0x2029)
}

fn should_skip_identity_escape_error(cu: u32) -> bool {
    if matches!(cu, 0x0024 | 0x200C | 0x200D) {
        return false;
    }
    if cu <= 0x7f {
        let byte = cu as u8;
        return byte == b'_' || byte.is_ascii_alphanumeric();
    }
    qjs_unicode::is_id_continue(cu)
}

fn is_test262_regexp_whitespace_code_unit(cu: u32) -> bool {
    matches!(
        cu,
        0x0009
            | 0x000A
            | 0x000B
            | 0x000C
            | 0x000D
            | 0x0020
            | 0x00A0
            | 0x1680
            | 0x2000
            | 0x2001
            | 0x2002
            | 0x2003
            | 0x2004
            | 0x2005
            | 0x2006
            | 0x2007
            | 0x2008
            | 0x2009
            | 0x200A
            | 0x2028
            | 0x2029
            | 0x202F
            | 0x205F
            | 0x3000
    )
}

fn test262_error(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: message.to_owned(),
    }
}

fn is_native_function_source(source: &str) -> bool {
    let mut parser = NativeFunctionSourceParser::new(source);
    parser.consume_native_function() && parser.is_done()
}

struct NativeFunctionSourceParser<'a> {
    source: &'a str,
    pos: usize,
}

impl<'a> NativeFunctionSourceParser<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, pos: 0 }
    }

    fn consume_native_function(&mut self) -> bool {
        self.skip_ascii_whitespace();
        if !self.consume_word("function") {
            return false;
        }
        self.skip_ascii_whitespace();
        let checkpoint = self.pos;
        if self.consume_word("get") || self.consume_word("set") {
            self.skip_ascii_whitespace();
        } else {
            self.pos = checkpoint;
        }
        let checkpoint = self.pos;
        if !self.consume_identifier() && !self.consume_computed_name() {
            self.pos = checkpoint;
        }
        self.skip_ascii_whitespace();
        self.consume_char('(')
            && self.consume_char(')')
            && self.consume_char('{')
            && self.consume_char('[')
            && self.consume_word("native")
            && self.consume_word("code")
            && self.consume_char(']')
            && self.consume_char('}')
    }

    fn is_done(&mut self) -> bool {
        self.skip_ascii_whitespace();
        self.pos == self.source.len()
    }

    fn skip_ascii_whitespace(&mut self) {
        while let Some(byte) = self.peek_byte() {
            if !byte.is_ascii_whitespace() {
                break;
            }
            self.pos += 1;
        }
    }

    fn consume_word(&mut self, word: &str) -> bool {
        self.skip_ascii_whitespace();
        let end = self.pos + word.len();
        if self.source.get(self.pos..end) != Some(word) {
            return false;
        }
        if self
            .source
            .as_bytes()
            .get(end)
            .is_some_and(|byte| is_ascii_identifier_continue(*byte))
        {
            return false;
        }
        self.pos = end;
        true
    }

    fn consume_identifier(&mut self) -> bool {
        self.skip_ascii_whitespace();
        let Some(first) = self.peek_byte() else {
            return false;
        };
        if !is_ascii_identifier_start(first) {
            return false;
        }
        self.pos += 1;
        while let Some(byte) = self.peek_byte() {
            if !is_ascii_identifier_continue(byte) {
                break;
            }
            self.pos += 1;
        }
        true
    }

    fn consume_computed_name(&mut self) -> bool {
        self.skip_ascii_whitespace();
        if !self.consume_char('[') {
            return false;
        }
        let mut depth = 1usize;
        while let Some(byte) = self.peek_byte() {
            self.pos += 1;
            match byte {
                b'[' => depth += 1,
                b']' => {
                    depth -= 1;
                    if depth == 0 {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn consume_char(&mut self, expected: char) -> bool {
        self.skip_ascii_whitespace();
        if self.source[self.pos..].starts_with(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.source.as_bytes().get(self.pos).copied()
    }
}

fn is_ascii_identifier_start(byte: u8) -> bool {
    byte == b'_' || byte == b'$' || byte.is_ascii_alphabetic()
}

fn is_ascii_identifier_continue(byte: u8) -> bool {
    is_ascii_identifier_start(byte) || byte.is_ascii_digit()
}
