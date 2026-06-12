//! Shared AST and source span types.

mod class;
mod expression;
mod span;
mod statement;

pub use class::{
    ClassBody, ClassElement, ClassField, ClassMember, ClassMemberKey, MethodKind, StaticBlock,
};
pub use expression::{
    ArrayElement, AssignmentOp, AssignmentTarget, AssignmentTargetElement,
    AssignmentTargetProperty, BinaryOp, CallArgument, Expr, FunctionParams, Literal,
    MemberProperty, ObjectProperty, ObjectPropertyKey, ObjectPropertyKind, UnaryOp, UpdateOp,
};
pub use span::Span;
pub use statement::{
    BindingElement, BindingPattern, CatchClause, DefaultExport, ExportDecl, ExportSpecifier,
    ForInLeft, ForInit, ImportDecl, ImportSpecifier, ModuleDecl, ModuleExportName,
    ObjectBindingProperty, Script, Stmt, SwitchCase, VarDeclarator, VarKind,
};
