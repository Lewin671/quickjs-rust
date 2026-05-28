use qjs_ast::{Expr, Span, Stmt};
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
            span: Span::new(start, end),
        })
    }

    pub(crate) fn function_parameters(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect(&TokenKind::LeftParen)?;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RightParen) {
            loop {
                let param_token = self.advance();
                let TokenKind::Identifier(param) = param_token.kind else {
                    return Err(ParseError {
                        message: "expected parameter name".to_owned(),
                        span: param_token.span,
                    });
                };
                params.push(param);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RightParen)?;
        Ok(params)
    }
}
