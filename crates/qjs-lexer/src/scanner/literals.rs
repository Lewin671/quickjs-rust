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

    pub(super) fn number(&mut self) {
        let start = self.cursor;
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
