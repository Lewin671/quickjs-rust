use crate::expression::Expr;
use crate::span::Span;

/// A variable declarator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VarDeclarator {
    /// Binding name.
    pub name: String,
    /// Optional initializer.
    pub init: Option<Expr>,
    /// Source span.
    pub span: Span,
}

/// Variable declaration kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VarKind {
    /// `var`.
    Var,
    /// `let`.
    Let,
    /// `const`.
    Const,
}
