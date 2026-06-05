use crate::expression::{Expr, MemberProperty, ObjectPropertyKey};
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
    /// Object destructuring assignment.
    Object {
        /// Assignment properties.
        properties: Vec<ObjectAssignmentProperty>,
        /// Source span.
        span: Span,
    },
    /// Array destructuring assignment.
    Array {
        /// Assignment elements, with `None` for elisions.
        elements: Vec<Option<ArrayAssignmentElement>>,
        /// Source span.
        span: Span,
    },
}

impl AssignmentTarget {
    /// Returns the source span for this assignment target.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Identifier { span, .. }
            | Self::Member { span, .. }
            | Self::Object { span, .. }
            | Self::Array { span, .. } => *span,
        }
    }
}

/// Array destructuring assignment element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrayAssignmentElement {
    /// Assignment target receiving the iterated value.
    pub target: AssignmentTarget,
    /// Optional default initializer used for `undefined`.
    pub default: Option<Expr>,
    /// Source span.
    pub span: Span,
}

/// Object destructuring assignment property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectAssignmentProperty {
    /// Property key read from the source value.
    pub key: ObjectPropertyKey,
    /// Assignment target receiving the property value.
    pub target: AssignmentTarget,
    /// Source span.
    pub span: Span,
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
