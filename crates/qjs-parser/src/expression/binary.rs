use qjs_ast::{BinaryOp, Expr, Span};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn nullish_coalescing(&mut self) -> Result<Expr, ParseError> {
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
}
