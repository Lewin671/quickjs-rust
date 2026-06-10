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
        let start_token = self
            .peek()
            .cloned()
            .expect("parser should always have eof token");
        let member_start = start_token.span.start;

        // `static` is a modifier only when it is followed by another member
        // start; `static() {}` or `static = 1` use `static` as the name.
        let is_static = matches!(&start_token.kind, TokenKind::Identifier(name) if name == "static")
            && self.token_starts_member_after_modifier(1);
        if is_static {
            self.advance();
        }

        // `get`/`set` introduce an accessor only when followed by a member
        // name start; `get() {}` or `set = 1` use them as the name.
        let accessor_token = self
            .peek()
            .cloned()
            .expect("parser should always have eof token");
        let accessor_kind = match &accessor_token.kind {
            TokenKind::Identifier(name) if name == "get" || name == "set" => {
                if self.token_starts_member_after_modifier(1) {
                    self.advance();
                    Some(if name == "get" {
                        MethodKind::Getter
                    } else {
                        MethodKind::Setter
                    })
                } else {
                    None
                }
            }
            TokenKind::Star => {
                return Err(ParseError {
                    message: "generator class methods are not yet supported".to_owned(),
                    span: accessor_token.span,
                });
            }
            _ => None,
        };

        let (key, key_text) = self.class_member_key()?;

        if !self.at(&TokenKind::LeftParen) {
            return Err(ParseError {
                message: "class fields are not yet supported".to_owned(),
                span: Span::new(member_start, self.previous_end()),
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
        let end = self.previous_end();

        let is_constructor = !is_static
            && accessor_kind.is_none()
            && matches!(key_text.as_deref(), Some("constructor"));

        let kind = match accessor_kind {
            Some(MethodKind::Getter) => {
                if !params.is_empty() {
                    return Err(ParseError {
                        message: "getter must not have parameters".to_owned(),
                        span: Span::new(member_start, end),
                    });
                }
                MethodKind::Getter
            }
            Some(MethodKind::Setter) => {
                if params.positional.len() != 1 || params.rest.is_some() {
                    return Err(ParseError {
                        message: "setter must have exactly one parameter".to_owned(),
                        span: Span::new(member_start, end),
                    });
                }
                MethodKind::Setter
            }
            _ if is_constructor => MethodKind::Constructor,
            _ => MethodKind::Method,
        };

        self.validate_member_restrictions(is_static, kind, key_text.as_deref(), member_start, end)?;

        let value = Expr::Function {
            name: key_text.clone(),
            params,
            body,
            constructable: false,
            lexical_this: false,
            lexical_arguments: false,
            span: Span::new(member_start, end),
        };
        Ok(ClassMember {
            kind,
            key,
            is_static,
            value,
            span: Span::new(member_start, end),
        })
    }

    /// Parses a class member key (literal name or `[expr]`), returning the key
    /// and its literal text when statically known.
    fn class_member_key(&mut self) -> Result<(ClassMemberKey, Option<String>), ParseError> {
        if self.at(&TokenKind::LeftBracket) {
            self.advance();
            let expr = self.assignment()?;
            self.expect(&TokenKind::RightBracket)?;
            return Ok((ClassMemberKey::Computed(expr), None));
        }
        let token = self
            .peek()
            .cloned()
            .expect("parser should always have eof token");
        let name = class_member_name(&token.kind).ok_or_else(|| ParseError {
            message: "expected class member name".to_owned(),
            span: token.span,
        })?;
        self.advance();
        Ok((ClassMemberKey::Literal(name.clone()), Some(name)))
    }

    /// Reports whether the token `offset` ahead can begin a class member name,
    /// used to disambiguate `static`/`get`/`set` as modifiers versus names.
    fn token_starts_member_after_modifier(&self, offset: usize) -> bool {
        match self.peek_nth(offset).map(|token| &token.kind) {
            Some(TokenKind::LeftBracket) => true,
            Some(kind) => class_member_name(kind).is_some(),
            None => false,
        }
    }

    fn previous_end(&self) -> usize {
        self.tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end
    }

    fn validate_member_restrictions(
        &self,
        is_static: bool,
        kind: MethodKind,
        key_text: Option<&str>,
        start: usize,
        end: usize,
    ) -> Result<(), ParseError> {
        let span = Span::new(start, end);
        match key_text {
            Some("constructor") => {
                // A getter/setter named `constructor` is a syntax error; a
                // static member named `constructor` is allowed.
                if !is_static && matches!(kind, MethodKind::Getter | MethodKind::Setter) {
                    return Err(ParseError {
                        message: "class constructor may not be an accessor".to_owned(),
                        span,
                    });
                }
            }
            Some("prototype") if is_static => {
                return Err(ParseError {
                    message: "classes may not have a static property named `prototype`".to_owned(),
                    span,
                });
            }
            _ => {}
        }
        Ok(())
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
