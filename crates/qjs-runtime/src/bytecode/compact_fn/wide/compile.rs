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
    // The frame's own `let`/`const` declarations: neither seeded on entry nor
    // received from an enclosing scope. They live in the register file behind
    // a temporal-dead-zone marker; a received cell that is not read-only is
    // not admitted at all, since it would be written through the cell.
    let slot_is_lexical = |slot: usize| -> bool {
        slot < local_count
            && slot < u128::BITS as usize
            && bytecode.locals.get(slot).is_some_and(|local| {
                !local.parameter
                    && !local.hoisted
                    && !local.is_received_upvalue()
                    && !local.sloppy_global_fallback
                    && !local.compiler_temporary
            })
    };
    let slot_is_own_binding = |slot: usize| -> bool {
        bytecode.locals.get(slot).is_some_and(|local| {
            (local.parameter || local.hoisted) && !local.sloppy_global_fallback
        })
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
            // A fused receiver local must be readable exactly like a
            // `LoadLocal`: filled on entry, or an own lexical behind its
            // marker.
            Op::GetPropNamed { cache, .. } => {
                if let Some(slot) = cache.local_slot()
                    && !slot_is_initialized_local(slot)
                    && !slot_is_lexical(slot)
                {
                    return None;
                }
            }
            Op::GetPropIndex(encoded) => {
                let (index, local_slot) = crate::bytecode::ir::decode_index_receiver(*encoded);
                if u16::try_from(index).is_err() {
                    return None;
                }
                if let Some(slot) = local_slot
                    && !slot_is_initialized_local(slot)
                    && !slot_is_lexical(slot)
                {
                    return None;
                }
            }
            Op::LoadLocal(slot) if !slot_is_initialized_local(*slot) && !slot_is_lexical(*slot) => {
                return None;
            }
            Op::ClearLocal(slot) if !slot_is_lexical(*slot) => return None,
            // A declaration store initializes an own lexical (`const`
            // included) or writes one of the frame's own hoisted bindings.
            Op::StoreLocal(slot) => {
                if *slot >= local_count || *slot >= u128::BITS as usize {
                    return None;
                }
                if upvalue_slots & (1_u128 << *slot) != 0 {
                    return None;
                }
                if !slot_is_lexical(*slot)
                    && !bytecode
                        .locals
                        .get(*slot)
                        .is_some_and(|local| local.mutable && slot_is_own_binding(*slot))
                {
                    return None;
                }
            }
            // An assignment expression writes a mutable own binding: a hoisted
            // one like a store, a lexical one after its initialization check.
            // A name resolved from an enclosing scope (a global lexical
            // binding, a writable captured cell) is written through that
            // binding and keeps the interpreter.
            Op::AssignLocal(slot) => {
                if *slot >= local_count || *slot >= u128::BITS as usize {
                    return None;
                }
                if upvalue_slots & (1_u128 << *slot) != 0 {
                    return None;
                }
                if !bytecode.locals.get(*slot).is_some_and(|local| {
                    local.mutable && (slot_is_own_binding(*slot) || slot_is_lexical(*slot))
                }) {
                    return None;
                }
            }
            Op::NewArray { elements }
                if !elements.iter().all(|element| {
                    matches!(element, crate::bytecode::ir::ArrayElementKind::Expr)
                }) || u16::try_from(elements.len()).is_err() =>
            {
                return None;
            }
            Op::New(argc) if *argc > 3 => return None,
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
    // Likewise a body the interpreter's virtual-object lowering rewrites --
    // an object or array literal it can keep in slots instead of allocating
    // -- keeps the interpreter, where that lowering runs.
    if bytecode
        .virtual_object_program
        .get_or_init(|| crate::bytecode::virtual_object::lower(bytecode))
        .lowers_anything()
    {
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
            Op::LoadLocal(slot)
            | Op::StoreLocal(slot)
            | Op::AssignLocal(slot)
            | Op::ClearLocal(slot) => Some(*slot + 1),
            Op::GetPropNamed { cache, .. } => cache.local_slot().map(|slot| slot + 1),
            Op::GetPropIndex(encoded) => crate::bytecode::ir::decode_index_receiver(*encoded)
                .1
                .map(|slot| slot + 1),
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
    let mut global_names: Vec<String> = Vec::new();
    let mut lexical_slots: Vec<u16> = Vec::new();
    let note_lexical = |slot: usize, lexical_slots: &mut Vec<u16>| -> Option<u16> {
        let slot = u16::try_from(slot).ok()?;
        if !lexical_slots.contains(&slot) {
            lexical_slots.push(slot);
        }
        Some(slot)
    };

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
            Op::LoadLocal(slot) if slot_is_lexical(*slot) => {
                let src = note_lexical(*slot, &mut lexical_slots)?;
                required_authoritative_slots |= 1_u128 << *slot;
                ops.push(WideOp::MoveChecked {
                    dst: register(depth),
                    src,
                });
            }
            Op::ClearLocal(slot) => {
                let slot = note_lexical(*slot, &mut lexical_slots)?;
                required_authoritative_slots |= 1_u128 << slot;
                ops.push(WideOp::ClearLocal { slot });
            }
            Op::AssignLocal(slot) if slot_is_lexical(*slot) => {
                let dst = note_lexical(*slot, &mut lexical_slots)?;
                required_authoritative_slots |= 1_u128 << *slot;
                ops.push(WideOp::AssignChecked {
                    dst,
                    src: register(depth.checked_sub(1)?),
                });
            }
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
            Op::LoadGlobal(name) if is_this_read(name) => {
                requires_this = true;
                ops.push(WideOp::LoadThis {
                    dst: register(depth),
                });
            }
            Op::LoadGlobal(name) => {
                let index = match global_names.iter().position(|known| known == name) {
                    Some(index) => index,
                    None => {
                        global_names.push(name.clone());
                        global_names.len() - 1
                    }
                };
                ops.push(WideOp::LoadGlobal {
                    dst: register(depth),
                    index: u16::try_from(index).ok()?,
                });
            }
            Op::GetPropIndex(encoded) => {
                let (index, local_slot) = crate::bytecode::ir::decode_index_receiver(*encoded);
                let index = u16::try_from(index).ok()?;
                match local_slot {
                    Some(slot) if slot_is_lexical(slot) => {
                        let src = note_lexical(slot, &mut lexical_slots)?;
                        required_authoritative_slots |= 1_u128 << slot;
                        ops.push(WideOp::MoveChecked {
                            dst: register(depth),
                            src,
                        });
                        ops.push(WideOp::GetPropIndex {
                            dst: register(depth),
                            obj: register(depth),
                            index,
                        });
                    }
                    Some(slot) => {
                        let slot_bit = 1_u128 << slot;
                        if upvalue_slots & slot_bit != 0 {
                            ops.push(WideOp::LoadUpvalueLocal {
                                dst: register(depth),
                                slot: u16::try_from(slot).ok()?,
                            });
                            ops.push(WideOp::GetPropIndex {
                                dst: register(depth),
                                obj: register(depth),
                                index,
                            });
                        } else {
                            required_authoritative_slots |= slot_bit;
                            ops.push(WideOp::GetPropIndex {
                                dst: register(depth),
                                obj: u16::try_from(slot).ok()?,
                                index,
                            });
                        }
                    }
                    None => ops.push(WideOp::GetPropIndex {
                        dst: register(depth.checked_sub(1)?),
                        obj: register(depth.checked_sub(1)?),
                        index,
                    }),
                }
            }
            Op::New(argc) => {
                let base = register(depth.checked_sub(u16::try_from(*argc).ok()? + 1)?);
                ops.push(WideOp::New {
                    dst: base,
                    base,
                    argc: u8::try_from(*argc).ok()?,
                });
            }
            Op::NewArray { elements } => {
                let count = u16::try_from(elements.len()).ok()?;
                let base = register(depth.checked_sub(count)?);
                ops.push(WideOp::NewArray {
                    dst: base,
                    base,
                    count,
                });
            }
            Op::GetPropNamed { key, cache } => {
                let index = u16::try_from(named_reads.len()).ok()?;
                named_reads.push(NamedReadSite {
                    key: Rc::clone(key),
                    cache: cache.clone(),
                });
                match cache.local_slot() {
                    Some(slot) if slot_is_lexical(slot) => {
                        let src = note_lexical(slot, &mut lexical_slots)?;
                        required_authoritative_slots |= 1_u128 << slot;
                        ops.push(WideOp::MoveChecked {
                            dst: register(depth),
                            src,
                        });
                        ops.push(WideOp::GetPropNamed {
                            dst: register(depth),
                            obj: register(depth),
                            index,
                        });
                    }
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
            Op::StoreLocal(slot) if slot_is_lexical(*slot) => {
                let dst = note_lexical(*slot, &mut lexical_slots)?;
                required_authoritative_slots |= 1_u128 << *slot;
                ops.push(WideOp::Move {
                    dst,
                    src: register(depth.checked_sub(1)?),
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
        global_names,
        lexical_slots,
        tdz_marker: crate::Value::Function(crate::Function::uninitialized_lexical_marker()),
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
        Op::FunctionPrologueEnd | Op::ClearLocal(_) => simple(0, 0),
        Op::LoadConst(_) | Op::LoadLocal(_) | Op::Dup | Op::LoadGlobal(_) => simple(0, 1),
        Op::GetPropIndex(encoded) => {
            if crate::bytecode::ir::decode_index_receiver(*encoded)
                .1
                .is_some()
            {
                simple(0, 1)
            } else {
                simple(1, 1)
            }
        }
        Op::New(argc) => simple(u16::try_from(*argc).ok()?.checked_add(1)?, 1),
        Op::NewArray { elements } => simple(u16::try_from(elements.len()).ok()?, 1),
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
