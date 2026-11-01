//! Continuing, in the interpreter, an activation the wide compact tier began,
//! and handing it back at a loop no accelerator claims.
//!
//! The tier exits at the backward edge of a loop a loop accelerator may
//! claim, because the accelerators attach to the interpreter's edges. When
//! none claims the loop -- at the exit itself, or after one entered and
//! deoptimized -- the interpreter would run the rest of the loop generically,
//! which is slower than the tier. Such a frame stops at the next probed
//! backedge instead and returns its locals and stack, and the tier keeps the
//! loop from then on.

use super::Vm;
use crate::bytecode::frame_stack::FrameExit;
use crate::bytecode::ir::Bytecode;
use crate::bytecode::vm_loop_dispatch::LoopPlanView;
use crate::bytecode::vm_result::Completion;
use crate::bytecode::{DirectCallSlots, compact_fn};
use crate::{RuntimeError, Value};

/// How an interpreter continuation of a wide activation ended.
pub(in crate::bytecode) enum Resumed {
    /// The frame ran to completion.
    Finished(Result<Value, RuntimeError>),
    /// The frame stopped at the probed backward jump at `backedge`, which no
    /// accelerator claimed, before performing it. The locals and the
    /// operand stack are back in the tier's registers.
    HandedBack { backedge: usize },
    /// An accelerator ran the loop at the probed backedge to its end; the
    /// frame stopped at instruction `ip` after it, with the locals and the
    /// operand stack, `depth` values deep, back in the tier's registers.
    LoopFinished { ip: usize, depth: usize },
}

/// The tier's registers an interpreter continuation takes over.
pub(in crate::bytecode) struct WideRegisters<'r> {
    /// `0..locals.len()` of the register file.
    pub(in crate::bytecode) locals: &'r mut [Value],
    /// The frame's own bindings among `locals`, one bit per slot.
    pub(in crate::bytecode) own_locals: u128,
    /// The operand-stack registers that follow the locals.
    pub(in crate::bytecode) stack: &'r mut [Value],
    /// The operand-stack depth at the exit.
    pub(in crate::bytecode) depth: usize,
    /// The value an uninitialized lexical register holds.
    pub(in crate::bytecode) tdz_marker: &'r Value,
}

/// Continues, in the interpreter, a slot-seeded direct call the wide compact
/// tier began: the tier reached an instruction it leaves to the interpreter
/// (an operation outside its set, or the backward edge of a loop a loop
/// accelerator claims) and hands over its locals and operand stack.
///
/// The frame is built exactly as `eval_direct_call_bytecode` builds it for
/// the same call, then brought to the state the tier had reached: what the
/// `FunctionPrologueEnd` at instruction 0 does, the tier's own locals (its
/// temporal-dead-zone marker becoming an uninitialized slot), and the stack.
/// Admitted bodies run no lowered program, so `ip` indexes the code this
/// frame executes.
///
/// Where a continuation starts, beyond its instruction.
pub(in crate::bytecode) enum ResumeFrom {
    /// An instruction the tier leaves to the interpreter.
    Exit,
    /// A probed backward `Jump` to this loop header (see below).
    ProbedBackedge(usize),
    /// Inside a loop whose typed program the tier ran and which
    /// deoptimized: the frame starts with those programs declined, as an
    /// interpreter frame that ran them itself would, and may hand the loop
    /// back at the next probed backedge whose accelerators decline.
    LoopDeoptimized { declined_typed_loop_programs: u128 },
}

