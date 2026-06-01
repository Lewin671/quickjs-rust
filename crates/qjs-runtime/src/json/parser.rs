use std::collections::HashMap;

use crate::{ArrayRef, ObjectRef, RuntimeError, Value, object_prototype, to_js_string};

pub(crate) fn native_json_parse(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let source = to_js_string(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    JsonParser::new(&source, env).parse()
}

struct JsonParser<'a> {
    source: &'a str,
    cursor: usize,
    env: &'a HashMap<String, Value>,
}

impl<'a> JsonParser<'a> {
    fn new(source: &'a str, env: &'a HashMap<String, Value>) -> Self {
        Self {
            source,
            cursor: 0,
            env,
        }
    }

    fn parse(mut self) -> Result<Value, RuntimeError> {
        self.skip_whitespace();
        let value = self.value()?;
        self.skip_whitespace();
        if self.cursor == self.source.len() {
            Ok(value)
        } else {
            Err(self.syntax_error())
        }
    }

    fn value(&mut self) -> Result<Value, RuntimeError> {
        self.skip_whitespace();
        match self.peek() {
            Some('"') => self.string().map(Value::String),
            Some('[') => self.array(),
            Some('{') => self.object(),
            Some('t') => self.literal("true", Value::Boolean(true)),
            Some('f') => self.literal("false", Value::Boolean(false)),
            Some('n') => self.literal("null", Value::Null),
            Some('-' | '0'..='9') => self.number().map(Value::Number),
            _ => Err(self.syntax_error()),
        }
    }

    fn array(&mut self) -> Result<Value, RuntimeError> {
        self.expect_char('[')?;
        let mut elements = Vec::new();
        self.skip_whitespace();
        if self.consume_char(']') {
            return Ok(Value::Array(ArrayRef::new(elements)));
        }

        loop {
            elements.push(self.value()?);
            self.skip_whitespace();
            if self.consume_char(']') {
                break;
            }
            self.expect_char(',')?;
        }
        Ok(Value::Array(ArrayRef::new(elements)))
    }

    fn object(&mut self) -> Result<Value, RuntimeError> {
        self.expect_char('{')?;
        let object = ObjectRef::with_prototype(HashMap::new(), object_prototype(self.env));
        self.skip_whitespace();
        if self.consume_char('}') {
            return Ok(Value::Object(object));
        }

        loop {
            self.skip_whitespace();
            if self.peek() != Some('"') {
                return Err(self.syntax_error());
            }
            let key = self.string()?;
            self.skip_whitespace();
            self.expect_char(':')?;
            let value = self.value()?;
            object.set(key, value);
            self.skip_whitespace();
            if self.consume_char('}') {
                break;
            }
            self.expect_char(',')?;
        }
        Ok(Value::Object(object))
    }

    fn string(&mut self) -> Result<String, RuntimeError> {
        self.expect_char('"')?;
        let mut output = String::new();
        loop {
            let Some(ch) = self.next_char() else {
                return Err(self.syntax_error());
            };
            match ch {
                '"' => return Ok(output),
                '\\' => output.push(self.escape()?),
                ch if ch <= '\u{1f}' => return Err(self.syntax_error()),
                ch => output.push(ch),
            }
        }
    }

    fn escape(&mut self) -> Result<char, RuntimeError> {
        let Some(ch) = self.next_char() else {
            return Err(self.syntax_error());
        };
        match ch {
            '"' | '\\' | '/' => Ok(ch),
            'b' => Ok('\u{08}'),
            'f' => Ok('\u{0c}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            'u' => self.unicode_escape(),
            _ => Err(self.syntax_error()),
        }
    }

    fn unicode_escape(&mut self) -> Result<char, RuntimeError> {
        let mut value = 0u32;
        for _ in 0..4 {
            let Some(ch) = self.next_char() else {
                return Err(self.syntax_error());
            };
            let Some(digit) = ch.to_digit(16) else {
                return Err(self.syntax_error());
            };
            value = value * 16 + digit;
        }
        char::from_u32(value).ok_or_else(|| self.syntax_error())
    }

    fn number(&mut self) -> Result<f64, RuntimeError> {
        let start = self.cursor;
        self.consume_char('-');
        match self.peek() {
            Some('0') => {
                self.next_char();
                if matches!(self.peek(), Some('0'..='9')) {
                    return Err(self.syntax_error());
                }
            }
            Some('1'..='9') => {
                self.next_char();
                while matches!(self.peek(), Some('0'..='9')) {
                    self.next_char();
                }
            }
            _ => return Err(self.syntax_error()),
        }

        if self.consume_char('.') {
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(self.syntax_error());
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.next_char();
            }
        }

        if matches!(self.peek(), Some('e' | 'E')) {
            self.next_char();
            if matches!(self.peek(), Some('+' | '-')) {
                self.next_char();
            }
            if !matches!(self.peek(), Some('0'..='9')) {
                return Err(self.syntax_error());
            }
            while matches!(self.peek(), Some('0'..='9')) {
                self.next_char();
            }
        }

        self.source[start..self.cursor]
            .parse::<f64>()
            .map_err(|_| self.syntax_error())
    }

    fn literal(&mut self, literal: &str, value: Value) -> Result<Value, RuntimeError> {
        if self.source[self.cursor..].starts_with(literal) {
            self.cursor += literal.len();
            Ok(value)
        } else {
            Err(self.syntax_error())
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), RuntimeError> {
        if self.consume_char(expected) {
            Ok(())
        } else {
            Err(self.syntax_error())
        }
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.next_char();
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some('\t' | '\n' | '\r' | ' ')) {
            self.next_char();
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.cursor..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }

    fn syntax_error(&self) -> RuntimeError {
        RuntimeError {
            thrown: None,
            message: "JSON syntax error".to_owned(),
        }
    }
}
