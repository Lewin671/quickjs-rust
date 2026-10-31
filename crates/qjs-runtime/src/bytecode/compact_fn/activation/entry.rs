//! Prepare an admitted inline window without retaining its large move temporaries
//! in the driver frame while native callbacks recurse. No JS executes here.
use super::*;

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub(super) fn try_enter_call(
    callee: &mut Value,
    env: &mut crate::context::Context<'_>,
    registers: &mut Vec<Value>,
    frames: &mut Vec<CompactFrame>,
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
) -> Result<bool, RuntimeError> {
    let Some(inline) = inline_callee(callee) else {
        return Ok(false);
    };
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
    frames.push(CompactFrame {
        activation: std::mem::replace(env.activation, inline.activation),
        callee: std::mem::replace(current_callee, std::mem::replace(callee, Value::Undefined)),
        upvalue_owner: current_owner.take(),
        upvalue_slots: *current_slots,
        base: *current_base,
        len: *current_len,
        resume_pc,
        dst,
    });
    *current_owner = inline.upvalue_owner;
    *current_slots = inline.upvalue_slots;
    *current_base = callee_base;
    *current_len = callee_len;
    *pc = 0;
    Ok(true)
}
