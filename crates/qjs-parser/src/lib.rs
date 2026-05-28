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
    Parser::new(tokens).parse_script()
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

#[cfg(test)]
mod tests;
