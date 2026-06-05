//! Shared AST and source span types.

mod expression;
mod span;
mod statement;

pub use expression::{
    ArrayAssignmentElement, AssignmentOp, AssignmentTarget, BinaryOp, Expr, Literal,
    MemberProperty, ObjectAssignmentProperty, ObjectProperty, ObjectPropertyKey,
    ObjectPropertyKind, UnaryOp, UpdateOp,
};
pub use span::Span;
pub use statement::{
    CatchClause, ClassMethod, ForInLeft, ForInit, Script, Stmt, SwitchCase, VarDeclarator, VarKind,
};
