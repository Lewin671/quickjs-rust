//! Shared AST and source span types.

mod expression;
mod span;
mod statement;

pub use expression::{
    AssignmentOp, AssignmentTarget, BinaryOp, Expr, Literal, MemberProperty, ObjectProperty,
    ObjectPropertyKey, UnaryOp, UpdateOp,
};
pub use span::Span;
pub use statement::{
    CatchClause, ForInLeft, ForInit, Script, Stmt, SwitchCase, VarDeclarator, VarKind,
};
