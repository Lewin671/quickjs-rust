use crate::expression::{AssignmentTarget, Expr};
use crate::span::Span;

/// A JavaScript script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Script {
    /// Top-level statements.
    pub body: Vec<Stmt>,
}

/// A statement node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Stmt {
    /// An expression used as a statement.
    Expr(Expr),
    /// A block statement.
    Block {
        /// Statements in the block.
        body: Vec<Stmt>,
        /// Source span.
        span: Span,
    },
    /// An if statement.
    If {
        /// Condition expression.
        test: Expr,
        /// Consequent statement.
        consequent: Box<Stmt>,
        /// Optional alternate statement.
        alternate: Option<Box<Stmt>>,
        /// Source span.
        span: Span,
    },
    /// A while statement.
    While {
        /// Loop condition.
        test: Expr,
        /// Loop body.
        body: Box<Stmt>,
        /// Source span.
        span: Span,
    },
    /// A do-while statement.
    DoWhile {
        /// Loop body.
        body: Box<Stmt>,
        /// Loop condition.
        test: Expr,
        /// Source span.
        span: Span,
    },
    /// A for statement.
    For {
        /// Optional initializer.
        init: Option<ForInit>,
        /// Optional loop condition.
        test: Option<Expr>,
        /// Optional update expression.
        update: Option<Expr>,
        /// Loop body.
        body: Box<Stmt>,
        /// Source span.
        span: Span,
    },
    /// A for-in statement.
    ForIn {
        /// Loop binding or assignment target.
        left: ForInLeft,
        /// Enumerated expression.
        right: Expr,
        /// Loop body.
        body: Box<Stmt>,
        /// Source span.
        span: Span,
    },
    /// A switch statement.
    Switch {
        /// Discriminant expression.
        discriminant: Expr,
        /// Switch clauses in source order.
        cases: Vec<SwitchCase>,
        /// Source span.
        span: Span,
    },
    /// A try statement.
    Try {
        /// Protected block statements.
        block: Vec<Stmt>,
        /// Optional catch clause.
        handler: Option<CatchClause>,
        /// Optional finally block statements.
        finalizer: Option<Vec<Stmt>>,
        /// Source span.
        span: Span,
    },
    /// A function declaration.
    FunctionDecl {
        /// Function name.
        name: String,
        /// Parameter names.
        params: Vec<String>,
        /// Function body statements.
        body: Vec<Stmt>,
        /// Source span.
        span: Span,
    },
    /// A return statement.
    Return {
        /// Optional return value.
        argument: Option<Expr>,
        /// Source span.
        span: Span,
    },
    /// A throw statement.
    Throw {
        /// Optional thrown value.
        argument: Option<Expr>,
        /// Source span.
        span: Span,
    },
    /// A debugger statement.
    Debugger {
        /// Source span.
        span: Span,
    },
    /// A break statement.
    Break {
        /// Source span.
        span: Span,
    },
    /// A continue statement.
    Continue {
        /// Source span.
        span: Span,
    },
    /// A variable declaration.
    VarDecl {
        /// Declaration kind.
        kind: VarKind,
        /// Variable declarators.
        declarations: Vec<VarDeclarator>,
        /// Source span.
        span: Span,
    },
    /// An empty statement represented by `;`.
    Empty,
}

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
    /// Assignment target loop binding.
    Target(AssignmentTarget),
}

impl ForInLeft {
    /// Returns the source span for this for-in loop head.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::VarDecl { span, .. } => *span,
            Self::Target(target) => target.span(),
        }
    }
}

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
