use qjs_ast::{
    BindingElement, BindingPattern, ForInit, ObjectBindingProperty, ObjectBindingPropertyKey, Span,
    Stmt, VarDeclarator, VarKind,
};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

impl Parser {
    pub(super) fn variable_declaration(&mut self) -> Result<Stmt, ParseError> {
        let ForInit::VarDecl {
            kind,
            declarations,
            span,
        } = self.for_variable_declaration()?
        else {
            unreachable!("for variable declaration helper always returns VarDecl");
        };
        self.consume_statement_terminator(span.end)?;
        Ok(Stmt::VarDecl {
            kind,
            declarations,
            span,
        })
    }

    pub(super) fn for_variable_declaration(&mut self) -> Result<ForInit, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let kind = if self.match_kind(&TokenKind::Var) {
            VarKind::Var
        } else if self.match_kind(&TokenKind::Let) {
            VarKind::Let
        } else {
            self.expect(&TokenKind::Const)?;
            VarKind::Const
        };

        let declarations = self.variable_declarator_list(kind)?;
        let end = declarations.last().map_or(start, |decl| decl.span.end);
        Ok(ForInit::VarDecl {
            kind,
            declarations,
            span: Span::new(start, end),
        })
    }

    fn variable_declarator_list(
        &mut self,
        kind: VarKind,
    ) -> Result<Vec<VarDeclarator>, ParseError> {
        let mut declarations = Vec::new();
        loop {
            let binding = self.variable_declaration_binding_pattern(kind)?;

            let init = if self.match_kind(&TokenKind::Equal) {
                Some(self.assignment()?)
            } else {
                if !matches!(binding, BindingPattern::Identifier { .. }) {
                    return Err(ParseError {
                        message: "destructuring declarations require an initializer".to_owned(),
                        span: binding.span(),
                    });
                }
                if kind == VarKind::Const {
                    return Err(ParseError {
                        message: "const declarations require an initializer".to_owned(),
                        span: binding.span(),
                    });
                }
                None
            };
            let end = init
                .as_ref()
                .map_or(binding.span().end, |expr| expr.span().end);
            let start = binding.span().start;
            declarations.push(VarDeclarator {
                binding,
                init,
                span: Span::new(start, end),
            });

            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        Ok(declarations)
    }

    fn variable_declaration_binding_pattern(
        &mut self,
        kind: VarKind,
    ) -> Result<BindingPattern, ParseError> {
        if kind == VarKind::Var && !self.strict && self.at(&TokenKind::Let) {
            let token = self.advance();
            return Ok(BindingPattern::Identifier {
                name: "let".to_owned(),
                span: token.span,
            });
        }
        self.binding_pattern()
    }

    pub(crate) fn binding_pattern(&mut self) -> Result<BindingPattern, ParseError> {
        if self.at(&TokenKind::LeftBracket) {
            return self.array_binding_pattern();
        }
        if self.at(&TokenKind::LeftBrace) {
            return self.object_binding_pattern();
        }

        let token = self.advance();
        let TokenKind::Identifier(name) = token.kind else {
            return Err(ParseError {
                message: "expected binding identifier".to_owned(),
                span: token.span,
            });
        };
        self.validate_binding_identifier_name(&name, token.span)?;
        Ok(BindingPattern::Identifier {
            name,
            span: token.span,
        })
    }

    /// Validates that `name` (its StringValue, so escaped spellings are caught)
    /// may legally name a binding in the current context, rejecting reserved
    /// words, context-sensitive `await`, and strict-mode reserved names.
    pub(crate) fn validate_binding_identifier_name(
        &self,
        name: &str,
        span: Span,
    ) -> Result<(), ParseError> {
        if crate::helpers::is_reserved_identifier_name(name) {
            return Err(ParseError {
                message: format!("`{name}` is a reserved word"),
                span,
            });
        }
        // `await` may not be used as a binding identifier inside an async
        // function (parameters or local bindings). Ordinary nested functions
        // reset the async context, so `await` is a legal binding name there.
        if (self.in_async || self.in_static_block) && name == "await" {
            return Err(ParseError {
                message: "`await` is not allowed as a binding identifier here".to_owned(),
                span,
            });
        }
        // Strict-mode reserved words (including escaped spellings such as
        // `package`) may not name a binding. The lexer keeps escaped
        // spellings as Identifier tokens, so this StringValue check is reached
        // for both plain and escaped forms.
        if self.strict && crate::statement::functions::is_strict_reserved_word(name) {
            return Err(ParseError {
                message: format!("`{name}` is a reserved word in strict mode"),
                span,
            });
        }
        if self.strict && matches!(name, "eval" | "arguments") {
            return Err(ParseError {
                message: format!("`{name}` cannot be used as a binding name in strict mode"),
                span,
            });
        }
        Ok(())
    }

    pub(crate) fn binding_element(&mut self) -> Result<BindingElement, ParseError> {
        let binding = self.binding_pattern()?;
        let default = if self.match_kind(&TokenKind::Equal) {
            Some(self.assignment()?)
        } else {
            None
        };
        let end = default
            .as_ref()
            .map_or(binding.span().end, |expr| expr.span().end);
        let start = binding.span().start;
        Ok(BindingElement {
            binding,
            default,
            span: Span::new(start, end),
        })
    }

    fn array_binding_pattern(&mut self) -> Result<BindingPattern, ParseError> {
        let start = self.advance().span.start;
        let mut elements = Vec::new();
        let mut rest = None;
        while !self.at(&TokenKind::RightBracket) {
            if self.match_kind(&TokenKind::Comma) {
                elements.push(None);
                continue;
            }
            if self.match_kind(&TokenKind::DotDotDot) {
                let pattern = self.binding_pattern()?;
                if self.at(&TokenKind::Equal) {
                    return Err(ParseError {
                        message: "rest element must not have a default".to_owned(),
                        span: pattern.span(),
                    });
                }
                rest = Some(Box::new(pattern));
                break;
            }

            elements.push(Some(self.binding_element()?));
            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        if !self.at(&TokenKind::RightBracket) {
            let token = self.peek().expect("parser should always have eof token");
            return Err(ParseError {
                message: "expected `RightBracket`".to_owned(),
                span: token.span,
            });
        }
        let end = self.advance().span.end;
        Ok(BindingPattern::Array {
            elements,
            rest,
            span: Span::new(start, end),
        })
    }

    fn object_binding_pattern(&mut self) -> Result<BindingPattern, ParseError> {
        let start = self.advance().span.start;
        let mut properties = Vec::new();
        let mut rest = None;
        while !self.at(&TokenKind::RightBrace) {
            if self.match_kind(&TokenKind::DotDotDot) {
                let token = self.advance();
                let TokenKind::Identifier(name) = token.kind else {
                    return Err(ParseError {
                        message: "expected rest binding identifier".to_owned(),
                        span: token.span,
                    });
                };
                self.validate_binding_identifier_name(&name, token.span)?;
                rest = Some(Box::new(BindingPattern::Identifier {
                    name,
                    span: token.span,
                }));
                break;
            }
            let key_token = self.advance();
            let key_span = key_token.span;
            let shorthand = matches!(key_token.kind, TokenKind::Identifier(_));
            let key = match key_token.kind {
                TokenKind::LeftBracket => {
                    let expr = self.assignment()?;
                    self.expect(&TokenKind::RightBracket)?;
                    ObjectBindingPropertyKey::Computed(expr)
                }
                TokenKind::Identifier(key) | TokenKind::String(key) | TokenKind::Number(key) => {
                    ObjectBindingPropertyKey::Literal(key)
                }
                kind => {
                    if let Some(key) = crate::expression::keyword_property_name(&kind) {
                        ObjectBindingPropertyKey::Literal(key.to_owned())
                    } else {
                        return Err(ParseError {
                            message: "expected binding property name".to_owned(),
                            span: key_span,
                        });
                    }
                }
            };

            let binding = if self.match_kind(&TokenKind::Colon) {
                self.binding_pattern()?
            } else if shorthand {
                let name = key
                    .as_literal()
                    .expect("shorthand keys are always literal")
                    .to_owned();
                // A shorthand binding `{ x }` is also a binding identifier, so
                // it may not be a reserved word -- including escaped spellings
                // like `{ break }`, whose StringValue is still `break`.
                self.validate_binding_identifier_name(&name, key_span)?;
                BindingPattern::Identifier {
                    name,
                    span: key_span,
                }
            } else {
                return Err(ParseError {
                    message: "expected `:` after binding property name".to_owned(),
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
                .map_or(binding.span().end, |expr| expr.span().end);
            properties.push(ObjectBindingProperty {
                key,
                binding,
                default,
                span: Span::new(key_span.start, end),
            });

            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
        }
        if !self.at(&TokenKind::RightBrace) {
            let token = self.peek().expect("parser should always have eof token");
            return Err(ParseError {
                message: "expected `RightBrace`".to_owned(),
                span: token.span,
            });
        }
        let end = self.advance().span.end;
        Ok(BindingPattern::Object {
            properties,
            rest,
            span: Span::new(start, end),
        })
    }
}
