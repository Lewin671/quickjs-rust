use crate::expression::Expr;
use crate::span::Span;

/// Object literal property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectProperty {
    /// Property key syntax.
    pub key: ObjectPropertyKey,
    /// Property definition kind.
    pub kind: ObjectPropertyKind,
    /// Whether this is the `{ __proto__: expr }` prototype special form
    /// (Annex B.3.1): a colon data property whose literal key is `__proto__`.
    /// Shorthand, computed, and method/accessor `__proto__` forms set this to
    /// `false` and stay ordinary properties.
    pub is_proto_setter: bool,
    /// Property value expression.
    pub value: Expr,
    /// Source span.
    pub span: Span,
}

/// Object literal property definition kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectPropertyKind {
    /// Data property or shorthand/method value.
    Data,
    /// Getter accessor.
    Getter,
    /// Setter accessor.
    Setter,
    /// Object spread property, as in `{ ...source }`.
    Spread,
}

/// Object literal property key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectPropertyKey {
    /// Literal property name after syntactic normalization.
    Literal(String),
    /// Computed property name expression, as in `{ [expr]: value }`.
    Computed(Expr),
}
