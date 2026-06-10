//! Destructuring assignment patterns: the ObjectLiteral / ArrayLiteral
//! cover grammar reparsed as assignment targets.

use qjs_ast::{
    AssignmentOp, AssignmentTarget, AssignmentTargetElement, AssignmentTargetProperty, Expr, Span,
};
use qjs_lexer::TokenKind;

use crate::expression::keyword_property_name;
use crate::helpers::assignment_target;
use crate::{ParseError, Parser};

impl Parser {
    /// Attempts to parse a destructuring assignment starting at `{` or `[`.
    ///
    /// This is the cover-grammar reparse: the tokens are speculatively read
    /// as an assignment pattern, and when they do not form a pattern
    /// followed by `=`, the cursor rewinds so the caller parses them as an
    /// ordinary literal expression instead.
    pub(crate) fn try_destructuring_assignment(&mut self) -> Result<Option<Expr>, ParseError> {
        if !self.at(&TokenKind::LeftBrace) && !self.at(&TokenKind::LeftBracket) {
            return Ok(None);
        }
        let start_cursor = self.cursor;
        let Ok(target) = self.assignment_pattern() else {
            self.cursor = start_cursor;
            return Ok(None);
        };
        if !self.match_kind(&TokenKind::Equal) {
            self.cursor = start_cursor;
            return Ok(None);
        }
        let value = self.assignment()?;
        let span = Span::new(target.span().start, value.span().end);
        Ok(Some(Expr::Assignment {
            target,
            op: AssignmentOp::Assign,
            value: Box::new(value),
            span,
        }))
    }

    /// Parses a destructuring assignment target: a nested pattern or a
    /// simple identifier / member reference.
    pub(crate) fn assignment_pattern(&mut self) -> Result<AssignmentTarget, ParseError> {
        if self.at(&TokenKind::LeftBracket) || self.at(&TokenKind::LeftBrace) {
            let start_cursor = self.cursor;
            let nested = if self.at(&TokenKind::LeftBracket) {
                self.array_assignment_pattern()
            } else {
                self.object_assignment_pattern()
            };
            // A literal followed by a member or call continuation is a
            // LeftHandSideExpression target, not a nested pattern
            // (e.g. `[ {}[key()] ] = value`).
            match nested {
                Ok(pattern) if !self.at_member_continuation() => return Ok(pattern),
                Ok(_) | Err(_) => self.cursor = start_cursor,
            }
        }
        self.simple_assignment_target()
    }

    fn at_member_continuation(&self) -> bool {
        matches!(
            self.peek().map(|token| &token.kind),
            Some(TokenKind::Dot | TokenKind::LeftBracket | TokenKind::LeftParen)
        ) || self.at_template_literal()
    }

    fn simple_assignment_target(&mut self) -> Result<AssignmentTarget, ParseError> {
        let expr = self.call()?;
        assignment_target(expr)
    }

    fn array_assignment_pattern(&mut self) -> Result<AssignmentTarget, ParseError> {
        let start = self.advance().span.start;
        let mut elements = Vec::new();
        let mut rest = None;
        while !self.at(&TokenKind::RightBracket) {
            if self.match_kind(&TokenKind::Comma) {
                elements.push(None);
                continue;
            }
            if self.match_kind(&TokenKind::DotDotDot) {
                let target = self.assignment_pattern()?;
                if self.at(&TokenKind::Equal) {
                    return Err(ParseError {
                        message: "rest element must not have a default".to_owned(),
                        span: target.span(),
                    });
                }
                rest = Some(Box::new(target));
                break;
            }

            let target = self.assignment_pattern()?;
            let default = if self.match_kind(&TokenKind::Equal) {
                Some(self.assignment()?)
            } else {
                None
            };
            let end = default
                .as_ref()
                .map_or(target.span().end, |expr| expr.span().end);
            let span = Span::new(target.span().start, end);
            elements.push(Some(AssignmentTargetElement {
                target,
                default,
                span,
            }));
            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RightBracket)?;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(AssignmentTarget::ArrayPattern {
            elements,
            rest,
            span: Span::new(start, end),
        })
    }

    fn object_assignment_pattern(&mut self) -> Result<AssignmentTarget, ParseError> {
        let start = self.advance().span.start;
        let mut properties = Vec::new();
        let mut rest = None;
        while !self.at(&TokenKind::RightBrace) {
            if self.match_kind(&TokenKind::DotDotDot) {
                // Rest targets must be simple references, not nested patterns.
                rest = Some(Box::new(self.simple_assignment_target()?));
                break;
            }

            let key_token = self.advance();
            let key_span = key_token.span;
            let shorthand = matches!(key_token.kind, TokenKind::Identifier(_));
            let key = match key_token.kind {
                TokenKind::Identifier(key) | TokenKind::String(key) | TokenKind::Number(key) => key,
                kind => keyword_property_name(&kind)
                    .map(str::to_owned)
                    .ok_or(ParseError {
                        message: "expected assignment property name".to_owned(),
                        span: key_span,
                    })?,
            };

            let target = if self.match_kind(&TokenKind::Colon) {
                self.assignment_pattern()?
            } else if shorthand {
                AssignmentTarget::Identifier {
                    name: key.clone(),
                    span: key_span,
                }
            } else {
                return Err(ParseError {
                    message: "expected `:` after assignment property name".to_owned(),
                    span: key_span,
                });
            };
            let default = if self.match_kind(&TokenKind::Equal) {
                Some(self.assignment()?)
            } else {
                None
            };
            let end = default
                .as_ref()
                .map_or(target.span().end, |expr| expr.span().end);
            properties.push(AssignmentTargetProperty {
                key,
                target,
                default,
                span: Span::new(key_span.start, end),
            });
            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RightBrace)?;
        let end = self.tokens[self.cursor - 1].span.end;
        Ok(AssignmentTarget::ObjectPattern {
            properties,
            rest,
            span: Span::new(start, end),
        })
    }
}
