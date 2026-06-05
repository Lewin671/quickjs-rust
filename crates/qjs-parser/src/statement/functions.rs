use qjs_ast::{ClassMethod, Expr, Span, Stmt};
use qjs_lexer::{Token, TokenKind};

use crate::helpers::property_name;
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

    pub(super) fn class_declaration(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.match_identifier_text("class");
        let name_token = self.advance();
        let TokenKind::Identifier(name) = name_token.kind else {
            return Err(ParseError {
                message: "expected class name".to_owned(),
                span: name_token.span,
            });
        };
        if self.match_identifier_text("extends") {
            self.skip_class_extends()?;
        }
        self.expect(&TokenKind::LeftBrace)?;
        let mut methods = Vec::new();
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            methods.push(self.class_method()?);
        }
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBrace)?;
        Ok(Stmt::ClassDecl {
            name,
            methods,
            span: Span::new(start, end),
        })
    }

    fn skip_class_extends(&mut self) -> Result<(), ParseError> {
        if self.match_identifier_text("class") {
            if self
                .peek()
                .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_)))
                && !self
                    .peek_nth(1)
                    .is_some_and(|token| token.kind == TokenKind::LeftBrace)
            {
                self.advance();
            }
            self.skip_balanced_block()
        } else {
            let _ = self.assignment()?;
            Ok(())
        }
    }

    fn skip_balanced_block(&mut self) -> Result<(), ParseError> {
        self.expect(&TokenKind::LeftBrace)?;
        let mut depth = 1usize;
        while depth > 0 {
            let token = self.advance();
            match token.kind {
                TokenKind::LeftBrace => depth += 1,
                TokenKind::RightBrace => depth -= 1,
                TokenKind::Eof => {
                    return Err(ParseError {
                        message: "unterminated class body".to_owned(),
                        span: token.span,
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn class_method(&mut self) -> Result<ClassMethod, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let is_static = self.at_identifier_text("static")
            && !self
                .peek_nth(1)
                .is_some_and(|token| token.kind == TokenKind::LeftParen);
        if is_static {
            self.advance();
        }
        let name_token = self.advance();
        let Some(name) = property_name(name_token.kind) else {
            return Err(ParseError {
                message: "expected class method name".to_owned(),
                span: name_token.span,
            });
        };
        let params = self.function_parameters()?;
        let body = self.block_body()?;
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;
        Ok(ClassMethod {
            name,
            params,
            body,
            is_static,
            span: Span::new(start, end),
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
