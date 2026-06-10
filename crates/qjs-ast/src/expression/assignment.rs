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
    /// An array destructuring assignment pattern.
    ArrayPattern {
        /// Elements in source order. `None` represents an elision.
        elements: Vec<Option<AssignmentTargetElement>>,
        /// Optional trailing rest target.
        rest: Option<Box<AssignmentTarget>>,
        /// Source span.
        span: Span,
    },
    /// An object destructuring assignment pattern.
    ObjectPattern {
        /// Properties in source order.
        properties: Vec<AssignmentTargetProperty>,
        /// Optional trailing rest target.
        rest: Option<Box<AssignmentTarget>>,
        /// Source span.
        span: Span,
    },
}

/// An array assignment pattern element with an optional default initializer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignmentTargetElement {
    /// Nested assignment target.
    pub target: AssignmentTarget,
    /// Optional default initializer.
    pub default: Option<Expr>,
    /// Source span.
    pub span: Span,
}

/// An object assignment pattern property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignmentTargetProperty {
    /// Literal property key.
    pub key: String,
    /// Nested assignment target.
    pub target: AssignmentTarget,
    /// Optional default initializer.
    pub default: Option<Expr>,
    /// Source span.
    pub span: Span,
}

impl AssignmentTarget {
    /// Returns the source span for this assignment target.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Identifier { span, .. }
            | Self::Member { span, .. }
            | Self::ArrayPattern { span, .. }
            | Self::ObjectPattern { span, .. } => *span,
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
