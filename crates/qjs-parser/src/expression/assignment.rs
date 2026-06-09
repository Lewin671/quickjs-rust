use qjs_ast::{AssignmentOp, Expr, FunctionParams, Span, Stmt};
use qjs_lexer::TokenKind;

use crate::helpers::{assignment_target, body_has_strict_directive};
use crate::{ParseError, Parser};

impl Parser {
    pub(crate) fn assignment(&mut self) -> Result<Expr, ParseError> {
        if let Some(arrow) = self.arrow_function()? {
            return Ok(arrow);
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
        let body_start = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .start;
        let body = if self.at(&TokenKind::LeftBrace) {
            self.block_body()?
        } else {
            let expr = self.assignment()?;
            let span = expr.span();
            vec![Stmt::Return {
                argument: Some(expr),
                span,
            }]
        };
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
        Ok(Some(Expr::Function {
            name: None,
            params,
            body,
            constructable: false,
            lexical_this: true,
            lexical_arguments: true,
            span: Span::new(start, end),
        }))
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
