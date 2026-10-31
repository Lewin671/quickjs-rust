//! Inline entry owns callee preparation/move temporaries only until its window
//! is installed. The driver retains none across recursive native re-entry.
use super::*;

#[inline(never)]
#[allow(clippy::too_many_arguments)]
pub(super) fn try_enter_call(
    callee: &mut Value,
    receiver: &mut Option<Value>,
    root_bytecode: &Bytecode,
    env: &mut crate::context::Context<'_>,
    registers: &mut Vec<Value>,
    frames: &mut Vec<WideFrame>,
    current_callee: &mut Value,
    current_this: &mut Option<Value>,
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
    if inline.numeric
        && let Some(value) = try_numeric_call(
            callee,
            &registers[argument_index..argument_index + argument_count],
            &inline.activation,
        )
    {
        crate::diagnostics::count!(ordinary_call_attempts);
        if let Some(receiver) = receiver.take() {
            release(receiver);
        }
        execute::store(&mut registers[*current_base + dst as usize], value);
        *pc = resume_pc;
        return Ok(true);
    }
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
            Some(&slot) if slot < callee_len => {
                release(std::mem::replace(&mut callee_side[slot], value))
            }
            _ => release(value),
        }
    }
    let callee_this = if inline.requires_this {
        Some(match receiver.take() {
            Some(this @ (Value::Array(_) | Value::Function(_))) => this,
            Some(Value::Object(object)) if !crate::symbol::is_symbol_primitive(&object) => {
                Value::Object(object)
            }
            receiver => crate::function::function_call_this(receiver, env, inline.is_strict),
        })
    } else {
        if let Some(receiver) = receiver.take() {
            release(receiver);
        }
        None
    };
    if inline.has_lexical_slots
        && let Some(program) = super::super::program_for(running_bytecode(callee, root_bytecode))
    {
        seed_lexical_markers(
            program,
            &mut registers[callee_base..callee_base + callee_len],
        );
    }
    frames.push(WideFrame {
        activation: std::mem::replace(env.activation, inline.activation),
        callee: std::mem::replace(current_callee, std::mem::replace(callee, Value::Undefined)),
        this_value: std::mem::replace(current_this, callee_this),
        base: *current_base,
        len: *current_len,
        resume_pc,
        dst,
        constructs: false,
    });
    *current_base = callee_base;
    *current_len = callee_len;
    *pc = 0;
    Ok(true)
}
