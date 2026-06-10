//! Parser for a small JavaScript subset.

mod cursor;
mod expression;
mod helpers;
mod statement;

use qjs_ast::{Script, Span};
use qjs_lexer::{Token, lex};

/// A parse error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    /// Human-readable message.
    pub message: String,
    /// Source span.
    pub span: Span,
}

/// Parses source text into a script AST.
///
/// # Errors
///
/// Returns a structured error for lexing or parsing failures.
pub fn parse_script(source: &str) -> Result<Script, ParseError> {
    let tokens = lex(source).map_err(|error| ParseError {
        message: error.message,
        span: error.span,
    })?;
    Parser::new(tokens, source.to_owned()).parse_script()
}

struct Parser {
    source: String,
    tokens: Vec<Token>,
    cursor: usize,
    strict: bool,
    allow_in: bool,
    /// Whether `super.prop`/`super[expr]` member access is currently allowed,
    /// i.e. the parser is inside a method or accessor body (or an arrow nested
    /// in one).
    in_method: bool,
    /// Whether `super(...)` calls are currently allowed, i.e. the parser is
    /// inside a derived-class constructor body (or an arrow nested in one).
    in_derived_constructor: bool,
    /// Whether the parser is inside a class field initializer expression, where
    /// `arguments` is a syntax error.
    in_field_initializer: bool,
}

#[cfg(test)]
mod tests;
