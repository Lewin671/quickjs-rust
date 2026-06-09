use qjs_ast::Span;

use crate::{LexError, TemplateSegment, Token, TokenKind};

use super::{
    Lexer, TemplateState, char_class::is_identifier_continue, keywords::identifier_or_keyword,
};

const SURROGATE_ESCAPE_SENTINEL_BASE: u32 = 0xF0000;

impl Lexer<'_> {
    pub(super) fn identifier(&mut self) {
        let start = self.cursor;
        while matches!(self.peek(), Some(ch) if is_identifier_continue(ch)) {
            self.advance();
        }
        let text = &self.source[start..self.cursor];
        self.tokens.push(Token {
            kind: identifier_or_keyword(text),
            span: Span::new(start, self.cursor),
        });
    }

    pub(super) fn number(&mut self) -> Result<(), LexError> {
        let start = self.cursor;
        if self.peek() == Some('0') {
            if let Some(radix) = self.prefixed_number_radix() {
                self.advance();
                self.advance();
                let digits_start = self.cursor;
                while matches!(self.peek(), Some(ch) if is_digit_for_radix(ch, radix) || ch == '_')
                {
                    self.advance();
                }
                let digits = &self.source[digits_start..self.cursor];
                if digits.is_empty() {
                    return Err(LexError {
                        message: "expected digits after numeric literal prefix".to_owned(),
                        span: Span::new(start, self.cursor),
                    });
                }
                validate_numeric_separators(digits, start + 2, radix)?;
                if self.peek() == Some('n') {
                    self.advance();
                    self.tokens.push(Token {
                        kind: TokenKind::BigInt(self.source[start..self.cursor - 1].to_owned()),
                        span: Span::new(start, self.cursor),
                    });
                    return Ok(());
                }
                if digits.contains('_') {
                    return Err(invalid_numeric_separator(
                        start + 2 + digits.find('_').unwrap_or(0),
                    ));
                }
                if matches!(self.peek(), Some(ch) if is_identifier_continue(ch)) {
                    return Err(LexError {
                        message: "invalid digit in numeric literal".to_owned(),
                        span: Span::new(start, self.cursor + self.peek().map_or(0, char::len_utf8)),
                    });
                }
                self.tokens.push(Token {
                    kind: TokenKind::Number(self.source[start..self.cursor].to_owned()),
                    span: Span::new(start, self.cursor),
                });
                return Ok(());
            }
        }

        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit() || ch == '_') {
            self.advance();
        }
        if self.peek() == Some('n') {
            let digits = &self.source[start..self.cursor];
            validate_decimal_bigint_literal(digits, start)?;
            self.advance();
            self.tokens.push(Token {
                kind: TokenKind::BigInt(digits.to_owned()),
                span: Span::new(start, self.cursor),
            });
            return Ok(());
        }
        if self.source[start..self.cursor].contains('_') {
            return Err(invalid_numeric_separator(
                start + self.source[start..self.cursor].find('_').unwrap_or(0),
            ));
        }
        if self.peek() == Some('.') {
            self.advance();
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                self.advance();
            }
        }
        self.exponent_part(start)?;
        self.reject_identifier_continue_after_number(start)?;
        self.tokens.push(Token {
            kind: TokenKind::Number(self.source[start..self.cursor].to_owned()),
            span: Span::new(start, self.cursor),
        });
        Ok(())
    }

    pub(super) fn number_starting_with_dot(&mut self) -> Result<(), LexError> {
        let start = self.cursor;
        self.advance();
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            self.advance();
        }
        self.exponent_part(start)?;
        self.reject_identifier_continue_after_number(start)?;
        self.tokens.push(Token {
            kind: TokenKind::Number(self.source[start..self.cursor].to_owned()),
            span: Span::new(start, self.cursor),
        });
        Ok(())
    }

    fn prefixed_number_radix(&self) -> Option<u32> {
        match (self.peek(), self.peek_nth(1)) {
            (Some('0'), Some('x' | 'X')) => Some(16),
            (Some('0'), Some('b' | 'B')) => Some(2),
            (Some('0'), Some('o' | 'O')) => Some(8),
            _ => None,
        }
    }

    fn exponent_part(&mut self, start: usize) -> Result<(), LexError> {
        if !matches!(self.peek(), Some('e' | 'E')) {
            return Ok(());
        }
        self.advance();
        if matches!(self.peek(), Some('+' | '-')) {
            self.advance();
        }
        let digits_start = self.cursor;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            self.advance();
        }
        if self.cursor == digits_start {
            return Err(LexError {
                message: "expected digits in numeric literal exponent".to_owned(),
                span: Span::new(start, self.cursor),
            });
        }
        Ok(())
    }

    fn reject_identifier_continue_after_number(&self, start: usize) -> Result<(), LexError> {
        if matches!(self.peek(), Some(ch) if is_identifier_continue(ch)) {
            return Err(LexError {
                message: "invalid identifier after numeric literal".to_owned(),
                span: Span::new(start, self.cursor + self.peek().map_or(0, char::len_utf8)),
            });
        }
        Ok(())
    }

    pub(super) fn string(&mut self, quote: char) -> Result<(), LexError> {
        let start = self.cursor;
        self.advance();
        let mut value = String::new();
        while let Some(ch) = self.peek() {
            if ch == quote {
                self.advance();
                self.tokens.push(Token {
                    kind: TokenKind::String(value),
                    span: Span::new(start, self.cursor),
                });
                return Ok(());
            }
            if ch == '\\' {
                if let Some(escaped) = self.escape_sequence(start)? {
                    value.push(escaped);
                }
                continue;
            }
            if matches!(ch, '\n' | '\r') {
                return Err(LexError {
                    message: "unterminated string literal".to_owned(),
                    span: Span::new(start, self.cursor),
                });
            }
            value.push(ch);
            self.advance();
        }

        Err(LexError {
            message: "unterminated string literal".to_owned(),
            span: Span::new(start, self.cursor),
        })
    }

    pub(super) fn template_literal(&mut self) -> Result<(), LexError> {
        let start = self.cursor;
        self.advance();
        let mut value = String::new();
        let mut raw = String::new();
        while let Some(ch) = self.peek() {
            if ch == '`' {
                self.advance();
                self.tokens.push(Token {
                    kind: TokenKind::TemplateNoSubstitution(TemplateSegment { cooked: value, raw }),
                    span: Span::new(start, self.cursor),
                });
                return Ok(());
            }
            if ch == '$' && self.peek_nth(1) == Some('{') {
                self.advance();
                self.advance();
                self.tokens.push(Token {
                    kind: TokenKind::TemplateHead(TemplateSegment { cooked: value, raw }),
                    span: Span::new(start, self.cursor),
                });
                self.template_stack.push(TemplateState { brace_depth: 0 });
                return Ok(());
            }
            if ch == '\\' {
                if self.template_line_continuation(&mut raw) {
                    continue;
                }
                let escape_start = self.cursor;
                if let Some(escaped) = self.escape_sequence(start)? {
                    value.push(escaped);
                }
                raw.push_str(&self.source[escape_start..self.cursor]);
                continue;
            }
            if Self::is_line_terminator(ch) {
                self.template_line_terminator(&mut value, &mut raw);
                continue;
            }
            raw.push(ch);
            value.push(ch);
            self.advance();
        }

        Err(LexError {
            message: "unterminated template literal".to_owned(),
            span: Span::new(start, self.cursor),
        })
    }

    pub(super) fn template_after_substitution(&mut self) -> Result<(), LexError> {
        let start = self.cursor;
        self.advance();
        let mut value = String::new();
        let mut raw = String::new();
        while let Some(ch) = self.peek() {
            if ch == '`' {
                self.advance();
                self.template_stack.pop();
                self.tokens.push(Token {
                    kind: TokenKind::TemplateTail(TemplateSegment { cooked: value, raw }),
                    span: Span::new(start, self.cursor),
                });
                return Ok(());
            }
            if ch == '$' && self.peek_nth(1) == Some('{') {
                self.advance();
                self.advance();
                self.tokens.push(Token {
                    kind: TokenKind::TemplateMiddle(TemplateSegment { cooked: value, raw }),
                    span: Span::new(start, self.cursor),
                });
                return Ok(());
            }
            if ch == '\\' {
                if self.template_line_continuation(&mut raw) {
                    continue;
                }
                let escape_start = self.cursor;
                if let Some(escaped) = self.escape_sequence(start)? {
                    value.push(escaped);
                }
                raw.push_str(&self.source[escape_start..self.cursor]);
                continue;
            }
            if Self::is_line_terminator(ch) {
                self.template_line_terminator(&mut value, &mut raw);
                continue;
            }
            raw.push(ch);
            value.push(ch);
            self.advance();
        }

        Err(LexError {
            message: "unterminated template literal".to_owned(),
            span: Span::new(start, self.cursor),
        })
    }

    fn template_line_continuation(&mut self, raw: &mut String) -> bool {
        if self.peek() != Some('\\')
            || !matches!(self.peek_nth(1), Some(ch) if Self::is_line_terminator(ch))
        {
            return false;
        }
        self.advance();
        raw.push('\\');
        self.template_raw_line_terminator(raw);
        true
    }

    fn template_line_terminator(&mut self, cooked: &mut String, raw: &mut String) {
        let terminator = self.template_raw_line_terminator(raw);
        cooked.push(terminator);
    }

    fn template_raw_line_terminator(&mut self, raw: &mut String) -> char {
        match self.advance().expect("line terminator should be present") {
            '\r' => {
                if self.peek() == Some('\n') {
                    self.advance();
                }
                raw.push('\n');
                '\n'
            }
            ch => {
                raw.push(ch);
                ch
            }
        }
    }

    fn is_line_terminator(ch: char) -> bool {
        matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
    }

    fn escape_sequence(&mut self, literal_start: usize) -> Result<Option<char>, LexError> {
        self.advance();
        let Some(ch) = self.advance() else {
            return Err(LexError {
                message: "unterminated escape sequence".to_owned(),
                span: Span::new(literal_start, self.cursor),
            });
        };

        let escaped = match ch {
            '\'' => Some('\''),
            '"' => Some('"'),
            '`' => Some('`'),
            '\\' => Some('\\'),
            'b' => Some('\u{0008}'),
            'f' => Some('\u{000c}'),
            'n' => Some('\n'),
            'r' => Some('\r'),
            't' => Some('\t'),
            'v' => Some('\u{000b}'),
            '0' if !matches!(self.peek(), Some(next) if next.is_ascii_digit()) => Some('\0'),
            '0'..='7' => Some(self.legacy_octal_escape(ch, literal_start)?),
            '8' | '9' => {
                return Err(LexError {
                    message: "legacy octal escape sequence is not supported".to_owned(),
                    span: Span::new(literal_start, self.cursor),
                });
            }
            'x' => Some(self.fixed_hex_escape(literal_start, 2)?),
            'u' => Some(self.unicode_escape(literal_start)?),
            '\n' => None,
            '\r' => {
                if self.peek() == Some('\n') {
                    self.advance();
                }
                None
            }
            '\u{2028}' | '\u{2029}' => None,
            other => Some(other),
        };
        Ok(escaped)
    }

    fn legacy_octal_escape(
        &mut self,
        first_digit: char,
        literal_start: usize,
    ) -> Result<char, LexError> {
        let mut digits = String::from(first_digit);
        let max_digits = if matches!(first_digit, '0'..='3') {
            3
        } else {
            2
        };
        while digits.len() < max_digits {
            let Some(next) = self.peek() else {
                break;
            };
            if !matches!(next, '0'..='7') {
                break;
            }
            digits.push(next);
            self.advance();
        }
        let value = u32::from_str_radix(&digits, 8).map_err(|_| LexError {
            message: "invalid legacy octal escape sequence".to_owned(),
            span: Span::new(literal_start, self.cursor),
        })?;
        char::from_u32(value).ok_or_else(|| LexError {
            message: "invalid legacy octal escape sequence".to_owned(),
            span: Span::new(literal_start, self.cursor),
        })
    }

    fn unicode_escape(&mut self, literal_start: usize) -> Result<char, LexError> {
        if self.peek() == Some('{') {
            self.advance();
            let digits_start = self.cursor;
            while matches!(self.peek(), Some(ch) if ch.is_ascii_hexdigit()) {
                self.advance();
            }
            if self.cursor == digits_start || self.peek() != Some('}') {
                return Err(LexError {
                    message: "invalid unicode escape sequence".to_owned(),
                    span: Span::new(literal_start, self.cursor),
                });
            }
            let value =
                u32::from_str_radix(&self.source[digits_start..self.cursor], 16).map_err(|_| {
                    LexError {
                        message: "invalid unicode escape sequence".to_owned(),
                        span: Span::new(literal_start, self.cursor),
                    }
                })?;
            self.advance();
            return char::from_u32(value).ok_or_else(|| LexError {
                message: "invalid unicode escape sequence".to_owned(),
                span: Span::new(literal_start, self.cursor),
            });
        }

        self.fixed_hex_escape(literal_start, 4)
    }

    fn fixed_hex_escape(&mut self, literal_start: usize, digits: usize) -> Result<char, LexError> {
        let digits_start = self.cursor;
        for _ in 0..digits {
            if !matches!(self.peek(), Some(ch) if ch.is_ascii_hexdigit()) {
                return Err(LexError {
                    message: "invalid escape sequence".to_owned(),
                    span: Span::new(literal_start, self.cursor),
                });
            }
            self.advance();
        }
        let value =
            u32::from_str_radix(&self.source[digits_start..self.cursor], 16).map_err(|_| {
                LexError {
                    message: "invalid escape sequence".to_owned(),
                    span: Span::new(literal_start, self.cursor),
                }
            })?;
        if (0xD800..=0xDFFF).contains(&value) {
            return char::from_u32(SURROGATE_ESCAPE_SENTINEL_BASE + value - 0xD800).ok_or_else(
                || LexError {
                    message: "invalid escape sequence".to_owned(),
                    span: Span::new(literal_start, self.cursor),
                },
            );
        }
        char::from_u32(value).ok_or_else(|| LexError {
            message: "invalid escape sequence".to_owned(),
            span: Span::new(literal_start, self.cursor),
        })
    }
}

