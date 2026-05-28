use crate::span::Span;
use crate::statement::Stmt;

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
    /// A constructor call expression.
    New {
        /// Constructor expression.
        callee: Box<Expr>,
        /// Argument expressions.
        arguments: Vec<Expr>,
        /// Source span.
        span: Span,
    },
    /// A function expression.
    Function {
        /// Optional function name.
        name: Option<String>,
        /// Parameter names.
        params: Vec<String>,
        /// Function body statements.
        body: Vec<Stmt>,
        /// Whether the function can be called with `new`.
        constructable: bool,
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
    /// A `this` expression.
    This { span: Span },
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
            | Self::New { span, .. }
            | Self::Function { span, .. }
            | Self::Member { span, .. }
            | Self::This { span }
            | Self::Identifier { span, .. } => *span,
        }
    }
}

/// Object literal property.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectProperty {
    /// Property key syntax.
    pub key: ObjectPropertyKey,
    /// Property value expression.
    pub value: Expr,
    /// Source span.
    pub span: Span,
}

/// Object literal property key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectPropertyKey {
    /// Literal property name after syntactic normalization.
    Literal(String),
    /// Computed property name expression, as in `{ [expr]: value }`.
    Computed(Expr),
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
    /// Prototype-chain instance test.
    Instanceof,
    /// Logical and.
    LogicalAnd,
    /// Logical or.
    LogicalOr,
    /// Nullish coalescing.
    NullishCoalescing,
}
