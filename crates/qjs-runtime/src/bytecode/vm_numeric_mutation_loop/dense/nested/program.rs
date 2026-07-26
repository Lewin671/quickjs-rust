//! Predecoded numeric instructions for nested dense regions.
//!
//! The nested executor has already proved that every operation is a pure
//! Number operation before it takes an Array lease. Lowering the shared
//! `NumberInstruction` stream here removes the second dynamic dispatch on
//! `BinaryOp` or `UnaryOp` from each hot inner iteration.

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use super::super::{DenseAccess, NumberInstruction, Register, array_index_from_number};
use super::LocalBank;

#[derive(Clone, Copy, Debug)]
pub(super) enum NestedInstruction {
    Constant(f64),
    LoadLocal(usize),
    DenseLoad {
        receiver: usize,
        index: Register,
    },
    DenseStore {
        receiver: usize,
        index: Register,
        value: Register,
    },
    Add {
        left: Register,
        right: Register,
    },
    Sub {
        left: Register,
        right: Register,
    },
    Mul {
        left: Register,
        right: Register,
    },
    Div {
        left: Register,
        right: Register,
    },
    Rem {
        left: Register,
        right: Register,
    },
    Shl {
        left: Register,
        right: Register,
    },
    Shr {
        left: Register,
        right: Register,
    },
    UShr {
        left: Register,
        right: Register,
    },
    BitwiseAnd {
        left: Register,
        right: Register,
    },
    BitwiseXor {
        left: Register,
        right: Register,
    },
    BitwiseOr {
        left: Register,
        right: Register,
    },
    Plus {
        value: Register,
    },
    Minus {
        value: Register,
    },
    BitwiseNot {
        value: Register,
    },
    Increment {
        value: Register,
    },
    Decrement {
        value: Register,
    },
}

impl NestedInstruction {
    pub(super) fn lower(operation: NumberInstruction) -> Option<Self> {
        Some(match operation {
            NumberInstruction::Constant(value) => Self::Constant(value),
            NumberInstruction::LoadLocal(local) => Self::LoadLocal(local),
            NumberInstruction::DenseLoad { receiver, index } => Self::DenseLoad { receiver, index },
            NumberInstruction::DenseStore {
                receiver,
                index,
                value,
            } => Self::DenseStore {
                receiver,
                index,
                value,
            },
            NumberInstruction::Binary {
                operation,
                left,
                right,
            } => match operation {
                BinaryOp::Add => Self::Add { left, right },
                BinaryOp::Sub => Self::Sub { left, right },
                BinaryOp::Mul => Self::Mul { left, right },
                BinaryOp::Div => Self::Div { left, right },
                BinaryOp::Rem => Self::Rem { left, right },
                BinaryOp::Shl => Self::Shl { left, right },
                BinaryOp::Shr => Self::Shr { left, right },
                BinaryOp::UShr => Self::UShr { left, right },
                BinaryOp::BitwiseAnd => Self::BitwiseAnd { left, right },
                BinaryOp::BitwiseXor => Self::BitwiseXor { left, right },
                BinaryOp::BitwiseOr => Self::BitwiseOr { left, right },
                _ => return None,
            },
            NumberInstruction::Unary { operation, value } => match operation {
                UnaryOp::Plus => Self::Plus { value },
                UnaryOp::Minus => Self::Minus { value },
                UnaryOp::BitwiseNot => Self::BitwiseNot { value },
                _ => return None,
            },
            NumberInstruction::Update { operation, value } => match operation {
                UpdateOp::Increment => Self::Increment { value },
                UpdateOp::Decrement => Self::Decrement { value },
            },
            NumberInstruction::LoadInvariant(_) | NumberInstruction::MathRound { .. } => {
                return None;
            }
        })
    }
}

#[inline(always)]
pub(super) fn run_operation<A: DenseAccess>(
    operation: NestedInstruction,
    access: &mut A,
    bank: &LocalBank,
    registers: &[f64],
) -> Option<f64> {
    Some(match operation {
        NestedInstruction::Constant(value) => value,
        NestedInstruction::LoadLocal(local) => bank.number(local)?,
        NestedInstruction::DenseLoad { receiver, index } => {
            access.load_number(receiver, array_index_from_number(registers[index])?)?
        }
        NestedInstruction::DenseStore {
            receiver,
            index,
            value,
        } => {
            let index = array_index_from_number(registers[index])?;
            let value = registers[value];
            access
                .stage_store(receiver, index, value)
                .then_some(value)?
        }
        NestedInstruction::Add { left, right } => registers[left] + registers[right],
        NestedInstruction::Sub { left, right } => registers[left] - registers[right],
        NestedInstruction::Mul { left, right } => registers[left] * registers[right],
        NestedInstruction::Div { left, right } => registers[left] / registers[right],
        NestedInstruction::Rem { left, right } => {
            crate::operations::number_remainder(registers[left], registers[right])
        }
        NestedInstruction::Shl { left, right } => f64::from(
            crate::to_int32_number(registers[left])
                << (crate::to_uint32_number(registers[right]) & 0x1f),
        ),
        NestedInstruction::Shr { left, right } => f64::from(
            crate::to_int32_number(registers[left])
                >> (crate::to_uint32_number(registers[right]) & 0x1f),
        ),
        NestedInstruction::UShr { left, right } => f64::from(
            crate::to_uint32_number(registers[left])
                >> (crate::to_uint32_number(registers[right]) & 0x1f),
        ),
        NestedInstruction::BitwiseAnd { left, right } => f64::from(
            crate::to_int32_number(registers[left]) & crate::to_int32_number(registers[right]),
        ),
        NestedInstruction::BitwiseXor { left, right } => f64::from(
            crate::to_int32_number(registers[left]) ^ crate::to_int32_number(registers[right]),
        ),
        NestedInstruction::BitwiseOr { left, right } => f64::from(
            crate::to_int32_number(registers[left]) | crate::to_int32_number(registers[right]),
        ),
        NestedInstruction::Plus { value } => registers[value],
        NestedInstruction::Minus { value } => -registers[value],
        NestedInstruction::BitwiseNot { value } => {
            f64::from(!crate::to_int32_number(registers[value]))
        }
        NestedInstruction::Increment { value } => registers[value] + 1.0,
        NestedInstruction::Decrement { value } => registers[value] - 1.0,
    })
}
