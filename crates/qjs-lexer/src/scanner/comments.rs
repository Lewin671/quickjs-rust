use qjs_ast::Span;

use crate::{LexError, TokenKind};

use super::{
    Lexer,
    char_class::{is_identifier_continue, is_js_whitespace_or_line_terminator},
};

impl Lexer<'_> {
    pub(super) fn hashbang_comment(&mut self) {
        debug_assert_eq!(self.cursor, 0);
        debug_assert_eq!(self.peek(), Some('#'));
        debug_assert_eq!(self.peek_nth(1), Some('!'));

        self.advance();
        self.advance();
        self.skip_line_comment_tail();
    }

    pub(super) fn html_open_comment(&mut self) -> bool {
        if self.peek() != Some('<')
            || self.peek_nth(1) != Some('!')
            || self.peek_nth(2) != Some('-')
            || self.peek_nth(3) != Some('-')
        {
            return false;
        }

        self.advance();
        self.advance();
        self.advance();
        self.advance();
        self.skip_line_comment_tail();
        true
    }

    pub(super) fn html_close_comment(&mut self) -> bool {
        if self.peek() != Some('-')
            || self.peek_nth(1) != Some('-')
            || self.peek_nth(2) != Some('>')
            || !self.html_close_comment_allowed()
        {
            return false;
        }

        self.advance();
        self.advance();
        self.advance();
        self.skip_line_comment_tail();
        true
    }

    pub(super) fn slash_or_comment(&mut self) -> Result<(), LexError> {
        let start = self.cursor;
        self.advance();

        match self.peek() {
            Some('/') => {
                self.advance();
                self.skip_line_comment_tail();
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
                | TokenKind::PrivateName(_)
                | TokenKind::Number(_)
                | TokenKind::BigInt(_)
                | TokenKind::String(_)
                | TokenKind::TemplateNoSubstitution(_)
                | TokenKind::TemplateTail(_)
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
            if is_line_terminator(ch) {
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
                        had_escape: false,
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

    fn skip_line_comment_tail(&mut self) {
        while match self.peek() {
            Some(ch) => !is_line_terminator(ch),
            None => false,
        } {
            self.advance();
        }
    }

    fn html_close_comment_allowed(&self) -> bool {
        let mut index = self.cursor;
        loop {
            index = self.skip_html_close_prefix_whitespace(index);
            if index == 0 {
                return true;
            }

            let Some(previous) = self.source[..index].chars().next_back() else {
                return true;
            };
            if is_line_terminator(previous) {
                return true;
            }
            if self.source[..index].ends_with("*/")
                && let Some(start) = self.source[..index].rfind("/*")
            {
                if self.source[start..index].chars().any(is_line_terminator) {
                    return true;
                }
                index = start;
                continue;
            }
            return false;
        }
    }

    fn skip_html_close_prefix_whitespace(&self, mut index: usize) -> usize {
        while let Some(ch) = self.source[..index].chars().next_back() {
            if is_line_terminator(ch) || !is_js_whitespace_or_line_terminator(ch) {
                break;
            }
            index -= ch.len_utf8();
        }
        index
    }
}

fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}
