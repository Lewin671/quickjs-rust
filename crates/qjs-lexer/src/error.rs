use qjs_ast::Span;

/// A lexer error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexError {
    /// Human-readable message.
    pub message: String,
    /// Source span.
    pub span: Span,
}
