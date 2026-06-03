use qjs_ast::Span;

use crate::{LexError, TokenKind};

use super::{Lexer, char_class::is_identifier_continue};

impl Lexer<'_> {
    pub(super) fn slash_or_comment(&mut self) -> Result<(), LexError> {
        let start = self.cursor;
        self.advance();

        match self.peek() {
            Some('/') => {
                self.advance();
                while !matches!(self.peek(), None | Some('\n' | '\r')) {
                    self.advance();
                }
                Ok(())
            }
            Some('*') => {
                self.advance();
                self.block_comment(start)
            }
            Some('=') => {
                self.advance();
                self.push(TokenKind::SlashEqual, start);
                Ok(())
            }
            _ if self.slash_starts_regexp_literal() => self.regexp_literal(start),
            _ => {
                self.push(TokenKind::Slash, start);
                Ok(())
            }
        }
    }

    fn slash_starts_regexp_literal(&self) -> bool {
        let Some(previous) = self.tokens.last() else {
            return true;
        };

        !matches!(
            previous.kind,
            TokenKind::Identifier(_)
                | TokenKind::Number(_)
                | TokenKind::String(_)
                | TokenKind::RegularExpression { .. }
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::This
                | TokenKind::RightParen
                | TokenKind::RightBracket
                | TokenKind::RightBrace
        )
    }

    fn regexp_literal(&mut self, start: usize) -> Result<(), LexError> {
        let mut pattern = String::new();
        let mut escaped = false;
        let mut in_character_class = false;

        while let Some(ch) = self.peek() {
            if matches!(ch, '\n' | '\r') {
                return Err(LexError {
                    message: "unterminated regular expression literal".to_owned(),
                    span: Span::new(start, self.cursor),
                });
            }

            if escaped {
                pattern.push(ch);
                self.advance();
                escaped = false;
                continue;
            }

            match ch {
                '\\' => {
                    pattern.push(ch);
                    self.advance();
                    escaped = true;
                }
                '[' => {
                    pattern.push(ch);
                    self.advance();
                    in_character_class = true;
                }
                ']' => {
                    pattern.push(ch);
                    self.advance();
                    in_character_class = false;
                }
                '/' if !in_character_class => {
                    self.advance();
                    let flags_start = self.cursor;
                    while matches!(self.peek(), Some(ch) if is_identifier_continue(ch)) {
                        self.advance();
                    }
                    let flags = self.source[flags_start..self.cursor].to_owned();
                    self.tokens.push(crate::Token {
                        kind: TokenKind::RegularExpression { pattern, flags },
                        span: Span::new(start, self.cursor),
                    });
                    return Ok(());
                }
                _ => {
                    pattern.push(ch);
                    self.advance();
                }
            }
        }

        Err(LexError {
            message: "unterminated regular expression literal".to_owned(),
            span: Span::new(start, self.cursor),
        })
    }

    fn block_comment(&mut self, start: usize) -> Result<(), LexError> {
        while let Some(ch) = self.advance() {
            if ch == '*' && self.peek() == Some('/') {
                self.advance();
                return Ok(());
            }
        }

        Err(LexError {
            message: "unterminated block comment".to_owned(),
            span: Span::new(start, self.cursor),
        })
    }
}
