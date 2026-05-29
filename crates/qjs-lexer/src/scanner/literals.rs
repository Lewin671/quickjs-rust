use qjs_ast::Span;

use crate::{LexError, Token, TokenKind};

use super::{Lexer, char_class::is_identifier_continue, keywords::identifier_or_keyword};

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
                while matches!(self.peek(), Some(ch) if is_digit_for_radix(ch, radix)) {
                    self.advance();
                }
                if self.cursor == digits_start {
                    return Err(LexError {
                        message: "expected digits after numeric literal prefix".to_owned(),
                        span: Span::new(start, self.cursor),
                    });
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

        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            self.advance();
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

    pub(super) fn template_no_substitution(&mut self) -> Result<(), LexError> {
        let start = self.cursor;
        self.advance();
        let mut value = String::new();
        while let Some(ch) = self.peek() {
            if ch == '`' {
                self.advance();
                self.tokens.push(Token {
                    kind: TokenKind::String(value),
                    span: Span::new(start, self.cursor),
                });
                return Ok(());
            }
            if ch == '$' && self.peek_nth(1) == Some('{') {
                return Err(LexError {
                    message: "template substitution is not supported yet".to_owned(),
                    span: Span::new(start, self.cursor + 2),
                });
            }
            if ch == '\\' {
                if let Some(escaped) = self.escape_sequence(start)? {
                    value.push(escaped);
                }
                continue;
            }
            value.push(ch);
            self.advance();
        }

        Err(LexError {
            message: "unterminated template literal".to_owned(),
            span: Span::new(start, self.cursor),
        })
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
            '0' | '1'..='9' => {
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
            other => Some(other),
        };
        Ok(escaped)
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
        char::from_u32(value).ok_or_else(|| LexError {
            message: "invalid escape sequence".to_owned(),
            span: Span::new(literal_start, self.cursor),
        })
    }
}

fn is_digit_for_radix(ch: char, radix: u32) -> bool {
    ch.is_digit(radix)
}
