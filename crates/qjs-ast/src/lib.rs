//! Shared AST and source span types.

/// A half-open byte range in the original source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    /// Start byte offset.
    pub start: usize,
    /// End byte offset.
    pub end: usize,
}

impl Span {
    /// Creates a new source span.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

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
    /// An empty statement represented by `;`.
    Empty,
}

/// An expression node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// A literal value.
    Literal(Literal),
    /// A binary expression.
    Binary {
        /// Left-hand expression.
        left: Box<Expr>,
        /// Binary operator.
        op: BinaryOp,
        /// Right-hand expression.
        right: Box<Expr>,
        /// Source span.
        span: Span,
    },
    /// An identifier reference.
    Identifier { name: String, span: Span },
}

impl Expr {
    /// Returns the source span for this expression.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Literal(literal) => literal.span(),
            Self::Binary { span, .. } | Self::Identifier { span, .. } => *span,
        }
    }
}

/// A literal expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Literal {
    /// Numeric literal text.
    Number { raw: String, span: Span },
    /// String literal contents after quote removal.
    String { value: String, span: Span },
    /// Boolean literal.
    Boolean { value: bool, span: Span },
    /// Null literal.
    Null { span: Span },
}

impl Literal {
    /// Returns the source span for this literal.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Number { span, .. }
            | Self::String { span, .. }
            | Self::Boolean { span, .. }
            | Self::Null { span } => *span,
        }
    }
}

/// Binary operators currently supported by the parser and runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOp {
    /// Addition.
    Add,
    /// Subtraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
    /// Remainder.
    Rem,
    /// Loose equality.
    Eq,
    /// Strict equality.
    StrictEq,
    /// Loose inequality.
    Ne,
    /// Strict inequality.
    StrictNe,
    /// Less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Ge,
    /// Logical and.
    LogicalAnd,
    /// Logical or.
    LogicalOr,
}
