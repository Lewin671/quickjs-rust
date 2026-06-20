use qjs_ast::{BinaryOp, Expr, Span};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn nullish_coalescing(&mut self) -> Result<Expr, ParseError> {
        let mut left_start = self.cursor;
        let mut expr = self.logical_or()?;
        while self.at(&TokenKind::QuestionQuestion) {
            let operator = self.cursor;
            if starts_with_logical_operator(&expr)
                && !self.expression_range_is_parenthesized(left_start, operator)
            {
                return Err(ParseError {
                    message: "`??` cannot be mixed with `&&` or `||` without parentheses"
                        .to_owned(),
                    span: expr.span(),
                });
            }

            self.expect(&TokenKind::QuestionQuestion)?;
            let right_start = self.cursor;
            let right = self.logical_or()?;
            if starts_with_logical_operator(&right)
                && !self.expression_range_is_parenthesized(right_start, self.cursor)
            {
                return Err(ParseError {
                    message: "`??` cannot be mixed with `&&` or `||` without parentheses"
                        .to_owned(),
                    span: right.span(),
                });
            }

            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::NullishCoalescing,
                right: Box::new(right),
                span,
            };
            left_start = operator;
        }
        Ok(expr)
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
        if !self.allow_in {
            return self.binary_left_assoc(
                Self::shift,
                &[
                    (TokenKind::Less, BinaryOp::Lt),
                    (TokenKind::LessEqual, BinaryOp::Le),
                    (TokenKind::Greater, BinaryOp::Gt),
                    (TokenKind::GreaterEqual, BinaryOp::Ge),
                    (TokenKind::Instanceof, BinaryOp::Instanceof),
                ],
            );
        }
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

    /// Parses a `ShiftExpression`, used as the right operand of a `#x in obj`
    /// ergonomic brand check.
    pub(crate) fn shift_expression(&mut self) -> Result<Expr, ParseError> {
        self.shift()
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
        let start = self.cursor;
        let left = self.unary()?;
        if !self.match_kind(&TokenKind::StarStar) {
            return Ok(left);
        }

        // A direct unary expression (not wrapped in parentheses) as the left
        // operand of ** is a SyntaxError. We detect this by checking both the
        // AST node type AND that the expression starts at the same position as
        // our cursor entry — a parenthesized unary would have started with `(`
        // at a different token position within the primary expression path.
        if matches!(&left, Expr::Unary { .. }) {
            let first_token = self.tokens.get(start);
            let is_direct_unary = first_token.is_some_and(|token| {
                matches!(
                    token.kind,
                    TokenKind::Plus
                        | TokenKind::Minus
                        | TokenKind::Bang
                        | TokenKind::Tilde
                        | TokenKind::Typeof
                        | TokenKind::Void
                        | TokenKind::Delete
                )
            });
            if is_direct_unary {
                return Err(crate::ParseError {
                    message: "unary expression cannot be the left operand of `**`; use parentheses"
                        .to_owned(),
                    span: left.span(),
                });
            }
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

    fn expression_range_is_parenthesized(&self, start: usize, end: usize) -> bool {
        let Some(first) = self.tokens.get(start) else {
            return false;
        };
        if first.kind != TokenKind::LeftParen || end <= start + 1 {
            return false;
        }
        let Some(last) = self.tokens.get(end - 1) else {
            return false;
        };
        if last.kind != TokenKind::RightParen {
            return false;
        }

        let mut depth = 0usize;
        for index in start..end {
            match self.tokens[index].kind {
                TokenKind::LeftParen => depth += 1,
                TokenKind::RightParen => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return index == end - 1;
                    }
                }
                _ => {}
            }
        }
        false
    }
}

fn starts_with_logical_operator(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Binary {
            op: BinaryOp::LogicalAnd | BinaryOp::LogicalOr,
            ..
        }
    )
}
