use qjs_ast::{Expr, FunctionParams, Span, Stmt};
use qjs_lexer::{Token, TokenKind};

use crate::helpers::body_has_strict_directive;
use crate::{ParseError, Parser};

impl Parser {
    /// Parses a function declaration. `is_async` is set when an `async` prefix
    /// was already consumed by the caller; the `function` keyword is the
    /// current token.
    pub(super) fn function_declaration_with_async(
        &mut self,
        start: usize,
        is_async: bool,
    ) -> Result<Stmt, ParseError> {
        self.expect(&TokenKind::Function)?;
        let is_generator = self.match_kind(&TokenKind::Star);
        let name_token = self.advance();
        let TokenKind::Identifier(name) = name_token.kind else {
            return Err(ParseError {
                message: "expected function name".to_owned(),
                span: name_token.span,
            });
        };
        // The BindingIdentifier of a function declaration is checked against the
        // *enclosing* Yield/Await context and strict mode, not against whether
        // the function being declared is itself a generator/async function.
        self.check_binding_identifier(&name, name_token.span)?;

        let params = self.function_parameters_with_context(is_generator, is_async)?;
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let body = self.function_body_with_context(is_generator, is_async)?;
        self.reject_invalid_function_parameters(&params, &body, body_start)?;
        reject_strict_function_name(&name, &body, body_start)?;
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
            is_generator,
            is_async,
            span: Span::new(start.min(body_start), end),
        })
    }

    /// Reports whether the parser is positioned at `async function` with no
    /// line terminator between the two tokens, which begins an async function
    /// declaration or expression. `async` is contextual, so a following line
    /// terminator (which would force ASI) leaves `async` as an identifier.
    pub(crate) fn at_async_function_keyword(&self) -> bool {
        let Some(async_token) = self.peek() else {
            return false;
        };
        if async_token.had_escape
            || !matches!(&async_token.kind, TokenKind::Identifier(name) if name == "async")
        {
            return false;
        }
        let Some(function_token) = self.peek_nth(1) else {
            return false;
        };
        function_token.kind == TokenKind::Function
            && !self.has_line_terminator_between(async_token.span.end, function_token.span.start)
    }

    pub(crate) fn escaped_async_function_keyword_error(&self) -> Option<ParseError> {
        let async_token = self.peek()?;
        if !async_token.had_escape
            || !matches!(&async_token.kind, TokenKind::Identifier(name) if name == "async")
        {
            return None;
        }
        let function_token = self.peek_nth(1)?;
        if function_token.kind == TokenKind::Function
            && !self.has_line_terminator_between(async_token.span.end, function_token.span.start)
        {
            return Some(ParseError {
                message: "`async` function keyword must not contain escape sequences".to_owned(),
                span: async_token.span,
            });
        }
        None
    }

    pub(super) fn function_declaration(&mut self) -> Result<Stmt, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        self.function_declaration_with_async(start, false)
    }

    pub(crate) fn function_expression(&mut self, start: usize) -> Result<Expr, ParseError> {
        self.function_expression_with_async(start, false)
    }

    /// Parses a function expression. `is_async` is set when an `async` prefix
    /// was already consumed by the caller; the `function` keyword has already
    /// been consumed by the caller.
    pub(crate) fn function_expression_with_async(
        &mut self,
        start: usize,
        is_async: bool,
    ) -> Result<Expr, ParseError> {
        let is_generator = self.match_kind(&TokenKind::Star);

        let name = if let Some(Token {
            kind: TokenKind::Identifier(_),
            ..
        }) = self.peek()
        {
            let token = self.advance();
            let TokenKind::Identifier(name) = token.kind else {
                unreachable!("peek checked identifier");
            };
            // A named function expression's BindingIdentifier is in the scope of
            // its own name, so `await`/`yield` are checked against the inner
            // generator/async context as well as the enclosing one.
            if is_generator && name == "yield" {
                return Err(ParseError {
                    message: "function expression may not be named `yield` in a generator"
                        .to_owned(),
                    span: token.span,
                });
            }
            if is_async && name == "await" {
                return Err(ParseError {
                    message: "function expression may not be named `await` in an async function"
                        .to_owned(),
                    span: token.span,
                });
            }
            self.check_binding_identifier(&name, token.span)?;
            Some(name)
        } else {
            None
        };

        let params = self.function_parameters_with_context(is_generator, is_async)?;
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let body = self.function_body_with_context(is_generator, is_async)?;
        self.reject_invalid_function_parameters(&params, &body, body_start)?;
        if let Some(ref fn_name) = name {
            reject_strict_function_name(fn_name, &body, body_start)?;
        }
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
            constructable: !is_generator && !is_async,
            lexical_this: false,
            lexical_arguments: false,
            is_generator,
            is_async,
            span: Span::new(start, end),
        })
    }

    /// Parses a function body block, setting the generator yield context for
    /// the duration of the body. Regular function and generator boundaries
    /// reset the surrounding `super`/yield context; arrows reuse this through
    /// the inherited context instead.
    pub(crate) fn function_body(&mut self, is_generator: bool) -> Result<Vec<Stmt>, ParseError> {
        self.function_body_with_context(is_generator, false)
    }

    /// Parses a function body block, setting both the generator yield context
    /// and the async await context for the duration of the body. Regular
    /// function boundaries reset the surrounding generator/async context;
    /// arrows reuse it through the inherited context instead.
    pub(crate) fn function_body_with_context(
        &mut self,
        is_generator: bool,
        is_async: bool,
    ) -> Result<Vec<Stmt>, ParseError> {
        let previous_generator = self.in_generator;
        let previous_async = self.in_async;
        let previous_static_block = self.in_static_block;
        let previous_function = self.in_function;
        self.in_generator = is_generator;
        self.in_async = is_async;
        self.in_static_block = false;
        self.in_function = true;
        let body = self.without_super_context(Self::block_body);
        self.in_generator = previous_generator;
        self.in_async = previous_async;
        self.in_static_block = previous_static_block;
        self.in_function = previous_function;
        body
    }

    pub(crate) fn function_parameters(&mut self) -> Result<FunctionParams, ParseError> {
        self.function_parameters_with_context(false, false)
    }

    /// Parses a formal parameter list. Generator parameters parse with the
    /// surrounding yield context disabled so a `yield` in a default initializer
    /// is an early error rather than a yield expression; async parameters
    /// likewise make an `await` expression in a default an early error.
    pub(crate) fn function_parameters_with_context(
        &mut self,
        is_generator: bool,
        is_async: bool,
    ) -> Result<FunctionParams, ParseError> {
        let previous_generator = self.in_generator;
        let previous_generator_params = self.in_generator_params;
        let previous_async = self.in_async;
        let previous_async_params = self.in_async_params;
        let previous_static_block = self.in_static_block;
        let previous_function = self.in_function;
        // Inside a generator/async parameter list `yield`/`await` is in the
        // keyword context (so it is recognized) but the corresponding
        // expression is an early error.
        self.in_generator = is_generator;
        self.in_generator_params = is_generator;
        self.in_async = is_async;
        self.in_async_params = is_async;
        self.in_static_block = false;
        self.in_function = true;
        let params = self.parse_function_parameters();
        self.in_generator = previous_generator;
        self.in_generator_params = previous_generator_params;
        self.in_async = previous_async;
        self.in_async_params = previous_async_params;
        self.in_static_block = previous_static_block;
        self.in_function = previous_function;
        params
    }

    fn parse_function_parameters(&mut self) -> Result<FunctionParams, ParseError> {
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
        let previous_field_initializer = self.in_field_initializer;
        self.in_method = false;
        self.in_derived_constructor = false;
        self.in_field_initializer = false;
        let result = body(self);
        self.in_method = previous_method;
        self.in_derived_constructor = previous_derived;
        self.in_field_initializer = previous_field_initializer;
        result
    }

    /// Validates a BindingIdentifier against the current strict-mode and
    /// Yield/Await context. `eval`/`arguments` are reserved binding names in
    /// strict mode; `yield` is reserved in strict mode or inside a generator;
    /// `await` is reserved inside an async function. Used for function
    /// declaration names (checked in the enclosing context) and other places
    /// that bind an identifier.
    pub(crate) fn check_binding_identifier(
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
        if self.strict && matches!(name, "eval" | "arguments") {
            return Err(ParseError {
                message: format!("`{name}` may not be used as a binding name in strict mode"),
                span,
            });
        }
        if (self.strict || self.in_generator) && name == "yield" {
            return Err(ParseError {
                message: "`yield` may not be used as a binding name here".to_owned(),
                span,
            });
        }
        if (self.in_async || self.in_static_block) && name == "await" {
            return Err(ParseError {
                message: "`await` may not be used as a binding name here".to_owned(),
                span,
            });
        }
        if self.strict && is_strict_reserved_word(name) {
            return Err(ParseError {
                message: format!("`{name}` is a reserved word in strict mode"),
                span,
            });
        }
        Ok(())
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
        // A parameter name may not also appear as a lexically declared name
        // (`let`/`const`/`class`) at the top level of the function body. This
        // holds for all functions, independent of strict mode.
        let lexical_names = body_lexically_declared_names(body);
        for (name, span) in params.named_spans() {
            if lexical_names.contains(&name) {
                return Err(ParseError {
                    message: format!(
                        "parameter `{name}` conflicts with a lexical declaration in the body"
                    ),
                    span,
                });
            }
        }
        Ok(())
    }
}

