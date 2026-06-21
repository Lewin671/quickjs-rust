//! Dynamic `import(...)` call and `import.meta` meta-property parsing.
//!
//! `import` is a contextual keyword (lexed as an identifier). When followed by
//! `(` it is a dynamic import call (`ImportCall`); when followed by `.` it is a
//! meta-property, of which only `import.meta` is valid. Both forms are parsed
//! under either goal symbol — the runtime, not the parser, rejects `import.meta`
//! in a script. The `import`/`export` *declaration* forms are handled separately
//! under the Module goal (see `statement::module`).

use qjs_ast::{Expr, Span};
use qjs_lexer::TokenKind;

use crate::{Goal, ParseError, Parser};

impl Parser {
    /// Reports whether the cursor is at an `import(...)` call or `import.meta`
    /// meta-property expression. Only a following `(` or `.` selects these
    /// expression forms rather than a plain `import` identifier reference.
    pub(crate) fn at_import_expression(&self) -> bool {
        let Some(token) = self.peek() else {
            return false;
        };
        if token.had_escape
            || !matches!(&token.kind, TokenKind::Identifier(name) if name == "import")
        {
            return false;
        }
        matches!(
            self.peek_nth(1).map(|token| &token.kind),
            Some(TokenKind::LeftParen | TokenKind::Dot)
        )
    }

    /// Parses `import(AssignmentExpression ,opt)`, `import(AssignmentExpression,
    /// AssignmentExpression ,opt)`, or `import.meta`. The `import` keyword is the
    /// current token. Spread arguments, an empty argument list, and a third
    /// argument are early SyntaxErrors (ImportCall must not be extended).
    pub(crate) fn import_expression(&mut self) -> Result<Expr, ParseError> {
        let import_token = self.advance();
        let start = import_token.span.start;
        if self.match_kind(&TokenKind::Dot) {
            let property = self.advance();
            if !matches!(&property.kind, TokenKind::Identifier(name) if name == "meta")
                || property.had_escape
            {
                return Err(ParseError {
                    message: "only `import.meta` is a valid `import.` meta-property".to_owned(),
                    span: property.span,
                });
            }
            // `import.meta` is only legal when the syntactic goal is Module; it
            // is an early SyntaxError in a Script, a `Function`/`AsyncFunction`/
            // generator constructor body, or any other non-module parse.
            if self.goal != Goal::Module {
                return Err(ParseError {
                    message: "`import.meta` is only valid in a module".to_owned(),
                    span: Span::new(start, property.span.end),
                });
            }
            return Ok(Expr::ImportMeta {
                span: Span::new(start, property.span.end),
            });
        }
        self.expect(&TokenKind::LeftParen)?;
        if self.at(&TokenKind::RightParen) {
            return Err(ParseError {
                message: "import() requires a specifier argument".to_owned(),
                span: import_token.span,
            });
        }
        if self.at(&TokenKind::DotDotDot) {
            return Err(ParseError {
                message: "import() does not accept a spread argument".to_owned(),
                span: import_token.span,
            });
        }
        let specifier = self.assignment_allow_in()?;
        let mut options = None;
        if self.match_kind(&TokenKind::Comma) && !self.at(&TokenKind::RightParen) {
            if self.at(&TokenKind::DotDotDot) {
                return Err(ParseError {
                    message: "import() does not accept a spread argument".to_owned(),
                    span: import_token.span,
                });
            }
            options = Some(Box::new(self.assignment_allow_in()?));
            // A trailing comma after the options argument is allowed; a third
            // argument is not (ImportCall must not be extended).
            if self.match_kind(&TokenKind::Comma) && !self.at(&TokenKind::RightParen) {
                return Err(ParseError {
                    message: "import() accepts at most two arguments".to_owned(),
                    span: import_token.span,
                });
            }
        }
        let end = self
            .peek()
            .expect("parser should always have eof token")
            .span
            .end;
        self.expect(&TokenKind::RightParen)?;
        Ok(Expr::ImportCall {
            specifier: Box::new(specifier),
            options,
            span: Span::new(start, end),
        })
    }
}
