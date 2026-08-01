//! Lowers a stack-machine body into the compact register form.
//!
//! The transformation is the classic one: because the operand-stack depth at
//! every instruction is statically determined for structured bytecode, each
//! stack slot can be given a fixed register index at compile time. Depth `d`
//! becomes register `local_count + d`, which turns every push/pop pair into a
//! direct register reference and removes the operand traffic entirely.
//!
//! Admission is all-or-nothing and deliberately narrow: an opcode outside the
//! supported set, an unbalanced merge point, or a register file wider than
//! `MAX_REGISTERS` rejects the whole body. Rejection is free -- the caller
//! keeps the ordinary interpreter -- so the bar for adding an opcode here is
//! evidence that it appears in an admitted body, not completeness.

use super::{CompactFunctionProgram, CompactOp, MAX_REGISTERS};
use crate::bytecode::ir::{Bytecode, Op};

/// What one bytecode instruction does to the operand stack and control flow.
struct Effect {
    pops: u16,
    pushes: u16,
    /// Branch target, if this instruction has one.
    target: Option<usize>,
    /// Whether control reaches the following instruction.
    falls_through: bool,
}

pub(super) fn compile(bytecode: &Bytecode) -> Option<CompactFunctionProgram> {
    // Bodies that suspend, catch, or resolve names dynamically keep the
    // ordinary interpreter: this tier has no completion protocol beyond
    // return-or-propagate.
    if bytecode.contains_direct_eval() || bytecode.contains_with() || bytecode.global_scope {
        return None;
    }

    let code = &bytecode.code;
    if code.is_empty() {
        return None;
    }
    let local_count = bytecode.locals.len();
    if local_count >= MAX_REGISTERS {
        return None;
    }
    let upvalue_slots = bytecode
        .direct_readonly_received_upvalue_slots()
        .unwrap_or(0);
    // Slots a fresh activation is guaranteed to have a value in: parameters
    // are seeded from the arguments, hoisted `var`s from `undefined`, and
    // received upvalues from their cell. Requiring every read to land in this
    // set is what lets the tier skip temporal-dead-zone checking entirely
    // rather than reproduce the general path's diagnostics.
    let mut initialized_slots = upvalue_slots;
    for &slot in bytecode.parameter_slots() {
        if slot >= u128::BITS as usize {
            return None;
        }
        initialized_slots |= 1_u128 << slot;
    }
    for &slot in bytecode.hoisted_slots() {
        if slot >= u128::BITS {
            return None;
        }
        initialized_slots |= 1_u128 << slot;
    }

    // Validate every instruction, including ones no path reaches. An
    // unsupported opcode in dead code means this body's shape is outside what
    // the tier was designed and tested for, and admitting it would rest on the
    // reachability analysis below being exactly right rather than on the
    // opcode set being closed.
    if !matches!(code[0], Op::FunctionPrologueEnd) {
        return None;
    }
    for (ip, op) in code.iter().enumerate() {
        effect_of(op)?;
        match op {
            // An executable parameter prologue would run before instruction
            // zero's semantics; only the canonical single marker is admitted.
            Op::FunctionPrologueEnd if ip != 0 => return None,
            // Backward edges are loops. Nothing in this unit bounds or tests
            // them, and the sentinel it targets has none, so they are rejected
            // rather than left to work by accident.
            Op::Jump(target) | Op::JumpIfFalse(target) if *target <= ip => return None,
            Op::Jump(target) | Op::JumpIfFalse(target) if *target > code.len() => return None,
            Op::LoadConst(index) if *index >= bytecode.constants.len() => return None,
            Op::LoadLocal(slot) => {
                if *slot >= local_count || *slot >= u128::BITS as usize {
                    return None;
                }
                if initialized_slots & (1_u128 << *slot) == 0 {
                    return None;
                }
            }
            Op::StoreLocal(slot) => {
                // A write must reach indexed storage directly: received
                // upvalues are read-only here, and an immutable binding needs
                // the general path's diagnostic.
                if *slot >= local_count || *slot >= u128::BITS as usize {
                    return None;
                }
                if upvalue_slots & (1_u128 << *slot) != 0 {
                    return None;
                }
                if !bytecode
                    .locals
                    .get(*slot)
                    .is_some_and(|local| local.mutable)
                {
                    return None;
                }
            }
            // Arity beyond the fixed forms drags in argument-vector
            // construction the tier has no evidence for.
            Op::Call(argc) if *argc > 3 => return None,
            _ => {}
        }
    }

    let entry_depth = propagate_depths(code)?;
    // The register file replaces the operand stack only: locals keep living in
    // the frame's indexed storage. Seeding them into registers would be faster
    // still, but it would have to reproduce this body's uninitialized-lexical
    // and upvalue-cell semantics at entry, which is a separate unit.
    let register_count = entry_depth
        .iter()
        .flatten()
        .copied()
        .max()
        .unwrap_or(0)
        .saturating_add(2) as usize;
    if register_count > MAX_REGISTERS {
        return None;
    }

    let mut ops = Vec::with_capacity(code.len());
    // Maps a bytecode index to the compact index that its first emitted
    // operation occupies, so branch targets can be rewritten in a second pass.
    let mut compact_index = vec![0_u32; code.len() + 1];
    let mut required_authoritative_slots = 0_u128;

    let register = |depth: u16| -> u16 { depth };

    for (ip, op) in code.iter().enumerate() {
        compact_index[ip] = u32::try_from(ops.len()).ok()?;
        let Some(depth) = entry_depth[ip] else {
            // Unreachable instruction: emit nothing. Nothing can branch to it,
            // because a branch would have given it an entry depth.
            continue;
        };
        match op {
            Op::FunctionPrologueEnd => {}
            Op::Pop => ops.push(CompactOp::Drop {
                src: register(depth.checked_sub(1)?),
            }),
            Op::LoadConst(index) => ops.push(CompactOp::LoadConst {
                dst: register(depth),
                index: u32::try_from(*index).ok()?,
            }),
            Op::LoadLocal(slot) => {
                let slot_index = u16::try_from(*slot).ok()?;
                if *slot >= u128::BITS as usize {
                    return None;
                }
                let slot_bit = 1_u128 << *slot;
                if upvalue_slots & slot_bit != 0 {
                    ops.push(CompactOp::LoadUpvalueLocal {
                        dst: register(depth),
                        slot: slot_index,
                    });
                } else {
                    required_authoritative_slots |= slot_bit;
                    ops.push(CompactOp::LoadLocal {
                        dst: register(depth),
                        slot: slot_index,
                    });
                }
            }
            Op::StoreLocal(slot) => {
                // Prevalidation above already proved this slot is an indexed,
                // mutable, non-upvalue local.
                let slot_bit = 1_u128 << *slot;
                required_authoritative_slots |= slot_bit;
                ops.push(CompactOp::StoreLocal {
                    slot: u16::try_from(*slot).ok()?,
                    src: register(depth.checked_sub(1)?),
                });
            }
            Op::Binary(binary_op) => {
                let left = register(depth.checked_sub(2)?);
                let right = register(depth.checked_sub(1)?);
                ops.push(CompactOp::Binary {
                    dst: left,
                    op: *binary_op,
                    left,
                    right,
                });
            }
            Op::JumpIfFalse(target) => ops.push(CompactOp::JumpIfFalsy {
                cond: register(depth.checked_sub(1)?),
                // Rewritten below; `target` is still a bytecode index here.
                target: u32::try_from(*target).ok()?,
            }),
            Op::Jump(target) => ops.push(CompactOp::Jump {
                target: u32::try_from(*target).ok()?,
            }),
            Op::Call(argc) => {
                let argc_u8 = u8::try_from(*argc).ok()?;
                let base = register(depth.checked_sub(u16::try_from(*argc).ok()? + 1)?);
                ops.push(CompactOp::Call {
                    dst: base,
                    base,
                    argc: argc_u8,
                });
            }
            Op::Return => ops.push(CompactOp::Return {
                src: register(depth.checked_sub(1)?),
            }),
            _ => return None,
        }
    }
    compact_index[code.len()] = u32::try_from(ops.len()).ok()?;

    // Second pass: rewrite branch targets from bytecode indices to compact ones.
    for op in &mut ops {
        match op {
            CompactOp::Jump { target } | CompactOp::JumpIfFalsy { target, .. } => {
                *target = *compact_index.get(*target as usize)?;
            }
            _ => {}
        }
    }

    Some(CompactFunctionProgram {
        ops,
        register_count,
        required_authoritative_slots,
        scratch_pool: std::cell::OnceCell::new(),
    })
}

