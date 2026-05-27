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
        /// Source span.
        span: Span,
    },
    /// A variable declaration.
    VarDecl {
        /// Declaration kind.
        kind: VarKind,
        /// Binding name.
        name: String,
        /// Optional initializer.
        init: Option<Expr>,
        /// Source span.
        span: Span,
    },
    /// An empty statement represented by `;`.
    Empty,
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

/// An expression node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// A literal value.
    Literal(Literal),
    /// An array literal.
    Array {
        /// Array elements.
        elements: Vec<Expr>,
        /// Source span.
        span: Span,
    },
    /// An object literal.
    Object {
        /// Object properties.
        properties: Vec<ObjectProperty>,
        /// Source span.
        span: Span,
    },
    /// A unary expression.
    Unary {
        /// Unary operator.
        op: UnaryOp,
        /// Operand expression.
        argument: Box<Expr>,
        /// Source span.
        span: Span,
    },
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
    /// An assignment expression.
    Assignment {
        /// Assigned target.
        target: AssignmentTarget,
        /// Assigned value.
        value: Box<Expr>,
        /// Source span.
        span: Span,
    },
    /// A call expression.
    Call {
        /// Callee expression.
        callee: Box<Expr>,
        /// Argument expressions.
        arguments: Vec<Expr>,
        /// Source span.
        span: Span,
    },
    /// A member access expression.
    Member {
        /// Object expression.
        object: Box<Expr>,
        /// Property expression or name.
        property: MemberProperty,
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
            Self::Array { span, .. }
            | Self::Object { span, .. }
            | Self::Unary { span, .. }
            | Self::Binary { span, .. }
            | Self::Assignment { span, .. }
            | Self::Call { span, .. }
            | Self::Member { span, .. }
            | Self::Identifier { span, .. } => *span,
        }
    }
}

/// Object literal property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectProperty {
    /// Property key after syntactic normalization.
    pub key: String,
    /// Property value expression.
    pub value: Expr,
    /// Source span.
    pub span: Span,
}

/// An assignment target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssignmentTarget {
    /// Identifier assignment.
    Identifier { name: String, span: Span },
    /// Member assignment.
    Member {
        /// Object expression.
        object: Box<Expr>,
        /// Property expression or name.
        property: MemberProperty,
        /// Source span.
        span: Span,
    },
}

impl AssignmentTarget {
    /// Returns the source span for this assignment target.
    #[must_use]
    pub const fn span(&self) -> Span {
        match self {
            Self::Identifier { span, .. } | Self::Member { span, .. } => *span,
        }
    }
}

/// Member access property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemberProperty {
    /// Dot property name, as in `object.name`.
    Named(String),
    /// Computed property expression, as in `object[index]`.
    Computed(Box<Expr>),
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

/// Unary operators currently supported by the parser and runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOp {
    /// Numeric positive.
    Plus,
    /// Numeric negation.
    Minus,
    /// Logical negation.
    Not,
    /// Type query.
    Typeof,
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
