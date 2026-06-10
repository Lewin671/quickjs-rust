use crate::expression::{Expr, FunctionParams};
use crate::span::Span;

mod control;
mod declaration;
mod script;

pub use control::{CatchClause, ForInLeft, ForInit, SwitchCase};
pub use declaration::{
    BindingElement, BindingPattern, ObjectBindingProperty, VarDeclarator, VarKind,
};
pub use script::Script;

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
    /// A for-of statement.
    ForOf {
        /// Loop binding or assignment target.
        left: ForInLeft,
        /// Iterable expression.
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
        /// Formal parameters.
        params: FunctionParams,
        /// Function body statements.
        body: Vec<Stmt>,
        /// Source span.
        span: Span,
    },
    /// A labeled statement.
    Labelled {
        /// Label name.
        label: String,
        /// Labeled body statement.
        body: Box<Stmt>,
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
        /// Optional label target.
        label: Option<String>,
        /// Source span.
        span: Span,
    },
    /// A continue statement.
    Continue {
        /// Optional label target.
        label: Option<String>,
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
