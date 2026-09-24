//! A number-only leaf program in register form.
//!
//! The straight-line stack program `compile_number_only_program` validates
//! knows every operand's stack depth statically, so each depth becomes a
//! fixed register after the leaf's locals, a local read becomes a reference
//! to the local's own register, and the evaluator neither pushes nor pops.
//! The commonest operators have operations of their own, so evaluating them
//! is one dispatch rather than two.

use qjs_ast::BinaryOp;

use super::{MAX_FAST_LOCALS, MAX_FAST_STACK, NumberOnlyOp, number_binary};

/// A leaf's locals, then one register per operand-stack depth.
const REGISTERS: usize = MAX_FAST_LOCALS + MAX_FAST_STACK;

#[derive(Clone, Copy, Debug)]
pub(super) enum RegisterOp {
    Const {
        dst: u8,
        value: f64,
    },
    Move {
        dst: u8,
        src: u8,
    },
    Add {
        dst: u8,
        left: u8,
        right: u8,
    },
    Sub {
        dst: u8,
        left: u8,
        right: u8,
    },
    Mul {
        dst: u8,
        left: u8,
        right: u8,
    },
    BitAnd {
        dst: u8,
        left: u8,
        right: u8,
    },
    BitOr {
        dst: u8,
        left: u8,
        right: u8,
    },
    BitXor {
        dst: u8,
        left: u8,
        right: u8,
    },
    Shl {
        dst: u8,
        left: u8,
        right: u8,
    },
    Shr {
        dst: u8,
        left: u8,
        right: u8,
    },
    UShr {
        dst: u8,
        left: u8,
        right: u8,
    },
    Binary {
        dst: u8,
        op: BinaryOp,
        left: u8,
        right: u8,
    },
    BinaryConst {
        dst: u8,
        op: BinaryOp,
        left: u8,
        right: f64,
    },
    Return {
        src: u8,
    },
}

/// Lowers a validated stack program; `None` for one whose registers do not
/// fit, which then keeps the general leaf path.
pub(super) fn lower(ops: &[NumberOnlyOp]) -> Option<Vec<RegisterOp>> {
    let temp = |depth: usize| -> Option<u8> {
        (depth < MAX_FAST_STACK)
            .then(|| u8::try_from(MAX_FAST_LOCALS + depth).ok())
            .flatten()
    };
    let local = |slot: usize| -> Option<u8> {
        (slot < MAX_FAST_LOCALS)
            .then(|| u8::try_from(slot).ok())
            .flatten()
    };
    // The register holding each operand-stack value.
    let mut stack: Vec<u8> = Vec::with_capacity(MAX_FAST_STACK);
    let mut lowered = Vec::with_capacity(ops.len());
    for op in ops {
        match *op {
            NumberOnlyOp::LoadConst(value) => {
                let dst = temp(stack.len())?;
                lowered.push(RegisterOp::Const { dst, value });
                stack.push(dst);
            }
            NumberOnlyOp::LoadLocal(slot) => stack.push(local(slot)?),
            NumberOnlyOp::StoreLocal(slot) => {
                let slot = local(slot)?;
                let src = stack.pop()?;
                // An operand still on the stack that reads the local keeps
                // the value it read, not the one stored now.
                for (depth, operand) in stack.iter_mut().enumerate() {
                    if *operand == slot {
                        let dst = temp(depth)?;
                        lowered.push(RegisterOp::Move { dst, src: slot });
                        *operand = dst;
                    }
                }
                if src != slot {
                    lowered.push(RegisterOp::Move { dst: slot, src });
                }
            }
            NumberOnlyOp::Binary(op) => {
                let right = stack.pop()?;
                let left = stack.pop()?;
                let dst = temp(stack.len())?;
                lowered.push(binary(op, dst, left, right));
                stack.push(dst);
            }
            NumberOnlyOp::BinaryConstRight(op, right) => {
                let left = stack.pop()?;
                let dst = temp(stack.len())?;
                lowered.push(RegisterOp::BinaryConst {
                    dst,
                    op,
                    left,
                    right,
                });
                stack.push(dst);
            }
            NumberOnlyOp::Return => {
                lowered.push(RegisterOp::Return { src: stack.pop()? });
                return Some(lowered);
            }
        }
    }
    None
}

