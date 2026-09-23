//! Lowers a stack-machine body into the wide register form.
//!
//! The transformation is the numeric tier's: every operand-stack depth is a
//! fixed register, locals occupy the low registers, and admission is
//! all-or-nothing. What differs is the admitted set -- named property reads
//! and writes, `this`, resolved calls, `Dup`, and backward edges -- and the
//! side tables that hold the named-property sites so the operation word
//! stays eight bytes.

use std::rc::Rc;

use super::{NamedReadSite, NamedWriteSite, ProbedBackedge, WideOp, WideProgram};
use crate::bytecode::compact_fn::MAX_REGISTERS;
use crate::bytecode::compact_fn::compile::MAX_CALL_ARITY;
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

/// Why a body was declined: the instruction (if one is to blame) and a
/// reason. Only diagnostic builds read it (`QJS_CF_TRACE`, see
/// docs/benchmarking.md "Execution counters").
#[derive(Debug, Default)]
#[cfg_attr(not(feature = "perf-counters"), allow(dead_code))]
pub(super) struct Decline {
    pub(super) ip: Option<usize>,
    pub(super) reason: &'static str,
}

fn decline<T>(trace: &mut Decline, ip: Option<usize>, reason: &'static str) -> Option<T> {
    *trace = Decline { ip, reason };
    None
}

pub(super) fn compile(bytecode: &Bytecode) -> Option<WideProgram> {
    compile_traced(bytecode, &mut Decline::default())
}

