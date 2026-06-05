use qjs_ast::{Span, Stmt};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

impl Parser {
    pub(super) fn return_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Return)?;
        let argument = if self.at(&TokenKind::Semicolon) || self.at(&TokenKind::RightBrace) {
            None
        } else {
            Some(self.expression()?)
        };
        self.match_kind(&TokenKind::Semicolon);
        let end = argument
            .as_ref()
            .map_or(start + "return".len(), |expr| expr.span().end);
        Ok(Stmt::Return {
            argument,
            span: Span::new(start, end),
        })
    }

    pub(super) fn throw_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Throw)?;
        let argument = if self.at(&TokenKind::Semicolon)
            || self.at(&TokenKind::RightBrace)
            || self.at(&TokenKind::Eof)
        {
            None
        } else {
            Some(self.expression()?)
        };
        let mut end = argument
            .as_ref()
            .map_or(start + "throw".len(), |expr| expr.span().end);
        if self.match_kind(&TokenKind::Semicolon) {
            end = self.tokens[self.cursor - 1].span.end;
        }
        Ok(Stmt::Throw {
            argument,
            span: Span::new(start, end),
        })
    }

    pub(super) fn debugger_statement(&mut self) -> Stmt {
        let token = self.advance();
        self.match_kind(&TokenKind::Semicolon);
        let end = self.tokens[self.cursor.saturating_sub(1)].span.end;
        Stmt::Debugger {
            span: Span::new(token.span.start, end),
        }
    }

    pub(super) fn break_or_continue_statement(&mut self, kind: TokenKind) -> Stmt {
        let token = self.advance();
        let label = if self.peek().is_some_and(|token| {
            matches!(token.kind, TokenKind::Identifier(_)) && !token.preceded_by_line_terminator
        }) {
            let label_token = self.advance();
            let TokenKind::Identifier(label) = label_token.kind else {
                unreachable!("peek checked label identifier");
            };
            Some(label)
        } else {
            None
        };
        self.match_kind(&TokenKind::Semicolon);
        let end = self.tokens[self.cursor.saturating_sub(1)].span.end;
        let span = Span::new(token.span.start, end);
        if kind == TokenKind::Break {
            Stmt::Break { label, span }
        } else {
            Stmt::Continue { label, span }
        }
    }
}
