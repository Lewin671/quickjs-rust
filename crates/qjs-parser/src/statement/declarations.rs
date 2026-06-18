use qjs_ast::{
    BindingElement, BindingPattern, ForInit, ObjectBindingProperty, ObjectBindingPropertyKey, Span,
    Stmt, VarDeclarator, VarKind,
};
use qjs_lexer::TokenKind;

use crate::{ParseError, Parser};

impl Parser {
    /// Recognizes the start of a `using` / `await using` declaration. Both are
    /// contextual: `using` is a declaration only when immediately followed (no
    /// LineTerminator) by a `BindingIdentifier` on the same line, and `await
    /// using` only where `await` is a keyword (async function or module). A
    /// `using` followed by a newline, `[`, `{`, `=`, or anything other than a
    /// binding identifier is an ordinary `using` identifier expression.
    pub(super) fn using_declaration_kind(&self) -> Option<VarKind> {
        // `await using x` -- only where `await` is a keyword (async function or,
        // because module top level sets `in_async`, module code).
        if self.in_async
            && self.token_is_identifier(0, "await")
            && self.token_is_identifier(1, "using")
            && self.identifier_follows_on_same_line(1)
            && self.tokens_on_same_line(0, 1)
        {
            return Some(VarKind::AwaitUsing);
        }
        // `using x`
        if self.token_is_identifier(0, "using") && self.identifier_follows_on_same_line(0) {
            return Some(VarKind::Using);
        }
        None
    }

    fn token_is_identifier(&self, offset: usize, name: &str) -> bool {
        matches!(
            self.peek_nth(offset),
            Some(token) if matches!(&token.kind, TokenKind::Identifier(value) if value == name)
        )
    }

    /// Whether the token after `peek_nth(offset)` is a `BindingIdentifier` (a
    /// plain identifier, not `[`/`{`) on the same source line.
    fn identifier_follows_on_same_line(&self, offset: usize) -> bool {
        let Some(next) = self.peek_nth(offset + 1) else {
            return false;
        };
        matches!(next.kind, TokenKind::Identifier(_))
            && self.tokens_on_same_line(offset, offset + 1)
    }

    fn tokens_on_same_line(&self, left_offset: usize, right_offset: usize) -> bool {
        let (Some(left), Some(right)) = (self.peek_nth(left_offset), self.peek_nth(right_offset))
        else {
            return false;
        };
        !self.has_line_terminator_between(left.span.end, right.span.start)
    }

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
        } else if let Some(using_kind) = self.using_declaration_kind() {
            if using_kind == VarKind::AwaitUsing {
                self.advance(); // `await`
            }
            self.advance(); // `using`
            using_kind
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
                if kind.is_using() {
                    return Err(ParseError {
                        message: "`using` declarations require an initializer".to_owned(),
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
        // `using`/`await using` bind only simple identifiers; array/object
        // binding patterns are a SyntaxError.
        if kind.is_using()
            && !matches!(self.peek(), Some(token) if matches!(token.kind, TokenKind::Identifier(_)))
        {
            let span = self.peek().map_or(Span::new(0, 0), |token| token.span);
            return Err(ParseError {
                message: "`using` declarations may only bind identifiers".to_owned(),
                span,
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
        // `import` and `export` are reserved words and may not name a binding
        // (including their escaped spellings, which reach the parser as plain
        // Identifier tokens). They stay valid as property names and as the
        // `import(...)`/`import.meta` contextual forms, which never reach here.
        if matches!(name, "import" | "export") {
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
        // `yield` may not be used as a binding identifier inside a generator
        // (parameters or local bindings), a class static block, or anywhere in
        // strict mode. Ordinary nested functions reset the generator context,
        // so `yield` is a legal binding name there in sloppy code.
        if (self.strict || self.in_generator || self.in_static_block) && name == "yield" {
            return Err(ParseError {
                message: "`yield` is not allowed as a binding identifier here".to_owned(),
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
