use qjs_ast::{CatchClause, ForInLeft, ForInit, Span, Stmt, SwitchCase, VarKind};
use qjs_lexer::TokenKind;

use crate::helpers::{assignment_target, stmt_end, var_kind};
use crate::{ParseError, Parser};

impl Parser {
    pub(super) fn if_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::If)?;
        self.expect(&TokenKind::LeftParen)?;
        let test = self.expression()?;
        self.expect(&TokenKind::RightParen)?;
        let consequent = self.statement()?;
        let alternate = if self.match_kind(&TokenKind::Else) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        let end = alternate
            .as_deref()
            .map_or_else(|| stmt_end(&consequent), stmt_end);
        Ok(Stmt::If {
            test,
            consequent: Box::new(consequent),
            alternate,
            span: Span::new(start, end),
        })
    }

    pub(super) fn while_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LeftParen)?;
        let test = self.expression()?;
        self.expect(&TokenKind::RightParen)?;
        let body = self.statement()?;
        let end = stmt_end(&body);
        Ok(Stmt::While {
            test,
            body: Box::new(body),
            span: Span::new(start, end),
        })
    }

    pub(super) fn do_while_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Do)?;
        let body = self.statement()?;
        self.expect(&TokenKind::While)?;
        self.expect(&TokenKind::LeftParen)?;
        let test = self.expression()?;
        self.expect(&TokenKind::RightParen)?;
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::DoWhile {
            body: Box::new(body),
            test,
            span: Span::new(start, end),
        })
    }

    pub(super) fn for_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::For)?;
        self.expect(&TokenKind::LeftParen)?;
        if self.at(&TokenKind::Var) || self.at(&TokenKind::Let) || self.at(&TokenKind::Const) {
            let var_head_start = self.cursor;
            let kind_token = self.advance();
            let kind = var_kind(&kind_token.kind).expect("token should be declaration kind");
            let name_token = self.advance();
            let name = for_head_binding_name(&name_token.kind, kind).ok_or_else(|| ParseError {
                message: "expected binding identifier".to_owned(),
                span: name_token.span,
            })?;
            let init = if kind == VarKind::Var && self.match_kind(&TokenKind::Equal) {
                Some(self.expression_no_in()?)
            } else {
                None
            };
            if self.match_kind(&TokenKind::In) {
                let right = self.expression()?;
                self.expect(&TokenKind::RightParen)?;
                let body = self.statement()?;
                let end = stmt_end(&body);
                return Ok(Stmt::ForIn {
                    left: ForInLeft::VarDecl {
                        kind,
                        name,
                        init,
                        span: Span::new(kind_token.span.start, name_token.span.end),
                    },
                    right,
                    body: Box::new(body),
                    span: Span::new(start, end),
                });
            }
            if self.match_contextual_keyword("of") {
                let right = self.expression()?;
                self.expect(&TokenKind::RightParen)?;
                let body = self.statement()?;
                let end = stmt_end(&body);
                return Ok(Stmt::ForOf {
                    left: ForInLeft::VarDecl {
                        kind,
                        name,
                        init: None,
                        span: Span::new(kind_token.span.start, name_token.span.end),
                    },
                    right,
                    body: Box::new(body),
                    span: Span::new(start, end),
                });
            }
            self.cursor = var_head_start;
        } else if !self.at(&TokenKind::Semicolon) {
            let cursor = self.cursor;
            let left = self.call()?;
            if self.match_kind(&TokenKind::In) {
                let left = assignment_target(left)?;
                let right = self.expression()?;
                self.expect(&TokenKind::RightParen)?;
                let body = self.statement()?;
                let end = stmt_end(&body);
                return Ok(Stmt::ForIn {
                    left: ForInLeft::Target(left),
                    right,
                    body: Box::new(body),
                    span: Span::new(start, end),
                });
            }
            if self.match_contextual_keyword("of") {
                let left = assignment_target(left)?;
                let right = self.expression()?;
                self.expect(&TokenKind::RightParen)?;
                let body = self.statement()?;
                let end = stmt_end(&body);
                return Ok(Stmt::ForOf {
                    left: ForInLeft::Target(left),
                    right,
                    body: Box::new(body),
                    span: Span::new(start, end),
                });
            }
            self.cursor = cursor;
        }

        let init = if self.match_kind(&TokenKind::Semicolon) {
            None
        } else if self.at(&TokenKind::Var) || self.at(&TokenKind::Let) || self.at(&TokenKind::Const)
        {
            let init = self.for_variable_declaration()?;
            self.expect(&TokenKind::Semicolon)?;
            Some(init)
        } else {
            let init = self.expression()?;
            self.expect(&TokenKind::Semicolon)?;
            Some(ForInit::Expr(init))
        };

        let test = if self.at(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(&TokenKind::Semicolon)?;

        let update = if self.at(&TokenKind::RightParen) {
            None
        } else {
            Some(self.expression()?)
        };
        self.expect(&TokenKind::RightParen)?;
        let body = self.statement()?;
        let end = stmt_end(&body);
        Ok(Stmt::For {
            init,
            test,
            update,
            body: Box::new(body),
            span: Span::new(start, end),
        })
    }

    pub(super) fn switch_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Switch)?;
        self.expect(&TokenKind::LeftParen)?;
        let discriminant = self.expression()?;
        self.expect(&TokenKind::RightParen)?;
        self.expect(&TokenKind::LeftBrace)?;

        let mut cases = Vec::new();
        let mut seen_default = false;
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            let clause_start = self
                .peek()
                .expect("parser should always have eof token")
                .span
                .start;
            let test = if self.match_kind(&TokenKind::Case) {
                let test = self.expression()?;
                self.expect(&TokenKind::Colon)?;
                Some(test)
            } else if self.match_kind(&TokenKind::Default) {
                if seen_default {
                    return Err(ParseError {
                        message: "switch statement cannot have multiple default clauses".to_owned(),
                        span: Span::new(clause_start, clause_start + "default".len()),
                    });
                }
                seen_default = true;
                self.expect(&TokenKind::Colon)?;
                None
            } else {
                let token = self.peek().expect("parser should always have eof token");
                return Err(ParseError {
                    message: "expected switch case or default clause".to_owned(),
                    span: token.span,
                });
            };

            let mut consequent = Vec::new();
            while !self.at(&TokenKind::Case)
                && !self.at(&TokenKind::Default)
                && !self.at(&TokenKind::RightBrace)
                && !self.at(&TokenKind::Eof)
            {
                consequent.push(self.statement()?);
            }
            let end = consequent
                .last()
                .map_or_else(|| self.tokens[self.cursor - 1].span.end, stmt_end);
            cases.push(SwitchCase {
                test,
                consequent,
                span: Span::new(clause_start, end),
            });
        }

        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBrace)?;
        Ok(Stmt::Switch {
            discriminant,
            cases,
            span: Span::new(start, end),
        })
    }

    pub(super) fn try_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Try)?;
        let block = self.block_body()?;
        let handler = if self.at(&TokenKind::Catch) {
            Some(self.catch_clause()?)
        } else {
            None
        };
        let finalizer = if self.match_kind(&TokenKind::Finally) {
            Some(self.block_body()?)
        } else {
            None
        };

        if handler.is_none() && finalizer.is_none() {
            let token = self.peek().expect("parser should always have eof token");
            return Err(ParseError {
                message: "try statement requires catch or finally".to_owned(),
                span: token.span,
            });
        }

        let end = finalizer
            .as_ref()
            .and_then(|body| body.last().map(stmt_end))
            .or_else(|| handler.as_ref().map(|handler| handler.span.end))
            .or_else(|| block.last().map(stmt_end))
            .unwrap_or(start + "try".len());
        Ok(Stmt::Try {
            block,
            handler,
            finalizer,
            span: Span::new(start, end),
        })
    }

    fn catch_clause(&mut self) -> Result<CatchClause, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Catch)?;
        let param = if self.match_kind(&TokenKind::LeftParen) {
            let token = self.advance();
            let TokenKind::Identifier(name) = token.kind else {
                return Err(ParseError {
                    message: "expected catch binding identifier".to_owned(),
                    span: token.span,
                });
            };
            self.expect(&TokenKind::RightParen)?;
            Some(name)
        } else {
            None
        };
        let body = self.block_body()?;
        let end = body.last().map_or(start + "catch".len(), stmt_end);
        Ok(CatchClause {
            param,
            body,
            span: Span::new(start, end),
        })
    }
}

fn for_head_binding_name(kind: &TokenKind, declaration_kind: qjs_ast::VarKind) -> Option<String> {
    match kind {
        TokenKind::Identifier(name) => Some(name.clone()),
        TokenKind::Let if declaration_kind == qjs_ast::VarKind::Var => Some("let".to_owned()),
        _ => None,
    }
}
