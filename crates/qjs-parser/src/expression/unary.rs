use qjs_ast::{Expr, Span, UnaryOp, UpdateOp};
use qjs_lexer::TokenKind;

use crate::helpers::assignment_target;
use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn unary(&mut self) -> Result<Expr, ParseError> {
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
        let callee = self.member_chain()?;
        let mut expr = if self.match_kind(&TokenKind::LeftParen) {
            let call = self.finish_call(callee)?;
            let Expr::Call {
                callee,
                arguments,
                span,
            } = call
            else {
                unreachable!("finish_call must produce a call expression");
            };
            Expr::New {
                callee,
                arguments,
                span: Span::new(start, span.end),
            }
        } else {
            let end = callee.span().end;
            Expr::New {
                callee: Box::new(callee),
                arguments: Vec::new(),
                span: Span::new(start, end),
            }
        };
        expr = self.finish_call_member_chain(expr)?;
        Ok(expr)
    }
}
