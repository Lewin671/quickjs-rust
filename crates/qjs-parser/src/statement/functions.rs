use qjs_ast::{ArrayElement, Expr, FunctionParams, Span, Stmt};
use qjs_lexer::{Token, TokenKind};

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
        let body = self.block_body()?;
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
        let body = self.block_body()?;
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
            let body = self.block_body()?;
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
                    let rest_token = self.advance();
                    let TokenKind::Identifier(rest_name) = rest_token.kind else {
                        return Err(ParseError {
                            message: "expected rest parameter name".to_owned(),
                            span: rest_token.span,
                        });
                    };
                    rest = Some(rest_name);
                    break;
                }
                let param_token = self.advance();
                let TokenKind::Identifier(param) = param_token.kind else {
                    return Err(ParseError {
                        message: "expected parameter name".to_owned(),
                        span: param_token.span,
                    });
                };
                positional.push(param);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RightParen) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RightParen)?;
        Ok(FunctionParams { positional, rest })
    }
}