fn binary(op: BinaryOp, dst: u8, left: u8, right: u8) -> RegisterOp {
    match op {
        BinaryOp::Add => RegisterOp::Add { dst, left, right },
        BinaryOp::Sub => RegisterOp::Sub { dst, left, right },
        BinaryOp::Mul => RegisterOp::Mul { dst, left, right },
        BinaryOp::BitwiseAnd => RegisterOp::BitAnd { dst, left, right },
        BinaryOp::BitwiseOr => RegisterOp::BitOr { dst, left, right },
        BinaryOp::BitwiseXor => RegisterOp::BitXor { dst, left, right },
        BinaryOp::Shl => RegisterOp::Shl { dst, left, right },
        BinaryOp::Shr => RegisterOp::Shr { dst, left, right },
        BinaryOp::UShr => RegisterOp::UShr { dst, left, right },
        op => RegisterOp::Binary {
            dst,
            op,
            left,
            right,
        },
    }
}

/// Evaluates a lowered program on one number per parameter; `None` for too
/// few arguments.
pub(super) fn eval(
    ops: &[RegisterOp],
    parameter_slots: &[usize],
    arguments: &[f64],
) -> Option<f64> {
    let mut registers = [0.0_f64; REGISTERS];
    for (index, &slot) in parameter_slots.iter().enumerate() {
        *registers.get_mut(slot)? = *arguments.get(index)?;
    }
    let int32 = crate::to_int32_number;
    let uint32 = crate::to_uint32_number;
    for op in ops {
        match *op {
            RegisterOp::Const { dst, value } => registers[usize::from(dst)] = value,
            RegisterOp::Move { dst, src } => {
                registers[usize::from(dst)] = registers[usize::from(src)];
            }
            RegisterOp::Add { dst, left, right } => {
                registers[usize::from(dst)] =
                    registers[usize::from(left)] + registers[usize::from(right)];
            }
            RegisterOp::Sub { dst, left, right } => {
                registers[usize::from(dst)] =
                    registers[usize::from(left)] - registers[usize::from(right)];
            }
            RegisterOp::Mul { dst, left, right } => {
                registers[usize::from(dst)] =
                    registers[usize::from(left)] * registers[usize::from(right)];
            }
            RegisterOp::BitAnd { dst, left, right } => {
                registers[usize::from(dst)] = f64::from(
                    int32(registers[usize::from(left)]) & int32(registers[usize::from(right)]),
                );
            }
            RegisterOp::BitOr { dst, left, right } => {
                registers[usize::from(dst)] = f64::from(
                    int32(registers[usize::from(left)]) | int32(registers[usize::from(right)]),
                );
            }
            RegisterOp::BitXor { dst, left, right } => {
                registers[usize::from(dst)] = f64::from(
                    int32(registers[usize::from(left)]) ^ int32(registers[usize::from(right)]),
                );
            }
            RegisterOp::Shl { dst, left, right } => {
                registers[usize::from(dst)] = f64::from(
                    int32(registers[usize::from(left)])
                        << (uint32(registers[usize::from(right)]) & 0x1f),
                );
            }
            RegisterOp::Shr { dst, left, right } => {
                registers[usize::from(dst)] = f64::from(
                    int32(registers[usize::from(left)])
                        >> (uint32(registers[usize::from(right)]) & 0x1f),
                );
            }
            RegisterOp::UShr { dst, left, right } => {
                registers[usize::from(dst)] = f64::from(
                    uint32(registers[usize::from(left)])
                        >> (uint32(registers[usize::from(right)]) & 0x1f),
                );
            }
            RegisterOp::Binary {
                dst,
                op,
                left,
                right,
            } => {
                registers[usize::from(dst)] = number_binary(
                    registers[usize::from(left)],
                    op,
                    registers[usize::from(right)],
                )?;
            }
            RegisterOp::BinaryConst {
                dst,
                op,
                left,
                right,
            } => {
                registers[usize::from(dst)] =
                    number_binary(registers[usize::from(left)], op, right)?;
            }
            RegisterOp::Return { src } => return Some(registers[usize::from(src)]),
        }
    }
    None
}
