use qjs_ast::BinaryOp;

use crate::Value;

use super::{
    Bytecode, FastOp, FastValue, MAX_FAST_STACK, parameter_index, pop_number, push_number,
};

/// Prevalidated Number-only arithmetic that avoids materializing `FastValue`
/// values for every intermediate result of a multi-step leaf expression.
///
/// This deliberately accepts only parameter reads, number literals, and the
/// five arithmetic operators whose Number result is unconditional. The normal
/// leaf VM remains responsible for coercion, local writes, captures, and every
/// other bytecode shape.
#[derive(Clone, Debug)]
pub(super) struct NumericLeafArithmeticProgram {
    ops: Vec<NumericLeafArithmeticOp>,
}

#[derive(Clone, Copy, Debug)]
enum NumericLeafArithmeticOp {
    Argument(usize),
    Constant(f64),
    Binary(NumericLeafArithmeticBinary),
    BinaryConstRight {
        op: NumericLeafArithmeticBinary,
        right: f64,
    },
}

#[derive(Clone, Copy, Debug)]
enum NumericLeafArithmeticBinary {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

impl NumericLeafArithmeticBinary {
    fn from_binary_op(op: BinaryOp) -> Option<Self> {
        match op {
            BinaryOp::Add => Some(Self::Add),
            BinaryOp::Sub => Some(Self::Sub),
            BinaryOp::Mul => Some(Self::Mul),
            BinaryOp::Div => Some(Self::Div),
            BinaryOp::Rem => Some(Self::Rem),
            _ => None,
        }
    }

    #[inline(always)]
    fn eval(self, left: f64, right: f64) -> f64 {
        match self {
            Self::Add => left + right,
            Self::Sub => left - right,
            Self::Mul => left * right,
            Self::Div => left / right,
            Self::Rem => crate::operations::number_remainder(left, right),
        }
    }
}

impl NumericLeafArithmeticProgram {
    pub(super) fn compile(ops: &[FastOp], bytecode: &Bytecode) -> Option<Self> {
        let mut program = Vec::with_capacity(ops.len());
        let mut stack_depth = 0_usize;
        for (index, op) in ops.iter().enumerate() {
            match op {
                FastOp::LoadLocal(slot) => {
                    let argument_index = parameter_index(bytecode, *slot)?;
                    if stack_depth == MAX_FAST_STACK {
                        return None;
                    }
                    program.push(NumericLeafArithmeticOp::Argument(argument_index));
                    stack_depth += 1;
                }
                FastOp::LoadConst(FastValue::Number(value)) => {
                    if stack_depth == MAX_FAST_STACK {
                        return None;
                    }
                    program.push(NumericLeafArithmeticOp::Constant(*value));
                    stack_depth += 1;
                }
                FastOp::Binary(op) => {
                    let op = NumericLeafArithmeticBinary::from_binary_op(*op)?;
                    stack_depth = stack_depth.checked_sub(1)?;
                    program.push(NumericLeafArithmeticOp::Binary(op));
                }
                FastOp::BinaryConstRight(op, right) => {
                    let op = NumericLeafArithmeticBinary::from_binary_op(*op)?;
                    if stack_depth == 0 {
                        return None;
                    }
                    program.push(NumericLeafArithmeticOp::BinaryConstRight { op, right: *right });
                }
                FastOp::Return if index + 1 == ops.len() && stack_depth == 1 => {
                    return Some(Self { ops: program });
                }
                _ => return None,
            }
        }
        None
    }

    pub(super) fn eval(&self, arguments: &[Value]) -> Option<f64> {
        let mut stack = [0.0; MAX_FAST_STACK];
        let mut stack_len = 0_usize;
        for op in &self.ops {
            match op {
                NumericLeafArithmeticOp::Argument(index) => {
                    let Value::Number(value) = arguments.get(*index)? else {
                        return None;
                    };
                    push_number(&mut stack, &mut stack_len, *value)?;
                }
                NumericLeafArithmeticOp::Constant(value) => {
                    push_number(&mut stack, &mut stack_len, *value)?;
                }
                NumericLeafArithmeticOp::Binary(op) => {
                    let right = pop_number(&stack, &mut stack_len)?;
                    let left = pop_number(&stack, &mut stack_len)?;
                    push_number(&mut stack, &mut stack_len, op.eval(left, right))?;
                }
                NumericLeafArithmeticOp::BinaryConstRight { op, right } => {
                    let left = pop_number(&stack, &mut stack_len)?;
                    push_number(&mut stack, &mut stack_len, op.eval(left, *right))?;
                }
            }
        }
        let result = pop_number(&stack, &mut stack_len)?;
        (stack_len == 0).then_some(result)
    }
}
