use qjs_ast::{CallArgument, Expr, MemberProperty, Span};
use qjs_lexer::TokenKind;

use crate::helpers::property_name;
use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn call(&mut self) -> Result<Expr, ParseError> {
        let expr = self.primary()?;
        self.finish_call_member_chain(expr)
    }

    pub(crate) fn member_chain(&mut self) -> Result<Expr, ParseError> {
        let expr = self.primary()?;
        self.finish_member_chain(expr)
    }

    pub(crate) fn finish_call_member_chain(&mut self, mut expr: Expr) -> Result<Expr, ParseError> {
        loop {
            if self.match_kind(&TokenKind::LeftParen) {
                expr = self.finish_call(expr)?;
                continue;
            }

            if self.match_kind(&TokenKind::LeftBracket) {
                expr = self.finish_computed_member(expr)?;
                continue;
            }

            if self.match_kind(&TokenKind::Dot) {
                expr = self.finish_named_member(expr)?;
                continue;
            }

            if self.at_template_literal() {
                expr = self.finish_tagged_template_literal(expr)?;
                continue;
            }

            break;
        }
        Ok(expr)
    }

    pub(crate) fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        let mut arguments = Vec::new();
        if !self.at(&TokenKind::RightParen) {
            loop {
                if self.match_kind(&TokenKind::DotDotDot) {
                    arguments.push(CallArgument::Spread(self.assignment()?));
                } else {
                    arguments.push(CallArgument::Expr(self.assignment()?));
                }
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RightParen) {
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
        let span = Span::new(callee.span().start, end);
        Ok(Expr::Call {
            callee: Box::new(callee),
            arguments,
            span,
        })
    }

    fn finish_member_chain(&mut self, mut expr: Expr) -> Result<Expr, ParseError> {
        loop {
            if self.match_kind(&TokenKind::LeftBracket) {
                expr = self.finish_computed_member(expr)?;
                continue;
            }

            if self.match_kind(&TokenKind::Dot) {
                expr = self.finish_named_member(expr)?;
                continue;
            }

            break;
        }
        Ok(expr)
    }

    fn finish_computed_member(&mut self, object: Expr) -> Result<Expr, ParseError> {
        let property = self.expression()?;
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBracket)?;
        let span = Span::new(object.span().start, end);
        Ok(Expr::Member {
            object: Box::new(object),
            property: MemberProperty::Computed(Box::new(property)),
            span,
        })
    }

    fn finish_named_member(&mut self, object: Expr) -> Result<Expr, ParseError> {
        let property_token = self.advance();
        if let TokenKind::PrivateName(name) = &property_token.kind {
            if matches!(&object, Expr::Super { .. }) {
                return Err(ParseError {
                    message: "private names are not valid on `super` property access".to_owned(),
                    span: Span::new(object.span().start, property_token.span.end),
                });
            }
            let name = name.clone();
            self.note_private_reference(&name, property_token.span);
            let span = Span::new(object.span().start, property_token.span.end);
            return Ok(Expr::Member {
                object: Box::new(object),
                property: MemberProperty::Private(name),
                span,
            });
        }
        let Some(name) = property_name(property_token.kind) else {
            return Err(ParseError {
                message: "expected property name".to_owned(),
                span: property_token.span,
            });
        };
        let span = Span::new(object.span().start, property_token.span.end);
        Ok(Expr::Member {
            object: Box::new(object),
            property: MemberProperty::Named(name),
            span,
        })
    }
}
