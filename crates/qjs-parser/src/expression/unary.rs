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
        if let TokenKind::PrivateName(name) = &token.kind {
            return self.private_in_expression(name.clone(), token.span);
        }
        // `await UnaryExpression` is only a keyword inside an async function
        // body. In an async parameter list it is an early error; an ordinary
        // nested function resets the async context, so `await` is an identifier
        // there again.
        if self.in_async && matches!(&token.kind, TokenKind::Identifier(name) if name == "await") {
            return self.await_expression(token.span);
        }
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
        // `delete obj.#x` (a private member reference) is a syntax error.
        if op == UnaryOp::Delete
            && matches!(
                &argument,
                Expr::Member {
                    property: qjs_ast::MemberProperty::Private(_),
                    ..
                }
            )
        {
            return Err(ParseError {
                message: "cannot delete a private member".to_owned(),
                span: Span::new(token.span.start, argument.span().end),
            });
        }
        let span = Span::new(token.span.start, argument.span().end);
        Ok(Expr::Unary {
            op,
            argument: Box::new(argument),
            span,
        })
    }

    /// Parses an `await UnaryExpression` expression. The caller has confirmed
    /// an async context and an `await` token, which is the current token.
    fn await_expression(&mut self, await_span: Span) -> Result<Expr, ParseError> {
        // `await` is an early error in an async function's parameter list.
        if self.in_async_params {
            return Err(ParseError {
                message: "`await` is not allowed in async function parameters".to_owned(),
                span: await_span,
            });
        }
        self.advance();
        let argument = self.unary()?;
        let span = Span::new(await_span.start, argument.span().end);
        Ok(Expr::Await {
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

    /// Parses an ergonomic brand check `#name in ShiftExpression`. A private
    /// name in any other expression position is a syntax error.
    fn private_in_expression(&mut self, name: String, span: Span) -> Result<Expr, ParseError> {
        self.advance();
        if !self.allow_in || !self.at(&TokenKind::In) {
            return Err(ParseError {
                message: format!(
                    "private name `#{name}` is only valid on the left of `in` or as a member \
                     access"
                ),
                span,
            });
        }
        self.note_private_reference(&name, span);
        self.expect(&TokenKind::In)?;
        let object = self.shift_expression()?;
        let full = Span::new(span.start, object.span().end);
        Ok(Expr::PrivateIn {
            name,
            object: Box::new(object),
            span: full,
        })
    }

    fn new_expression(&mut self, start: usize) -> Result<Expr, ParseError> {
        self.expect(&TokenKind::New)?;
        if self.match_kind(&TokenKind::Dot) {
            let property = self.advance();
            if !matches!(&property.kind, TokenKind::Identifier(name) if name == "target")
                || property.had_escape
            {
                return Err(ParseError {
                    message: "only `new.target` is a valid `new.` meta-property".to_owned(),
                    span: property.span,
                });
            }
            return Ok(Expr::NewTarget {
                span: Span::new(start, property.span.end),
            });
        }
        let callee = self.member_chain()?;
        // `import(...)` is a CallExpression and `import.meta` a meta-property;
        // neither is a valid constructor for `new` (no-new-call-expression).
        if matches!(callee, Expr::ImportCall { .. } | Expr::ImportMeta { .. }) {
            return Err(ParseError {
                message: "`import` is not a valid `new` operand".to_owned(),
                span: callee.span(),
            });
        }
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