/// With `ResumeFrom::ProbedBackedge`, `ip` is a probed backward `Jump` to
/// that target.
/// If no accelerator claims the loop there, no instruction runs and the
/// frame hands back at once; otherwise the frame may hand back at a later
/// probed backedge whose accelerators decline.
pub(in crate::bytecode) fn resume_direct_call_bytecode(
    bytecode: &Bytecode,
    env: crate::context::ActivationContext,
    runtime: &mut crate::context::RuntimeServices,
    direct_call_slots: DirectCallSlots<'_>,
    ip: usize,
    registers: WideRegisters<'_>,
    from: ResumeFrom,
) -> Resumed {
    let mut vm = Vm::new_with_globals_upvalues_with_stack_and_direct_call_slots(
        bytecode,
        runtime,
        env,
        Vec::new(),
        Vec::new(),
        Some(direct_call_slots),
    );
    vm.enter_function_body();
    for (slot, register) in registers.locals.iter_mut().enumerate() {
        if !is_own(registers.own_locals, slot) {
            continue;
        }
        let value = std::mem::replace(register, Value::Undefined);
        if let Some(cell) = vm.local_upvalue_cell(slot) {
            if value.is_uninitialized_lexical_marker() {
                let replacement = cell.fresh_uninitialized();
                vm.local_upvalues[slot] = Some(replacement);
            } else {
                let result = if cell.is_initialized() {
                    cell.assign(value, vm.execution_is_strict())
                } else {
                    cell.initialize(value)
                };
                if let Err(error) = result {
                    return Resumed::Finished(Err(error));
                }
            }
            vm.locals[slot] = None;
        } else if let Some(target) = vm.current.locals.get_mut(slot) {
            *target = (!value.is_uninitialized_lexical_marker()).then_some(value);
        }
    }
    for register in registers.stack.iter_mut().take(registers.depth) {
        vm.current
            .stack
            .push(std::mem::replace(register, Value::Undefined));
    }
    vm.current.ip = ip;
    // This is an existing scope instance, not an entry from the prologue.
    // Force exposure after restoring its initialized/TDZ registers, and seed
    // the cursor at the continuation so tracking cannot clear live loop cells
    // while mistaking the first resumed instruction for fresh scope entry.
    if let Some(scope) = bytecode.binding_plan.pc_scopes.get(ip).copied() {
        if vm.current.binding_scope.is_some() {
            vm.current.binding_scope = Some(scope);
        }
        vm.force_binding_exposure(scope);
    }
    let resumed = match from {
        ResumeFrom::ProbedBackedge(target) => {
            vm.current.ip = ip + 1;
            if vm.jump_with_loop_plans(LoopPlanView::for_bytecode(bytecode), target, ip) {
                // A loop the accelerator finished leaves the frame past its
                // backedge; the rest of the body runs on the tier again.
                if vm.current.ip > ip
                    && compact_fn::resumes_at(bytecode, vm.current.ip, vm.current.stack.len())
                {
                    Resumed::LoopFinished {
                        ip: vm.current.ip,
                        depth: vm.current.stack.len(),
                    }
                } else {
                    vm.current.cold_mut().wide_handback = Some(None);
                    run(&mut vm)
                }
            } else {
                vm.current.ip = ip;
                Resumed::HandedBack { backedge: ip }
            }
        }
        ResumeFrom::LoopDeoptimized {
            declined_typed_loop_programs,
        } => {
            vm.current.declined_typed_loop_programs = declined_typed_loop_programs;
            vm.current.cold_mut().wide_handback = Some(None);
            run(&mut vm)
        }
        ResumeFrom::Exit => Resumed::Finished(vm.run()),
    };
    if let Resumed::HandedBack { .. } | Resumed::LoopFinished { .. } = resumed {
        hand_back(&mut vm, registers);
    }
    vm.recycle_binding_slots();
    if let Some(cold) = vm.current.cold.take() {
        bytecode.recycle_cold_frame(cold);
    }
    resumed
}

fn run(vm: &mut Vm<'_, '_>) -> Resumed {
    match vm.run_completion() {
        Ok(Completion::Return(value)) => Resumed::Finished(Ok(value)),
        Ok(_) => match vm
            .current
            .cold()
            .and_then(|cold| cold.wide_handback.flatten())
        {
            Some(backedge) => Resumed::HandedBack { backedge },
            None => Resumed::Finished(Err(RuntimeError {
                thrown: None,
                message: "a resumed direct call suspended".to_owned(),
            })),
        },
        Err(error) => Resumed::Finished(Err(error)),
    }
}

fn hand_back(vm: &mut Vm<'_, '_>, registers: WideRegisters<'_>) {
    for (slot, register) in registers.locals.iter_mut().enumerate() {
        if !is_own(registers.own_locals, slot) {
            continue;
        }
        *register = if let Some(cell) = vm.local_upvalue_cell(slot) {
            cell.value_if_initialized()
                .unwrap_or_else(|| registers.tdz_marker.clone())
        } else {
            vm.current
                .locals
                .get_mut(slot)
                .and_then(Option::take)
                .unwrap_or_else(|| registers.tdz_marker.clone())
        };
    }
    let depth = vm.current.stack.len();
    for register in registers.stack[..depth].iter_mut().rev() {
        *register = vm.current.stack.pop().unwrap_or(Value::Undefined);
    }
}

fn is_own(own_locals: u128, slot: usize) -> bool {
    slot < u128::BITS as usize && own_locals & (1_u128 << slot) != 0
}

impl Vm<'_, '_> {
    /// After an ordinary backward jump from `backedge` that no accelerator
    /// claimed: whether this frame hands the loop back to the wide tier.
    /// It does when it was resumed at a probed backedge, is the only frame on
    /// this VM, and stands at another probed backedge with the operand stack
    /// the tier expects there. The jump is undone so the tier performs it.
    #[cold]
    pub(super) fn hand_back_to_wide(&mut self, backedge: usize) -> Option<FrameExit> {
        let Some(Some(None)) = self.current.cold().map(|cold| cold.wide_handback) else {
            return None;
        };
        if !self.callers.is_empty()
            || !compact_fn::hands_back_at(
                &self.current.bytecode,
                backedge,
                self.current.stack.len(),
            )
        {
            return None;
        }
        self.current.ip = backedge;
        self.current.cold_mut().wide_handback = Some(Some(backedge));
        Some(FrameExit::Completed(Completion::Yield(Value::Undefined)))
    }
}
