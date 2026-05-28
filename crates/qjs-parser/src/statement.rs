use qjs_ast::{
    CatchClause, Expr, ForInLeft, ForInit, Script, Span, Stmt, SwitchCase, VarDeclarator, VarKind,
};
use qjs_lexer::{Token, TokenKind};

use crate::helpers::{assignment_target, stmt_end, var_kind};
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

        if self.at(&TokenKind::Var) || self.at(&TokenKind::Let) || self.at(&TokenKind::Const) {
            return self.variable_declaration();
        }

        let expr = self.expression()?;
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::Expr(expr))
    }

    fn block_statement(&mut self) -> Result<Stmt, ParseError> {
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

    fn if_statement(&mut self) -> Result<Stmt, ParseError> {
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

    fn while_statement(&mut self) -> Result<Stmt, ParseError> {
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

    fn do_while_statement(&mut self) -> Result<Stmt, ParseError> {
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

    fn for_statement(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::For)?;
        self.expect(&TokenKind::LeftParen)?;
        if self.at(&TokenKind::Var) || self.at(&TokenKind::Let) || self.at(&TokenKind::Const) {
            let kind_token = self.advance();
            let kind = var_kind(&kind_token.kind).expect("token should be declaration kind");
            let name_token = self.advance();
            let TokenKind::Identifier(name) = name_token.kind else {
                return Err(ParseError {
                    message: "expected binding identifier".to_owned(),
                    span: name_token.span,
                });
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
                        span: Span::new(kind_token.span.start, name_token.span.end),
                    },
                    right,
                    body: Box::new(body),
                    span: Span::new(start, end),
                });
            }
            self.cursor -= 2;
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

    fn switch_statement(&mut self) -> Result<Stmt, ParseError> {
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

    fn try_statement(&mut self) -> Result<Stmt, ParseError> {
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

    pub(crate) fn block_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.expect(&TokenKind::LeftBrace)?;
        let mut body = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            body.push(self.statement()?);
        }
        self.expect(&TokenKind::RightBrace)?;
        Ok(body)
    }

    fn function_declaration(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Function)?;
        let name_token = self.advance();
        let TokenKind::Identifier(name) = name_token.kind else {
            return Err(ParseError {
                message: "expected function name".to_owned(),
                span: name_token.span,
            });
        };

        let params = self.function_parameters()?;
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let body = self.block_body()?;
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;

        Ok(Stmt::FunctionDecl {
            name,
            params,
            body,
            span: Span::new(start.min(body_start), end),
        })
    }

    pub(crate) fn function_expression(&mut self, start: usize) -> Result<Expr, ParseError> {
        let name = if let Some(Token {
            kind: TokenKind::Identifier(_),
            ..
        }) = self.peek()
        {
            let token = self.advance();
            let TokenKind::Identifier(name) = token.kind else {
                unreachable!("peek checked identifier");
            };
            Some(name)
        } else {
            None
        };

        let params = self.function_parameters()?;
        let body = self.block_body()?;
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;
        Ok(Expr::Function {
            name,
            params,
            body,
            constructable: true,
            span: Span::new(start, end),
        })
    }

    pub(crate) fn function_parameters(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(&TokenKind::LeftParen)?;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RightParen) {
            loop {
                let param_token = self.advance();
                let TokenKind::Identifier(param) = param_token.kind else {
                    return Err(ParseError {
                        message: "expected parameter name".to_owned(),
                        span: param_token.span,
                    });
                };
                params.push(param);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RightParen)?;
        Ok(params)
    }

    fn return_statement(&mut self) -> Result<Stmt, ParseError> {
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

    fn throw_statement(&mut self) -> Result<Stmt, ParseError> {
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

    fn debugger_statement(&mut self) -> Stmt {
        let token = self.advance();
        self.match_kind(&TokenKind::Semicolon);
        let end = self.tokens[self.cursor.saturating_sub(1)].span.end;
        Stmt::Debugger {
            span: Span::new(token.span.start, end),
        }
    }

    fn break_or_continue_statement(&mut self, kind: TokenKind) -> Stmt {
        let token = self.advance();
        self.match_kind(&TokenKind::Semicolon);
        let end = self.tokens[self.cursor.saturating_sub(1)].span.end;
        let span = Span::new(token.span.start, end);
        if kind == TokenKind::Break {
            Stmt::Break { span }
        } else {
            Stmt::Continue { span }
        }
    }

    fn variable_declaration(&mut self) -> Result<Stmt, ParseError> {
        let ForInit::VarDecl {
            kind,
            declarations,
            span,
        } = self.for_variable_declaration()?
        else {
            unreachable!("for variable declaration helper always returns VarDecl");
        };
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::VarDecl {
            kind,
            declarations,
            span,
        })
    }

    fn for_variable_declaration(&mut self) -> Result<ForInit, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let kind = if self.match_kind(&TokenKind::Var) {
            VarKind::Var
        } else if self.match_kind(&TokenKind::Let) {
            VarKind::Let
        } else {
            self.expect(&TokenKind::Const)?;
            VarKind::Const
        };

        let declarations = self.variable_declarator_list(kind)?;
        let end = declarations.last().map_or(start, |decl| decl.span.end);
        Ok(ForInit::VarDecl {
            kind,
            declarations,
            span: Span::new(start, end),
        })
    }

    fn variable_declarator_list(
        &mut self,
        kind: VarKind,
    ) -> Result<Vec<VarDeclarator>, ParseError> {
        let mut declarations = Vec::new();
        loop {
            let name_token = self.advance();
            let TokenKind::Identifier(name) = name_token.kind else {
                return Err(ParseError {
                    message: "expected binding identifier".to_owned(),
                    span: name_token.span,
                });
            };

            let init = if self.match_kind(&TokenKind::Equal) {
                Some(self.assignment()?)
            } else {
                if kind == VarKind::Const {
                    return Err(ParseError {
                        message: "const declarations require an initializer".to_owned(),
                        span: name_token.span,
                    });
                }
                None
            };
            let end = init
                .as_ref()
                .map_or(name_token.span.end, |expr| expr.span().end);
            declarations.push(VarDeclarator {
                name,
                init,
                span: Span::new(name_token.span.start, end),
            });

            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        Ok(declarations)
    }
}
