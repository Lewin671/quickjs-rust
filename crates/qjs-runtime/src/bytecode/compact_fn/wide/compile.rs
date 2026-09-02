//! Lowers a stack-machine body into the wide register form.
//!
//! The transformation is the numeric tier's: every operand-stack depth is a
//! fixed register, locals occupy the low registers, and admission is
//! all-or-nothing. What differs is the admitted set -- named property reads
//! and writes, `this`, resolved calls, `Dup`, and backward edges -- and the
//! side tables that hold the named-property sites so the operation word
//! stays eight bytes.

use std::rc::Rc;

use super::{NamedReadSite, NamedWriteSite, WideOp, WideProgram};
use crate::bytecode::compact_fn::MAX_REGISTERS;
use crate::bytecode::ir::{Bytecode, Op};

/// What one bytecode instruction does to the operand stack and control flow.
struct Effect {
    pops: u16,
    pushes: u16,
    target: Option<usize>,
    falls_through: bool,
}

/// Whether a `LoadGlobal` is the compiler's spelling of a `this` read.
fn is_this_read(name: &str) -> bool {
    name == "this"
}

pub(super) fn compile(bytecode: &Bytecode) -> Option<WideProgram> {
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
    // Slots a fresh activation is guaranteed to have a value in; requiring
    // every read to land in this set is what lets the tier skip
    // temporal-dead-zone checking rather than reproduce its diagnostics.
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

    // Validate every instruction, including unreachable ones: the opcode set
    // is closed, not the reachability analysis.
    if !matches!(code[0], Op::FunctionPrologueEnd) {
        return None;
    }
    let slot_is_initialized_local = |slot: usize| -> bool {
        slot < local_count
            && slot < u128::BITS as usize
            && initialized_slots & (1_u128 << slot) != 0
    };
    for (ip, op) in code.iter().enumerate() {
        effect_of(op)?;
        match op {
            Op::FunctionPrologueEnd if ip != 0 => return None,
            // Backward edges are loops. The operand-stack depth at the target
            // is checked by `propagate_depths`, and a loop grows no frames --
            // a backward jump only moves the program counter -- so a loop
            // whose body is representable costs what any other control flow
            // does.
            Op::Jump(target) | Op::JumpIfFalse(target) | Op::JumpIfTrue(target)
                if *target > code.len() =>
            {
                return None;
            }
            Op::LoadConst(index) if *index >= bytecode.constants.len() => return None,
            // `this` is the only free name; every other global read keeps the
            // ordinary interpreter's resolution.
            Op::LoadGlobal(name) if !is_this_read(name) => return None,
            // A fused named read names its receiver local; it must be filled
            // on entry exactly like a `LoadLocal`.
            Op::GetPropNamed { cache, .. } => {
                if let Some(slot) = cache.local_slot()
                    && !slot_is_initialized_local(slot)
                {
                    return None;
                }
            }
            Op::LoadLocal(slot) if !slot_is_initialized_local(*slot) => return None,
            // An assignment expression writes like a declaration store; the
            // temporal-dead-zone check it adds is moot for a slot admission
            // proved filled on entry.
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                // A write must reach indexed storage directly: received
                // upvalues are read-only here, and an immutable binding needs
                // the general path's diagnostic.
                if *slot >= local_count || *slot >= u128::BITS as usize {
                    return None;
                }
                if upvalue_slots & (1_u128 << *slot) != 0 {
                    return None;
                }
                // Only this frame's own bindings -- parameters and hoisted
                // declarations -- live in the register file. A name resolved
                // from an enclosing scope (a global lexical binding, a
                // writable captured cell) is written through that binding.
                if !bytecode.locals.get(*slot).is_some_and(|local| {
                    local.mutable
                        && (local.parameter || local.hoisted)
                        && !local.sloppy_global_fallback
                }) {
                    return None;
                }
            }
            // Arity beyond the fixed forms drags in argument-vector
            // construction the tier has no evidence for.
            Op::Call(argc) | Op::CallResolved(argc) if *argc > 3 => return None,
            _ => {}
        }
    }

    // A loop this tier would run natively is a loop the frame-based loop
    // accelerators can no longer see: they attach to backward edges of the
    // ordinary interpreter, and an admitted body never returns to it. A body
    // whose loops any of those accelerators claims therefore keeps the
    // ordinary interpreter, where the loop runs faster than this tier's
    // per-operation dispatch; only loops none of them handles are admitted.
    let has_backward_edge = code.iter().enumerate().any(|(ip, op)| {
        matches!(
            op,
            Op::Jump(target) | Op::JumpIfFalse(target) | Op::JumpIfTrue(target) if *target <= ip
        )
    });
    if has_backward_edge && body_has_loop_accelerator(bytecode) {
        return None;
    }

    let entry_depth = propagate_depths(code)?;
    let stack_registers = entry_depth
        .iter()
        .flatten()
        .copied()
        .max()
        .unwrap_or(0)
        .checked_add(1)? as usize;
    // A fused named read addresses its receiver local directly, so such
    // slots count as touched even though no `LoadLocal` names them.
    let local_registers = code
        .iter()
        .filter_map(|op| match op {
            Op::LoadLocal(slot) | Op::StoreLocal(slot) => Some(*slot + 1),
            Op::GetPropNamed { cache, .. } => cache.local_slot().map(|slot| slot + 1),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        .min(local_count);
    let register_count = local_registers.checked_add(stack_registers)?;
    if register_count > MAX_REGISTERS {
        return None;
    }

    let mut ops = Vec::with_capacity(code.len());
    let mut named_reads: Vec<NamedReadSite> = Vec::new();
    let mut named_writes: Vec<NamedWriteSite> = Vec::new();
    let mut compact_index = vec![0_u32; code.len() + 1];
    let mut required_authoritative_slots = 0_u128;
    let mut requires_this = false;

    let register = |depth: u16| -> u16 { (local_registers as u16).saturating_add(depth) };

    for (ip, op) in code.iter().enumerate() {
        compact_index[ip] = u32::try_from(ops.len()).ok()?;
        let Some(depth) = entry_depth[ip] else {
            continue;
        };
        match op {
            Op::FunctionPrologueEnd => {}
            Op::Pop => ops.push(WideOp::Drop {
                src: register(depth.checked_sub(1)?),
            }),
            Op::Dup => ops.push(WideOp::Dup {
                src: register(depth.checked_sub(1)?),
                dst: register(depth),
            }),
            Op::LoadConst(index) => ops.push(WideOp::LoadConst {
                dst: register(depth),
                index: u32::try_from(*index).ok()?,
            }),
            Op::LoadLocal(slot) => {
                let slot_index = u16::try_from(*slot).ok()?;
                let slot_bit = 1_u128 << *slot;
                if upvalue_slots & slot_bit != 0 {
                    ops.push(WideOp::LoadUpvalueLocal {
                        dst: register(depth),
                        slot: slot_index,
                    });
                } else {
                    required_authoritative_slots |= slot_bit;
                    ops.push(WideOp::Move {
                        dst: register(depth),
                        src: slot_index,
                    });
                }
            }
            Op::LoadGlobal(_) => {
                requires_this = true;
                ops.push(WideOp::LoadThis {
                    dst: register(depth),
                });
            }
            Op::GetPropNamed { key, cache } => {
                let index = u16::try_from(named_reads.len()).ok()?;
                named_reads.push(NamedReadSite {
                    key: Rc::clone(key),
                    cache: cache.clone(),
                });
                match cache.local_slot() {
                    // A fused site reads its receiver local without consuming
                    // an operand, so the pushed depth is the destination. A
                    // plain local is read in place; an upvalue-backed one is
                    // loaded through its cell first and read from the
                    // destination register.
                    Some(slot) => {
                        let slot_bit = 1_u128 << slot;
                        if upvalue_slots & slot_bit != 0 {
                            ops.push(WideOp::LoadUpvalueLocal {
                                dst: register(depth),
                                slot: u16::try_from(slot).ok()?,
                            });
                            ops.push(WideOp::GetPropNamed {
                                dst: register(depth),
                                obj: register(depth),
                                index,
                            });
                        } else {
                            required_authoritative_slots |= slot_bit;
                            ops.push(WideOp::GetPropNamed {
                                dst: register(depth),
                                obj: u16::try_from(slot).ok()?,
                                index,
                            });
                        }
                    }
                    None => ops.push(WideOp::GetPropNamed {
                        dst: register(depth.checked_sub(1)?),
                        obj: register(depth.checked_sub(1)?),
                        index,
                    }),
                }
            }
            Op::SetPropNamed {
                key,
                cache,
                is_strict,
            } => {
                let index = u16::try_from(named_writes.len()).ok()?;
                named_writes.push(NamedWriteSite {
                    key: Rc::clone(key),
                    cache: cache.clone(),
                    is_strict: *is_strict,
                });
                ops.push(WideOp::SetPropNamed {
                    obj: register(depth.checked_sub(2)?),
                    value: register(depth.checked_sub(1)?),
                    index,
                });
            }
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                required_authoritative_slots |= 1_u128 << *slot;
                ops.push(WideOp::Move {
                    dst: u16::try_from(*slot).ok()?,
                    src: register(depth.checked_sub(1)?),
                });
            }
            Op::Binary(binary_op) => {
                let left = register(depth.checked_sub(2)?);
                let right = register(depth.checked_sub(1)?);
                ops.push(WideOp::Binary {
                    dst: left,
                    op: *binary_op,
                    left,
                    right,
                });
            }
            Op::JumpIfFalse(target) => ops.push(WideOp::JumpIfFalsy {
                cond: register(depth.checked_sub(1)?),
                target: u32::try_from(*target).ok()?,
            }),
            Op::JumpIfTrue(target) => ops.push(WideOp::JumpIfTruthy {
                cond: register(depth.checked_sub(1)?),
                target: u32::try_from(*target).ok()?,
            }),
            Op::Unary(unary_op) => {
                let src = register(depth.checked_sub(1)?);
                ops.push(WideOp::Unary {
                    dst: src,
                    op: *unary_op,
                    src,
                });
            }
            Op::Typeof => {
                let src = register(depth.checked_sub(1)?);
                ops.push(WideOp::Typeof { dst: src, src });
            }
            Op::ToNumeric => ops.push(WideOp::ToNumeric {
                dst: register(depth.checked_sub(1)?),
            }),
            Op::Update(update_op) => ops.push(WideOp::Update {
                dst: register(depth.checked_sub(1)?),
                op: *update_op,
            }),
            Op::GetProp => {
                let obj = register(depth.checked_sub(2)?);
                ops.push(WideOp::GetProp {
                    dst: obj,
                    obj,
                    key: register(depth.checked_sub(1)?),
                });
            }
            Op::Jump(target) => ops.push(WideOp::Jump {
                target: u32::try_from(*target).ok()?,
            }),
            Op::Call(argc) => {
                let base = register(depth.checked_sub(u16::try_from(*argc).ok()? + 1)?);
                ops.push(WideOp::Call {
                    dst: base,
                    base,
                    argc: u8::try_from(*argc).ok()?,
                });
            }
            Op::CallResolved(argc) => {
                let argc_u16 = u16::try_from(*argc).ok()?;
                // `[receiver, callee, args...]` collapses to the result, which
                // replaces the receiver's register; the callee sits above it.
                ops.push(WideOp::CallResolved {
                    dst: register(depth.checked_sub(argc_u16 + 2)?),
                    base: register(depth.checked_sub(argc_u16 + 1)?),
                    argc: u8::try_from(*argc).ok()?,
                });
            }
            Op::Return => ops.push(WideOp::Return {
                src: register(depth.checked_sub(1)?),
            }),
            _ => return None,
        }
    }
    compact_index[code.len()] = u32::try_from(ops.len()).ok()?;

    for op in &mut ops {
        match op {
            WideOp::Jump { target }
            | WideOp::JumpIfFalsy { target, .. }
            | WideOp::JumpIfTruthy { target, .. } => {
                *target = *compact_index.get(*target as usize)?;
            }
            _ => {}
        }
    }

    Some(WideProgram {
        ops,
        named_reads,
        named_writes,
        register_count,
        required_authoritative_slots,
        requires_this,
    })
}

