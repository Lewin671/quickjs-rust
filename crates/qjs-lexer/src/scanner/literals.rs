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

    pub(super) fn string(&mut self, quote: char) -> Result<(), LexError> {
        let start = self.cursor;
        self.advance();
        let content_start = self.cursor;
        while let Some(ch) = self.peek() {
            if ch == quote {
                let value = self.source[content_start..self.cursor].to_owned();
                self.advance();
                self.tokens.push(Token {
                    kind: TokenKind::String(value),
                    span: Span::new(start, self.cursor),
                });
                return Ok(());
            }
            self.advance();
        }

        Err(LexError {
            message: "unterminated string literal".to_owned(),
            span: Span::new(start, self.cursor),
        })
    }
}

fn is_digit_for_radix(ch: char, radix: u32) -> bool {
    ch.is_digit(radix)
}
