use crate::span::Span;

/// A literal expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Literal {
    /// Numeric literal text.
    Number { raw: String, span: Span },
    /// String literal contents after quote removal.
    String { value: String, span: Span },
    /// Boolean literal.
    Boolean { value: bool, span: Span },
    /// Null literal.
    Null { span: Span },
}

impl Literal {
    /// Returns the source span for this literal.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Number { span, .. }
            | Self::String { span, .. }
            | Self::Boolean { span, .. }
            | Self::Null { span } => *span,
        }
    }
}
