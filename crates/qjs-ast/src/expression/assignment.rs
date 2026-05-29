use crate::expression::{Expr, MemberProperty};
use crate::span::Span;

/// An assignment target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssignmentTarget {
    /// Identifier assignment.
    Identifier { name: String, span: Span },
    /// Member assignment.
    Member {
        /// Object expression.
        object: Box<Expr>,
        /// Property expression or name.
        property: MemberProperty,
        /// Source span.
        span: Span,
    },
}

impl AssignmentTarget {
    /// Returns the source span for this assignment target.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Identifier { span, .. } | Self::Member { span, .. } => *span,
        }
    }
}

/// Assignment operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignmentOp {
    /// `=`.
    Assign,
    /// `+=`.
    AddAssign,
    /// `-=`.
    SubAssign,
    /// `*=`.
    MulAssign,
    /// `**=`.
    PowAssign,
    /// `/=`.
    DivAssign,
    /// `%=`.
    RemAssign,
    /// `<<=`.
    ShlAssign,
    /// `>>=`.
    ShrAssign,
    /// `>>>=`.
    UShrAssign,
    /// `&=`.
    BitwiseAndAssign,
    /// `^=`.
    BitwiseXorAssign,
    /// `|=`.
    BitwiseOrAssign,
    /// `&&=`.
    LogicalAndAssign,
    /// `||=`.
    LogicalOrAssign,
    /// `??=`.
    NullishAssign,
}
