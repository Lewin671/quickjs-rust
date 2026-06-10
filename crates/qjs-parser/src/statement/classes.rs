use qjs_ast::{
    ClassBody, ClassMember, ClassMemberKey, Expr, FunctionParams, MethodKind, Span, Stmt,
};
use qjs_lexer::{Token, TokenKind};

use crate::statement::duplicate_parameter_span;
use crate::{ParseError, Parser};

impl Parser {
    /// Parses a `class Name { ... }` declaration.
    pub(super) fn class_declaration(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.expect(&TokenKind::Class)?;
        let name_token = self.advance();
        let TokenKind::Identifier(name) = name_token.kind else {
            return Err(ParseError {
                message: "expected class name".to_owned(),
                span: name_token.span,
            });
        };
        self.reject_class_extends()?;
        let body = self.class_body()?;
        let span = Span::new(start, body.span.end);
        Ok(Stmt::ClassDecl { name, body, span })
    }

    /// Parses a `class` or `class Name` expression.
    pub(crate) fn class_expression(&mut self, start: usize) -> Result<Expr, ParseError> {
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
        self.reject_class_extends()?;
        let body = self.class_body()?;
        let span = Span::new(start, body.span.end);
        Ok(Expr::Class { name, body, span })
    }

    fn reject_class_extends(&mut self) -> Result<(), ParseError> {
        if self.at(&TokenKind::Extends) {
            let token = self.advance();
            return Err(ParseError {
                message: "class `extends` clauses are not yet supported".to_owned(),
                span: token.span,
            });
        }
        Ok(())
    }

    fn class_body(&mut self) -> Result<ClassBody, ParseError> {
        let open = self
            .peek()
            .expect("parser should always have eof token")
            .span;
        self.expect(&TokenKind::LeftBrace)?;

        // Class bodies are always strict-mode code.
        let previous_strict = self.strict;
        self.strict = true;
        let result = self.class_members(open.start);
        self.strict = previous_strict;
        result
    }

    fn class_members(&mut self, start: usize) -> Result<ClassBody, ParseError> {
        let mut members = Vec::new();
        let mut seen_constructor = false;
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            // Empty members: bare semicolons are allowed between definitions.
            if self.match_kind(&TokenKind::Semicolon) {
                continue;
            }
            let member = self.class_member()?;
            if member.kind == MethodKind::Constructor {
                if seen_constructor {
                    return Err(ParseError {
                        message: "a class may only have one constructor".to_owned(),
                        span: member.span,
                    });
                }
                seen_constructor = true;
            }
            members.push(member);
        }
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBrace)?;
        Ok(ClassBody {
            members,
            span: Span::new(start, end),
        })
    }

    fn class_member(&mut self) -> Result<ClassMember, ParseError> {
        let token = self
            .peek()
            .cloned()
            .expect("parser should always have eof token");
        self.reject_unsupported_member_modifier(&token)?;

        let name = class_member_name(&token.kind).ok_or_else(|| ParseError {
            message: "expected class member name".to_owned(),
            span: token.span,
        })?;
        self.advance();

        if !self.at(&TokenKind::LeftParen) {
            return Err(ParseError {
                message: "class fields are not yet supported".to_owned(),
                span: token.span,
            });
        }

        let params = self.function_parameters()?;
        reject_duplicate_method_parameters(&params)?;
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let body = self.block_body()?;
        self.reject_invalid_function_parameters(&params, &body, body_start)?;
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;

        let kind = if name == "constructor" {
            MethodKind::Constructor
        } else {
            MethodKind::Method
        };
        let value = Expr::Function {
            name: Some(name.clone()),
            params,
            body,
            constructable: false,
            lexical_this: false,
            lexical_arguments: false,
            span: Span::new(token.span.start, end),
        };
        Ok(ClassMember {
            kind,
            key: ClassMemberKey::Literal(name),
            value,
            span: Span::new(token.span.start, end),
        })
    }

    fn reject_unsupported_member_modifier(&self, token: &Token) -> Result<(), ParseError> {
        match &token.kind {
            TokenKind::Identifier(name) if name == "static" => Err(ParseError {
                message: "static class members are not yet supported".to_owned(),
                span: token.span,
            }),
            TokenKind::Identifier(name) if name == "get" || name == "set" => {
                // `get`/`set` used as a plain method name (followed by `(`) is
                // allowed; an accessor definition is not yet supported.
                if matches!(
                    self.peek_nth(1).map(|t| &t.kind),
                    Some(TokenKind::LeftParen)
                ) {
                    Ok(())
                } else {
                    Err(ParseError {
                        message: "class accessors are not yet supported".to_owned(),
                        span: token.span,
                    })
                }
            }
            TokenKind::Star => Err(ParseError {
                message: "generator class methods are not yet supported".to_owned(),
                span: token.span,
            }),
            TokenKind::LeftBracket => Err(ParseError {
                message: "computed class member names are not yet supported".to_owned(),
                span: token.span,
            }),
            _ => Ok(()),
        }
    }
}

fn class_member_name(kind: &TokenKind) -> Option<String> {
    match kind {
        TokenKind::Identifier(name) => Some(name.clone()),
        TokenKind::String(name) | TokenKind::Number(name) => Some(name.clone()),
        _ => crate::expression::keyword_property_name(kind).map(str::to_owned),
    }
}

fn reject_duplicate_method_parameters(params: &FunctionParams) -> Result<(), ParseError> {
    if let Some(span) = duplicate_parameter_span(params) {
        return Err(ParseError {
            message: "duplicate parameter name".to_owned(),
            span,
        });
    }
    Ok(())
}
