use crate::expression::{AssignmentTarget, Expr};
use crate::span::Span;
use crate::statement::{Stmt, VarDeclarator, VarKind};

/// A switch case or default clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SwitchCase {
    /// Optional case test; `None` represents `default`.
    pub test: Option<Expr>,
    /// Clause statements.
    pub consequent: Vec<Stmt>,
    /// Source span.
    pub span: Span,
}

/// A catch clause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatchClause {
    /// Optional catch binding.
    pub param: Option<String>,
    /// Catch block statements.
    pub body: Vec<Stmt>,
    /// Source span.
    pub span: Span,
}

/// A class method declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassMethod {
    /// Method name.
    pub name: String,
    /// Parameter names.
    pub params: Vec<String>,
    /// Function body statements.
    pub body: Vec<Stmt>,
    /// Whether the method is static.
    pub is_static: bool,
    /// Source span.
    pub span: Span,
}

/// A for statement initializer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForInit {
    /// Variable declaration initializer.
    VarDecl {
        /// Declaration kind.
        kind: VarKind,
        /// Variable declarators.
        declarations: Vec<VarDeclarator>,
        /// Source span.
        span: Span,
    },
    /// Expression initializer.
    Expr(Expr),
}

/// A for-in loop head.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForInLeft {
    /// Variable declaration loop binding.
    VarDecl {
        /// Declaration kind.
        kind: VarKind,
        /// Binding name.
        name: String,
        /// Source span.
        span: Span,
    },
    /// Variable declaration loop binding pattern.
    Binding {
        /// Declaration kind.
        kind: VarKind,
        /// Binding target.
        target: AssignmentTarget,
        /// Source span.
        span: Span,
    },
    /// Assignment target loop binding.
    Target(AssignmentTarget),
}

impl ForInLeft {
    /// Returns the source span for this for-in loop head.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::VarDecl { span, .. } | Self::Binding { span, .. } => *span,
            Self::Target(target) => target.span(),
        }
    }
}