/// Rejects `eval` or `arguments` as a function name when the function body
/// contains a strict directive prologue. This is an early error that cannot
/// be caught before parsing the body.
fn reject_strict_function_name(
    name: &str,
    body: &[Stmt],
    span_start: usize,
) -> Result<(), crate::ParseError> {
    if matches!(name, "eval" | "arguments") && body_has_strict_directive(body) {
        return Err(crate::ParseError {
            message: format!("`{name}` may not be used as a function name in strict mode"),
            span: qjs_ast::Span::new(span_start, span_start),
        });
    }
    Ok(())
}

/// Collects the names lexically declared at the top level of a function body:
/// `let`/`const` bindings and `class` declarations. `var` and `function`
/// declarations are var-scoped, not lexical, so they are excluded.
fn body_lexically_declared_names(body: &[Stmt]) -> Vec<String> {
    let mut names = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::VarDecl {
                kind: qjs_ast::VarKind::Let | qjs_ast::VarKind::Const,
                declarations,
                ..
            } => {
                for declarator in declarations {
                    names.extend(declarator.binding.names());
                }
            }
            Stmt::ClassDecl { name, .. } => names.push(name.clone()),
            _ => {}
        }
    }
    names
}

/// Reports whether `name` is a word that may not be used as a binding
/// identifier (or label/reference) in strict-mode code. Covers the
/// strict-mode future reserved words plus `yield` and `static`. `let` is
/// contextual and handled by its own dedicated path; `eval`/`arguments` have
/// their own dedicated diagnostics. The StringValue comparison is
/// escape-insensitive, so an escaped spelling such as `package` is rejected
/// the same as `package`.
pub(crate) fn is_strict_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "implements"
            | "interface"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "static"
            | "yield"
    )
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
