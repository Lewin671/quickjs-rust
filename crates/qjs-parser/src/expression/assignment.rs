use qjs_ast::{AssignmentOp, Expr, Span, Stmt};
use qjs_lexer::TokenKind;

use crate::helpers::assignment_target;
use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn assignment(&mut self) -> Result<Expr, ParseError> {
        if let Some(arrow) = self.arrow_function()? {
            return Ok(arrow);
        }

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

    fn arrow_function(&mut self) -> Result<Option<Expr>, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let Some(params) = self.arrow_parameters()? else {
            return Ok(None);
        };
        self.expect(&TokenKind::Arrow)?;
        let body = if self.at(&TokenKind::LeftBrace) {
            self.block_body()?
        } else {
            let expr = self.assignment()?;
            let span = expr.span();
            vec![Stmt::Return {
                argument: Some(expr),
                span,
            }]
        };
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;
        Ok(Some(Expr::Function {
            name: None,
            params,
            body,
            constructable: false,
            span: Span::new(start, end),
        }))
    }

    fn arrow_parameters(&mut self) -> Result<Option<Vec<String>>, ParseError> {
        match self.peek().map(|token| &token.kind) {
            Some(TokenKind::Identifier(_))
                if self
                    .peek_nth(1)
                    .is_some_and(|token| token.kind == TokenKind::Arrow) =>
            {
                let token = self.advance();
                let TokenKind::Identifier(param) = token.kind else {
                    unreachable!("peek checked identifier");
                };
                Ok(Some(vec![param]))
            }
            Some(TokenKind::LeftParen) => self.parenthesized_arrow_parameters(),
            _ => Ok(None),
        }
    }

    fn parenthesized_arrow_parameters(&mut self) -> Result<Option<Vec<String>>, ParseError> {
        let start_cursor = self.cursor;
        self.expect(&TokenKind::LeftParen)?;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RightParen) {
            loop {
                let token = self.advance();
                let TokenKind::Identifier(param) = token.kind else {
                    self.cursor = start_cursor;
                    return Ok(None);
                };
                params.push(param);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        if !self.match_kind(&TokenKind::RightParen) {
            self.cursor = start_cursor;
            return Ok(None);
        }
        if !self.at(&TokenKind::Arrow) {
            self.cursor = start_cursor;
            return Ok(None);
        }
        Ok(Some(params))
    }

    pub(crate) fn conditional(&mut self) -> Result<Expr, ParseError> {
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
}
