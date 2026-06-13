use qjs_ast::{
    ArrayElement, CallArgument, Expr, Literal, ObjectProperty, ObjectPropertyKey,
    ObjectPropertyKind, Span,
};
use qjs_lexer::{TemplateSegment, TokenKind};

use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn primary(&mut self) -> Result<Expr, ParseError> {
        // `async function ...` is an async function expression. `async` is
        // contextual; with no `function` immediately after it is a plain
        // identifier (and `async ... =>` arrow forms are handled earlier in the
        // assignment parser).
        if self.at_async_function_keyword() {
            let async_token = self.advance();
            self.expect(&TokenKind::Function)?;
            return self.function_expression_with_async(async_token.span.start, true);
        }
        // `import(...)` dynamic import and `import.meta` meta-property. `import`
        // is a contextual keyword (lexed as an identifier); a following `(` or
        // `.` selects these expression forms rather than a plain reference.
        if self.at_import_expression() {
            return self.import_expression();
        }
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier(name) => {
                if crate::helpers::is_reserved_identifier_name(&name) {
                    return Err(ParseError {
                        message: format!("`{name}` is a reserved word"),
                        span: token.span,
                    });
                }
                if (self.strict || self.in_generator) && name == "yield" {
                    return Err(ParseError {
                        message: "`yield` may not be used as an identifier here".to_owned(),
                        span: token.span,
                    });
                }
                if self.in_static_block && matches!(name.as_str(), "arguments" | "await" | "yield")
                {
                    return Err(ParseError {
                        message: format!("`{name}` is not allowed in a class static block"),
                        span: token.span,
                    });
                }
                if self.in_field_initializer && name == "arguments" {
                    return Err(ParseError {
                        message: "'arguments' is not allowed in a class field initializer"
                            .to_owned(),
                        span: token.span,
                    });
                }
                Ok(Expr::Identifier {
                    name,
                    span: token.span,
                })
            }
            TokenKind::Let => Ok(Expr::Identifier {
                name: "let".to_owned(),
                span: token.span,
            }),
            TokenKind::Number(raw) => Ok(Expr::Literal(Literal::Number {
                raw,
                span: token.span,
            })),
            TokenKind::BigInt(raw) => Ok(Expr::Literal(Literal::BigInt {
                raw,
                span: token.span,
            })),
            TokenKind::String(value) => {
                self.reject_strict_legacy_octal_escape(
                    &self.source[token.span.start..token.span.end],
                    token.span,
                )?;
                Ok(Expr::Literal(Literal::String {
                    value,
                    span: token.span,
                }))
            }
            TokenKind::TemplateNoSubstitution(segment) => {
                self.reject_strict_legacy_octal_escape(&segment.raw, token.span)?;
                Ok(Expr::Literal(Literal::String {
                    value: segment.cooked,
                    span: token.span,
                }))
            }
            TokenKind::TemplateHead(segment) => self.template_literal(segment, token.span.start),
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
            TokenKind::Super => self.super_expression(token.span),
            TokenKind::Function => self.function_expression(token.span.start),
            TokenKind::Class => self.class_expression(token.span.start),
            TokenKind::RegularExpression { pattern, flags } => {
                Ok(regexp_constructor_expr(token.span, pattern, flags))
            }
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

    /// Parses a `super` keyword reference, validating that it appears in a
    /// legal context. `super.x`/`super[x]` require a method or accessor body;
    /// `super(...)` requires a derived-class constructor body. A bare `super`
    /// (not followed by `.`, `[`, or `(`) is always a syntax error.
    fn super_expression(&mut self, span: Span) -> Result<Expr, ParseError> {
        match self.peek().map(|token| &token.kind) {
            Some(TokenKind::Dot | TokenKind::LeftBracket) => {
                if !self.in_method {
                    return Err(ParseError {
                        message: "'super' property access is only allowed in methods".to_owned(),
                        span,
                    });
                }
            }
            Some(TokenKind::LeftParen) => {
                if !self.in_derived_constructor {
                    return Err(ParseError {
                        message: "'super' calls are only allowed in derived class constructors"
                            .to_owned(),
                        span,
                    });
                }
            }
            _ => {
                return Err(ParseError {
                    message: "'super' must be followed by '.', '[', or '('".to_owned(),
                    span,
                });
            }
        }
        Ok(Expr::Super { span })
    }

    fn regexp_literal(&mut self, start: usize) -> Result<Expr, ParseError> {
        let mut pattern = String::new();
        let mut previous_end = start + 1;
        loop {
            let token = self.advance();
            if token.span.start > previous_end {
                pattern.push_str(&self.source[previous_end..token.span.start]);
            }
            match token.kind {
                TokenKind::Slash => {
                    let end = token.span.end;
                    let mut arguments = vec![CallArgument::Expr(Expr::Literal(Literal::String {
                        value: pattern,
                        span: Span::new(start, end),
                    }))];
                    if let Some(flags) = self.regexp_flags() {
                        arguments.push(CallArgument::Expr(Expr::Literal(Literal::String {
                            span: flags.span,
                            value: flags.value,
                        })));
                    }
                    let span_end = arguments
                        .last()
                        .map_or(end, |argument| call_argument_span(argument).end);
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
                    match escaped.kind {
                        TokenKind::Identifier(text)
                        | TokenKind::Number(text)
                        | TokenKind::String(text) => pattern.push_str(&text),
                        kind => pattern.push_str(regexp_token_text(&kind).ok_or(ParseError {
                            message: "unsupported regular expression escape".to_owned(),
                            span: escaped.span,
                        })?),
                    }
                    previous_end = escaped.span.end;
                    continue;
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
            previous_end = token.span.end;
        }
    }

    pub(crate) fn template_literal(
        &mut self,
        head: TemplateSegment,
        start: usize,
    ) -> Result<Expr, ParseError> {
        self.reject_strict_legacy_octal_escape(&head.raw, Span::new(start, start))?;
        let mut parts = vec![head.cooked];
        let mut expressions = Vec::new();
        loop {
            expressions.push(self.assignment()?);
            let token = self.advance();
            match token.kind {
                TokenKind::TemplateMiddle(segment) => {
                    self.reject_strict_legacy_octal_escape(&segment.raw, token.span)?;
                    parts.push(segment.cooked);
                }
                TokenKind::TemplateTail(segment) => {
                    self.reject_strict_legacy_octal_escape(&segment.raw, token.span)?;
                    parts.push(segment.cooked);
                    return Ok(Expr::Template {
                        parts,
                        expressions,
                        span: Span::new(start, token.span.end),
                    });
                }
                _ => {
                    return Err(ParseError {
                        message: "expected template literal segment".to_owned(),
                        span: token.span,
                    });
                }
            }
        }
    }

    pub(crate) fn at_template_literal(&self) -> bool {
        matches!(
            self.peek().map(|token| &token.kind),
            Some(TokenKind::TemplateNoSubstitution(_) | TokenKind::TemplateHead(_))
        )
    }

    pub(crate) fn finish_tagged_template_literal(&mut self, tag: Expr) -> Result<Expr, ParseError> {
        let token = self
            .peek()
            .cloned()
            .expect("parser should always have eof token");
        match token.kind {
            TokenKind::TemplateNoSubstitution(segment) => {
                self.advance();
                let span = Span::new(tag.span().start, token.span.end);
                Ok(Expr::TaggedTemplate {
                    tag: Box::new(tag),
                    cooked: vec![segment.cooked],
                    raw: vec![segment.raw],
                    expressions: Vec::new(),
                    span,
                })
            }
            TokenKind::TemplateHead(segment) => {
                let start = tag.span().start;
                self.advance();
                let mut cooked = vec![segment.cooked];
                let mut raw = vec![segment.raw];
                let mut expressions = Vec::new();
                loop {
                    expressions.push(self.assignment()?);
                    let token = self.advance();
                    match token.kind {
                        TokenKind::TemplateMiddle(segment) => {
                            cooked.push(segment.cooked);
                            raw.push(segment.raw);
                        }
                        TokenKind::TemplateTail(segment) => {
                            cooked.push(segment.cooked);
                            raw.push(segment.raw);
                            return Ok(Expr::TaggedTemplate {
                                tag: Box::new(tag),
                                cooked,
                                raw,
                                expressions,
                                span: Span::new(start, token.span.end),
                            });
                        }
                        _ => {
                            return Err(ParseError {
                                message: "expected template literal segment".to_owned(),
                                span: token.span,
                            });
                        }
                    }
                }
            }
            _ => Err(ParseError {
                message: "expected template literal".to_owned(),
                span: token.span,
            }),
        }
    }

    fn reject_strict_legacy_octal_escape(&self, raw: &str, span: Span) -> Result<(), ParseError> {
        if !self.strict || !has_legacy_octal_escape(raw) {
            return Ok(());
        }
        Err(ParseError {
            message: "legacy octal escape sequence is not allowed in strict mode".to_owned(),
            span,
        })
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
                    elements.push(ArrayElement::Elision);
                    self.advance();
                    if self.at(&TokenKind::RightBracket) {
                        break;
                    }
                    continue;
                }
                if self.match_kind(&TokenKind::DotDotDot) {
                    elements.push(ArrayElement::Spread(self.assignment()?));
                } else {
                    elements.push(ArrayElement::Expr(self.assignment()?));
                }
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
                if self.at(&TokenKind::DotDotDot) {
                    let spread_token = self.advance();
                    let argument = self.assignment()?;
                    properties.push(ObjectProperty {
                        key: ObjectPropertyKey::Literal(String::new()),
                        kind: ObjectPropertyKind::Spread,
                        is_proto_setter: false,
                        span: Span::new(spread_token.span.start, argument.span().end),
                        value: argument,
                    });
                    if !self.match_kind(&TokenKind::Comma) {
                        break;
                    }
                    if self.at(&TokenKind::RightBrace) {
                        break;
                    }
                    continue;
                }
                if self.at(&TokenKind::Star) {
                    let property = self.object_generator_method(false, None)?;
                    properties.push(property);
                    if !self.match_kind(&TokenKind::Comma) {
                        break;
                    }
                    if self.at(&TokenKind::RightBrace) {
                        break;
                    }
                    continue;
                }
                // `async m() {}` / `async *m() {}` async methods. `async` is a
                // modifier only when it is followed (no line terminator) by a
                // method-name start; otherwise it is a property name.
                if self.at_async_method_prefix() {
                    let async_token = self.advance();
                    let property = if self.at(&TokenKind::Star) {
                        self.object_generator_method(true, Some(async_token.span.start))?
                    } else {
                        self.object_async_method(async_token.span.start)?
                    };
                    properties.push(property);
                    if !self.match_kind(&TokenKind::Comma) {
                        break;
                    }
                    if self.at(&TokenKind::RightBrace) {
                        break;
                    }
                    continue;
                }
                let key_token = self.advance();
                let key_span = key_token.span;
                // A `\u`-escaped `get`/`set` is never the accessor keyword
                // (12.7.2): it is treated as an ordinary property name, which
                // makes `get m() {}` a SyntaxError.
                let key_had_escape = key_token.had_escape;
                let (key, kind, is_proto_setter, value) = if is_get_accessor_start(&key_token.kind)
                    && !key_had_escape
                    && !self.at(&TokenKind::Colon)
                    && !self.at(&TokenKind::Comma)
                    && !self.at(&TokenKind::LeftParen)
                    && !self.at(&TokenKind::RightBrace)
                {
                    let (key, kind, value) = self.object_getter_property(key_span)?;
                    (key, kind, false, value)
                } else if is_set_accessor_start(&key_token.kind)
                    && !key_had_escape
                    && !self.at(&TokenKind::Colon)
                    && !self.at(&TokenKind::Comma)
                    && !self.at(&TokenKind::LeftParen)
                    && !self.at(&TokenKind::RightBrace)
                {
                    let (key, kind, value) = self.object_setter_property(key_span)?;
                    (key, kind, false, value)
                } else {
                    let (key, shorthand_value) = self.object_property_key(key_token)?;
                    let (value, is_colon_data) =
                        self.object_property_value(key_span, &key, shorthand_value)?;
                    // `{ __proto__: expr }` sets [[Prototype]] only for the
                    // literal-key colon data form, not shorthand/computed/method.
                    let is_proto_setter = is_colon_data
                        && matches!(&key, ObjectPropertyKey::Literal(name) if name == "__proto__");
                    (key, ObjectPropertyKind::Data, is_proto_setter, value)
                };
                let span = Span::new(key_span.start, value.span().end);
                properties.push(ObjectProperty {
                    key,
                    kind,
                    is_proto_setter,
                    value,
                    span,
                });
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
            TokenKind::String(name) => Ok((ObjectPropertyKey::Literal(name), None)),
            TokenKind::Number(raw) => Ok((
                ObjectPropertyKey::Literal(crate::helpers::numeric_property_key(&raw)),
                None,
            )),
            TokenKind::True => Ok((ObjectPropertyKey::Literal("true".to_owned()), None)),
            TokenKind::False => Ok((ObjectPropertyKey::Literal("false".to_owned()), None)),
            TokenKind::Null => Ok((ObjectPropertyKey::Literal("null".to_owned()), None)),
            TokenKind::LeftBracket => {
                let name = self.assignment()?;
                self.expect(&TokenKind::RightBracket)?;
                Ok((ObjectPropertyKey::Computed(name), None))
            }
            kind => {
                if let Some(name) = keyword_property_name(&kind) {
                    Ok((ObjectPropertyKey::Literal(name.to_owned()), None))
                } else {
                    Err(ParseError {
                        message: "expected property name".to_owned(),
                        span: key_token.span,
                    })
                }
            }
        }
    }

    /// Parses an object property value. The returned flag is `true` only for
    /// the plain colon data form (`PropertyName : AssignmentExpression`), which
    /// is the only shape eligible for the `__proto__` prototype special form.
    fn object_property_value(
        &mut self,
        key_span: Span,
        key: &ObjectPropertyKey,
        shorthand_value: Option<Expr>,
    ) -> Result<(Expr, bool), ParseError> {
        if self.at(&TokenKind::LeftParen) {
            let method_name = match key {
                ObjectPropertyKey::Literal(name) => Some(name.clone()),
                ObjectPropertyKey::Computed(_) => None,
            };
            let params = self.function_parameters()?;
            reject_duplicate_method_parameters(&params)?;
            let body_start = self
                .peek()
                .expect("parser should always have eof token")
                .span
                .start;
            let body = self.function_body(false)?;
            self.reject_invalid_function_parameters(&params, &body, body_start)?;
            let end = self
                .tokens
                .get(self.cursor.saturating_sub(1))
                .expect("parser should always have eof token")
                .span
                .end;
            return Ok((
                Expr::Function {
                    name: method_name,
                    params,
                    body,
                    constructable: false,
                    lexical_this: false,
                    lexical_arguments: false,
                    is_generator: false,
                    is_async: false,
                    span: Span::new(key_span.start, end),
                },
                false,
            ));
        }

        if self.match_kind(&TokenKind::Colon) {
            return Ok((self.assignment()?, true));
        }

        if let Some(value) = shorthand_value {
            return Ok((value, false));
        }

        Err(ParseError {
            message: "expected `:` after property name".to_owned(),
            span: match key {
                ObjectPropertyKey::Literal(_) => key_span,
                ObjectPropertyKey::Computed(expr) => expr.span(),
            },
        })
    }

    fn object_getter_property(
        &mut self,
        start_span: Span,
    ) -> Result<(ObjectPropertyKey, ObjectPropertyKind, Expr), ParseError> {
        let key_token = self.advance();
        let key_span = key_token.span;
        let (key, _) = self.object_property_key(key_token)?;
        let params = self.function_parameters()?;
        if !params.is_empty() {
            return Err(ParseError {
                message: "getter must not have parameters".to_owned(),
                span: key_span,
            });
        }
        let body = self.function_body(false)?;
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;
        let name = match &key {
            ObjectPropertyKey::Literal(name) => Some(name.clone()),
            ObjectPropertyKey::Computed(_) => None,
        };
        Ok((
            key,
            ObjectPropertyKind::Getter,
            Expr::Function {
                name,
                params,
                body,
                constructable: false,
                lexical_this: false,
                lexical_arguments: false,
                is_generator: false,
                is_async: false,
                span: Span::new(start_span.start, end),
            },
        ))
    }

    fn object_setter_property(
        &mut self,
        start_span: Span,
    ) -> Result<(ObjectPropertyKey, ObjectPropertyKind, Expr), ParseError> {
        let key_token = self.advance();
        let key_span = key_token.span;
        let (key, _) = self.object_property_key(key_token)?;
        let params = self.function_parameters()?;
        if params.positional.len() != 1 || params.rest.is_some() {
            return Err(ParseError {
                message: "setter must have exactly one parameter".to_owned(),
                span: key_span,
            });
        }
        reject_duplicate_method_parameters(&params)?;
        let body = self.function_body(false)?;
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;
        let name = match &key {
            ObjectPropertyKey::Literal(name) => Some(name.clone()),
            ObjectPropertyKey::Computed(_) => None,
        };
        Ok((
            key,
            ObjectPropertyKind::Setter,
            Expr::Function {
                name,
                params,
                body,
                constructable: false,
                lexical_this: false,
                lexical_arguments: false,
                is_generator: false,
                is_async: false,
                span: Span::new(start_span.start, end),
            },
        ))
    }

    /// Parses a `*name() { ... }` generator method in an object literal. The
    /// leading `*` is the current token. `is_async` marks an `async *name()`
    /// async generator method, with `async_start` the byte offset of `async`.
    fn object_generator_method(
        &mut self,
        is_async: bool,
        async_start: Option<usize>,
    ) -> Result<ObjectProperty, ParseError> {
        let star = self.advance();
        let start = async_start.unwrap_or(star.span.start);
        let key_token = self.advance();
        let (key, _) = self.object_property_key(key_token)?;
        let method_name = match &key {
            ObjectPropertyKey::Literal(name) => Some(name.clone()),
            ObjectPropertyKey::Computed(_) => None,
        };
        let params = self.function_parameters_with_context(true, is_async)?;
        reject_duplicate_method_parameters(&params)?;
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let body = self.function_body_with_context(true, is_async)?;
        self.reject_invalid_function_parameters(&params, &body, body_start)?;
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;
        let value = Expr::Function {
            name: method_name,
            params,
            body,
            constructable: false,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: true,
            is_async,
            span: Span::new(start, end),
        };
        Ok(ObjectProperty {
            key,
            kind: ObjectPropertyKind::Data,
            is_proto_setter: false,
            value,
            span: Span::new(start, end),
        })
    }

    /// Parses an `async name() { ... }` async method in an object literal. The
    /// `async` keyword has already been consumed; `start` is its byte offset.
    fn object_async_method(&mut self, start: usize) -> Result<ObjectProperty, ParseError> {
        let key_token = self.advance();
        let key_span = key_token.span;
        let (key, _) = self.object_property_key(key_token)?;
        let method_name = match &key {
            ObjectPropertyKey::Literal(name) => Some(name.clone()),
            ObjectPropertyKey::Computed(_) => None,
        };
        if !self.at(&TokenKind::LeftParen) {
            return Err(ParseError {
                message: "expected `(` after async method name".to_owned(),
                span: key_span,
            });
        }
        let params = self.function_parameters_with_context(false, true)?;
        reject_duplicate_method_parameters(&params)?;
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let body = self.function_body_with_context(false, true)?;
        self.reject_invalid_function_parameters(&params, &body, body_start)?;
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;
        let value = Expr::Function {
            name: method_name,
            params,
            body,
            constructable: false,
            lexical_this: false,
            lexical_arguments: false,
            is_generator: false,
            is_async: true,
            span: Span::new(start, end),
        };
        Ok(ObjectProperty {
            key,
            kind: ObjectPropertyKind::Data,
            is_proto_setter: false,
            value,
            span: Span::new(start, end),
        })
    }

    /// Reports whether the parser is at an `async` method prefix in an object
    /// literal or class body: an `async` identifier with no following line
    /// terminator, followed by a token that begins a method name (or `*` for an
    /// async generator) rather than `(`, `:`, `,`, `}`, or `=` which would make
    /// `async` itself the property/field name.
    pub(crate) fn at_async_method_prefix(&self) -> bool {
        let Some(async_token) = self.peek() else {
            return false;
        };
        // A `\u`-escaped spelling of `async` is never the method modifier
        // keyword (12.7.2): `async m() {}` is a SyntaxError, not a method.
        if async_token.had_escape
            || !matches!(&async_token.kind, TokenKind::Identifier(name) if name == "async")
        {
            return false;
        }
        let Some(next) = self.peek_nth(1) else {
            return false;
        };
        if self.has_line_terminator_between(async_token.span.end, next.span.start) {
            return false;
        }
        token_starts_async_method_name(&next.kind)
    }
}

