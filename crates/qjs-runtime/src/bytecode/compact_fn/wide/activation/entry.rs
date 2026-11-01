//! Inline entry owns callee preparation/move temporaries only until its window
//! is installed. The driver retains none across recursive native re-entry.
//! Returned ordinary activations are released and rebound to later calls of
//! the same driver run instead of rebuilt (`compact_fn::activation_stack`).
use super::*;

/// The driver's inline-call fast path for an admitted callee
/// (`inline_callee`): a numeric answer and the argument moves into the
/// callee window run in the driver; the activation and frame record are
/// built out of line (`push_callee_frame`) so the driver's own stack frame
/// stays small across recursive native re-entry.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub(super) fn enter_call(
    inline: InlineCallee,
    callee: &mut Value,
    receiver: &mut Option<Value>,
    root_bytecode: &Bytecode,
    runtime: &mut crate::context::RuntimeServices,
    registers: &mut Vec<Value>,
    frames: &mut Vec<WideFrame>,
    activations: &mut crate::bytecode::compact_fn::activation_stack::ActivationStack,
    current_callee: &mut Value,
    current_base: &mut usize,
    current_len: &mut usize,
    pc: &mut usize,
    argument_index: usize,
    argument_count: usize,
    dst: u16,
    resume_pc: usize,
) -> Result<(), RuntimeError> {
    // A numeric answer needs no activation at all.
    if inline.numeric
        && let Some(value) = try_numeric_call(
            callee,
            &registers[argument_index..argument_index + argument_count],
        )
    {
        crate::diagnostics::count!(ordinary_call_attempts);
        if let Some(receiver) = receiver.take() {
            release(receiver);
        }
        execute::store(&mut registers[*current_base + dst as usize], value);
        *pc = resume_pc;
        return Ok(());
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
    if inline.has_lexical_slots
        && let Some(program) = super::super::program_for(running_bytecode(callee, root_bytecode))
    {
        seed_lexical_markers(
            program,
            &mut registers[callee_base..callee_base + callee_len],
        );
    }
    push_callee_frame(
        callee,
        receiver.take(),
        inline.requires_this.then_some(inline.is_strict),
        runtime,
        frames,
        activations,
        current_callee,
        *current_base,
        *current_len,
        resume_pc,
        dst,
    );
    *current_base = callee_base;
    *current_len = callee_len;
    *pc = 0;
    Ok(())
}

/// Binds the callee's activation -- its receiver coerced as the general call
/// path does when the body reads `this` (`this_strict`) -- and suspends the
/// caller in a frame record; the caller's activation stays where it is.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn push_callee_frame(
    callee: &mut Value,
    receiver: Option<Value>,
    this_strict: Option<bool>,
    runtime: &mut crate::context::RuntimeServices,
    frames: &mut Vec<WideFrame>,
    activations: &mut crate::bytecode::compact_fn::activation_stack::ActivationStack,
    current_callee: &mut Value,
    caller_base: usize,
    caller_len: usize,
    resume_pc: usize,
    dst: u16,
) {
    let Value::Function(function) = &*callee else {
        unreachable!("admitted indexed callee")
    };
    let depth = frames.len();
    let activation = activations.callee_slot(depth, function, runtime);
    let this = match (this_strict, receiver) {
        (None, receiver) => {
            if let Some(receiver) = receiver {
                release(receiver);
            }
            Value::Undefined
        }
        (Some(_), Some(this @ (Value::Array(_) | Value::Function(_)))) => this,
        (Some(_), Some(Value::Object(object))) if !crate::symbol::is_symbol_primitive(&object) => {
            Value::Object(object)
        }
        (Some(strict), receiver) => crate::function::function_call_this(
            receiver,
            &crate::context::Context {
                runtime,
                activation: &mut *activation,
                native: None,
            },
            strict,
        ),
    };
    crate::function::bind_indexed_call_owner(function, activation, this, None);
    frames.push(WideFrame {
        callee: std::mem::replace(current_callee, std::mem::replace(callee, Value::Undefined)),
        base: caller_base,
        len: caller_len,
        resume_pc,
        dst,
        constructs: false,
    });
}
