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
        if is_generator && self.strict && name == "yield" {
            return Err(ParseError {
                message: "generator declaration may not be named `yield` in strict mode".to_owned(),
                span: name_token.span,
            });
        }
        if is_async && name == "await" {
            return Err(ParseError {
                message: "async function declaration may not be named `await`".to_owned(),
                span: name_token.span,
            });
        }

        let params = self.function_parameters_with_context(is_generator, is_async)?;
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let body = self.function_body_with_context(is_generator, is_async)?;
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
        if !matches!(&async_token.kind, TokenKind::Identifier(name) if name == "async") {
            return false;
        }
        let Some(function_token) = self.peek_nth(1) else {
            return false;
        };
        function_token.kind == TokenKind::Function
            && !self.has_line_terminator_between(async_token.span.end, function_token.span.start)
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
            if is_generator && self.strict && name == "yield" {
                return Err(ParseError {
                    message: "generator expression may not be named `yield` in strict mode"
                        .to_owned(),
                    span: token.span,
                });
            }
            if is_async && name == "await" {
                return Err(ParseError {
                    message: "async function expression may not be named `await`".to_owned(),
                    span: token.span,
                });
            }
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
        self.in_generator = is_generator;
        self.in_async = is_async;
        let body = self.without_super_context(Self::block_body);
        self.in_generator = previous_generator;
        self.in_async = previous_async;
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
        // Inside a generator/async parameter list `yield`/`await` is in the
        // keyword context (so it is recognized) but the corresponding
        // expression is an early error.
        self.in_generator = is_generator;
        self.in_generator_params = is_generator;
        self.in_async = is_async;
        self.in_async_params = is_async;
        let params = self.parse_function_parameters();
        self.in_generator = previous_generator;
        self.in_generator_params = previous_generator_params;
        self.in_async = previous_async;
        self.in_async_params = previous_async_params;
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