/// Reports whether a token can follow the `async` method modifier as the start
/// of a method name (or generator marker). Punctuators that would make `async`
/// a plain property name or field are excluded.
pub(crate) fn token_starts_async_method_name(kind: &TokenKind) -> bool {
    match kind {
        TokenKind::Star
        | TokenKind::Identifier(_)
        | TokenKind::PrivateName(_)
        | TokenKind::String(_)
        | TokenKind::Number(_)
        | TokenKind::LeftBracket => true,
        other => keyword_property_name(other).is_some(),
    }
}

fn has_legacy_octal_escape(raw: &str) -> bool {
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            continue;
        }
        let Some(escaped) = chars.next() else {
            return false;
        };
        match escaped {
            '0' => {
                if matches!(chars.peek(), Some('0'..='9')) {
                    return true;
                }
            }
            '1'..='7' => return true,
            _ => {}
        }
    }
    false
}

fn reject_duplicate_method_parameters(params: &qjs_ast::FunctionParams) -> Result<(), ParseError> {
    if let Some(span) = crate::statement::duplicate_parameter_span(params) {
        return Err(ParseError {
            message: "duplicate parameter name".to_owned(),
            span,
        });
    }
    Ok(())
}

pub(crate) fn keyword_property_name(kind: &TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::This => Some("this"),
        TokenKind::Var => Some("var"),
        TokenKind::Let => Some("let"),
        TokenKind::Const => Some("const"),
        TokenKind::If => Some("if"),
        TokenKind::Else => Some("else"),
        TokenKind::While => Some("while"),
        TokenKind::Do => Some("do"),
        TokenKind::For => Some("for"),
        TokenKind::Switch => Some("switch"),
        TokenKind::Case => Some("case"),
        TokenKind::Default => Some("default"),
        TokenKind::Try => Some("try"),
        TokenKind::Catch => Some("catch"),
        TokenKind::Finally => Some("finally"),
        TokenKind::Break => Some("break"),
        TokenKind::Continue => Some("continue"),
        TokenKind::Function => Some("function"),
        TokenKind::Class => Some("class"),
        TokenKind::Extends => Some("extends"),
        TokenKind::Super => Some("super"),
        TokenKind::Return => Some("return"),
        TokenKind::Throw => Some("throw"),
        TokenKind::Debugger => Some("debugger"),
        TokenKind::Typeof => Some("typeof"),
        TokenKind::Void => Some("void"),
        TokenKind::In => Some("in"),
        TokenKind::With => Some("with"),
        TokenKind::Delete => Some("delete"),
        TokenKind::New => Some("new"),
        TokenKind::Instanceof => Some("instanceof"),
        _ => None,
    }
}

fn regexp_constructor_expr(span: Span, pattern: String, flags: String) -> Expr {
    let closing_slash = span.end - flags.len() - 1;
    let mut arguments = vec![CallArgument::Expr(Expr::Literal(Literal::String {
        value: pattern,
        span: Span::new(span.start, closing_slash + 1),
    }))];
    if !flags.is_empty() {
        arguments.push(CallArgument::Expr(Expr::Literal(Literal::String {
            value: flags,
            span: Span::new(closing_slash + 1, span.end),
        })));
    }

    Expr::New {
        callee: Box::new(Expr::Identifier {
            name: "RegExp".to_owned(),
            span: Span::new(span.start, span.start + 1),
        }),
        span,
        arguments,
    }
}

fn call_argument_span(argument: &CallArgument) -> Span {
    match argument {
        CallArgument::Expr(expr) | CallArgument::Spread(expr) => expr.span(),
    }
}

fn is_get_accessor_start(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Identifier(name) if name == "get")
}

fn is_set_accessor_start(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Identifier(name) if name == "set")
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
        TokenKind::Comma => Some(","),
        TokenKind::Colon => Some(":"),
        TokenKind::Pipe => Some("|"),
        TokenKind::Caret => Some("^"),
        _ => None,
    }
}
