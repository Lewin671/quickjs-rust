//! Parser for a small JavaScript subset.

use qjs_ast::{
    AssignmentOp, AssignmentTarget, BinaryOp, CatchClause, Expr, ForInLeft, ForInit, Literal,
    MemberProperty, ObjectProperty, ObjectPropertyKey, Script, Span, Stmt, SwitchCase, UnaryOp,
    UpdateOp, VarDeclarator, VarKind,
};
use qjs_lexer::{Token, TokenKind, lex};

/// A parse error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    /// Human-readable message.
    pub message: String,
    /// Source span.
    pub span: Span,
}

/// Parses source text into a script AST.
///
/// # Errors
///
/// Returns a structured error for lexing or parsing failures.
pub fn parse_script(source: &str) -> Result<Script, ParseError> {
    let tokens = lex(source).map_err(|error| ParseError {
        message: error.message,
        span: error.span,
    })?;
    Parser::new(tokens).parse_script()
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn parse_script(&mut self) -> Result<Script, ParseError> {
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

    fn block_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
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

    fn function_expression(&mut self, start: usize) -> Result<Expr, ParseError> {
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

    fn function_parameters(&mut self) -> Result<Vec<String>, ParseError> {
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

    fn expression(&mut self) -> Result<Expr, ParseError> {
        let first = self.assignment()?;
        if !self.match_kind(&TokenKind::Comma) {
            return Ok(first);
        }

        let start = first.span().start;
        let mut expressions = vec![first, self.assignment()?];
        while self.match_kind(&TokenKind::Comma) {
            expressions.push(self.assignment()?);
        }
        let end = expressions
            .last()
            .expect("sequence expression should have expressions")
            .span()
            .end;
        Ok(Expr::Sequence {
            expressions,
            span: Span::new(start, end),
        })
    }

    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.conditional()?;
        let op = if self.match_kind(&TokenKind::Equal) {
            AssignmentOp::Assign
        } else if self.match_kind(&TokenKind::PlusEqual) {
            AssignmentOp::AddAssign
        } else if self.match_kind(&TokenKind::MinusEqual) {
            AssignmentOp::SubAssign
        } else if self.match_kind(&TokenKind::StarEqual) {
            AssignmentOp::MulAssign
        } else if self.match_kind(&TokenKind::StarStarEqual) {
            AssignmentOp::PowAssign
        } else if self.match_kind(&TokenKind::SlashEqual) {
            AssignmentOp::DivAssign
        } else if self.match_kind(&TokenKind::PercentEqual) {
            AssignmentOp::RemAssign
        } else if self.match_kind(&TokenKind::LessLessEqual) {
            AssignmentOp::ShlAssign
        } else if self.match_kind(&TokenKind::GreaterGreaterEqual) {
            AssignmentOp::ShrAssign
        } else if self.match_kind(&TokenKind::GreaterGreaterGreaterEqual) {
            AssignmentOp::UShrAssign
        } else if self.match_kind(&TokenKind::AmpersandEqual) {
            AssignmentOp::BitwiseAndAssign
        } else if self.match_kind(&TokenKind::CaretEqual) {
            AssignmentOp::BitwiseXorAssign
        } else if self.match_kind(&TokenKind::PipeEqual) {
            AssignmentOp::BitwiseOrAssign
        } else if self.match_kind(&TokenKind::AmpersandAmpersandEqual) {
            AssignmentOp::LogicalAndAssign
        } else if self.match_kind(&TokenKind::PipePipeEqual) {
            AssignmentOp::LogicalOrAssign
        } else if self.match_kind(&TokenKind::QuestionQuestionEqual) {
            AssignmentOp::NullishAssign
        } else {
            return Ok(expr);
        };

        let target = assignment_target(expr)?;

        let value = self.assignment()?;
        let assignment_span = Span::new(target.span().start, value.span().end);
        Ok(Expr::Assignment {
            target,
            op,
            value: Box::new(value),
            span: assignment_span,
        })
    }

    fn conditional(&mut self) -> Result<Expr, ParseError> {
        let test = self.nullish_coalescing()?;
        if !self.match_kind(&TokenKind::Question) {
            return Ok(test);
        }

        let consequent = self.assignment()?;
        self.expect(&TokenKind::Colon)?;
        let alternate = self.assignment()?;
        let span = Span::new(test.span().start, alternate.span().end);
        Ok(Expr::Conditional {
            test: Box::new(test),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
            span,
        })
    }

    fn nullish_coalescing(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::logical_or,
            &[(TokenKind::QuestionQuestion, BinaryOp::NullishCoalescing)],
        )
    }

    fn logical_or(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::logical_and,
            &[(TokenKind::PipePipe, BinaryOp::LogicalOr)],
        )
    }

    fn logical_and(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::bitwise_or,
            &[(TokenKind::AmpersandAmpersand, BinaryOp::LogicalAnd)],
        )
    }

    fn bitwise_or(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(Self::bitwise_xor, &[(TokenKind::Pipe, BinaryOp::BitwiseOr)])
    }

    fn bitwise_xor(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::bitwise_and,
            &[(TokenKind::Caret, BinaryOp::BitwiseXor)],
        )
    }

    fn bitwise_and(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::equality,
            &[(TokenKind::Ampersand, BinaryOp::BitwiseAnd)],
        )
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::comparison,
            &[
                (TokenKind::EqualEqual, BinaryOp::Eq),
                (TokenKind::EqualEqualEqual, BinaryOp::StrictEq),
                (TokenKind::BangEqual, BinaryOp::Ne),
                (TokenKind::BangEqualEqual, BinaryOp::StrictNe),
            ],
        )
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::shift,
            &[
                (TokenKind::Less, BinaryOp::Lt),
                (TokenKind::LessEqual, BinaryOp::Le),
                (TokenKind::Greater, BinaryOp::Gt),
                (TokenKind::GreaterEqual, BinaryOp::Ge),
                (TokenKind::In, BinaryOp::In),
                (TokenKind::Instanceof, BinaryOp::Instanceof),
            ],
        )
    }

    fn shift(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::additive,
            &[
                (TokenKind::LessLess, BinaryOp::Shl),
                (TokenKind::GreaterGreater, BinaryOp::Shr),
                (TokenKind::GreaterGreaterGreater, BinaryOp::UShr),
            ],
        )
    }

    fn additive(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::multiplicative,
            &[
                (TokenKind::Plus, BinaryOp::Add),
                (TokenKind::Minus, BinaryOp::Sub),
            ],
        )
    }

    fn multiplicative(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::exponentiation,
            &[
                (TokenKind::Star, BinaryOp::Mul),
                (TokenKind::Slash, BinaryOp::Div),
                (TokenKind::Percent, BinaryOp::Rem),
            ],
        )
    }

    fn exponentiation(&mut self) -> Result<Expr, ParseError> {
        let left = self.unary()?;
        if !self.match_kind(&TokenKind::StarStar) {
            return Ok(left);
        }

        let right = self.exponentiation()?;
        let span = Span::new(left.span().start, right.span().end);
        Ok(Expr::Binary {
            left: Box::new(left),
            op: BinaryOp::Pow,
            right: Box::new(right),
            span,
        })
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        let token = self
            .peek()
            .expect("parser should always have eof token")
            .clone();
        if token.kind == TokenKind::PlusPlus || token.kind == TokenKind::MinusMinus {
            self.advance();
            let target = assignment_target(self.unary()?)?;
            let span = Span::new(token.span.start, target.span().end);
            return Ok(Expr::Update {
                target,
                op: if token.kind == TokenKind::PlusPlus {
                    UpdateOp::Increment
                } else {
                    UpdateOp::Decrement
                },
                prefix: true,
                span,
            });
        }

        let op = match token.kind {
            TokenKind::Plus => UnaryOp::Plus,
            TokenKind::Minus => UnaryOp::Minus,
            TokenKind::Bang => UnaryOp::Not,
            TokenKind::Tilde => UnaryOp::BitwiseNot,
            TokenKind::Typeof => UnaryOp::Typeof,
            TokenKind::Void => UnaryOp::Void,
            TokenKind::Delete => UnaryOp::Delete,
            TokenKind::New => return self.new_expression(token.span.start),
            _ => return self.postfix(),
        };
        self.advance();
        let argument = self.unary()?;
        let span = Span::new(token.span.start, argument.span().end);
        Ok(Expr::Unary {
            op,
            argument: Box::new(argument),
            span,
        })
    }

    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let expr = self.call()?;
        let Some(token) = self.peek().cloned() else {
            return Ok(expr);
        };
        let op = match token.kind {
            TokenKind::PlusPlus => UpdateOp::Increment,
            TokenKind::MinusMinus => UpdateOp::Decrement,
            _ => return Ok(expr),
        };
        self.advance();
        let start = expr.span().start;
        let target = assignment_target(expr)?;
        Ok(Expr::Update {
            target,
            op,
            prefix: false,
            span: Span::new(start, token.span.end),
        })
    }

    fn new_expression(&mut self, start: usize) -> Result<Expr, ParseError> {
        self.expect(&TokenKind::New)?;
        let expr = self.call()?;
        let (callee, arguments, end) = match expr {
            Expr::Call {
                callee,
                arguments,
                span,
            } => (callee, arguments, span.end),
            other => {
                let end = other.span().end;
                (Box::new(other), Vec::new(), end)
            }
        };
        Ok(Expr::New {
            callee,
            arguments,
            span: Span::new(start, end),
        })
    }

    fn binary_left_assoc(
        &mut self,
        next: fn(&mut Self) -> Result<Expr, ParseError>,
        operators: &[(TokenKind, BinaryOp)],
    ) -> Result<Expr, ParseError> {
        let mut expr = next(self)?;
        while let Some((kind, op)) = operators.iter().find(|(kind, _)| self.at(kind)) {
            self.expect(kind)?;
            let right = next(self)?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op: *op,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier(name) => Ok(Expr::Identifier {
                name,
                span: token.span,
            }),
            TokenKind::Number(raw) => Ok(Expr::Literal(Literal::Number {
                raw,
                span: token.span,
            })),
            TokenKind::String(value) => Ok(Expr::Literal(Literal::String {
                value,
                span: token.span,
            })),
            TokenKind::True => Ok(Expr::Literal(Literal::Boolean {
                value: true,
                span: token.span,
            })),
            TokenKind::False => Ok(Expr::Literal(Literal::Boolean {
                value: false,
                span: token.span,
            })),
            TokenKind::Null => Ok(Expr::Literal(Literal::Null { span: token.span })),
            TokenKind::This => Ok(Expr::This { span: token.span }),
            TokenKind::Function => self.function_expression(token.span.start),
            TokenKind::LeftBracket => self.array_literal(token.span.start),
            TokenKind::LeftBrace => self.object_literal(token.span.start),
            TokenKind::LeftParen => {
                let expr = self.expression()?;
                self.expect(&TokenKind::RightParen)?;
                Ok(expr)
            }
            _ => Err(ParseError {
                message: "expected expression".to_owned(),
                span: token.span,
            }),
        }
    }

    fn array_literal(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut elements = Vec::new();
        if !self.at(&TokenKind::RightBracket) {
            loop {
                elements.push(self.assignment()?);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RightBracket) {
                    break;
                }
            }
        }
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBracket)?;
        Ok(Expr::Array {
            elements,
            span: Span::new(start, end),
        })
    }

    fn object_literal(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut properties = Vec::new();
        if !self.at(&TokenKind::RightBrace) {
            loop {
                let key_token = self.advance();
                let (key, shorthand_value) = match key_token.kind {
                    TokenKind::Identifier(name) => {
                        let value = Expr::Identifier {
                            name: name.clone(),
                            span: key_token.span,
                        };
                        (ObjectPropertyKey::Literal(name), Some(value))
                    }
                    TokenKind::String(name) | TokenKind::Number(name) => {
                        (ObjectPropertyKey::Literal(name), None)
                    }
                    TokenKind::True => (ObjectPropertyKey::Literal("true".to_owned()), None),
                    TokenKind::False => (ObjectPropertyKey::Literal("false".to_owned()), None),
                    TokenKind::Null => (ObjectPropertyKey::Literal("null".to_owned()), None),
                    TokenKind::LeftBracket => {
                        let name = self.assignment()?;
                        self.expect(&TokenKind::RightBracket)?;
                        (ObjectPropertyKey::Computed(name), None)
                    }
                    _ => {
                        return Err(ParseError {
                            message: "expected property name".to_owned(),
                            span: key_token.span,
                        });
                    }
                };
                let value = if self.at(&TokenKind::LeftParen) {
                    let method_start = key_token.span.start;
                    let method_name = match &key {
                        ObjectPropertyKey::Literal(name) => Some(name.clone()),
                        ObjectPropertyKey::Computed(_) => None,
                    };
                    let params = self.function_parameters()?;
                    let body = self.block_body()?;
                    let end = self
                        .tokens
                        .get(self.cursor.saturating_sub(1))
                        .expect("parser should always have eof token")
                        .span
                        .end;
                    Expr::Function {
                        name: method_name,
                        params,
                        body,
                        constructable: false,
                        span: Span::new(method_start, end),
                    }
                } else if self.match_kind(&TokenKind::Colon) {
                    self.assignment()?
                } else if let Some(value) = shorthand_value {
                    value
                } else {
                    return Err(ParseError {
                        message: "expected `:` after property name".to_owned(),
                        span: match &key {
                            ObjectPropertyKey::Literal(_) => key_token.span,
                            ObjectPropertyKey::Computed(expr) => expr.span(),
                        },
                    });
                };
                let span = Span::new(key_token.span.start, value.span().end);
                properties.push(ObjectProperty { key, value, span });
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RightBrace) {
                    break;
                }
            }
        }
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBrace)?;
        Ok(Expr::Object {
            properties,
            span: Span::new(start, end),
        })
    }

    fn call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        loop {
            if self.match_kind(&TokenKind::LeftParen) {
                let mut arguments = Vec::new();
                if !self.at(&TokenKind::RightParen) {
                    loop {
                        arguments.push(self.assignment()?);
                        if !self.match_kind(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                let end = self
                    .peek()
                    .expect("parser should always have eof token")
                    .span
                    .end;
                self.expect(&TokenKind::RightParen)?;
                let span = Span::new(expr.span().start, end);
                expr = Expr::Call {
                    callee: Box::new(expr),
                    arguments,
                    span,
                };
                continue;
            }

            if self.match_kind(&TokenKind::LeftBracket) {
                let property = self.expression()?;
                let end = self
                    .peek()
                    .expect("parser should always have eof token")
                    .span
                    .end;
                self.expect(&TokenKind::RightBracket)?;
                let span = Span::new(expr.span().start, end);
                expr = Expr::Member {
                    object: Box::new(expr),
                    property: MemberProperty::Computed(Box::new(property)),
                    span,
                };
                continue;
            }

            if self.match_kind(&TokenKind::Dot) {
                let property_token = self.advance();
                let Some(name) = property_name(property_token.kind) else {
                    return Err(ParseError {
                        message: "expected property name".to_owned(),
                        span: property_token.span,
                    });
                };
                let span = Span::new(expr.span().start, property_token.span.end);
                expr = Expr::Member {
                    object: Box::new(expr),
                    property: MemberProperty::Named(name),
                    span,
                };
                continue;
            }

            break;
        }
        Ok(expr)
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek().is_some_and(|token| token.kind == *kind)
    }

    fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.cursor += 1;
            return true;
        }
        false
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<(), ParseError> {
        if self.match_kind(kind) {
            Ok(())
        } else {
            let token = self.peek().expect("parser should always have eof token");
            Err(ParseError {
                message: format!("expected `{kind:?}`"),
                span: token.span,
            })
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn advance(&mut self) -> Token {
        let token = self
            .peek()
            .expect("parser should always have eof token")
            .clone();
        self.cursor += 1;
        token
    }
}

fn property_name(kind: TokenKind) -> Option<String> {
    match kind {
        TokenKind::Identifier(name) => Some(name),
        TokenKind::True => Some("true".to_owned()),
        TokenKind::False => Some("false".to_owned()),
        TokenKind::Null => Some("null".to_owned()),
        TokenKind::This => Some("this".to_owned()),
        TokenKind::Var => Some("var".to_owned()),
        TokenKind::Let => Some("let".to_owned()),
        TokenKind::Const => Some("const".to_owned()),
        TokenKind::If => Some("if".to_owned()),
        TokenKind::Else => Some("else".to_owned()),
        TokenKind::While => Some("while".to_owned()),
        TokenKind::Do => Some("do".to_owned()),
        TokenKind::For => Some("for".to_owned()),
        TokenKind::Switch => Some("switch".to_owned()),
        TokenKind::Case => Some("case".to_owned()),
        TokenKind::Default => Some("default".to_owned()),
        TokenKind::Try => Some("try".to_owned()),
        TokenKind::Catch => Some("catch".to_owned()),
        TokenKind::Finally => Some("finally".to_owned()),
        TokenKind::Break => Some("break".to_owned()),
        TokenKind::Continue => Some("continue".to_owned()),
        TokenKind::Function => Some("function".to_owned()),
        TokenKind::Return => Some("return".to_owned()),
        TokenKind::Throw => Some("throw".to_owned()),
        TokenKind::Debugger => Some("debugger".to_owned()),
        TokenKind::Typeof => Some("typeof".to_owned()),
        TokenKind::Void => Some("void".to_owned()),
        TokenKind::In => Some("in".to_owned()),
        TokenKind::Delete => Some("delete".to_owned()),
        TokenKind::Instanceof => Some("instanceof".to_owned()),
        _ => None,
    }
}

fn var_kind(kind: &TokenKind) -> Option<VarKind> {
    match kind {
        TokenKind::Var => Some(VarKind::Var),
        TokenKind::Let => Some(VarKind::Let),
        TokenKind::Const => Some(VarKind::Const),
        _ => None,
    }
}

fn assignment_target(expr: Expr) -> Result<AssignmentTarget, ParseError> {
    match expr {
        Expr::Identifier { name, span } => Ok(AssignmentTarget::Identifier { name, span }),
        Expr::Member {
            object,
            property,
            span,
        } => Ok(AssignmentTarget::Member {
            object,
            property,
            span,
        }),
        other => Err(ParseError {
            message: "invalid assignment target".to_owned(),
            span: other.span(),
        }),
    }
}

fn stmt_end(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Expr(expr) => expr.span().end,
        Stmt::Block { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoWhile { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForIn { span, .. }
        | Stmt::Switch { span, .. }
        | Stmt::Try { span, .. }
        | Stmt::FunctionDecl { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Throw { span, .. }
        | Stmt::Debugger { span }
        | Stmt::Break { span }
        | Stmt::Continue { span }
        | Stmt::VarDecl { span, .. } => span.end,
        Stmt::Empty => 0,
    }
}

#[cfg(test)]
mod tests;
