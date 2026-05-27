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
    /// A comma-separated sequence expression.
    Sequence {
        /// Expressions evaluated from left to right.
        expressions: Vec<Expr>,
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
    /// A conditional expression.
    Conditional {
        /// Test expression.
        test: Box<Expr>,
        /// Expression evaluated when the test is truthy.
        consequent: Box<Expr>,
        /// Expression evaluated when the test is falsy.
        alternate: Box<Expr>,
        /// Source span.
        span: Span,
    },
    /// An assignment expression.
    Assignment {
        /// Assigned target.
        target: AssignmentTarget,
        /// Assignment operator.
        op: AssignmentOp,
        /// Assigned value.
        value: Box<Expr>,
        /// Source span.
        span: Span,
    },
    /// An update expression.
    Update {
        /// Updated target.
        target: AssignmentTarget,
        /// Update operator.
        op: UpdateOp,
        /// Whether this is a prefix update.
        prefix: bool,
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
            | Self::Sequence { span, .. }
            | Self::Unary { span, .. }
            | Self::Binary { span, .. }
            | Self::Conditional { span, .. }
            | Self::Assignment { span, .. }
            | Self::Update { span, .. }
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

/// Assignment operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssignmentOp {
    /// `=`.
    Assign,
    /// `+=`.
    AddAssign,
    /// `-=`.
    SubAssign,
    /// `*=`.
    MulAssign,
    /// `**=`.
    PowAssign,
    /// `/=`.
    DivAssign,
    /// `%=`.
    RemAssign,
    /// `<<=`.
    ShlAssign,
    /// `>>=`.
    ShrAssign,
    /// `>>>=`.
    UShrAssign,
    /// `&=`.
    BitwiseAndAssign,
    /// `^=`.
    BitwiseXorAssign,
    /// `|=`.
    BitwiseOrAssign,
    /// `&&=`.
    LogicalAndAssign,
    /// `||=`.
    LogicalOrAssign,
    /// `??=`.
    NullishAssign,
}

/// Update operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateOp {
    /// `++`.
    Increment,
    /// `--`.
    Decrement,
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
    /// Bitwise complement.
    BitwiseNot,
    /// Type query.
    Typeof,
    /// Undefined result after evaluating the operand.
    Void,
    /// Property deletion.
    Delete,
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
    /// Exponentiation.
    Pow,
    /// Left shift.
    Shl,
    /// Signed right shift.
    Shr,
    /// Unsigned right shift.
    UShr,
    /// Loose equality.
    Eq,
    /// Strict equality.
    StrictEq,
    /// Loose inequality.
    Ne,
    /// Strict inequality.
    StrictNe,
    /// Bitwise and.
    BitwiseAnd,
    /// Bitwise xor.
    BitwiseXor,
    /// Bitwise or.
    BitwiseOr,
    /// Less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Ge,
    /// Property existence.
    In,
    /// Logical and.
    LogicalAnd,
    /// Logical or.
    LogicalOr,
    /// Nullish coalescing.
    NullishCoalescing,
}
