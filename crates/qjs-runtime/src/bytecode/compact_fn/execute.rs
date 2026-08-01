//! The compact register executor.
//!
//! This is deliberately a small, separate, `#[inline(never)]` symbol. The
//! whole point of the tier is that its dispatch loop is short enough for the
//! register allocator to keep `pc` and the register base in machine registers,
//! which `Vm::run_current_activation` demonstrably cannot do. Anything that
//! inflates this function -- an extra opcode family, an inlined slow path --
//! spends the budget the tier exists to protect. Keep cold work behind
//! `#[inline(never)]` helpers.

use super::{CompactFunctionProgram, CompactOp};
use crate::bytecode::ir::Bytecode;
use crate::bytecode::util::stack_underflow;
use crate::bytecode::vm::Vm;
use crate::{RuntimeError, Value};

/// Runs `bytecode` on the compact tier, or returns `None` to leave it to the
/// ordinary interpreter.
///
/// `None` means "not admitted", and it is always decided before any observable
/// work. Once this returns `Some`, the body has run to completion or raised.
pub(in crate::bytecode) fn try_run_compact_function(
    vm: &mut Vm<'_>,
    bytecode: &Bytecode,
) -> Option<Result<Value, RuntimeError>> {
    let program = super::program_for(bytecode)?;
    // Every guard here is checked while falling back is still free: nothing
    // below has touched `ip`, the operand stack, or any local.
    //
    // Indexed storage must be the sole binding authority for every local this
    // body touches; a frame whose names can be reached dynamically keeps the
    // ordinary path, which consults the environment.
    if vm.current.direct_eval_with_stack
        || vm.current.authoritative_slots & program.required_authoritative_slots
            != program.required_authoritative_slots
    {
        return None;
    }
    // A fresh ordinary activation, with no state this tier cannot represent.
    // The tier has no unwinding, suspension, or frame-routing protocol, so it
    // must not inherit any of them mid-flight.
    if vm.current.ip != 0
        || !vm.current.stack.is_empty()
        || !vm.callers.is_empty()
        || vm.pending_frame_entry.is_some()
        || !vm.current.try_stack.is_empty()
        || !vm.current.disposable_scopes.is_empty()
        || !vm.current.with_stack.is_empty()
        || vm.current.pending_throw.is_some()
        || vm.current.pending_return.is_some()
        || vm.current.pending_jump.is_some()
        || vm.current.resume_mode.is_some()
        || vm.current.stop_at_prologue
    {
        return None;
    }
    crate::diagnostics::count!(compact_function_entries);
    let mut registers = program.take_registers();
    registers.clear();
    registers.resize(program.register_count, Value::Undefined);
    let result = execute(vm, program, &mut registers);
    program.recycle_registers(registers);
    Some(result)
}

#[inline(never)]
fn execute(
    vm: &mut Vm<'_>,
    program: &CompactFunctionProgram,
    registers: &mut [Value],
) -> Result<Value, RuntimeError> {
    let ops = &program.ops[..];
    let mut pc = 0_usize;
    loop {
        let Some(op) = ops.get(pc) else {
            // Falling off the end is an implicit `return undefined`.
            return Ok(Value::Undefined);
        };
        pc += 1;
        #[cfg(feature = "perf-counters")]
        crate::diagnostics::update(|c| c.compact_function_ops += 1);
        match *op {
            CompactOp::LoadConst { dst, index } => {
                let Some(value) = vm.current.bytecode.constants.get(index as usize) else {
                    return Err(constant_out_of_bounds());
                };
                registers[dst as usize] = value.clone();
            }
            CompactOp::LoadLocal { dst, slot } => {
                match vm.current.locals.get(slot as usize) {
                    Some(Some(value)) if !value.is_uninitialized_lexical_marker() => {
                        registers[dst as usize] =
                            crate::bytecode::vm_bindings::clone_local_value(value);
                    }
                    // An uninitialized or absent slot is a temporal dead zone
                    // read; the general path owns that diagnostic.
                    _ => registers[dst as usize] = load_local_general(vm, slot)?,
                }
            }
            CompactOp::LoadUpvalueLocal { dst, slot } => {
                registers[dst as usize] = load_local_general(vm, slot)?;
            }
            CompactOp::StoreLocal { slot, src } => {
                let Some(target) = vm.current.locals.get_mut(slot as usize) else {
                    return Err(local_out_of_bounds());
                };
                *target = Some(registers[src as usize].clone());
            }
            CompactOp::Binary {
                dst,
                op,
                left,
                right,
            } => {
                if let (Value::Number(left), Value::Number(right)) =
                    (&registers[left as usize], &registers[right as usize])
                    && let Some(value) =
                        crate::bytecode::vm_props::fast_number_binary_numbers(*left, op, *right)
                {
                    registers[dst as usize] = value;
                    continue;
                }
                let left = std::mem::replace(&mut registers[left as usize], Value::Undefined);
                let right = std::mem::replace(&mut registers[right as usize], Value::Undefined);
                registers[dst as usize] = binary_bridge(vm, left, op, right)?;
            }
            CompactOp::Drop { src } => {
                registers[src as usize] = Value::Undefined;
            }
            CompactOp::JumpIfFalsy { cond, target } => {
                if !crate::is_truthy(&registers[cond as usize]) {
                    pc = target as usize;
                }
            }
            CompactOp::Jump { target } => pc = target as usize,
            CompactOp::Call { dst, base, argc } => {
                registers[dst as usize] = call(vm, registers, base, argc)?;
            }
            CompactOp::Return { src } => {
                return Ok(std::mem::replace(
                    &mut registers[src as usize],
                    Value::Undefined,
                ));
            }
        }
    }
}

