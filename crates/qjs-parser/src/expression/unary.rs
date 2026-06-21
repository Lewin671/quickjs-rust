use qjs_ast::{AssignmentTarget, Expr, Span, UnaryOp, UpdateOp};
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
            if token.had_escape {
                return Err(ParseError {
                    message: "`await` keyword may not contain escapes".to_owned(),
                    span: token.span,
                });
            }
            return self.await_expression(token.span);
        }
        if token.kind == TokenKind::PlusPlus || token.kind == TokenKind::MinusMinus {
            self.advance();
            let target = assignment_target(self.unary()?, false)?;
            if self.strict {
                if let AssignmentTarget::Identifier { name, span, .. } = &target {
                    if matches!(name.as_str(), "eval" | "arguments") {
                        return Err(ParseError {
                            message: format!(
                                "`{name}` may not be an assignment target in strict mode"
                            ),
                            span: *span,
                        });
                    }
                }
            }
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
        if op == UnaryOp::Typeof
            && matches!(self.peek_nth(1), Some(next) if matches!(&next.kind, TokenKind::Identifier(name) if name == "import") && !next.had_escape)
            && !matches!(
                self.peek_nth(2).map(|next| &next.kind),
                Some(TokenKind::LeftParen | TokenKind::Dot)
            )
        {
            let import_token = self.peek_nth(1).expect("checked import token").clone();
            return Err(ParseError {
                message: "`import` must be followed by `(` or `.` after `typeof`".to_owned(),
                span: import_token.span,
            });
        }
        self.advance();
        let argument = self.unary()?;
        if op == UnaryOp::Delete {
            // `delete obj.#x` (a private member reference) is a syntax error.
            if matches!(
                &argument,
                Expr::Member {
                    property: qjs_ast::MemberProperty::Private(_),
                    ..
                }
            ) {
                return Err(ParseError {
                    message: "cannot delete a private member".to_owned(),
                    span: Span::new(token.span.start, argument.span().end),
                });
            }
            // In strict mode, `delete identifier` is a SyntaxError.
            if self.strict && matches!(&argument, Expr::Identifier { .. }) {
                return Err(ParseError {
                    message: "cannot delete an unqualified identifier in strict mode".to_owned(),
                    span: Span::new(token.span.start, argument.span().end),
                });
            }
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
        if self.has_line_terminator_between(expr.span().end, token.span.start) {
            return Ok(expr);
        }
        self.advance();
        let start = expr.span().start;
        let target = assignment_target(expr, false)?;
        if self.strict {
            if let AssignmentTarget::Identifier { name, span, .. } = &target {
                if matches!(name.as_str(), "eval" | "arguments") {
                    return Err(ParseError {
                        message: format!("`{name}` may not be an assignment target in strict mode"),
                        span: *span,
                    });
                }
            }
        }
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
        // The right operand is a ShiftExpression, which can never begin with a
        // private name, so `#a in #b in c` is a syntax error (a private name is
        // only valid on the left of `in`). A parenthesized `#a in (#b in c)` is
        // fine because the inner expression is a fresh relational context.
        if let Some(token) = self.peek()
            && let TokenKind::PrivateName(rhs) = &token.kind
        {
            return Err(ParseError {
                message: format!(
                    "private name `#{rhs}` is only valid on the left of `in` or as a member access"
                ),
                span: token.span,
            });
        }
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
            if !self.in_function {
                return Err(ParseError {
                    message: "`new.target` is only allowed inside functions".to_owned(),
                    span: Span::new(start, property.span.end),
                });
            }
            // `new.target` is a MetaProperty, i.e. a MemberExpression head, so
            // it can be followed by member access, calls, and optional chains
            // (`new.target?.()`, `new.target?.a`).
            let new_target = Expr::NewTarget {
                span: Span::new(start, property.span.end),
            };
            return self.finish_call_member_chain(new_target);
        }
        let callee_parenthesized = self.at(&TokenKind::LeftParen);
        if self.in_async
            && matches!(self.peek().map(|token| &token.kind), Some(TokenKind::Identifier(name)) if name == "await")
        {
            let await_span = self.peek().expect("checked await token").span;
            return Err(ParseError {
                message: "`await` is not a valid direct `new` operand".to_owned(),
                span: await_span,
            });
        }
        // `new NewExpression`: the operand of `new` may itself be a `new`
        // expression (`new new X`), which recurses rather than parsing as a
        // MemberExpression primary. Any `(args)` binds to the inner `new`.
        let callee = if self.at(&TokenKind::New) {
            let nested_start = self.peek().map_or(start, |token| token.span.start);
            self.new_expression(nested_start)?
        } else {
            let mut callee = self.member_chain()?;
            while self.at_template_literal() {
                callee = self.finish_tagged_template_literal(callee)?;
            }
            callee
        };
        // `import(...)` is a CallExpression; a direct `new import(...)` is a
        // syntax error, while `new (import(...))` is a covered expression and
        // reaches runtime. `import.meta` is a valid NewExpression operand and
        // fails later because the meta object is not a constructor.
        if matches!(callee, Expr::ImportCall { .. }) && !callee_parenthesized {
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