fn validate_decimal_bigint_literal(digits: &str, start: usize) -> Result<(), LexError> {
    validate_numeric_separators(digits, start, 10)?;
    if digits.len() > 1 && digits.starts_with('0') {
        return Err(LexError {
            message: "invalid BigInt literal with leading zero".to_owned(),
            span: Span::new(start, start + digits.len()),
        });
    }
    Ok(())
}

fn validate_numeric_separators(digits: &str, start: usize, radix: u32) -> Result<(), LexError> {
    let mut previous_separator = false;
    for (offset, ch) in digits.char_indices() {
        if ch == '_' {
            if offset == 0 || previous_separator {
                return Err(invalid_numeric_separator(start + offset));
            }
            previous_separator = true;
            continue;
        }
        if !is_digit_for_radix(ch, radix) {
            return Err(LexError {
                message: "invalid digit in numeric literal".to_owned(),
                span: Span::new(start + offset, start + offset + ch.len_utf8()),
            });
        }
        previous_separator = false;
    }
    if previous_separator {
        return Err(invalid_numeric_separator(start + digits.len() - 1));
    }
    Ok(())
}

fn invalid_numeric_separator(position: usize) -> LexError {
    LexError {
        message: "invalid numeric separator".to_owned(),
        span: Span::new(position, position + 1),
    }
}

fn is_digit_for_radix(ch: char, radix: u32) -> bool {
    ch.is_digit(radix)
}
