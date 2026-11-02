//! Operand temporaries live outside the debug native driver. Release inlining
//! retains its hot scalar/IC paths and introduces no extra opcode dispatch.
use super::*;
#[cfg_attr(debug_assertions, inline(never))]
#[cfg_attr(not(debug_assertions), inline(always))]
pub(super) fn execute(
    op: &WideOp,
    program: &WideProgram,
    activation: &mut WideActivation<'_, '_>,
    window: &mut [Value],
) -> Result<(), RuntimeError> {
    'operation: {
        match *op {
            WideOp::LoadConst { dst, index } => {
                let Some(value) = activation.bytecode.constants.get(index as usize) else {
                    break 'operation Err(execute::constant_out_of_bounds());
                };
                execute::store(
                    &mut window[dst as usize],
                    crate::bytecode::vm_bindings::clone_local_value(value),
                );
            }
            WideOp::Move { dst, src } | WideOp::Dup { src, dst } => {
                let value = crate::bytecode::vm_bindings::clone_local_value(&window[src as usize]);
                execute::store(&mut window[dst as usize], value);
            }
            WideOp::LoadUpvalueLocal { dst, slot } => {
                let Some(cell) = activation.upvalue_cell(slot as usize) else {
                    break 'operation Err(execute::uninitialized_local());
                };
                // One state borrow; a dead-zone cell takes the
                // interpreter's diagnostic.
                let value = match cell.value_if_initialized() {
                    Some(value) => value,
                    None => match activation.uninitialized_upvalue(slot as usize) {
                        Ok(value) => value,
                        Err(error) => break 'operation Err(error),
                    },
                };
                execute::store(&mut window[dst as usize], value);
            }
            WideOp::StoreUpvalueLocal { slot, src } => {
                let Some(cell) = activation.upvalue_cell(slot as usize) else {
                    break 'operation Err(execute::uninitialized_local());
                };
                let value = std::mem::replace(&mut window[src as usize], Value::Undefined);
                if let Err(error) = cell.assign(value, activation.strict) {
                    break 'operation Err(error);
                }
            }
            WideOp::LoadThis { dst } => {
                // Admission required a seeded receiver, so this never
                // fabricates one.
                let Some(value) = activation.env.activation.initialized_receiver() else {
                    break 'operation Err(execute::uninitialized_local());
                };
                execute::store(&mut window[dst as usize], value.clone());
            }
            WideOp::GetPropThis { dst, index } => {
                let (Some(site), Some(receiver)) = (
                    program.named_reads.get(index as usize),
                    activation.env.activation.initialized_receiver(),
                ) else {
                    break 'operation Err(execute::uninitialized_local());
                };
                if let Some(value) = property::get_prop_named_hot(receiver, &site.key, &site.cache)
                {
                    execute::store(&mut window[dst as usize], value);
                    return Ok(());
                }
                let receiver = receiver.clone();
                match property::get_prop_named(&receiver, &site.key, &site.cache, activation.env) {
                    Ok(value) => execute::store(&mut window[dst as usize], value),
                    Err(error) => break 'operation Err(error),
                }
            }
            WideOp::GetPropNamed { dst, obj, index } => {
                let Some(site) = program.named_reads.get(index as usize) else {
                    break 'operation Err(execute::constant_out_of_bounds());
                };
                // The receiver is borrowed: a fused site peeks its
                // local, and the plain form's result replaces it.
                if let Some(value) =
                    property::get_prop_named_hot(&window[obj as usize], &site.key, &site.cache)
                {
                    execute::store(&mut window[dst as usize], value);
                    return Ok(());
                }
                match property::get_prop_named(
                    &window[obj as usize],
                    &site.key,
                    &site.cache,
                    activation.env,
                ) {
                    Ok(value) => execute::store(&mut window[dst as usize], value),
                    Err(error) => break 'operation Err(error),
                }
            }
            WideOp::SetPropNamed { obj, value, index } => {
                let Some(site) = program.named_writes.get(index as usize) else {
                    break 'operation Err(execute::constant_out_of_bounds());
                };
                let assigned = std::mem::replace(&mut window[value as usize], Value::Undefined);
                if property::set_prop_named_hot(
                    &window[obj as usize],
                    site.cache.as_ref(),
                    &assigned,
                    activation.env,
                ) {
                    execute::store(&mut window[obj as usize], assigned);
                    return Ok(());
                }
                match property::set_prop_named(
                    &window[obj as usize],
                    &site.key,
                    site.cache.as_ref(),
                    site.is_strict,
                    assigned,
                    activation.env,
                    Some(&site.creation),
                ) {
                    // The assigned value stays in the object's
                    // register, matching `SetPropNamed`'s stack effect.
                    Ok(value) => execute::store(&mut window[obj as usize], value),
                    Err(error) => break 'operation Err(error),
                }
            }
            WideOp::SetPropThis { dst, value, index } => {
                let (Some(site), Some(receiver)) = (
                    program.named_writes.get(index as usize),
                    activation.env.activation.initialized_receiver(),
                ) else {
                    break 'operation Err(execute::uninitialized_local());
                };
                let assigned = std::mem::replace(&mut window[value as usize], Value::Undefined);
                if property::set_prop_named_hot(
                    receiver,
                    site.cache.as_ref(),
                    &assigned,
                    activation.env,
                ) {
                    execute::store(&mut window[dst as usize], assigned);
                    return Ok(());
                }
                let receiver = receiver.clone();
                match property::set_prop_named(
                    &receiver,
                    &site.key,
                    site.cache.as_ref(),
                    site.is_strict,
                    assigned,
                    activation.env,
                    Some(&site.creation),
                ) {
                    Ok(value) => execute::store(&mut window[dst as usize], value),
                    Err(error) => break 'operation Err(error),
                }
            }
            WideOp::Binary {
                dst,
                op,
                left,
                right,
            } => {
                if let (Value::Number(left), Value::Number(right)) =
                    (&window[left as usize], &window[right as usize])
                    && let Some(value) =
                        crate::bytecode::vm_props::fast_number_binary_numbers(*left, op, *right)
                {
                    execute::store(&mut window[dst as usize], value);
                    return Ok(());
                }
                let locals = program.local_registers;
                let left = take_operand(window, left, dst, locals);
                let right = take_operand(window, right, dst, locals);
                match activation.eval_binary(left, op, right) {
                    Ok(value) => execute::store(&mut window[dst as usize], value),
                    Err(error) => break 'operation Err(error),
                }
            }
            WideOp::GetProp { dst, obj, key } => {
                // `v[i]` on a dense array: answered in place, the
                // number key left in its register.
                if let (Value::Array(elements), Value::Number(number)) =
                    (&window[obj as usize], &window[key as usize])
                    && let Some(index) = crate::bytecode::vm_props::array_index_from_number(*number)
                    && let Some(value) = elements.plain_dense_index_value(index)
                {
                    execute::store(&mut window[dst as usize], value);
                    return Ok(());
                }
                if let Some(value) =
                    property::get_prop_string_own(&window[obj as usize], &window[key as usize])
                {
                    execute::store(&mut window[dst as usize], value);
                    return Ok(());
                }
                let key = take_operand(window, key, dst, program.local_registers);
                // A forwarded local receiver is read, not consumed.
                let result = if obj != dst && obj < program.local_registers {
                    property::get_prop_element_of(&window[obj as usize], key, activation.env)
                } else {
                    property::get_prop_element(&mut window[obj as usize], key, activation.env)
                };
                match result {
                    Ok(value) => execute::store(&mut window[dst as usize], value),
                    Err(error) => break 'operation Err(error),
                }
            }
            WideOp::GetPropIndex { dst, obj, index } => {
                if let Value::Array(elements) = &window[obj as usize]
                    && let Some(value) = elements.plain_dense_index_value(usize::from(index))
                {
                    execute::store(&mut window[dst as usize], value);
                    return Ok(());
                }
                match property::get_prop_index(&window[obj as usize], index, activation.env) {
                    Ok(value) => execute::store(&mut window[dst as usize], value),
                    Err(error) => break 'operation Err(error),
                }
            }
            _ => unreachable!("operand family"),
        }
        Ok(())
    }
}
