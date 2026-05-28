use qjs_ast::Span;

use crate::{LexError, TokenKind};

use super::Lexer;

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
            _ => {
                if self.peek() == Some('=') {
                    self.advance();
                    self.push(TokenKind::SlashEqual, start);
                } else {
                    self.push(TokenKind::Slash, start);
                }
                Ok(())
            }
        }
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
