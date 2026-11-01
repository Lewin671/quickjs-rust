//! Prepare an admitted inline window without retaining its large move temporaries
//! in the driver frame while native callbacks recurse. No JS executes here.
//! The argument moves run in the driver; the callee activation (a released
//! slot rebound when possible, `activation_stack`) and the frame record are
//! built out of line.
use super::*;

/// Enters an admitted callee (`inline_callee`) as a frame of this driver.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub(super) fn enter_call(
    inline: InlineCallee,
    callee: &mut Value,
    runtime: &mut crate::context::RuntimeServices,
    registers: &mut Vec<Value>,
    frames: &mut Vec<CompactFrame>,
    activations: &mut super::super::activation_stack::ActivationStack,
    current_callee: &mut Value,
    current_owner: &mut Option<Function>,
    current_slots: &mut u128,
    current_base: &mut usize,
    current_len: &mut usize,
    pc: &mut usize,
    argument_index: usize,
    argument_count: usize,
    dst: u16,
    resume_pc: usize,
) -> Result<(), RuntimeError> {
    if frames.len() >= MAX_FRAMES {
        return Err(call_stack_exhausted());
    }
    crate::diagnostics::count!(ordinary_call_attempts);
    crate::diagnostics::count!(compact_direct_calls);
    let callee_base = *current_base + *current_len;
    let callee_len = inline.register_count;
    reserve_registers(registers, callee_base + callee_len);
    let parameter_slots = callee_parameter_slots(callee);
    let (caller_side, callee_side) = registers.split_at_mut(callee_base);
    for index in 0..argument_count {
        let value = std::mem::replace(&mut caller_side[argument_index + index], Value::Undefined);
        match parameter_slots.get(index) {
            Some(&slot) if slot < callee_len => callee_side[slot] = value,
            _ => release(value),
        }
    }
    push_callee_frame(
        callee,
        runtime,
        frames,
        activations,
        current_callee,
        current_owner.take(),
        *current_slots,
        *current_base,
        *current_len,
        resume_pc,
        dst,
    );
    *current_owner = inline.upvalue_owner;
    *current_slots = inline.upvalue_slots;
    *current_base = callee_base;
    *current_len = callee_len;
    *pc = 0;
    Ok(())
}

/// Binds the callee's indexed activation and suspends the caller in a frame
/// record; the caller's activation stays where it is.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn push_callee_frame(
    callee: &mut Value,
    runtime: &mut crate::context::RuntimeServices,
    frames: &mut Vec<CompactFrame>,
    activations: &mut super::super::activation_stack::ActivationStack,
    current_callee: &mut Value,
    caller_owner: Option<Function>,
    caller_slots: u128,
    caller_base: usize,
    caller_len: usize,
    resume_pc: usize,
    dst: u16,
) {
    let Value::Function(function) = &*callee else {
        unreachable!("admitted indexed callee")
    };
    activations.callee_slot(frames.len(), function, runtime);
    frames.push(CompactFrame {
        callee: std::mem::replace(current_callee, std::mem::replace(callee, Value::Undefined)),
        upvalue_owner: caller_owner,
        upvalue_slots: caller_slots,
        base: caller_base,
        len: caller_len,
        resume_pc,
        dst,
    });
}
