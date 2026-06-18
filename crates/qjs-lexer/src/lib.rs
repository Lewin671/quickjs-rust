//! Tokenization for the Rust QuickJS rewrite.

mod error;
mod scanner;
mod token;

pub use error::LexError;
pub use token::{TemplateSegment, Token, TokenKind};

use scanner::Lexer;

/// Lexer options for source-text contexts that differ from ordinary script
/// parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LexOptions {
    /// Whether a leading `#!` is treated as a hashbang comment.
    pub hashbang: bool,
}

impl Default for LexOptions {
    fn default() -> Self {
        Self { hashbang: true }
    }
}

/// Lexes JavaScript source into tokens.
///
/// # Errors
///
/// Returns a `LexError` when an unsupported character or unterminated string is
/// encountered.
pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    lex_with_options(source, LexOptions::default())
}

/// Lexes JavaScript source with explicit source-text options.
///
/// # Errors
///
/// Returns a `LexError` when an unsupported character or unterminated string is
/// encountered.
pub fn lex_with_options(source: &str, options: LexOptions) -> Result<Vec<Token>, LexError> {
    Lexer::with_options(source, options).lex()
}

#[cfg(test)]
mod tests;
