use crate::{RuntimeError, Value, string::string_from_code_unit};

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
