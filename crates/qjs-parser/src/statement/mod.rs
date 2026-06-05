mod control;
mod declarations;
mod functions;
mod simple;

use qjs_ast::{Script, Span, Stmt};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn parse_script(&mut self) -> Result<Script, ParseError> {
        let mut body = Vec::new();
        while !self.at(&TokenKind::Eof) {
            body.push(self.statement()?);
        }
        Ok(Script { body })
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        if self.match_kind(&TokenKind::Semicolon) {
            return Ok(Stmt::Empty);
        }

        if self.at(&TokenKind::LeftBrace) {
            return self.block_statement();
        }

        if self.at(&TokenKind::If) {
            return self.if_statement();
        }

        if self.at(&TokenKind::While) {
            return self.while_statement();
        }

        if self.at(&TokenKind::With) {
            return self.with_statement();
        }

        if self.at(&TokenKind::Do) {
            return self.do_while_statement();
        }

        if self.at(&TokenKind::For) {
            return self.for_statement();
        }

        if self.at(&TokenKind::Switch) {
            return self.switch_statement();
        }

        if self.at(&TokenKind::Try) {
            return self.try_statement();
        }

        if self.at(&TokenKind::Function) {
            return self.function_declaration();
        }

        if self.at_identifier_text("class") {
            return self.class_declaration();
        }

        if self.at(&TokenKind::Return) {
            return self.return_statement();
        }

        if self.at(&TokenKind::Throw) {
            return self.throw_statement();
        }

        if self.at(&TokenKind::Debugger) {
            return Ok(self.debugger_statement());
        }

        if self.at(&TokenKind::Break) {
            return Ok(self.break_or_continue_statement(TokenKind::Break));
        }

        if self.at(&TokenKind::Continue) {
            return Ok(self.break_or_continue_statement(TokenKind::Continue));
        }

        if self.at(&TokenKind::Var)
            || self.at(&TokenKind::Const)
            || (self.at(&TokenKind::Let)
                && !self
                    .peek_nth(1)
                    .is_some_and(|token| token.kind == TokenKind::LeftBrace))
        {
            return self.variable_declaration();
        }

        if matches!(
            self.peek().map(|token| &token.kind),
            Some(TokenKind::Identifier(_))
        ) && self
            .peek_nth(1)
            .is_some_and(|token| token.kind == TokenKind::Colon)
        {
            return self.labelled_statement();
        }

        let expr = self.expression()?;
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::Expr(expr))
    }

    fn labelled_statement(&mut self) -> Result<Stmt, ParseError> {
        let token = self.advance();
        let TokenKind::Identifier(label) = token.kind else {
            unreachable!("caller checked label identifier");
        };
        self.expect(&TokenKind::Colon)?;
        let body = self.statement()?;
        let end = crate::helpers::stmt_end(&body);
        Ok(Stmt::Label {
            label,
            body: Box::new(body),
            span: Span::new(token.span.start, end),
        })
    }

    pub(super) fn block_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::LeftBrace)?;
        let mut body = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            body.push(self.statement()?);
        }
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBrace)?;
        Ok(Stmt::Block {
            body,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn block_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.expect(&TokenKind::LeftBrace)?;
        let mut body = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            body.push(self.statement()?);
        }
        self.expect(&TokenKind::RightBrace)?;
        Ok(body)
    }
}
