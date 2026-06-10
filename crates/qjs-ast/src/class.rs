use crate::expression::Expr;
use crate::span::Span;

/// A class body shared by class declarations and class expressions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassBody {
    /// Optional heritage expression from an `extends` clause. This is a
    /// `LeftHandSideExpression` evaluated to the parent constructor (or `null`).
    pub heritage: Option<Box<Expr>>,
    /// Class elements (methods and fields) in source order.
    pub elements: Vec<ClassElement>,
    /// Source span covering the `{ ... }` block.
    pub span: Span,
}

/// A single element of a class body: a method/accessor/constructor or a field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassElement {
    /// A method, accessor, or the constructor.
    Method(ClassMember),
    /// A public instance or static field.
    Field(ClassField),
}

/// A method, accessor, or constructor member of a class body.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassMember {
    /// Member kind.
    pub kind: MethodKind,
    /// Member key.
    pub key: ClassMemberKey,
    /// Whether the member is declared `static`.
    pub is_static: bool,
    /// The method function expression. Always an `Expr::Function`.
    pub value: Expr,
    /// Source span covering the whole member.
    pub span: Span,
}

/// A public class field declaration (`x;`, `x = expr;`, `static x = expr;`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassField {
    /// Field key.
    pub key: ClassMemberKey,
    /// Optional initializer expression (an `AssignmentExpression`). When absent
    /// the field initializes to `undefined`.
    pub initializer: Option<Expr>,
    /// Whether the field is declared `static`.
    pub is_static: bool,
    /// Source span covering the whole field.
    pub span: Span,
}

/// The key naming a class member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassMemberKey {
    /// A literal identifier or string-style key, for example `foo`.
    Literal(String),
    /// A computed key expression, as in `[expr]() {}`.
    Computed(Expr),
}

/// The kind of a class method member.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MethodKind {
    /// The class constructor.
    Constructor,
    /// A prototype or static method.
    Method,
    /// A getter accessor.
    Getter,
    /// A setter accessor.
    Setter,
}
