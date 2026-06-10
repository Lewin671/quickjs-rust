use crate::expression::Expr;

/// Member access property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemberProperty {
    /// Dot property name, as in `object.name`.
    Named(String),
    /// Computed property expression, as in `object[index]`.
    Computed(Box<Expr>),
    /// Private member access, as in `object.#name`. Holds the private name
    /// without the leading `#`.
    Private(String),
}