/// Computes the operand-stack depth on entry to each instruction, rejecting a
/// body whose merge points disagree.
///
/// A disagreement is not necessarily invalid bytecode, but it defeats the
/// fixed register assignment this tier depends on, so it declines.
fn propagate_depths(code: &[Op]) -> Option<Vec<Option<u16>>> {
    let mut entry_depth: Vec<Option<u16>> = vec![None; code.len()];
    entry_depth[0] = Some(0);
    let mut work = vec![0_usize];
    while let Some(ip) = work.pop() {
        let depth = entry_depth[ip]?;
        let effect = effect_of(code.get(ip)?)?;
        let next_depth = depth.checked_sub(effect.pops)?.checked_add(effect.pushes)?;
        let mut visit = |target: usize, depth: u16, work: &mut Vec<usize>| -> Option<()> {
            if target >= code.len() {
                // A branch past the end is the implicit-return position, which
                // carries no operands.
                return (depth == 0).then_some(());
            }
            match entry_depth[target] {
                Some(existing) => (existing == depth).then_some(())?,
                None => {
                    entry_depth[target] = Some(depth);
                    work.push(target);
                }
            }
            Some(())
        };
        if let Some(target) = effect.target {
            // A conditional branch peeks, so both edges carry `next_depth`.
            visit(target, next_depth, &mut work)?;
        }
        if effect.falls_through {
            visit(ip + 1, next_depth, &mut work)?;
        }
    }
    Some(entry_depth)
}

fn effect_of(op: &Op) -> Option<Effect> {
    let effect = match op {
        Op::FunctionPrologueEnd => Effect {
            pops: 0,
            pushes: 0,
            target: None,
            falls_through: true,
        },
        Op::LoadConst(_) | Op::LoadLocal(_) => Effect {
            pops: 0,
            pushes: 1,
            target: None,
            falls_through: true,
        },
        Op::Pop | Op::StoreLocal(_) => Effect {
            pops: 1,
            pushes: 0,
            target: None,
            falls_through: true,
        },
        Op::Binary(_) => Effect {
            pops: 2,
            pushes: 1,
            target: None,
            falls_through: true,
        },
        // `JumpIfFalse` peeks rather than pops, matching the interpreter.
        Op::JumpIfFalse(target) => Effect {
            pops: 0,
            pushes: 0,
            target: Some(*target),
            falls_through: true,
        },
        Op::Jump(target) => Effect {
            pops: 0,
            pushes: 0,
            target: Some(*target),
            falls_through: false,
        },
        Op::Call(argc) => Effect {
            pops: u16::try_from(*argc).ok()?.checked_add(1)?,
            pushes: 1,
            target: None,
            falls_through: true,
        },
        Op::Return => Effect {
            pops: 1,
            pushes: 0,
            target: None,
            falls_through: false,
        },
        _ => return None,
    };
    Some(effect)
}
