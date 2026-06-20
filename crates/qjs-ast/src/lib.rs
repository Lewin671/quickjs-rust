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
    AssignmentTargetProperty, AssignmentTargetPropertyKey, BinaryOp, CallArgument, Expr,
    FunctionParams, Literal, MemberProperty, ObjectProperty, ObjectPropertyKey, ObjectPropertyKind,
    UnaryOp, UpdateOp,
};
pub use span::Span;
pub use statement::{
    BindingElement, BindingPattern, CatchClause, DEFAULT_EXPORT_BINDING, DefaultExport, ExportDecl,
    ExportSpecifier, ForInLeft, ForInit, ImportAttributes, ImportDecl, ImportSpecifier, ModuleDecl,
    ModuleExportName, ObjectBindingProperty, ObjectBindingPropertyKey, Script, Stmt, SwitchCase,
    VarDeclarator, VarKind,
};
