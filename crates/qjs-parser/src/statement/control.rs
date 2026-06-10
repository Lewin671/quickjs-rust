use qjs_ast::{BindingPattern, CatchClause, ForInLeft, ForInit, Span, Stmt, SwitchCase, VarKind};
use qjs_lexer::TokenKind;

use crate::helpers::{stmt_end, var_kind};
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
        // `for await (... of ...)` iterates with the async protocol and is only
        // valid inside an async function. `await` is the contextual keyword
        // here, recognized only in async context.
        let is_await = if self.in_async
            && matches!(self.peek(), Some(token) if matches!(&token.kind, TokenKind::Identifier(name) if name == "await"))
        {
            let await_token = self.advance();
            Some(await_token.span)
        } else {
            None
        };
        self.expect(&TokenKind::LeftParen)?;
        if self.at(&TokenKind::Var) || self.at(&TokenKind::Let) || self.at(&TokenKind::Const) {
            let var_head_start = self.cursor;
            let kind_token = self.advance();
            let kind = var_kind(&kind_token.kind).expect("token should be declaration kind");
            if let Some(binding) = self.for_head_binding(kind) {
                let init = if kind == VarKind::Var
                    && matches!(binding, BindingPattern::Identifier { .. })
                    && self.match_kind(&TokenKind::Equal)
                {
                    Some(self.expression_no_in()?)
                } else {
                    None
                };
                let left_span = Span::new(kind_token.span.start, binding.span().end);
                if self.match_kind(&TokenKind::In) {
                    if self.strict && init.is_some() {
                        return Err(ParseError {
                            message:
                                "for-in variable declarations cannot have initializers in strict mode"
                                    .to_owned(),
                            span: self
                                .peek()
                                .expect("parser should always have eof token")
                                .span,
                        });
                    }
                    let left = ForInLeft::VarDecl {
                        kind,
                        binding,
                        init,
                        span: left_span,
                    };
                    return self.finish_for_in_of(start, left, ForKind::In, is_await);
                }
                if init.is_none() && self.match_contextual_keyword("of") {
                    let left = ForInLeft::VarDecl {
                        kind,
                        binding,
                        init: None,
                        span: left_span,
                    };
                    return self.finish_for_in_of(start, left, ForKind::Of, is_await);
                }
            }
            self.cursor = var_head_start;
        } else if !self.at(&TokenKind::Semicolon) {
            let cursor = self.cursor;
            if let Ok(left) = self.assignment_pattern() {
                if self.match_kind(&TokenKind::In) {
                    return self.finish_for_in_of(
                        start,
                        ForInLeft::Target(left),
                        ForKind::In,
                        is_await,
                    );
                }
                if self.match_contextual_keyword("of") {
                    return self.finish_for_in_of(
                        start,
                        ForInLeft::Target(left),
                        ForKind::Of,
                        is_await,
                    );
                }
            }
            self.cursor = cursor;
        }

        // A `for await` head that did not resolve to a for-of loop is a syntax
        // error: `for await` requires the async iteration `of` form.
        if let Some(await_span) = is_await {
            return Err(ParseError {
                message: "`for await` requires a `for await (... of ...)` loop".to_owned(),
                span: await_span,
            });
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
            let param = self.binding_pattern()?;
            self.expect(&TokenKind::RightParen)?;
            Some(param)
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

    /// Parses a for-in/for-of declaration binding, returning `None` (with
    /// the cursor rewound) when the tokens do not form a binding pattern.
    fn for_head_binding(&mut self, kind: VarKind) -> Option<BindingPattern> {
        let cursor = self.cursor;
        if self.at(&TokenKind::LeftBracket) || self.at(&TokenKind::LeftBrace) {
            match self.binding_pattern() {
                Ok(pattern) => return Some(pattern),
                Err(_) => {
                    self.cursor = cursor;
                    return None;
                }
            }
        }
        let token = self.peek()?;
        let name = match &token.kind {
            TokenKind::Identifier(name) => name.clone(),
            TokenKind::Let if kind == VarKind::Var => "let".to_owned(),
            _ => return None,
        };
        let span = token.span;
        self.advance();
        Some(BindingPattern::Identifier { name, span })
    }

    fn finish_for_in_of(
        &mut self,
        start: usize,
        left: ForInLeft,
        kind: ForKind,
        is_await: Option<Span>,
    ) -> Result<Stmt, ParseError> {
        // `for await` is only valid with the `of` form.
        if kind == ForKind::In
            && let Some(await_span) = is_await
        {
            return Err(ParseError {
                message: "`for await` may not be used with a for-in loop".to_owned(),
                span: await_span,
            });
        }
        let right = if kind == ForKind::In {
            self.expression()?
        } else {
            self.assignment()?
        };
        self.expect(&TokenKind::RightParen)?;
        let body = self.statement()?;
        let end = stmt_end(&body);
        let span = Span::new(start, end);
        let body = Box::new(body);
        Ok(match kind {
            ForKind::In => Stmt::ForIn {
                left,
                right,
                body,
                span,
            },
            ForKind::Of => Stmt::ForOf {
                left,
                right,
                body,
                is_await: is_await.is_some(),
                span,
            },
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ForKind {
    In,
    Of,
}
