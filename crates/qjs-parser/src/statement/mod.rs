mod classes;
mod control;
mod declarations;
mod functions;

pub(crate) use functions::duplicate_parameter_span;
mod simple;

use qjs_ast::{Script, Span, Stmt};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn parse_script(&mut self) -> Result<Script, ParseError> {
        self.strict = self.strict || self.directive_prologue_is_strict(self.cursor);
        let mut body = Vec::new();
        while !self.at(&TokenKind::Eof) {
            body.push(self.statement()?);
        }
        // Any private-name reference that never resolved to an enclosing class
        // is a syntax error.
        if let Some(reference) = self.pending_private_refs.first() {
            return Err(ParseError {
                message: format!(
                    "private name `#{}` is not declared in scope",
                    reference.name
                ),
                span: reference.span,
            });
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

        // `async function` (with no line terminator between) is an async
        // function declaration. `async` followed by anything else is a plain
        // identifier expression statement.
        if self.at_async_function_keyword() {
            let async_token = self.advance();
            return self.function_declaration_with_async(async_token.span.start, true);
        }

        if self.at(&TokenKind::Class) {
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

        if self.starts_labelled_statement() {
            return self.labelled_statement();
        }

        if self.at(&TokenKind::Var) || self.at(&TokenKind::Let) || self.at(&TokenKind::Const) {
            return self.variable_declaration();
        }

        let expr = self.expression()?;
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::Expr(expr))
    }

    fn starts_labelled_statement(&self) -> bool {
        matches!(self.peek(), Some(token) if matches!(token.kind, TokenKind::Identifier(_)))
            && matches!(self.peek_nth(1), Some(token) if token.kind == TokenKind::Colon)
    }

    fn labelled_statement(&mut self) -> Result<Stmt, ParseError> {
        let label_token = self.advance();
        let TokenKind::Identifier(label) = label_token.kind else {
            unreachable!("caller should check label token")
        };
        self.expect(&TokenKind::Colon)?;
        let body = self.statement()?;
        let end = crate::helpers::stmt_end(&body);
        Ok(Stmt::Labelled {
            label,
            body: Box::new(body),
            span: Span::new(label_token.span.start, end),
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
        let previous_strict = self.strict;
        self.strict = self.strict || self.directive_prologue_is_strict(self.cursor);
        let result = (|parser: &mut Self| {
            let mut body = Vec::new();
            while !parser.at(&TokenKind::RightBrace) && !parser.at(&TokenKind::Eof) {
                body.push(parser.statement()?);
            }
            parser.expect(&TokenKind::RightBrace).map(|()| body)
        })(self);
        self.strict = previous_strict;
        result
    }

    fn directive_prologue_is_strict(&self, mut cursor: usize) -> bool {
        while let Some(token) = self.tokens.get(cursor) {
            let TokenKind::String(value) = &token.kind else {
                return false;
            };
            if value == "use strict" {
                return true;
            }
            cursor += 1;
            if matches!(
                self.tokens.get(cursor).map(|token| &token.kind),
                Some(TokenKind::Semicolon)
            ) {
                cursor += 1;
            }
        }
        false
    }
}
