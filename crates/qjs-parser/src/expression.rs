use qjs_ast::{Expr, Span};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

mod access;
mod assignment;
mod binary;
mod pattern;
mod primary;
mod unary;

pub(crate) use primary::keyword_property_name;

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

    pub(crate) fn expression_no_in(&mut self) -> Result<Expr, ParseError> {
        let previous = self.allow_in;
        self.allow_in = false;
        let result = self.expression();
        self.allow_in = previous;
        result
    }
}
