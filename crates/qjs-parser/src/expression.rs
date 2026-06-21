use qjs_ast::{Expr, Span};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

mod access;
mod assignment;
mod binary;
mod helpers;
mod import;
mod pattern;
mod primary;
mod unary;

pub(crate) use helpers::{
    has_legacy_octal_escape, is_legacy_octal_or_non_octal_decimal_literal, keyword_property_name,
};

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

    pub(crate) fn assignment_allow_in(&mut self) -> Result<Expr, ParseError> {
        let previous = self.allow_in;
        self.allow_in = true;
        let result = self.assignment();
        self.allow_in = previous;
        result
    }
}
