use crate::expression::Expr;
use crate::span::Span;

/// A class body shared by class declarations and class expressions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassBody {
    /// Class members in source order.
    pub members: Vec<ClassMember>,
    /// Source span covering the `{ ... }` block.
    pub span: Span,
}

/// A single member of a class body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassMember {
    /// Member kind. Extensible for later slices (static, accessors, fields).
    pub kind: MethodKind,
    /// Member key.
    pub key: ClassMemberKey,
    /// The method function expression. Always an `Expr::Function`.
    pub value: Expr,
    /// Source span covering the whole member.
    pub span: Span,
}

/// The key naming a class member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassMemberKey {
    /// A literal identifier or string-style key, for example `foo`.
    Literal(String),
}

/// The kind of a class member.
///
/// Only `Constructor` and `Method` are produced in S1. Later slices extend
/// this with static methods, getters, setters, and fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodKind {
    /// The class constructor.
    Constructor,
    /// A prototype method.
    Method,
}
