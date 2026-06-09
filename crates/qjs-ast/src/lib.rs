//! Shared AST and source span types.

mod expression;
mod span;
mod statement;

pub use expression::{
    ArrayElement, AssignmentOp, AssignmentTarget, BinaryOp, CallArgument, Expr, FunctionParams,
    Literal, MemberProperty, ObjectProperty, ObjectPropertyKey, ObjectPropertyKind, UnaryOp,
    UpdateOp,
};
pub use span::Span;
pub use statement::{
    CatchClause, CatchParam, ForInLeft, ForInit, Script, Stmt, SwitchCase, VarDeclarator, VarKind,
};
