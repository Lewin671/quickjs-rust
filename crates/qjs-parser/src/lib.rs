//! Parser for a small JavaScript subset.

use qjs_ast::{
    AssignmentOp, AssignmentTarget, BinaryOp, Expr, ForInit, Literal, MemberProperty,
    ObjectProperty, Script, Span, Stmt, UnaryOp, UpdateOp, VarDeclarator, VarKind,
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

        if self.at(&TokenKind::Function) {
            return self.function_declaration();
        }

        if self.at(&TokenKind::Return) {
            return self.return_statement();
        }

        if self.at(&TokenKind::Throw) {
            return Ok(self.throw_statement());
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

        let body_start = self
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

        Ok(Stmt::FunctionDecl {
            name,
            params,
            body,
            span: Span::new(start.min(body_start), end),
        })
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

    fn throw_statement(&mut self) -> Stmt {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let throw = self.advance();
        let mut end = throw.span.end;
        while !self.at(&TokenKind::Semicolon)
            && !self.at(&TokenKind::RightBrace)
            && !self.at(&TokenKind::Eof)
        {
            end = self.advance().span.end;
        }
        if self.match_kind(&TokenKind::Semicolon) {
            end = self.tokens[self.cursor - 1].span.end;
        }
        Stmt::Throw {
            span: Span::new(start, end),
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
        } else if self.match_kind(&TokenKind::SlashEqual) {
            AssignmentOp::DivAssign
        } else if self.match_kind(&TokenKind::PercentEqual) {
            AssignmentOp::RemAssign
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
            Self::unary,
            &[
                (TokenKind::Star, BinaryOp::Mul),
                (TokenKind::Slash, BinaryOp::Div),
                (TokenKind::Percent, BinaryOp::Rem),
            ],
        )
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
            TokenKind::Delete => UnaryOp::Delete,
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
                let key = match key_token.kind {
                    TokenKind::Identifier(name)
                    | TokenKind::String(name)
                    | TokenKind::Number(name) => name,
                    TokenKind::True => "true".to_owned(),
                    TokenKind::False => "false".to_owned(),
                    TokenKind::Null => "null".to_owned(),
                    _ => {
                        return Err(ParseError {
                            message: "expected property name".to_owned(),
                            span: key_token.span,
                        });
                    }
                };
                self.expect(&TokenKind::Colon)?;
                let value = self.assignment()?;
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
        TokenKind::Var => Some("var".to_owned()),
        TokenKind::Let => Some("let".to_owned()),
        TokenKind::Const => Some("const".to_owned()),
        TokenKind::If => Some("if".to_owned()),
        TokenKind::Else => Some("else".to_owned()),
        TokenKind::While => Some("while".to_owned()),
        TokenKind::Do => Some("do".to_owned()),
        TokenKind::For => Some("for".to_owned()),
        TokenKind::Break => Some("break".to_owned()),
        TokenKind::Continue => Some("continue".to_owned()),
        TokenKind::Function => Some("function".to_owned()),
        TokenKind::Return => Some("return".to_owned()),
        TokenKind::Throw => Some("throw".to_owned()),
        TokenKind::Typeof => Some("typeof".to_owned()),
        TokenKind::In => Some("in".to_owned()),
        TokenKind::Delete => Some("delete".to_owned()),
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
        | Stmt::FunctionDecl { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Throw { span }
        | Stmt::Break { span }
        | Stmt::Continue { span }
        | Stmt::VarDecl { span, .. } => span.end,
        Stmt::Empty => 0,
    }
}

#[cfg(test)]
mod tests {
    use qjs_ast::{
        AssignmentOp, AssignmentTarget, BinaryOp, Expr, ForInit, MemberProperty, Stmt, UnaryOp,
        UpdateOp, VarKind,
    };

    use super::parse_script;

    #[test]
    fn parses_binary_precedence() {
        let script = parse_script("1 + 2 * 3;").expect("source should parse");
        let [Stmt::Expr(Expr::Binary { op, .. })] = script.body.as_slice() else {
            panic!("expected one binary expression statement");
        };
        assert_eq!(*op, BinaryOp::Add);
    }

    #[test]
    fn parses_comparison_before_equality() {
        let script = parse_script("1 + 2 >= 3 === true;").expect("source should parse");
        let [Stmt::Expr(Expr::Binary { op, left, .. })] = script.body.as_slice() else {
            panic!("expected one binary expression statement");
        };
        assert_eq!(*op, BinaryOp::StrictEq);
        let Expr::Binary { op: left_op, .. } = left.as_ref() else {
            panic!("expected comparison on left side");
        };
        assert_eq!(*left_op, BinaryOp::Ge);

        let script = parse_script("'x' in object;").expect("source should parse");
        let [Stmt::Expr(Expr::Binary { op, .. })] = script.body.as_slice() else {
            panic!("expected one binary expression statement");
        };
        assert_eq!(*op, BinaryOp::In);
    }

    #[test]
    fn parses_shift_and_bitwise_precedence() {
        let script = parse_script("1 | 2 ^ 3 & 4 === 4;").expect("source should parse");
        let [Stmt::Expr(Expr::Binary { op, right, .. })] = script.body.as_slice() else {
            panic!("expected one binary expression statement");
        };
        assert_eq!(*op, BinaryOp::BitwiseOr);
        let Expr::Binary { op: right_op, .. } = right.as_ref() else {
            panic!("expected bitwise xor on right side");
        };
        assert_eq!(*right_op, BinaryOp::BitwiseXor);

        let script = parse_script("1 + 2 << 3 < 30;").expect("source should parse");
        let [Stmt::Expr(Expr::Binary { op, left, .. })] = script.body.as_slice() else {
            panic!("expected one comparison expression statement");
        };
        assert_eq!(*op, BinaryOp::Lt);
        assert!(matches!(
            left.as_ref(),
            Expr::Binary {
                op: BinaryOp::Shl,
                ..
            }
        ));
    }

    #[test]
    fn parses_logical_precedence() {
        let script = parse_script("true || false && false;").expect("source should parse");
        let [Stmt::Expr(Expr::Binary { op, right, .. })] = script.body.as_slice() else {
            panic!("expected one binary expression statement");
        };
        assert_eq!(*op, BinaryOp::LogicalOr);
        let Expr::Binary { op: right_op, .. } = right.as_ref() else {
            panic!("expected logical and on right side");
        };
        assert_eq!(*right_op, BinaryOp::LogicalAnd);
    }

    #[test]
    fn parses_nullish_coalescing_expression() {
        let script = parse_script("null ?? 1 ?? 2;").expect("source should parse");
        let [Stmt::Expr(Expr::Binary { op, left, .. })] = script.body.as_slice() else {
            panic!("expected one binary expression statement");
        };
        assert_eq!(*op, BinaryOp::NullishCoalescing);
        assert!(matches!(
            left.as_ref(),
            Expr::Binary {
                op: BinaryOp::NullishCoalescing,
                ..
            }
        ));
    }

    #[test]
    fn parses_conditional_expression_as_right_associative() {
        let script = parse_script("false ? 1 : true ? 2 : 3;").expect("source should parse");
        let [Stmt::Expr(Expr::Conditional { alternate, .. })] = script.body.as_slice() else {
            panic!("expected one conditional expression statement");
        };
        assert!(matches!(alternate.as_ref(), Expr::Conditional { .. }));
    }

    #[test]
    fn parses_sequence_expression() {
        let script = parse_script("a = 1, b = 2, b;").expect("source should parse");
        let [Stmt::Expr(Expr::Sequence { expressions, .. })] = script.body.as_slice() else {
            panic!("expected one sequence expression statement");
        };
        assert_eq!(expressions.len(), 3);
        assert!(matches!(expressions[0], Expr::Assignment { .. }));
        assert!(matches!(expressions[1], Expr::Assignment { .. }));
    }

    #[test]
    fn parses_variable_declaration() {
        let script = parse_script("let answer = 40 + 2, missing;").expect("source should parse");
        let [
            Stmt::VarDecl {
                kind, declarations, ..
            },
        ] = script.body.as_slice()
        else {
            panic!("expected one variable declaration");
        };
        assert_eq!(*kind, VarKind::Let);
        assert_eq!(declarations.len(), 2);
        assert_eq!(declarations[0].name, "answer");
        assert!(matches!(
            declarations[0].init,
            Some(Expr::Binary {
                op: BinaryOp::Add,
                ..
            })
        ));
        assert_eq!(declarations[1].name, "missing");
        assert!(declarations[1].init.is_none());
    }

    #[test]
    fn rejects_const_without_initializer() {
        let error = parse_script("const answer;").expect_err("const should require initializer");
        assert_eq!(error.message, "const declarations require an initializer");
    }

    #[test]
    fn parses_assignment_as_right_associative() {
        let script = parse_script("a = b = 1;").expect("source should parse");
        let [
            Stmt::Expr(Expr::Assignment {
                target, op, value, ..
            }),
        ] = script.body.as_slice()
        else {
            panic!("expected one assignment expression statement");
        };
        assert_eq!(*op, AssignmentOp::Assign);
        let AssignmentTarget::Identifier { name, .. } = target else {
            panic!("expected identifier assignment target");
        };
        assert_eq!(name, "a");
        let Expr::Assignment {
            target: inner_target,
            ..
        } = value.as_ref()
        else {
            panic!("expected nested assignment");
        };
        let AssignmentTarget::Identifier {
            name: inner_name, ..
        } = inner_target
        else {
            panic!("expected identifier assignment target");
        };
        assert_eq!(inner_name, "b");
    }

    #[test]
    fn parses_update_and_compound_assignment() {
        let script = parse_script("++i; i++; i += 2; obj.count--;").expect("source should parse");
        let [
            Stmt::Expr(Expr::Update {
                op: UpdateOp::Increment,
                prefix: true,
                ..
            }),
            Stmt::Expr(Expr::Update {
                op: UpdateOp::Increment,
                prefix: false,
                ..
            }),
            Stmt::Expr(Expr::Assignment {
                op: AssignmentOp::AddAssign,
                ..
            }),
            Stmt::Expr(Expr::Update {
                op: UpdateOp::Decrement,
                prefix: false,
                ..
            }),
        ] = script.body.as_slice()
        else {
            panic!("expected update and compound assignment statements");
        };
    }

    #[test]
    fn rejects_invalid_assignment_target() {
        let error = parse_script("(1 + 2) = 3;").expect_err("assignment target should fail");
        assert_eq!(error.message, "invalid assignment target");
    }

    #[test]
    fn parses_if_else_statement() {
        let script = parse_script("if (true) { let x = 1; } else { let x = 2; }")
            .expect("source should parse");
        let [
            Stmt::If {
                consequent,
                alternate,
                ..
            },
        ] = script.body.as_slice()
        else {
            panic!("expected one if statement");
        };
        assert!(matches!(consequent.as_ref(), Stmt::Block { .. }));
        assert!(matches!(alternate.as_deref(), Some(Stmt::Block { .. })));
    }

    #[test]
    fn parses_while_statement() {
        let script = parse_script("while (x < 3) { x = x + 1; }").expect("source should parse");
        let [Stmt::While { body, .. }] = script.body.as_slice() else {
            panic!("expected one while statement");
        };
        assert!(matches!(body.as_ref(), Stmt::Block { .. }));
    }

    #[test]
    fn parses_do_while_statement() {
        let script = parse_script("do { x++; } while (x < 3);").expect("source should parse");
        let [Stmt::DoWhile { body, .. }] = script.body.as_slice() else {
            panic!("expected one do-while statement");
        };
        assert!(matches!(body.as_ref(), Stmt::Block { .. }));
    }

    #[test]
    fn parses_for_statement() {
        let script =
            parse_script("for (var i = 0; i < 3; i = i + 1) { i; }").expect("source should parse");
        let [
            Stmt::For {
                init,
                test,
                update,
                body,
                ..
            },
        ] = script.body.as_slice()
        else {
            panic!("expected one for statement");
        };
        assert!(matches!(
            init,
            Some(ForInit::VarDecl { declarations, .. })
                if declarations.len() == 1 && declarations[0].name == "i"
        ));
        assert!(matches!(
            test,
            Some(Expr::Binary {
                op: BinaryOp::Lt,
                ..
            })
        ));
        assert!(matches!(update, Some(Expr::Assignment { .. })));
        assert!(matches!(body.as_ref(), Stmt::Block { .. }));
    }

    #[test]
    fn parses_break_and_continue_statements() {
        let script =
            parse_script("while (true) { continue; break; }").expect("source should parse");
        let [Stmt::While { body, .. }] = script.body.as_slice() else {
            panic!("expected one while statement");
        };
        let Stmt::Block { body, .. } = body.as_ref() else {
            panic!("expected block body");
        };
        assert!(matches!(
            body.as_slice(),
            [Stmt::Continue { .. }, Stmt::Break { .. }]
        ));
    }

    #[test]
    fn parses_throw_statement_without_throw_expression_support() {
        let script = parse_script("if (false) { throw new Test262Error('fail'); }")
            .expect("source should parse");
        let [Stmt::If { consequent, .. }] = script.body.as_slice() else {
            panic!("expected one if statement");
        };
        let Stmt::Block { body, .. } = consequent.as_ref() else {
            panic!("expected block consequent");
        };
        assert!(matches!(body.as_slice(), [Stmt::Throw { .. }]));
    }

    #[test]
    fn parses_unary_before_multiplicative() {
        let script = parse_script("-1 * !false;").expect("source should parse");
        let [Stmt::Expr(Expr::Binary { left, right, .. })] = script.body.as_slice() else {
            panic!("expected one binary expression");
        };
        assert!(matches!(
            left.as_ref(),
            Expr::Unary {
                op: UnaryOp::Minus,
                ..
            }
        ));
        assert!(matches!(
            right.as_ref(),
            Expr::Unary {
                op: UnaryOp::Not,
                ..
            }
        ));

        let script = parse_script("typeof missing;").expect("source should parse");
        let [Stmt::Expr(Expr::Unary { op, .. })] = script.body.as_slice() else {
            panic!("expected one unary expression");
        };
        assert_eq!(*op, UnaryOp::Typeof);

        let script = parse_script("delete object.key;").expect("source should parse");
        let [Stmt::Expr(Expr::Unary { op, .. })] = script.body.as_slice() else {
            panic!("expected one unary expression");
        };
        assert_eq!(*op, UnaryOp::Delete);
    }

    #[test]
    fn parses_function_declaration_and_call() {
        let script =
            parse_script("function add(a, b) { return a + b; } add(1, 2);").expect("source");
        let [
            Stmt::FunctionDecl { name, params, .. },
            Stmt::Expr(Expr::Call { arguments, .. }),
        ] = script.body.as_slice()
        else {
            panic!("expected function declaration followed by call");
        };
        assert_eq!(name, "add");
        assert_eq!(params, &["a", "b"]);
        assert_eq!(arguments.len(), 2);
    }

    #[test]
    fn parses_array_literal() {
        let script = parse_script("[1, 2 + 3,];").expect("source should parse");
        let [Stmt::Expr(Expr::Array { elements, .. })] = script.body.as_slice() else {
            panic!("expected one array expression");
        };
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn parses_object_literal_and_member_assignment() {
        let script = parse_script("let object = { answer: 42, 'name': 7, }; object.answer = 43;")
            .expect("source should parse");
        let [
            Stmt::VarDecl { declarations, .. },
            Stmt::Expr(Expr::Assignment { target, .. }),
        ] = script.body.as_slice()
        else {
            panic!("expected object declaration followed by member assignment");
        };
        let Some(Expr::Object { properties, .. }) = &declarations[0].init else {
            panic!("expected object initializer");
        };
        assert_eq!(properties.len(), 2);
        assert_eq!(properties[0].key, "answer");
        assert_eq!(properties[1].key, "name");
        assert!(matches!(target, AssignmentTarget::Member { .. }));

        let script =
            parse_script("({ true: 1, false: 2, null: 3 });").expect("source should parse");
        let [Stmt::Expr(Expr::Object { properties, .. })] = script.body.as_slice() else {
            panic!("expected one object expression");
        };
        assert_eq!(properties[0].key, "true");
        assert_eq!(properties[1].key, "false");
        assert_eq!(properties[2].key, "null");
    }

    #[test]
    fn parses_member_access() {
        let script = parse_script("items[0].length;").expect("source should parse");
        let [
            Stmt::Expr(Expr::Member {
                object, property, ..
            }),
        ] = script.body.as_slice()
        else {
            panic!("expected member expression");
        };
        assert_eq!(property, &MemberProperty::Named("length".to_owned()));
        assert!(matches!(object.as_ref(), Expr::Member { .. }));
    }
}
