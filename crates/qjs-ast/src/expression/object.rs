use crate::expression::Expr;
use crate::span::Span;

/// Object literal property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectProperty {
    /// Property key syntax.
    pub key: ObjectPropertyKey,
    /// Property value expression.
    pub value: Expr,
    /// Source span.
    pub span: Span,
}

/// Object literal property key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectPropertyKey {
    /// Literal property name after syntactic normalization.
    Literal(String),
    /// Computed property name expression, as in `{ [expr]: value }`.
    Computed(Expr),
}
