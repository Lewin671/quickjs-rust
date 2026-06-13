use qjs_ast::{AssignmentOp, Expr, FunctionParams, Span, Stmt};
use qjs_lexer::TokenKind;

use crate::helpers::{assignment_target, body_has_strict_directive};
use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn assignment(&mut self) -> Result<Expr, ParseError> {
        if self.in_generator && self.at_yield_keyword() {
            return self.yield_expression();
        }
        if let Some(arrow) = self.async_arrow_function()? {
            return Ok(arrow);
        }
        if let Some(arrow) = self.arrow_function()? {
            return Ok(arrow);
        }
        if let Some(destructuring) = self.try_destructuring_assignment()? {
            return Ok(destructuring);
        }

        let expr = self.conditional()?;
        let op = if self.match_kind(&TokenKind::Equal) {
            AssignmentOp::Assign
        } else if self.match_kind(&TokenKind::PlusEqual) {
            AssignmentOp::AddAssign
        } else if self.match_kind(&TokenKind::MinusEqual) {
            AssignmentOp::SubAssign
        } else if self.match_kind(&TokenKind::StarEqual) {
            AssignmentOp::MulAssign
        } else if self.match_kind(&TokenKind::StarStarEqual) {
            AssignmentOp::PowAssign
        } else if self.match_kind(&TokenKind::SlashEqual) {
            AssignmentOp::DivAssign
        } else if self.match_kind(&TokenKind::PercentEqual) {
            AssignmentOp::RemAssign
        } else if self.match_kind(&TokenKind::LessLessEqual) {
            AssignmentOp::ShlAssign
        } else if self.match_kind(&TokenKind::GreaterGreaterEqual) {
            AssignmentOp::ShrAssign
        } else if self.match_kind(&TokenKind::GreaterGreaterGreaterEqual) {
            AssignmentOp::UShrAssign
        } else if self.match_kind(&TokenKind::AmpersandEqual) {
            AssignmentOp::BitwiseAndAssign
        } else if self.match_kind(&TokenKind::CaretEqual) {
            AssignmentOp::BitwiseXorAssign
        } else if self.match_kind(&TokenKind::PipeEqual) {
            AssignmentOp::BitwiseOrAssign
        } else if self.match_kind(&TokenKind::AmpersandAmpersandEqual) {
            AssignmentOp::LogicalAndAssign
        } else if self.match_kind(&TokenKind::PipePipeEqual) {
            AssignmentOp::LogicalOrAssign
        } else if self.match_kind(&TokenKind::QuestionQuestionEqual) {
            AssignmentOp::NullishAssign
        } else {
            return Ok(expr);
        };

        let target = assignment_target(expr)?;

        // Strict mode: assigning to the identifiers `eval` or `arguments`
        // (simple or compound) is an early SyntaxError.
        if self.strict
            && let qjs_ast::AssignmentTarget::Identifier { name, span } = &target
            && matches!(name.as_str(), "eval" | "arguments")
        {
            return Err(ParseError {
                message: format!("`{name}` may not be assigned in strict mode"),
                span: *span,
            });
        }

        let value = self.assignment()?;
        let assignment_span = Span::new(target.span().start, value.span().end);
        Ok(Expr::Assignment {
            target,
            op,
            value: Box::new(value),
            span: assignment_span,
        })
    }

    fn arrow_function(&mut self) -> Result<Option<Expr>, ParseError> {
        let start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let Some(params) = self.arrow_parameters()? else {
            return Ok(None);
        };
        self.expect(&TokenKind::Arrow)?;
        self.finish_arrow_function(start, params, false).map(Some)
    }

    /// Parses an async arrow function: `async x => body` or
    /// `async (params) => body`. `async` is contextual, so this uses
    /// cover-grammar lookahead — when the tokens after `async` do not form an
    /// arrow head, the cursor is rewound and `Ok(None)` lets the caller parse
    /// `async` as a plain identifier (e.g. the `async(x)` call expression).
    /// There must be no line terminator between `async` and the parameters.
    fn async_arrow_function(&mut self) -> Result<Option<Expr>, ParseError> {
        let Some(async_token) = self.peek() else {
            return Ok(None);
        };
        if !matches!(&async_token.kind, TokenKind::Identifier(name) if name == "async") {
            return Ok(None);
        }
        // A following line terminator disqualifies an async arrow head.
        let Some(next) = self.peek_nth(1) else {
            return Ok(None);
        };
        let async_end = async_token.span.end;
        let next_start = next.span.start;
        let next_is_ident = matches!(&next.kind, TokenKind::Identifier(_));
        let next_is_paren = next.kind == TokenKind::LeftParen;
        if (!next_is_ident && !next_is_paren)
            || self.has_line_terminator_between(async_end, next_start)
        {
            return Ok(None);
        }

        let start_cursor = self.cursor;
        let start = async_token.span.start;
        self.advance(); // consume `async`

        // Inside the parameter list and body, `await` is the keyword form.
        let previous_async = self.in_async;
        self.in_async = true;
        let parsed = self.async_arrow_head_and_body(start, start_cursor);
        self.in_async = previous_async;
        parsed
    }

    fn async_arrow_head_and_body(
        &mut self,
        start: usize,
        start_cursor: usize,
    ) -> Result<Option<Expr>, ParseError> {
        let params = if self.at(&TokenKind::LeftParen) {
            // `async (params) =>`: reuse the parenthesized cover grammar, which
            // rewinds on its own when the parentheses are not an arrow head.
            match self.parenthesized_arrow_parameters()? {
                Some(params) => params,
                None => {
                    self.cursor = start_cursor;
                    return Ok(None);
                }
            }
        } else {
            // `async ident =>`: a single identifier parameter, no line
            // terminator before the arrow.
            let token = self.advance();
            let TokenKind::Identifier(param) = token.kind else {
                unreachable!("caller checked identifier");
            };
            if !self.at(&TokenKind::Arrow) {
                self.cursor = start_cursor;
                return Ok(None);
            }
            self.reject_line_terminator_before_arrow(token.span)?;
            if param == "await" {
                return Err(ParseError {
                    message: "`await` is not allowed as an async arrow parameter".to_owned(),
                    span: token.span,
                });
            }
            if self.strict && restricted_strict_arrow_parameter(&param) {
                return Err(ParseError {
                    message: "restricted arrow parameter name in strict mode".to_owned(),
                    span: token.span,
                });
            }
            if reserved_arrow_parameter(&param, self.strict) {
                return Err(ParseError {
                    message: "reserved arrow parameter name".to_owned(),
                    span: token.span,
                });
            }
            FunctionParams::positional(vec![param])
        };
        self.expect(&TokenKind::Arrow)?;
        self.finish_arrow_function(start, params, true).map(Some)
    }

    /// Finishes an arrow function after its parameters and `=>` are consumed,
    /// parsing the body (with the async context already in effect for an async
    /// arrow) and assembling the function expression.
    fn finish_arrow_function(
        &mut self,
        start: usize,
        params: FunctionParams,
        is_async: bool,
    ) -> Result<Expr, ParseError> {
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let previous_static_block = self.in_static_block;
        self.in_static_block = false;
        let body = if self.at(&TokenKind::LeftBrace) {
            self.block_body()
        } else {
            self.assignment().map(|expr| {
                let span = expr.span();
                vec![Stmt::Return {
                    argument: Some(expr),
                    span,
                }]
            })
        };
        self.in_static_block = previous_static_block;
        let body = body?;
        if !params.is_simple() && body_has_strict_directive(&body) {
            return Err(ParseError {
                message: "strict directive not allowed with non-simple parameters".to_owned(),
                span: Span::new(body_start, body_start),
            });
        }
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .expect("parser should always have eof token")
            .span
            .end;
        Ok(Expr::Function {
            name: None,
            params,
            body,
            constructable: false,
            lexical_this: true,
            lexical_arguments: true,
            is_generator: false,
            is_async,
            span: Span::new(start, end),
        })
    }

    fn arrow_parameters(&mut self) -> Result<Option<FunctionParams>, ParseError> {
        match self.peek().map(|token| &token.kind) {
            Some(TokenKind::Identifier(_))
                if self
                    .peek_nth(1)
                    .is_some_and(|token| token.kind == TokenKind::Arrow) =>
            {
                let token = self.advance();
                self.reject_line_terminator_before_arrow(token.span)?;
                let TokenKind::Identifier(param) = token.kind else {
                    unreachable!("peek checked identifier");
                };
                if self.strict && restricted_strict_arrow_parameter(&param) {
                    return Err(ParseError {
                        message: "restricted arrow parameter name in strict mode".to_owned(),
                        span: token.span,
                    });
                }
                if reserved_arrow_parameter(&param, self.strict) {
                    return Err(ParseError {
                        message: "reserved arrow parameter name".to_owned(),
                        span: token.span,
                    });
                }
                Ok(Some(FunctionParams::positional(vec![param])))
            }
            Some(TokenKind::LeftParen) => self.parenthesized_arrow_parameters(),
            _ => Ok(None),
        }
    }

    fn parenthesized_arrow_parameters(&mut self) -> Result<Option<FunctionParams>, ParseError> {
        let start_cursor = self.cursor;
        self.expect(&TokenKind::LeftParen)?;
        let mut positional = Vec::new();
        let mut rest = None;
        if !self.at(&TokenKind::RightParen) {
            loop {
                if self.match_kind(&TokenKind::DotDotDot) {
                    let Ok(pattern) = self.binding_pattern() else {
                        self.cursor = start_cursor;
                        return Ok(None);
                    };
                    if self.at(&TokenKind::Equal) {
                        self.cursor = start_cursor;
                        return Ok(None);
                    }
                    rest = Some(pattern);
                    break;
                }
                let Ok(element) = self.binding_element() else {
                    self.cursor = start_cursor;
                    return Ok(None);
                };
                positional.push(element);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                if self.at(&TokenKind::RightParen) {
                    break;
                }
            }
        }
        if !self.match_kind(&TokenKind::RightParen) {
            self.cursor = start_cursor;
            return Ok(None);
        }
        if !self.at(&TokenKind::Arrow) {
            self.cursor = start_cursor;
            return Ok(None);
        }
        let close_paren_span = self.tokens[self.cursor - 1].span;
        self.reject_line_terminator_before_arrow(close_paren_span)?;
        let params = FunctionParams::new(positional, rest);
        let named = params.named_spans();
        if let Some(span) = duplicate_arrow_parameter_span(&named) {
            return Err(ParseError {
                message: "duplicate arrow parameter name".to_owned(),
                span,
            });
        }
        if self.strict {
            for (name, span) in &named {
                if restricted_strict_arrow_parameter(name) {
                    return Err(ParseError {
                        message: "restricted arrow parameter name in strict mode".to_owned(),
                        span: *span,
                    });
                }
            }
        }
        for (name, span) in &named {
            if reserved_arrow_parameter(name, self.strict) {
                return Err(ParseError {
                    message: "reserved arrow parameter name".to_owned(),
                    span: *span,
                });
            }
        }
        Ok(Some(params))
    }

    fn reject_line_terminator_before_arrow(&self, previous_span: Span) -> Result<(), ParseError> {
        let arrow = self.peek().expect("arrow lookahead should be present");
        if self.source[previous_span.end..arrow.span.start]
            .chars()
            .any(is_line_terminator)
        {
            return Err(ParseError {
                message: "line terminator before arrow".to_owned(),
                span: arrow.span,
            });
        }
        Ok(())
    }

    /// Reports whether the current token is the `yield` keyword. `yield` is
    /// lexed as a plain identifier, so it is only a keyword inside a generator
    /// body, which the caller checks via `in_generator`.
    fn at_yield_keyword(&self) -> bool {
        matches!(self.peek(), Some(token) if matches!(&token.kind, TokenKind::Identifier(name) if name == "yield"))
    }

    /// Parses a `yield`, `yield AssignmentExpression`, or
    /// `yield* AssignmentExpression` expression. The caller has already
    /// confirmed a generator context and a `yield` token.
    fn yield_expression(&mut self) -> Result<Expr, ParseError> {
        let yield_token = self.advance();
        let start = yield_token.span.start;

        // A yield expression may not appear in a generator's parameter list.
        if self.in_generator_params {
            return Err(ParseError {
                message: "`yield` is not allowed in generator parameters".to_owned(),
                span: yield_token.span,
            });
        }

        // `yield*` requires no line terminator between `yield` and `*`, and the
        // delegate form always has an operand.
        if self.at(&TokenKind::Star) {
            let star = self.peek().expect("star lookahead should be present");
            if self.has_line_terminator_between(yield_token.span.end, star.span.start) {
                return Err(ParseError {
                    message: "no line terminator allowed between `yield` and `*`".to_owned(),
                    span: star.span,
                });
            }
            self.advance();
            let argument = self.assignment()?;
            let span = Span::new(start, argument.span().end);
            return Ok(Expr::Yield {
                argument: Some(Box::new(argument)),
                delegate: true,
                span,
            });
        }

        // A bare `yield` has no operand when a line terminator follows or the
        // next token cannot begin an AssignmentExpression.
        let next = self.peek().expect("parser should always have eof token");
        let no_operand = self.has_line_terminator_between(yield_token.span.end, next.span.start)
            || !token_can_begin_assignment(&next.kind);
        if no_operand {
            return Ok(Expr::Yield {
                argument: None,
                delegate: false,
                span: yield_token.span,
            });
        }

        let argument = self.assignment()?;
        let span = Span::new(start, argument.span().end);
        Ok(Expr::Yield {
            argument: Some(Box::new(argument)),
            delegate: false,
            span,
        })
    }

    pub(crate) fn conditional(&mut self) -> Result<Expr, ParseError> {
        let test = self.nullish_coalescing()?;
        if !self.match_kind(&TokenKind::Question) {
            return Ok(test);
        }

        let consequent = self.assignment()?;
        self.expect(&TokenKind::Colon)?;
        let alternate = self.assignment()?;
        let span = Span::new(test.span().start, alternate.span().end);
        Ok(Expr::Conditional {
            test: Box::new(test),
            consequent: Box::new(consequent),
            alternate: Box::new(alternate),
            span,
        })
    }
}

fn duplicate_arrow_parameter_span(named: &[(String, Span)]) -> Option<Span> {
    for (index, (name, _)) in named.iter().enumerate() {
        for (candidate, span) in &named[index + 1..] {
            if candidate == name {
                return Some(*span);
            }
        }
    }
    None
}

fn restricted_strict_arrow_parameter(name: &str) -> bool {
    matches!(name, "arguments" | "eval" | "yield")
}

fn reserved_arrow_parameter(name: &str, strict: bool) -> bool {
    if name == "enum" {
        return true;
    }
    strict
        && matches!(
            name,
            "implements" | "interface" | "package" | "private" | "protected" | "public" | "static"
        )
}

fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// Reports whether a token can begin an AssignmentExpression, used to decide
/// whether a bare `yield` has an operand. The closing punctuators and
/// statement separators below cannot begin an expression.
fn token_can_begin_assignment(kind: &TokenKind) -> bool {
    !matches!(
        kind,
        TokenKind::RightParen
            | TokenKind::RightBracket
            | TokenKind::RightBrace
            | TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::Colon
            | TokenKind::Eof
    )
}
