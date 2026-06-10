use qjs_ast::{ArrayElement, Expr, FunctionParams, Span, Stmt};
use qjs_lexer::{Token, TokenKind};

use crate::helpers::body_has_strict_directive;
use crate::{ParseError, Parser};

impl Parser {
    pub(super) fn function_declaration(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Function)?;
        let name_token = self.advance();
        let TokenKind::Identifier(name) = name_token.kind else {
            return Err(ParseError {
                message: "expected function name".to_owned(),
                span: name_token.span,
            });
        };

        let params = self.function_parameters()?;
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let body = self.without_super_context(Self::block_body)?;
        self.reject_invalid_function_parameters(&params, &body, body_start)?;
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;

        Ok(Stmt::FunctionDecl {
            name,
            params,
            body,
            span: Span::new(start.min(body_start), end),
        })
    }

    pub(crate) fn function_expression(&mut self, start: usize) -> Result<Expr, ParseError> {
        if self.match_kind(&TokenKind::Star) {
            return self.generator_function_expression(start);
        }

        let name = if let Some(Token {
            kind: TokenKind::Identifier(_),
            ..
        }) = self.peek()
        {
            let token = self.advance();
            let TokenKind::Identifier(name) = token.kind else {
                unreachable!("peek checked identifier");
            };
            Some(name)
        } else {
            None
        };

        let params = self.function_parameters()?;
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let body = self.without_super_context(Self::block_body)?;
        self.reject_invalid_function_parameters(&params, &body, body_start)?;
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;
        Ok(Expr::Function {
            name,
            params,
            body,
            constructable: true,
            lexical_this: false,
            lexical_arguments: false,
            span: Span::new(start, end),
        })
    }

    fn generator_function_expression(&mut self, start: usize) -> Result<Expr, ParseError> {
        let name = if let Some(Token {
            kind: TokenKind::Identifier(_),
            ..
        }) = self.peek()
        {
            let token = self.advance();
            let TokenKind::Identifier(name) = token.kind else {
                unreachable!("peek checked identifier");
            };
            Some(name)
        } else {
            None
        };

        let params = self.function_parameters()?;
        if !self.generator_body_is_yield_only() {
            let body_start = self
                .peek()
                .expect("parser should always have eof token")
                .span
                .start;
            let body = self.without_super_context(Self::block_body)?;
            self.reject_invalid_function_parameters(&params, &body, body_start)?;
            let end = self
                .tokens
                .get(self.cursor.saturating_sub(1))
                .expect("parser should always have eof token")
                .span
                .end;
            return Ok(Expr::Function {
                name,
                params,
                body,
                constructable: false,
                lexical_this: false,
                lexical_arguments: false,
                span: Span::new(start.min(body_start), end),
            });
        }

        let (elements, body_span) = self.generator_yield_body()?;
        Ok(Expr::Function {
            name,
            params,
            body: vec![Stmt::Return {
                argument: Some(Expr::Array {
                    elements,
                    span: body_span,
                }),
                span: body_span,
            }],
            constructable: false,
            lexical_this: false,
            lexical_arguments: false,
            span: Span::new(start, body_span.end),
        })
    }

    fn generator_body_is_yield_only(&self) -> bool {
        match (self.peek(), self.peek_nth(1)) {
            (Some(open), Some(next)) if open.kind == TokenKind::LeftBrace => {
                next.kind == TokenKind::RightBrace
                    || matches!(&next.kind, TokenKind::Identifier(name) if name == "yield")
            }
            _ => false,
        }
    }

    fn generator_yield_body(&mut self) -> Result<(Vec<ArrayElement>, Span), ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::LeftBrace)?;
        let mut elements = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            let token = self.advance();
            let TokenKind::Identifier(keyword) = token.kind else {
                return Err(ParseError {
                    message: "expected `yield` in generator body".to_owned(),
                    span: token.span,
                });
            };
            if keyword != "yield" {
                return Err(ParseError {
                    message: "expected `yield` in generator body".to_owned(),
                    span: token.span,
                });
            }
            let value = self.assignment()?;
            elements.push(ArrayElement::Expr(value));
            self.match_kind(&TokenKind::Semicolon);
        }
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBrace)?;
        Ok((elements, Span::new(start, end)))
    }

    pub(crate) fn function_parameters(&mut self) -> Result<FunctionParams, ParseError> {
        self.expect(&TokenKind::LeftParen)?;
        let mut positional = Vec::new();
        let mut rest = None;
        if !self.at(&TokenKind::RightParen) {
            loop {
                if self.match_kind(&TokenKind::DotDotDot) {
                    let pattern = self.binding_pattern()?;
                    if self.at(&TokenKind::Equal) {
                        return Err(ParseError {
                            message: "rest parameter must not have a default".to_owned(),
                            span: pattern.span(),
                        });
                    }
                    rest = Some(pattern);
                    break;
                }
                positional.push(self.binding_element()?);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RightParen) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RightParen)?;
        let params = FunctionParams::new(positional, rest);
        if !params.is_simple()
            && let Some(span) = duplicate_parameter_span(&params)
        {
            return Err(ParseError {
                message: "duplicate parameter name".to_owned(),
                span,
            });
        }
        Ok(params)
    }

    /// Runs `body` with `super` contexts cleared, restoring them afterward.
    /// Regular function and constructor boundaries reset whether `super`
    /// member access and `super(...)` calls are allowed; arrow functions keep
    /// the surrounding context, so they do not use this helper.
    pub(crate) fn without_super_context<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        let previous_method = self.in_method;
        let previous_derived = self.in_derived_constructor;
        self.in_method = false;
        self.in_derived_constructor = false;
        let result = body(self);
        self.in_method = previous_method;
        self.in_derived_constructor = previous_derived;
        result
    }

    pub(crate) fn reject_invalid_function_parameters(
        &self,
        params: &FunctionParams,
        body: &[Stmt],
        span_start: usize,
    ) -> Result<(), ParseError> {
        let strict_body = body_has_strict_directive(body);
        if !params.is_simple() && strict_body {
            return Err(ParseError {
                message: "strict directive not allowed with non-simple parameters".to_owned(),
                span: Span::new(span_start, span_start),
            });
        }
        if (self.strict || strict_body)
            && let Some(span) = duplicate_parameter_span(params)
        {
            return Err(ParseError {
                message: "duplicate parameter name".to_owned(),
                span,
            });
        }
        if self.strict || strict_body {
            for (name, span) in params.named_spans() {
                if matches!(name.as_str(), "eval" | "arguments") {
                    return Err(ParseError {
                        message: "restricted parameter name in strict mode".to_owned(),
                        span,
                    });
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn duplicate_parameter_span(params: &FunctionParams) -> Option<Span> {
    let named = params.named_spans();
    for (index, (name, _)) in named.iter().enumerate() {
        for (candidate, span) in &named[index + 1..] {
            if candidate == name {
                return Some(*span);
            }
        }
    }
    None
}
