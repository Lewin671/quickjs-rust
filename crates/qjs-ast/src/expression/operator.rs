/// Update operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateOp {
    /// `++`.
    Increment,
    /// `--`.
    Decrement,
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
