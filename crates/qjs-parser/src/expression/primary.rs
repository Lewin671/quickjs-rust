use qjs_ast::{Expr, Literal, ObjectProperty, ObjectPropertyKey, Span};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier(name) => Ok(Expr::Identifier {
                name,
                span: token.span,
            }),
            TokenKind::Number(raw) => Ok(Expr::Literal(Literal::Number {
                raw,
                span: token.span,
            })),
            TokenKind::String(value) => Ok(Expr::Literal(Literal::String {
                value,
                span: token.span,
            })),
            TokenKind::True => Ok(Expr::Literal(Literal::Boolean {
                value: true,
                span: token.span,
            })),
            TokenKind::False => Ok(Expr::Literal(Literal::Boolean {
                value: false,
                span: token.span,
            })),
            TokenKind::Null => Ok(Expr::Literal(Literal::Null { span: token.span })),
            TokenKind::This => Ok(Expr::This { span: token.span }),
            TokenKind::Function => self.function_expression(token.span.start),
            TokenKind::Slash => self.regexp_literal(token.span.start),
            TokenKind::LeftBracket => self.array_literal(token.span.start),
            TokenKind::LeftBrace => self.object_literal(token.span.start),
            TokenKind::LeftParen => {
                let expr = self.expression()?;
                self.expect(&TokenKind::RightParen)?;
                Ok(expr)
            }
            _ => Err(ParseError {
                message: "expected expression".to_owned(),
                span: token.span,
            }),
        }
    }

    fn regexp_literal(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut pattern = String::new();
        loop {
            let token = self.advance();
            match token.kind {
                TokenKind::Slash => {
                    let end = token.span.end;
                    let mut arguments = vec![Expr::Literal(Literal::String {
                        value: pattern,
                        span: Span::new(start, end),
                    })];
                    if let Some(flags) = self.regexp_flags() {
                        arguments.push(Expr::Literal(Literal::String {
                            span: flags.span,
                            value: flags.value,
                        }));
                    }
                    let span_end = arguments.last().map_or(end, |argument| argument.span().end);
                    return Ok(Expr::New {
                        callee: Box::new(Expr::Identifier {
                            name: "RegExp".to_owned(),
                            span: Span::new(start, start + 1),
                        }),
                        span: Span::new(start, span_end),
                        arguments,
                    });
                }
                TokenKind::Dot => pattern.push('.'),
                TokenKind::Backslash => {
                    pattern.push('\\');
                    let escaped = self.advance();
                    if escaped.kind == TokenKind::Eof {
                        return Err(ParseError {
                            message: "unterminated regular expression literal".to_owned(),
                            span: escaped.span,
                        });
                    }
                    pattern.push_str(regexp_token_text(&escaped.kind).ok_or(ParseError {
                        message: "unsupported regular expression escape".to_owned(),
                        span: escaped.span,
                    })?);
                }
                TokenKind::Identifier(text) | TokenKind::Number(text) | TokenKind::String(text) => {
                    pattern.push_str(&text);
                }
                TokenKind::Eof => {
                    return Err(ParseError {
                        message: "unterminated regular expression literal".to_owned(),
                        span: token.span,
                    });
                }
                kind => pattern.push_str(regexp_token_text(&kind).ok_or(ParseError {
                    message: "unsupported regular expression literal".to_owned(),
                    span: token.span,
                })?),
            }
        }
    }

    fn regexp_flags(&mut self) -> Option<RegexpFlags> {
        let token = self.peek()?;
        let TokenKind::Identifier(value) = &token.kind else {
            return None;
        };
        let flags = RegexpFlags {
            value: value.clone(),
            span: token.span,
        };
        self.advance();
        Some(flags)
    }

    fn array_literal(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut elements = Vec::new();
        if !self.at(&TokenKind::RightBracket) {
            loop {
                if self.at(&TokenKind::Comma) {
                    elements.push(None);
                    self.advance();
                    if self.at(&TokenKind::RightBracket) {
                        break;
                    }
                    continue;
                }
                elements.push(Some(self.assignment()?));
                if !self.match_kind(&TokenKind::Comma) || self.at(&TokenKind::RightBracket) {
                    break;
                }
            }
        }
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBracket)?;
        Ok(Expr::Array {
            elements,
            span: Span::new(start, end),
        })
    }

    fn object_literal(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut properties = Vec::new();
        if !self.at(&TokenKind::RightBrace) {
            loop {
                let key_token = self.advance();
                let key_span = key_token.span;
                let (key, shorthand_value) = self.object_property_key(key_token)?;
                let value = self.object_property_value(key_span, &key, shorthand_value)?;
                let span = Span::new(key_span.start, value.span().end);
                properties.push(ObjectProperty { key, value, span });
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RightBrace) {
                    break;
                }
            }
        }
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBrace)?;
        Ok(Expr::Object {
            properties,
            span: Span::new(start, end),
        })
    }

    fn object_property_key(
        &mut self,
        key_token: qjs_lexer::Token,
    ) -> Result<(ObjectPropertyKey, Option<Expr>), ParseError> {
        match key_token.kind {
            TokenKind::Identifier(name) => {
                let value = Expr::Identifier {
                    name: name.clone(),
                    span: key_token.span,
                };
                Ok((ObjectPropertyKey::Literal(name), Some(value)))
            }
            TokenKind::String(name) | TokenKind::Number(name) => {
                Ok((ObjectPropertyKey::Literal(name), None))
            }
            TokenKind::True => Ok((ObjectPropertyKey::Literal("true".to_owned()), None)),
            TokenKind::False => Ok((ObjectPropertyKey::Literal("false".to_owned()), None)),
            TokenKind::Null => Ok((ObjectPropertyKey::Literal("null".to_owned()), None)),
            TokenKind::LeftBracket => {
                let name = self.assignment()?;
                self.expect(&TokenKind::RightBracket)?;
                Ok((ObjectPropertyKey::Computed(name), None))
            }
            _ => Err(ParseError {
                message: "expected property name".to_owned(),
                span: key_token.span,
            }),
        }
    }

    fn object_property_value(
        &mut self,
        key_span: Span,
        key: &ObjectPropertyKey,
        shorthand_value: Option<Expr>,
    ) -> Result<Expr, ParseError> {
        if self.at(&TokenKind::LeftParen) {
            let method_name = match key {
                ObjectPropertyKey::Literal(name) => Some(name.clone()),
                ObjectPropertyKey::Computed(_) => None,
            };
            let params = self.function_parameters()?;
            let body = self.block_body()?;
            let end = self
                .tokens
                .get(self.cursor.saturating_sub(1))
                .expect("parser should always have eof token")
                .span
                .end;
            return Ok(Expr::Function {
                name: method_name,
                params,
                body,
                constructable: false,
                span: Span::new(key_span.start, end),
            });
        }

        if self.match_kind(&TokenKind::Colon) {
            return self.assignment();
        }

        if let Some(value) = shorthand_value {
            return Ok(value);
        }

        Err(ParseError {
            message: "expected `:` after property name".to_owned(),
            span: match key {
                ObjectPropertyKey::Literal(_) => key_span,
                ObjectPropertyKey::Computed(expr) => expr.span(),
            },
        })
    }
}

struct RegexpFlags {
    value: String,
    span: Span,
}

fn regexp_token_text(kind: &TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::Star => Some("*"),
        TokenKind::Plus => Some("+"),
        TokenKind::Minus => Some("-"),
        TokenKind::Question => Some("?"),
        TokenKind::Slash => Some("/"),
        TokenKind::Backslash => Some("\\"),
        TokenKind::LeftParen => Some("("),
        TokenKind::RightParen => Some(")"),
        TokenKind::LeftBracket => Some("["),
        TokenKind::RightBracket => Some("]"),
        TokenKind::LeftBrace => Some("{"),
        TokenKind::RightBrace => Some("}"),
        TokenKind::Colon => Some(":"),
        TokenKind::Pipe => Some("|"),
        TokenKind::Caret => Some("^"),
        _ => None,
    }
}
