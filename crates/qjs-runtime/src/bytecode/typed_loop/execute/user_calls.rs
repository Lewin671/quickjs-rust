//! User leaf evaluation has no LoopFrame or Context dependency. All frame
//! kinds share one implementation; native/fallback temporaries stay separate.
use super::*;

#[inline(never)]
pub(super) fn call(
    program: &TypedLoopProgram,
    callee: &Value,
    receiver: &Value,
    args: &[Typed],
) -> Option<Value> {
    // Before the closed-form evaluators, not after. Preparation already
    // refused to flatten any body they can answer, so a graph hit here is a
    // body they would decline -- and reaching it through them first means
    // building an argument array and walking the body twice.
    if let Some(value) = program.helper_graphs.borrow().call(callee, args) {
        return Some(value.to_value());
    }
    if !crate::function::is_direct_leaf_function(callee) {
        return None;
    }
    let Value::Function(function) = callee else {
        return None;
    };
    let bytecode = function.bytecode.as_ref()?;
    // A body that is number-only arithmetic -- a hash's `safe_add` or `rol`,
    // spectral-norm's `A(i, j)` -- is evaluated on the argument numbers
    // directly, without boxing them for the general leaf evaluators.
    if crate::bytecode::vm_numeric_leaf::has_number_only_leaf(bytecode)
        && let Some(value) = call_number_only_leaf(function, bytecode, args)
    {
        return Some(value);
    }
    if let Some(value) = crate::bytecode::vm_numeric_leaf::try_eval_pure_number_shortcut(
        bytecode,
        &function.params,
        |index| match args.get(index)? {
            Typed::Number(value) => Some(*value),
            _ => None,
        },
        &function.upvalues,
    ) {
        crate::diagnostics::count!(ordinary_call_attempts);
        crate::diagnostics::count!(closed_form_leaf_evaluations);
        return Some(value);
    }
    boxed_call(function, bytecode, receiver, args)
}

#[inline(never)]
fn boxed_call(
    function: &crate::Function,
    bytecode: &crate::Bytecode,
    receiver: &Value,
    args: &[Typed],
) -> Option<Value> {
    // Every element comes from a `Typed`, so none owns anything to release;
    // skipping the array's drop keeps it from becoming an out-of-line call.
    let arguments: std::mem::ManuallyDrop<
        [Value; crate::bytecode::typed_loop::helper_graph::MAX_HELPER_ARITY],
    > = std::mem::ManuallyDrop::new(std::array::from_fn(|index| {
        args.get(index)
            .copied()
            .unwrap_or(Typed::Undefined)
            .to_value()
    }));
    let arguments = arguments.get(..args.len())?;
    let value = crate::bytecode::vm_numeric_leaf::try_eval_pure_numeric_leaf(
        bytecode,
        &function.params,
        arguments,
        &function.upvalues,
    )
    .or_else(|| {
        crate::bytecode::vm_this_property_leaf::try_eval_this_property_leaf(
            bytecode, receiver, arguments,
        )
    })?;
    // This operation answers an ordinary call, so it owes the same two counts
    // the interpreter's call path reports. They are taken here rather than
    // before the evaluators so a declined callee, which the interpreter then
    // runs and counts itself, is not counted twice.
    crate::diagnostics::count!(ordinary_call_attempts);
    crate::diagnostics::count!(closed_form_leaf_evaluations);
    Some(value)
}

/// A scalar shortcut reads only the argument registers its operations use.
/// Fallback materializes the ordinary typed slice, away from the hot frame.
#[inline(never)]
pub(super) fn registers(
    program: &TypedLoopProgram,
    callee: &Value,
    receiver: &Value,
    registers: &[Typed],
    arguments: [u16; crate::bytecode::typed_loop::helper_graph::MAX_HELPER_ARITY],
    arity: u8,
) -> Option<Value> {
    if program.helper_sites.is_empty()
        && crate::function::is_direct_leaf_function(callee)
        && let Value::Function(function) = callee
        && let Some(bytecode) = &function.bytecode
        && let Some(value) = crate::bytecode::vm_numeric_leaf::try_eval_pure_number_shortcut(
            bytecode,
            &function.params,
            |index| {
                if index >= usize::from(arity) {
                    return None;
                }
                match registers.get(usize::from(*arguments.get(index)?))? {
                    Typed::Number(value) => Some(*value),
                    _ => None,
                }
            },
            &function.upvalues,
        )
    {
        crate::diagnostics::count!(ordinary_call_attempts);
        crate::diagnostics::count!(closed_form_leaf_evaluations);
        return Some(value);
    }
    register_fallback(program, callee, receiver, registers, arguments, arity)
}
#[inline(never)]
fn register_fallback(
    program: &TypedLoopProgram,
    callee: &Value,
    receiver: &Value,
    registers: &[Typed],
    arguments: [u16; crate::bytecode::typed_loop::helper_graph::MAX_HELPER_ARITY],
    arity: u8,
) -> Option<Value> {
    let values = arguments.map(|register| registers[usize::from(register)]);
    call(program, callee, receiver, values.get(..usize::from(arity))?)
}
