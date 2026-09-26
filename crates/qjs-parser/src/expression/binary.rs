use qjs_ast::{BinaryOp, Expr, Span};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

/// Binary precedence levels, loosest first; the levels between these are
/// `&&`, `|`, `^`, `&` after `||`, and `+ -`, `* / %` after the shifts.
const LOGICAL_OR: u8 = 1;
const EQUALITY: u8 = 6;
const RELATIONAL: u8 = 7;
const SHIFT: u8 = 8;

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
        self.binary_at_least(LOGICAL_OR)
    }

    /// Parses a `ShiftExpression`, used as the right operand of a `#x in obj`
    /// ergonomic brand check.
    pub(crate) fn shift_expression(&mut self) -> Result<Expr, ParseError> {
        self.binary_at_least(SHIFT)
    }

    /// The left-associative binary levels from `||` down to `*`, by
    /// precedence climbing: an operand is parsed once and each operator
    /// binds by its level. The grammar's one-function-per-level descent
    /// built the same trees but passed every operand up through all ten
    /// levels, each returning the expression by value and testing its own
    /// operator list -- most of parsing a large script.
    fn binary_at_least(&mut self, minimum: u8) -> Result<Expr, ParseError> {
        let mut expr = self.exponentiation()?;
        while let Some((op, precedence)) = self.binary_operator()
            && precedence >= minimum
        {
            self.cursor += 1;
            let right = self.binary_at_least(precedence + 1)?;
            let span = Span::new(expr.span().start, right.span().end);
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    /// The binary operator at the cursor and its precedence; `in` only
    /// where the grammar's `[In]` parameter allows it.
    fn binary_operator(&self) -> Option<(BinaryOp, u8)> {
        let operator = match self.peek()?.kind {
            TokenKind::PipePipe => (BinaryOp::LogicalOr, LOGICAL_OR),
            TokenKind::AmpersandAmpersand => (BinaryOp::LogicalAnd, LOGICAL_OR + 1),
            TokenKind::Pipe => (BinaryOp::BitwiseOr, LOGICAL_OR + 2),
            TokenKind::Caret => (BinaryOp::BitwiseXor, LOGICAL_OR + 3),
            TokenKind::Ampersand => (BinaryOp::BitwiseAnd, LOGICAL_OR + 4),
            TokenKind::EqualEqual => (BinaryOp::Eq, EQUALITY),
            TokenKind::EqualEqualEqual => (BinaryOp::StrictEq, EQUALITY),
            TokenKind::BangEqual => (BinaryOp::Ne, EQUALITY),
            TokenKind::BangEqualEqual => (BinaryOp::StrictNe, EQUALITY),
            TokenKind::Less => (BinaryOp::Lt, RELATIONAL),
            TokenKind::LessEqual => (BinaryOp::Le, RELATIONAL),
            TokenKind::Greater => (BinaryOp::Gt, RELATIONAL),
            TokenKind::GreaterEqual => (BinaryOp::Ge, RELATIONAL),
            TokenKind::Instanceof => (BinaryOp::Instanceof, RELATIONAL),
            TokenKind::In if self.in_allowed() => (BinaryOp::In, RELATIONAL),
            TokenKind::LessLess => (BinaryOp::Shl, SHIFT),
            TokenKind::GreaterGreater => (BinaryOp::Shr, SHIFT),
            TokenKind::GreaterGreaterGreater => (BinaryOp::UShr, SHIFT),
            TokenKind::Plus => (BinaryOp::Add, SHIFT + 1),
            TokenKind::Minus => (BinaryOp::Sub, SHIFT + 1),
            TokenKind::Star => (BinaryOp::Mul, SHIFT + 2),
            TokenKind::Slash => (BinaryOp::Div, SHIFT + 2),
            TokenKind::Percent => (BinaryOp::Rem, SHIFT + 2),
            _ => return None,
        };
        Some(operator)
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