/// `compile`, recording in `trace` why a declined body was declined. A
/// representation limit hit while lowering an admitted body (an index that
/// does not fit its field) leaves the reason "lowering limit".
pub(super) fn compile_traced(bytecode: &Bytecode, trace: &mut Decline) -> Option<WideProgram> {
    *trace = Decline {
        ip: None,
        reason: "lowering limit",
    };
    // Bodies that suspend, catch, or resolve names dynamically keep the
    // ordinary interpreter: this tier has no completion protocol beyond
    // return-or-propagate.
    if bytecode.contains_direct_eval() || bytecode.contains_with() || bytecode.global_scope {
        return decline(trace, None, "direct eval, with, or global code");
    }
    let code = &bytecode.code;
    if code.is_empty() {
        return decline(trace, None, "empty body");
    }
    let local_count = bytecode.locals.len();
    if local_count >= MAX_REGISTERS {
        return decline(trace, None, "too many locals");
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
            return decline(trace, None, "parameter slot beyond 128");
        }
        initialized_slots |= 1_u128 << slot;
    }
    for &slot in bytecode.hoisted_slots() {
        if slot >= u128::BITS {
            return decline(trace, None, "hoisted slot beyond 128");
        }
        initialized_slots |= 1_u128 << slot;
    }

    // Validate every instruction, including unreachable ones: the opcode set
    // is closed, not the reachability analysis.
    if !matches!(code[0], Op::FunctionPrologueEnd) {
        return decline(trace, None, "no FunctionPrologueEnd at ip 0");
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
        if effect_of(op).is_none() {
            return decline(trace, Some(ip), "operation not in this tier");
        }
        match op {
            Op::FunctionPrologueEnd if ip != 0 => {
                return decline(trace, Some(ip), "FunctionPrologueEnd after ip 0");
            }
            // Backward edges are loops. The operand-stack depth at the target
            // is checked by `propagate_depths`, and a loop grows no frames --
            // a backward jump only moves the program counter -- so a loop
            // whose body is representable costs what any other control flow
            // does.
            Op::Jump(target) | Op::JumpIfFalse(target) | Op::JumpIfTrue(target)
                if *target > code.len() =>
            {
                return decline(trace, Some(ip), "jump target out of range");
            }
            Op::LoadConst(index) if *index >= bytecode.constants.len() => {
                return decline(trace, Some(ip), "constant index out of range");
            }
            // A fused receiver local must be readable exactly like a
            // `LoadLocal`: filled on entry, or an own lexical behind its
            // marker.
            Op::GetPropNamed { cache, .. } => {
                if let Some(slot) = cache.local_slot()
                    && !slot_is_initialized_local(slot)
                    && !slot_is_lexical(slot)
                {
                    return decline(trace, Some(ip), "fused receiver local not initialized");
                }
            }
            Op::GetPropIndex(encoded) => {
                let (index, local_slot) = crate::bytecode::ir::decode_index_receiver(*encoded);
                if u16::try_from(index).is_err() {
                    return decline(trace, Some(ip), "index out of range");
                }
                if let Some(slot) = local_slot
                    && !slot_is_initialized_local(slot)
                    && !slot_is_lexical(slot)
                {
                    return decline(trace, Some(ip), "indexed receiver local not initialized");
                }
            }
            Op::LoadLocal(slot) if !slot_is_initialized_local(*slot) && !slot_is_lexical(*slot) => {
                return decline(
                    trace,
                    Some(ip),
                    "local read may be uninitialized (TDZ or received cell)",
                );
            }
            Op::ClearLocal(slot) if !slot_is_lexical(*slot) => {
                return decline(trace, Some(ip), "clear of a non-lexical slot");
            }
            // A declaration store initializes an own lexical (`const`
            // included) or writes one of the frame's own hoisted bindings.
            Op::StoreLocal(slot) => {
                if *slot >= local_count || *slot >= u128::BITS as usize {
                    return decline(trace, Some(ip), "store slot out of range");
                }
                if upvalue_slots & (1_u128 << *slot) != 0 {
                    return decline(trace, Some(ip), "store to a received upvalue");
                }
                if !slot_is_lexical(*slot)
                    && !bytecode
                        .locals
                        .get(*slot)
                        .is_some_and(|local| local.mutable && slot_is_own_binding(*slot))
                {
                    return decline(trace, Some(ip), "store to a non-own or immutable binding");
                }
            }
            // An assignment expression writes a mutable own binding: a hoisted
            // one like a store, a lexical one after its initialization check.
            // A name resolved from an enclosing scope (a global lexical
            // binding, a writable captured cell) is written through that
            // binding and keeps the interpreter.
            Op::AssignLocal(slot) => {
                if *slot >= local_count || *slot >= u128::BITS as usize {
                    return decline(trace, Some(ip), "assign slot out of range");
                }
                if upvalue_slots & (1_u128 << *slot) != 0 {
                    return decline(trace, Some(ip), "assign to a received upvalue");
                }
                if !bytecode.locals.get(*slot).is_some_and(|local| {
                    local.mutable && (slot_is_own_binding(*slot) || slot_is_lexical(*slot))
                }) {
                    return decline(trace, Some(ip), "assign to a non-own or immutable binding");
                }
            }
            Op::NewArray { elements }
                if !elements.iter().all(|element| {
                    matches!(element, crate::bytecode::ir::ArrayElementKind::Expr)
                }) || u16::try_from(elements.len()).is_err() =>
            {
                return decline(trace, Some(ip), "array literal with holes or spreads");
            }
            Op::New(argc) if *argc > MAX_CALL_ARITY => {
                return decline(trace, Some(ip), "construct arity above the limit");
            }
            // The register window passes any arity; see the numeric tier's
            // `MAX_CALL_ARITY` for why it is bounded at all.
            Op::Call(argc) | Op::CallResolved(argc) if *argc > MAX_CALL_ARITY => {
                return decline(trace, Some(ip), "call arity above the limit");
            }
            _ => {}
        }
    }

    // A loop this tier would run natively is a loop the frame-based loop
    // accelerators can no longer see: they attach to backward edges of the
    // ordinary interpreter. A body whose loops any of those accelerators
    // claims must therefore not run them here, where they run slower than
    // under the accelerators; only loops none of them handles run natively.
    let has_backward_edge = code.iter().enumerate().any(|(ip, op)| {
        matches!(
            op,
            Op::Jump(target) | Op::JumpIfFalse(target) | Op::JumpIfTrue(target) if *target <= ip
        )
    });
    // Such a body still runs here up to the loop: each backward edge exits
    // to the interpreter, whose edge dispatch gives the accelerators the loop
    // exactly as a frame that ran from entry would.
    let lowering = bytecode
        .virtual_object_program
        .get_or_init(|| crate::bytecode::virtual_object::lower(bytecode));
    // A body whose virtual-object lowering keeps a literal in slots instead
    // of allocating it keeps the interpreter, where that lowering runs; an
    // exit could not hand such a literal over. A body lowered only by
    // in-place fusion runs here, but its loops stay with the interpreter's
    // fused instructions, as they did before it was admitted.
    if lowering.virtualizes_values() {
        return decline(
            trace,
            None,
            "virtual-object lowering keeps a literal in slots",
        );
    }
    let exit_backedges =
        has_backward_edge && (lowering.lowers_anything() || body_has_loop_accelerator(bytecode));
    // A fused body's loops belong to the interpreter's fused instructions
    // whether or not an accelerator claims them. Otherwise an unconditional
    // backedge exits only for the accelerators, and is probed: when none
    // claims the loop, the loop stays here instead of running generically.
    let probe_backedges = exit_backedges;
    let mut probed_backedges: Vec<ProbedBackedge> = Vec::new();

    let Some(entry_depth) = propagate_depths(code) else {
        return decline(trace, None, "inconsistent operand-stack depth");
    };
    let has_exit = exit_backedges || code.iter().any(is_exit_safe);
    // Nothing in an admitted body -- a function body -- observes a statement
    // completion value, but the interpreter's loop accelerators type-guard
    // the completion temporaries of the loops an exit hands them; keep those
    // written where an exit can reach them.
    let completion_is_dead = |slot: usize| {
        !has_exit
            && bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.is_completion_temporary())
    };
    let folds = fold_local_binaries(code, &entry_depth, &completion_is_dead, |slot| {
        slot < local_count
            && slot < u128::BITS as usize
            && upvalue_slots & (1_u128 << slot) == 0
            && !slot_is_lexical(slot)
    });
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
    // An exit hands every own local to the interpreter frame, so every
    // parameter must have a register even when only code past the exit reads
    // it: the frame the interpreter builds seeds no argument values.
    let local_registers = if has_exit {
        let parameters = bytecode
            .parameter_slots()
            .iter()
            .map(|slot| slot + 1)
            .max()
            .unwrap_or(0);
        local_registers.max(parameters).min(local_count)
    } else {
        local_registers
    };
    if has_exit && local_registers > u128::BITS as usize {
        return decline(trace, None, "exit with more than 128 locals");
    }
    // The locals an exit hands over: the frame's own bindings. Received
    // cells and global-fallback slots are read through their cells or the
    // realm on both sides and are never copied.
    let own_locals = (0..local_registers)
        .filter(|&slot| {
            bytecode
                .locals
                .get(slot)
                .is_some_and(|local| !local.is_received_upvalue() && !local.sloppy_global_fallback)
        })
        .fold(0_u128, |mask, slot| mask | (1_u128 << slot));
    let register_count = local_registers.checked_add(stack_registers)?;
    if register_count > MAX_REGISTERS {
        return decline(trace, None, "too many registers");
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
        if folds[ip] == LocalFold::Elided {
            if let Op::LoadLocal(slot) | Op::StoreLocal(slot) | Op::AssignLocal(slot) = op {
                required_authoritative_slots |= 1_u128 << *slot;
            }
            continue;
        }
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
            // A store into a dead completion temporary only releases the
            // value.
            Op::StoreLocal(slot) | Op::AssignLocal(slot) if completion_is_dead(*slot) => {
                ops.push(WideOp::Drop {
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
            Op::Update(update_op) if let LocalFold::Update(slot) = folds[ip] => {
                ops.push(WideOp::Update {
                    dst: slot,
                    op: *update_op,
                });
            }
            Op::Binary(binary_op) if let LocalFold::Binary(slot) = folds[ip] => {
                ops.push(WideOp::Binary {
                    dst: slot,
                    op: *binary_op,
                    left: slot,
                    right: register(depth.checked_sub(1)?),
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
            Op::Jump(target) if probe_backedges && *target <= ip => {
                let ip = u32::try_from(ip).ok()?;
                ops.push(WideOp::Exit { ip, depth });
                probed_backedges.push(ProbedBackedge {
                    ip,
                    jump_pc: u32::try_from(ops.len()).ok()?,
                    depth,
                });
                ops.push(WideOp::Jump {
                    target: u32::try_from(*target).ok()?,
                });
            }
            Op::Jump(target) | Op::JumpIfFalse(target) | Op::JumpIfTrue(target)
                if exit_backedges && *target <= ip =>
            {
                ops.push(WideOp::Exit {
                    ip: u32::try_from(ip).ok()?,
                    depth,
                });
            }
            op if is_exit_safe(op) => ops.push(WideOp::Exit {
                ip: u32::try_from(ip).ok()?,
                depth,
            }),
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
            // The guarded form is a plain one-argument method call with a
            // frameless answer for the intrinsic Math functions, which the
            // driver's native fast paths give it here.
            Op::CallResolvedGuardedMathUnary => ops.push(WideOp::CallResolved {
                dst: register(depth.checked_sub(3)?),
                base: register(depth.checked_sub(2)?),
                argc: 1,
            }),
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
            Op::Throw => ops.push(WideOp::Throw {
                src: register(depth.checked_sub(1)?),
            }),
            _ => return decline(trace, Some(ip), "operation not lowered"),
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

    // Lowering skips what only an exit can reach, but the interpreter frame
    // an exit builds runs that code too, and it must be seeded with the same
    // receiver: a `this` read anywhere requires one.
    requires_this |= code
        .iter()
        .any(|op| matches!(op, Op::LoadGlobal(name) if is_this_read(name)));

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
        local_registers: u16::try_from(local_registers).ok()?,
        own_locals,
        has_exits: has_exit,
        activations: std::cell::Cell::new(0),
        exits: std::cell::Cell::new(0),
        exit_heavy: std::cell::Cell::new(false),
        probed_backedges: probed_backedges.into_boxed_slice(),
        native_backedges: std::cell::Cell::new(0),
    })
}

/// How an instruction takes part in `x = x op y` folded onto the local.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocalFold {
    None,
    /// The load of the left operand or the store of the result: emits
    /// nothing, since the binary operation reads and writes the local.
    Elided,
    /// A binary operation whose left operand and destination are this local.
    Binary(u16),
    /// An increment or decrement applied to this local in place.
    Update(u16),
}

/// Finds the statement shapes whose value only moves between a local and
/// the operand stack, and folds each onto the local:
///
/// - `x = x op y` and `x op= y`: `LoadLocal(x) <expr> Binary` followed by
///   `StoreLocal(x)` (or `AssignLocal(x)`), or by `Dup Store(x) Pop` where
///   the value is discarded, with `<expr>` straight-line code that neither
///   writes `x` nor exits. The binary operation reads its left operand from
///   the local when it executes, which is the value the load would have
///   copied, and writes the result back: the local is the only reference to
///   its value, so a string append reuses the buffer instead of copying the
///   accumulator every time.
/// - a discarded `x++` or `x--`: `LoadLocal(x) ToNumeric Dup Update
///   Store(x) Pop` becomes one update of the local, which applies the same
///   single numeric conversion.
/// - `LoadLocal(c) Store(d)` between two dead completion temporaries: a
///   statement value no function body can observe.
///
/// No jump may land inside a folded shape.
fn fold_local_binaries(
    code: &[Op],
    entry_depth: &[Option<u16>],
    completion: &impl Fn(usize) -> bool,
    plain_local: impl Fn(usize) -> bool,
) -> Vec<LocalFold> {
    let mut folds = vec![LocalFold::None; code.len()];
    let mut jump_target = vec![false; code.len() + 1];
    for op in code {
        if let Op::Jump(target) | Op::JumpIfFalse(target) | Op::JumpIfTrue(target) = op
            && let Some(flag) = jump_target.get_mut(*target)
        {
            *flag = true;
        }
    }
    let store_to = |ip: usize| match code.get(ip) {
        Some(Op::StoreLocal(slot) | Op::AssignLocal(slot)) if !completion(*slot) => Some(*slot),
        _ => None,
    };
    // A store into a completion temporary discards the value as `Pop` does.
    let discards = |ip: usize| match code.get(ip) {
        Some(Op::Pop) => true,
        Some(Op::StoreLocal(slot) | Op::AssignLocal(slot)) => completion(*slot),
        _ => false,
    };
    let free = |range: std::ops::RangeInclusive<usize>, folds: &[LocalFold]| {
        range
            .clone()
            .all(|ip| !jump_target[ip] && folds.get(ip) == Some(&LocalFold::None))
    };
    for (ip, op) in code.iter().enumerate() {
        match op {
            // Postfix `LoadLocal ToNumeric Dup Update Store Pop` and prefix
            // `LoadLocal ToNumeric Update Dup Store Pop`.
            Op::Update(_) if ip >= 2 => {
                let postfix = matches!(code[ip - 1], Op::Dup);
                let (load, store) = if postfix {
                    (ip.checked_sub(3), ip + 1)
                } else {
                    (Some(ip - 2), ip + 2)
                };
                let Some(load) = load else {
                    continue;
                };
                let Some(slot) = store_to(store) else {
                    continue;
                };
                let shape = if postfix {
                    matches!(code[ip - 2], Op::ToNumeric)
                } else {
                    matches!(code[ip - 1], Op::ToNumeric) && matches!(code[ip + 1], Op::Dup)
                };
                if shape
                    && matches!(code[load], Op::LoadLocal(loaded) if loaded == slot)
                    && discards(store + 1)
                    && plain_local(slot)
                    && let Ok(slot_u16) = u16::try_from(slot)
                    && free(load + 1..=store + 1, &folds)
                    && folds[load] == LocalFold::None
                {
                    folds[load..=store + 1].fill(LocalFold::Elided);
                    folds[ip] = LocalFold::Update(slot_u16);
                }
            }
            Op::LoadLocal(from)
                if completion(*from) && discards(ip + 1) && free(ip..=ip + 1, &folds) =>
            {
                folds[ip] = LocalFold::Elided;
                folds[ip + 1] = LocalFold::Elided;
            }
            Op::Binary(_) => fold_binary(
                code,
                entry_depth,
                &plain_local,
                &store_to,
                &discards,
                &jump_target,
                ip,
                &mut folds,
            ),
            _ => {}
        }
    }
    folds
}

/// The `x = x op y` fold of [`fold_local_binaries`] for the binary operation
/// at `binary`.
#[allow(clippy::too_many_arguments)]
fn fold_binary(
    code: &[Op],
    entry_depth: &[Option<u16>],
    plain_local: &impl Fn(usize) -> bool,
    store_to: &impl Fn(usize) -> Option<usize>,
    discards: &impl Fn(usize) -> bool,
    jump_target: &[bool],
    binary: usize,
    folds: &mut [LocalFold],
) {
    let (slot, tail) = if let Some(slot) = store_to(binary + 1) {
        (slot, binary + 1)
    } else if matches!(code.get(binary + 1), Some(Op::Dup))
        && let Some(slot) = store_to(binary + 2)
        && discards(binary + 3)
    {
        (slot, binary + 3)
    } else {
        return;
    };
    let Ok(slot_u16) = u16::try_from(slot) else {
        return;
    };
    let Some(depth) = entry_depth[binary].and_then(|depth| depth.checked_sub(2)) else {
        return;
    };
    if !plain_local(slot)
        || (binary..=tail).any(|ip| jump_target[ip] || folds[ip] != LocalFold::None)
    {
        return;
    }
    let mut load = None;
    for ip in (0..binary).rev() {
        let Some(entry) = entry_depth[ip] else {
            break;
        };
        if entry == depth {
            load = Some(ip);
            break;
        }
        let writes_slot = matches!(
            code[ip],
            Op::StoreLocal(target) | Op::AssignLocal(target) | Op::ClearLocal(target)
                if target == slot
        );
        // Everything after the load is the right operand: it must leave the
        // loaded value alone, never popping down to it.
        let consumes_left =
            effect_of(&code[ip]).is_none_or(|effect| entry.saturating_sub(effect.pops) <= depth);
        if consumes_left
            || jump_target[ip]
            || writes_slot
            || is_exit_safe(&code[ip])
            || matches!(
                code[ip],
                Op::Jump(_) | Op::JumpIfFalse(_) | Op::JumpIfTrue(_)
            )
        {
            break;
        }
    }
    let Some(load) = load else {
        return;
    };
    if !matches!(code[load], Op::LoadLocal(loaded) if loaded == slot)
        || folds[load] != LocalFold::None
    {
        return;
    }
    folds[load] = LocalFold::Elided;
    folds[binary] = LocalFold::Binary(slot_u16);
    folds[binary + 1..=tail].fill(LocalFold::Elided);
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

/// Operations this tier does not run but may leave to the interpreter
/// mid-body: at one of them the activation exits, handing its locals and
/// operand stack to an interpreter frame that continues from that
/// instruction (`vm::resume_direct_call_bytecode`). Each depends only on the
/// frame's locals, stack and environment -- nothing that must have been set
/// up at entry, such as an `arguments` object, a closure over this frame's
/// locals, a handler, or a per-iteration scope.
fn is_exit_safe(op: &Op) -> bool {
    matches!(
        op,
        Op::SetProp { .. }
            | Op::SetPropIndex { .. }
            | Op::RequireObjectCoercible
            | Op::NewObjectDataLiteral { .. }
            | Op::AppendStringLiteralLocal { .. }
    )
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
        Op::CallResolvedGuardedMathUnary => simple(3, 1),
        Op::Return | Op::Throw => Effect {
            pops: 1,
            pushes: 0,
            target: None,
            falls_through: false,
        },
        // An exit ends this tier's view of the path: whatever follows runs
        // in the interpreter.
        op if is_exit_safe(op) => Effect {
            pops: 0,
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
