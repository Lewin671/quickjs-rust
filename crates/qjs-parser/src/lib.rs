//! Parser for a small JavaScript subset.

use qjs_ast::{BinaryOp, Expr, Literal, Script, Span, Stmt};
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

        let expr = self.expression()?;
        self.match_kind(&TokenKind::Semicolon);
        Ok(Stmt::Expr(expr))
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.logical_or()
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
            Self::primary,
            &[
                (TokenKind::Star, BinaryOp::Mul),
                (TokenKind::Slash, BinaryOp::Div),
                (TokenKind::Percent, BinaryOp::Rem),
            ],
        )
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

#[cfg(test)]
mod tests {
    use qjs_ast::{BinaryOp, Expr, Stmt};

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
}
