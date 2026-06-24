use crate::expression::{Expr, MemberProperty};
use crate::span::Span;

/// An assignment target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssignmentTarget {
    /// Identifier assignment.
    Identifier {
        name: String,
        span: Span,
        /// Whether the identifier was enclosed in parentheses as the
        /// assignment target. Parenthesized identifiers are assignable, but are
        /// not IdentifierRef for NamedEvaluation.
        parenthesized: bool,
    },
    /// Member assignment.
    Member {
        /// Object expression.
        object: Box<Expr>,
        /// Property expression or name.
        property: MemberProperty,
        /// Source span.
        span: Span,
    },
    /// A function-call result as an assignment/update target (AnnexB sloppy-mode
    /// web compatibility, e.g. `f() = x`, `f()++`). The call is evaluated and a
    /// runtime ReferenceError is then thrown; strict mode rejects it at parse
    /// time.
    CallExpression {
        /// The call expression to evaluate before throwing.
        call: Box<Expr>,
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
    /// Property key.
    pub key: AssignmentTargetPropertyKey,
    /// Nested assignment target.
    pub target: AssignmentTarget,
    /// Optional default initializer.
    pub default: Option<Expr>,
    /// Source span.
    pub span: Span,
}

/// An object assignment pattern property key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssignmentTargetPropertyKey {
    /// A literal property key.
    Literal(String),
    /// A computed property key expression.
    Computed(Expr),
}

impl AssignmentTargetPropertyKey {
    /// Returns the literal key name, if this key is not computed.
    #[must_use]
    pub fn as_literal(&self) -> Option<&str> {
        match self {
            Self::Literal(key) => Some(key),
            Self::Computed(_) => None,
        }
    }
}

impl AssignmentTarget {
    /// Returns the source span for this assignment target.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Identifier { span, .. }
            | Self::Member { span, .. }
            | Self::CallExpression { span, .. }
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
