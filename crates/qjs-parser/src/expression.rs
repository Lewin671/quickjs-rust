use qjs_ast::{AssignmentOp, BinaryOp, Expr, Span, UnaryOp, UpdateOp};
use qjs_lexer::TokenKind;

use crate::helpers::assignment_target;
use crate::{ParseError, Parser};

mod access;
mod primary;

impl Parser {
    pub(crate) fn expression(&mut self) -> Result<Expr, ParseError> {
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

    pub(crate) fn assignment(&mut self) -> Result<Expr, ParseError> {
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
}
