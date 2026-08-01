//! The compact register executor.
//!
//! This is deliberately a small, separate, `#[inline(never)]` symbol. The
//! whole point of the tier is that its dispatch loop is short enough for the
//! register allocator to keep `pc` and the register base in machine registers,
//! which `Vm::run_current_activation` demonstrably cannot do. Anything that
//! inflates this function -- an extra opcode family, an inlined slow path --
//! spends the budget the tier exists to protect. Keep cold work behind
//! `#[inline(never)]` helpers.

use super::activation::{CompactActivation, call_from_activation};
use super::{CompactFunctionProgram, CompactOp};
use crate::{RuntimeError, Value};

#[inline(never)]
pub(super) fn execute(
    activation: &mut CompactActivation<'_>,
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
                let Some(value) = activation.bytecode.constants.get(index as usize) else {
                    return Err(constant_out_of_bounds());
                };
                // `Value::clone` stays an out-of-line call; the local-value
                // clone inlines its primitive cases, which is what a constant
                // pool of numbers actually needs.
                registers[dst as usize] = crate::bytecode::vm_bindings::clone_local_value(value);
            }
            CompactOp::Move { dst, src } => {
                registers[dst as usize] =
                    crate::bytecode::vm_bindings::clone_local_value(&registers[src as usize]);
            }
            CompactOp::LoadUpvalueLocal { dst, slot } => {
                let Some(cell) = activation.upvalue_cell(slot as usize) else {
                    return Err(uninitialized_local());
                };
                let value = cell.get();
                registers[dst as usize] = if value.is_uninitialized_lexical_marker() {
                    activation.uninitialized_upvalue(slot as usize)?
                } else {
                    value
                };
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
                registers[dst as usize] = activation.eval_binary(left, op, right)?;
            }
            CompactOp::Drop { src } => {
                registers[src as usize] = Value::Undefined;
            }
            CompactOp::JumpIfFalsy { cond, target } => {
                // The general `is_truthy` consults the `[[IsHTMLDDA]]` slot,
                // which showed up in the profile for a loop condition that is
                // always a number. Numbers answer here; everything else keeps
                // the shared implementation.
                let falsy = match &registers[cond as usize] {
                    Value::Number(number) => *number == 0.0 || number.is_nan(),
                    other => !crate::is_truthy(other),
                };
                if falsy {
                    pc = target as usize;
                }
            }
            CompactOp::Jump { target } => pc = target as usize,
            CompactOp::Call { dst, base, argc } => {
                let base = base as usize;
                let callee = registers[base].clone();
                let value = call_from_activation(
                    activation,
                    callee,
                    &registers[base + 1..base + 1 + argc as usize],
                )?;
                registers[dst as usize] = value;
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

#[cold]
fn constant_out_of_bounds() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "compact function constant index out of bounds".to_owned(),
    }
}

#[cold]
fn uninitialized_local() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "compact function read an uninitialized local slot".to_owned(),
    }
}
