//! Cold operand ownership/coercion/reference work. Its temporaries leave the
//! native stack before the driver invokes callbacks; hot dispatch keeps slices.
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
            WideOp::ToPropertyKey { dst } => {
                let is_key = match &window[dst as usize] {
                    Value::String(_) => true,
                    Value::Number(number) => {
                        crate::bytecode::vm_props::array_index_from_number(*number).is_some()
                    }
                    _ => false,
                };
                if !is_key
                    && let Err(error) = to_property_key(&mut window[dst as usize], activation.env)
                {
                    break 'operation Err(error);
                }
            }
            WideOp::Unary { dst, op, src } => {
                let value = std::mem::replace(&mut window[src as usize], Value::Undefined);
                match eval_unary(op, value, activation.env) {
                    Ok(value) => execute::store(&mut window[dst as usize], value),
                    Err(error) => break 'operation Err(error),
                }
            }
            WideOp::Typeof { dst, src } => {
                let value = std::mem::replace(&mut window[src as usize], Value::Undefined);
                let name = crate::bytecode::util::typeof_value(value);
                execute::store(&mut window[dst as usize], Value::String(name));
            }
            WideOp::ToNumeric { dst } => {
                if !matches!(window[dst as usize], Value::Number(_)) {
                    let value = std::mem::replace(&mut window[dst as usize], Value::Undefined);
                    match to_numeric(value, activation.env) {
                        Ok(value) => execute::store(&mut window[dst as usize], value),
                        Err(error) => break 'operation Err(error),
                    }
                }
            }
            WideOp::Update { dst, op } => {
                if let Value::Number(number) = &mut window[dst as usize] {
                    *number = match op {
                        UpdateOp::Increment => *number + 1.0,
                        UpdateOp::Decrement => *number - 1.0,
                    };
                } else {
                    let value = std::mem::replace(&mut window[dst as usize], Value::Undefined);
                    match update(op, value, activation.env) {
                        Ok(value) => execute::store(&mut window[dst as usize], value),
                        Err(error) => break 'operation Err(error),
                    }
                }
            }
            WideOp::LoadGlobal { dst, index } => {
                let Some(name) = program.global_names.get(index as usize) else {
                    break 'operation Err(execute::constant_out_of_bounds());
                };
                let site = &program.global_reads[index as usize];
                match property::load_global_cached(name, site, activation.env) {
                    Ok(value) => execute::store(&mut window[dst as usize], value),
                    Err(error) => break 'operation Err(error),
                }
            }
            WideOp::NewArray { dst, base, count } => {
                let values: Vec<Value> = (0..count as usize)
                    .map(|offset| {
                        std::mem::replace(&mut window[base as usize + offset], Value::Undefined)
                    })
                    .collect();
                execute::store(
                    &mut window[dst as usize],
                    Value::Array(crate::ArrayRef::new(activation.env.realm(), values)),
                );
            }
            WideOp::ClearLocal { slot } => {
                execute::store(&mut window[slot as usize], program.tdz_marker.clone());
            }
            WideOp::MoveChecked { dst, src } => {
                if window[src as usize].is_uninitialized_lexical_marker() {
                    break 'operation Err(activation.uninitialized_lexical(src as usize));
                }
                let value = crate::bytecode::vm_bindings::clone_local_value(&window[src as usize]);
                execute::store(&mut window[dst as usize], value);
            }
            WideOp::AssignChecked { dst, src } => {
                if window[dst as usize].is_uninitialized_lexical_marker() {
                    break 'operation Err(activation.uninitialized_lexical(dst as usize));
                }
                let value = crate::bytecode::vm_bindings::clone_local_value(&window[src as usize]);
                execute::store(&mut window[dst as usize], value);
            }
            WideOp::ReferenceCheck { event } => {
                let references = &activation.bytecode.binding_plan.references;
                let statically_elided = references.skips_indexed_entry()
                    && activation.env.activation.scope_cursor().is_none();
                if !statically_elided
                    && !references.event_starts_elidable(event as usize, &activation.env.activation)
                    && let Err(error) =
                        references.select_checked_starts(event as usize, activation.env)
                {
                    break 'operation Err(error);
                }
            }
            _ => unreachable!("cold opcode family"),
        }
        Ok(())
    }
}