/// Computes the operand-stack depth on entry to each instruction, rejecting a
/// body whose merge points disagree. Backward edges are ordinary edges here:
/// a loop header already visited must be reached at the same depth.
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
            visit(target, next_depth, &mut work)?;
        }
        if effect.falls_through {
            visit(ip + 1, next_depth, &mut work)?;
        }
    }
    Some(entry_depth)
}

fn effect_of(op: &Op) -> Option<Effect> {
    let simple = |pops: u16, pushes: u16| Effect {
        pops,
        pushes,
        target: None,
        falls_through: true,
    };
    let effect = match op {
        Op::FunctionPrologueEnd => simple(0, 0),
        Op::LoadConst(_) | Op::LoadLocal(_) | Op::Dup => simple(0, 1),
        Op::LoadGlobal(name) if is_this_read(name) => simple(0, 1),
        Op::Pop | Op::StoreLocal(_) | Op::AssignLocal(_) => simple(1, 0),
        Op::Binary(_) | Op::GetProp => simple(2, 1),
        Op::Unary(_) | Op::Typeof | Op::ToNumeric | Op::Update(_) => simple(1, 1),
        // A fused named read takes its receiver from a local, not the stack.
        Op::GetPropNamed { cache, .. } if cache.local_slot().is_some() => simple(0, 1),
        Op::GetPropNamed { .. } => simple(1, 1),
        Op::SetPropNamed { .. } => simple(2, 1),
        Op::JumpIfFalse(target) | Op::JumpIfTrue(target) => Effect {
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
        Op::Call(argc) => simple(u16::try_from(*argc).ok()?.checked_add(1)?, 1),
        Op::CallResolved(argc) => simple(u16::try_from(*argc).ok()?.checked_add(2)?, 1),
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

/// Whether any frame-based loop accelerator has a plan or program for a loop
/// in this body. The plans are compiled here if the interpreter has not yet
/// asked for them, exactly as `FrameProgramView::loop_plans` would.
fn body_has_loop_accelerator(bytecode: &Bytecode) -> bool {
    let typed = bytecode
        .typed_loop_programs
        .get_or_init(|| crate::bytecode::typed_loop::compile_all(bytecode));
    let control = bytecode
        .control_loop_plans
        .get_or_init(|| crate::bytecode::vm_control_loop::ControlLoopPlan::compile_all(bytecode));
    let numeric = bytecode
        .numeric_loop_plans
        .get_or_init(|| crate::bytecode::vm_numeric_loop::NumericLoopPlan::compile_all(bytecode));
    let mutation = bytecode.numeric_mutation_loop_plans.get_or_init(|| {
        crate::bytecode::vm_numeric_mutation_loop::NumericMutationLoopPlan::compile_all(bytecode)
    });
    !typed.is_empty() || !control.is_empty() || !numeric.is_empty() || !mutation.is_empty()
}
