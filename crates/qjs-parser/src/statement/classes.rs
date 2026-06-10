use qjs_ast::{
    ClassBody, ClassElement, ClassField, ClassMember, ClassMemberKey, Expr, FunctionParams,
    MethodKind, Span, Stmt,
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
        let heritage = self.class_heritage()?;
        let body = self.class_body(heritage)?;
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
        let heritage = self.class_heritage()?;
        let body = self.class_body(heritage)?;
        let span = Span::new(start, body.span.end);
        Ok(Expr::Class { name, body, span })
    }

    /// Parses an optional `extends LeftHandSideExpression` heritage clause.
    fn class_heritage(&mut self) -> Result<Option<Box<Expr>>, ParseError> {
        if !self.match_kind(&TokenKind::Extends) {
            return Ok(None);
        }
        // The heritage is a LeftHandSideExpression: a member/call chain, with
        // `super` not permitted as the bare base.
        let heritage = self.without_super_context(Self::call)?;
        Ok(Some(Box::new(heritage)))
    }

    fn class_body(&mut self, heritage: Option<Box<Expr>>) -> Result<ClassBody, ParseError> {
        let open = self
            .peek()
            .expect("parser should always have eof token")
            .span;
        self.expect(&TokenKind::LeftBrace)?;

        // Class bodies are always strict-mode code.
        let previous_strict = self.strict;
        self.strict = true;
        let result = self.class_members(open.start, heritage);
        self.strict = previous_strict;
        result
    }

    fn class_members(
        &mut self,
        start: usize,
        heritage: Option<Box<Expr>>,
    ) -> Result<ClassBody, ParseError> {
        let has_heritage = heritage.is_some();
        let mut elements = Vec::new();
        let mut seen_constructor = false;
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            // Empty members: bare semicolons are allowed between definitions.
            if self.match_kind(&TokenKind::Semicolon) {
                continue;
            }
            let element = self.class_element(has_heritage)?;
            if let ClassElement::Method(member) = &element
                && member.kind == MethodKind::Constructor
            {
                if seen_constructor {
                    return Err(ParseError {
                        message: "a class may only have one constructor".to_owned(),
                        span: member.span,
                    });
                }
                seen_constructor = true;
            }
            elements.push(element);
        }
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightBrace)?;
        Ok(ClassBody {
            heritage,
            elements,
            span: Span::new(start, end),
        })
    }

    fn class_element(&mut self, has_heritage: bool) -> Result<ClassElement, ParseError> {
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
            // No parameter list follows: this is a field, not a method. A real
            // `get`/`set` accessor prefix requires a method body, so a field
            // here would be a malformed accessor.
            if accessor_kind.is_some() {
                return Err(ParseError {
                    message: "expected `(` after accessor name".to_owned(),
                    span: Span::new(member_start, self.previous_end()),
                });
            }
            return self.class_field(is_static, key, key_text.as_deref(), member_start);
        }

        let params = self.function_parameters()?;
        reject_duplicate_method_parameters(&params)?;
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;

        let is_constructor = !is_static
            && accessor_kind.is_none()
            && matches!(key_text.as_deref(), Some("constructor"));

        // Every class member body may use `super.x`; only a derived-class
        // constructor body may use `super(...)`. Methods reset whatever
        // surrounding context existed (e.g. a class nested in a method).
        let previous_method = self.in_method;
        let previous_derived = self.in_derived_constructor;
        self.in_method = true;
        self.in_derived_constructor = is_constructor && has_heritage;
        let body = self.block_body();
        self.in_method = previous_method;
        self.in_derived_constructor = previous_derived;
        let body = body?;
        self.reject_invalid_function_parameters(&params, &body, body_start)?;
        let end = self.previous_end();

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
        Ok(ClassElement::Method(ClassMember {
            kind,
            key,
            is_static,
            value,
            span: Span::new(member_start, end),
        }))
    }

    /// Parses a public class field after its key has been consumed:
    /// `= AssignmentExpression`, then ASI (a `;`, a `}`, EOF, or a preceding
    /// line terminator terminates the field).
    fn class_field(
        &mut self,
        is_static: bool,
        key: ClassMemberKey,
        key_text: Option<&str>,
        member_start: usize,
    ) -> Result<ClassElement, ParseError> {
        let key_end = self.previous_end();
        self.validate_field_restrictions(is_static, key_text, member_start, key_end)?;

        let initializer = if self.match_kind(&TokenKind::Equal) {
            // Field initializers may use `super.x` but not `arguments`; they
            // form their own implicit method-like scope.
            let previous_method = self.in_method;
            let previous_field_initializer = self.in_field_initializer;
            self.in_method = true;
            self.in_field_initializer = true;
            let expr = self.assignment();
            self.in_method = previous_method;
            self.in_field_initializer = previous_field_initializer;
            Some(expr?)
        } else {
            None
        };

        let end = self.previous_end();
        self.consume_field_terminator(end)?;
        Ok(ClassElement::Field(ClassField {
            key,
            initializer,
            is_static,
            span: Span::new(member_start, end),
        }))
    }

    /// Enforces the ASI rule for a field: the next token must be `;`, `}`,
    /// EOF, or separated from the field by a line terminator. A `;` is
    /// consumed; the others stay for the surrounding loop.
    fn consume_field_terminator(&mut self, field_end: usize) -> Result<(), ParseError> {
        if self.match_kind(&TokenKind::Semicolon) {
            return Ok(());
        }
        if self.at(&TokenKind::RightBrace) || self.at(&TokenKind::Eof) {
            return Ok(());
        }
        let next = self
            .peek()
            .expect("parser should always have eof token")
            .clone();
        if self.has_line_terminator_between(field_end, next.span.start) {
            return Ok(());
        }
        Err(ParseError {
            message: "expected `;` or newline after class field".to_owned(),
            span: next.span,
        })
    }

    /// Reports whether the source between two byte offsets contains a line
    /// terminator, used for class-field ASI.
    fn has_line_terminator_between(&self, start: usize, end: usize) -> bool {
        self.source
            .get(start..end)
            .is_some_and(|slice| slice.chars().any(is_line_terminator))
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
            // A getter/setter named `constructor` is a syntax error; a
            // static member named `constructor` is allowed.
            Some("constructor")
                if !is_static && matches!(kind, MethodKind::Getter | MethodKind::Setter) =>
            {
                return Err(ParseError {
                    message: "class constructor may not be an accessor".to_owned(),
                    span,
                });
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

    fn validate_field_restrictions(
        &self,
        is_static: bool,
        key_text: Option<&str>,
        start: usize,
        end: usize,
    ) -> Result<(), ParseError> {
        let span = Span::new(start, end);
        match key_text {
            // An instance field named `constructor` and a static field named
            // `prototype` are both syntax errors; a static `constructor` field
            // is likewise forbidden.
            Some("constructor") => {
                return Err(ParseError {
                    message: "class fields may not be named `constructor`".to_owned(),
                    span,
                });
            }
            Some("prototype") if is_static => {
                return Err(ParseError {
                    message: "static class fields may not be named `prototype`".to_owned(),
                    span,
                });
            }
            _ => {}
        }
        Ok(())
    }
}

fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
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