/// Performs one call.
///
/// A slot-seeded direct-leaf callee -- which is every call in a recursive
/// admitted body -- is dispatched straight from the registers. The entry it
/// reaches already takes an argument slice, so staging the operands on the
/// operand stack only for `Vm::call` to re-derive the same eligibility and pop
/// them back off is pure round trip.
///
/// Every other callee shape (native, bound, Proxy, getter, constructor) still
/// goes through the operand stack into `Vm::call`, so there remains exactly one
/// implementation of general call semantics.
#[inline(never)]
fn call(
    vm: &mut Vm<'_>,
    registers: &mut [Value],
    base: u16,
    argc: u8,
) -> Result<Value, RuntimeError> {
    let base = base as usize;
    let argc = argc as usize;
    if crate::function::is_direct_leaf_function(&registers[base]) {
        let callee = registers[base].clone();
        // An admitted body has no exception handler, so `handle_call_result`
        // would only rewrap an error this frame must propagate anyway.
        return crate::function::call_direct_leaf_function(
            callee,
            Value::Undefined,
            &registers[base + 1..base + 1 + argc],
            &vm.current.env,
            vm.current.module_host.clone(),
            #[cfg(feature = "agents")]
            vm.current.agent_context.clone(),
        );
    }
    let depth = vm.current.stack.len();
    vm.current.stack.push(registers[base].clone());
    for offset in 1..=argc {
        vm.current.stack.push(registers[base + offset].clone());
    }
    vm.call(argc)?;
    // The same no-handler property means the call either produced exactly one
    // value or propagated an error.
    if vm.current.stack.len() != depth + 1 {
        vm.current.stack.truncate(depth);
        return Err(unbalanced_call());
    }
    vm.current.stack.pop().ok_or_else(stack_underflow)
}

/// Evaluates one non-numeric binary operation through the interpreter's own
/// implementation.
///
/// Staging the operands on the operand stack keeps every coercion, string
/// concatenation, `valueOf`/`toString` callback, and error message identical
/// to the ordinary path instead of reimplementing them in this symbol.
#[inline(never)]
fn binary_bridge(
    vm: &mut Vm<'_>,
    left: Value,
    op: qjs_ast::BinaryOp,
    right: Value,
) -> Result<Value, RuntimeError> {
    let depth = vm.current.stack.len();
    vm.current.stack.push(left);
    vm.current.stack.push(right);
    match vm.eval_binary(op) {
        Ok(value) => Ok(value),
        Err(error) => {
            // A user coercion hook may have unwound partway; restore the
            // operand stack this tier promised to leave empty.
            vm.current.stack.truncate(depth);
            Err(error)
        }
    }
}

#[inline(never)]
fn load_local_general(vm: &mut Vm<'_>, slot: u16) -> Result<Value, RuntimeError> {
    vm.load_local(slot as usize)
}

#[cold]
fn constant_out_of_bounds() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "compact function constant index out of bounds".to_owned(),
    }
}

#[cold]
fn local_out_of_bounds() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "compact function local slot out of bounds".to_owned(),
    }
}

#[cold]
fn unbalanced_call() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "compact function call did not produce exactly one value".to_owned(),
    }
}
