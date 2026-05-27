//! Parser for a small JavaScript subset.

use qjs_ast::{BinaryOp, Expr, Literal, Script, Span, Stmt, UnaryOp, VarKind};
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

        if self.at(&TokenKind::Function) {
            return self.function_declaration();
        }

        if self.at(&TokenKind::Return) {
            return self.return_statement();
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

    fn variable_declaration(&mut self) -> Result<Stmt, ParseError> {
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

        let name_token = self.advance();
        let TokenKind::Identifier(name) = name_token.kind else {
            return Err(ParseError {
                message: "expected binding identifier".to_owned(),
                span: name_token.span,
            });
        };

        let init = if self.match_kind(&TokenKind::Equal) {
            Some(self.expression()?)
        } else {
            if kind == VarKind::Const {
                return Err(ParseError {
                    message: "const declarations require an initializer".to_owned(),
                    span: name_token.span,
                });
            }
            None
        };

        self.match_kind(&TokenKind::Semicolon);
        let end = init
            .as_ref()
            .map_or(name_token.span.end, |expr| expr.span().end);
        Ok(Stmt::VarDecl {
            kind,
            name,
            init,
            span: Span::new(start, end),
        })
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.logical_or()?;
        if !self.match_kind(&TokenKind::Equal) {
            return Ok(expr);
        }

        let (name, span) = match expr {
            Expr::Identifier { name, span } => (name, span),
            other => {
                return Err(ParseError {
                    message: "invalid assignment target".to_owned(),
                    span: other.span(),
                });
            }
        };

        let value = self.assignment()?;
        let assignment_span = Span::new(span.start, value.span().end);
        Ok(Expr::Assignment {
            name,
            value: Box::new(value),
            span: assignment_span,
        })
    }

    fn logical_or(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::logical_and,
            &[(TokenKind::PipePipe, BinaryOp::LogicalOr)],
        )
    }

    fn logical_and(&mut self) -> Result<Expr, ParseError> {
        self.binary_left_assoc(
            Self::equality,
            &[(TokenKind::AmpersandAmpersand, BinaryOp::LogicalAnd)],
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
            Self::additive,
            &[
                (TokenKind::Less, BinaryOp::Lt),
                (TokenKind::LessEqual, BinaryOp::Le),
                (TokenKind::Greater, BinaryOp::Gt),
                (TokenKind::GreaterEqual, BinaryOp::Ge),
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
        let op = match token.kind {
            TokenKind::Plus => UnaryOp::Plus,
            TokenKind::Minus => UnaryOp::Minus,
            TokenKind::Bang => UnaryOp::Not,
            _ => return self.call(),
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

    fn call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        while self.match_kind(&TokenKind::LeftParen) {
            let mut arguments = Vec::new();
            if !self.at(&TokenKind::RightParen) {
                loop {
                    arguments.push(self.expression()?);
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

fn stmt_end(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Expr(expr) => expr.span().end,
        Stmt::Block { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::FunctionDecl { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::VarDecl { span, .. } => span.end,
        Stmt::Empty => 0,
    }
}

#[cfg(test)]
mod tests {
    use qjs_ast::{BinaryOp, Expr, Stmt, UnaryOp, VarKind};

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
    fn parses_variable_declaration() {
        let script = parse_script("let answer = 40 + 2;").expect("source should parse");
        let [
            Stmt::VarDecl {
                kind, name, init, ..
            },
        ] = script.body.as_slice()
        else {
            panic!("expected one variable declaration");
        };
        assert_eq!(*kind, VarKind::Let);
        assert_eq!(name, "answer");
        assert!(matches!(
            init,
            Some(Expr::Binary {
                op: BinaryOp::Add,
                ..
            })
        ));
    }

    #[test]
    fn rejects_const_without_initializer() {
        let error = parse_script("const answer;").expect_err("const should require initializer");
        assert_eq!(error.message, "const declarations require an initializer");
    }

    #[test]
    fn parses_assignment_as_right_associative() {
        let script = parse_script("a = b = 1;").expect("source should parse");
        let [Stmt::Expr(Expr::Assignment { name, value, .. })] = script.body.as_slice() else {
            panic!("expected one assignment expression statement");
        };
        assert_eq!(name, "a");
        let Expr::Assignment {
            name: inner_name, ..
        } = value.as_ref()
        else {
            panic!("expected nested assignment");
        };
        assert_eq!(inner_name, "b");
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
}
