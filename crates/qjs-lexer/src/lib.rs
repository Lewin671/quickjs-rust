//! Tokenization for the Rust QuickJS rewrite.

mod error;
mod scanner;
mod token;

pub use error::LexError;
pub use token::{TemplateSegment, Token, TokenKind};

use scanner::Lexer;

/// Lexes JavaScript source into tokens.
///
/// # Errors
///
/// Returns a `LexError` when an unsupported character or unterminated string is
/// encountered.
pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).lex()
}

#[cfg(test)]
mod tests;
