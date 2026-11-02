//! Operand temporaries live outside the debug native driver. Release inlining
//! retains its hot scalar/IC paths and introduces no extra opcode dispatch.
use super::*;
#[cfg_attr(debug_assertions, inline(never))]
#[cfg_attr(not(debug_assertions), inline(always))]
pub(super) fn execute(
    op: &CompactOp,
    _program: &super::super::CompactFunctionProgram,
    activation: &mut CompactActivation<'_, '_>,
    window: &mut [Value],
) -> Result<(), RuntimeError> {
    'operation: {
        match *op {
            CompactOp::LoadConst { dst, index } => {
                let Some(value) = activation.bytecode.constants.get(index as usize) else {
                    break 'operation Err(execute::constant_out_of_bounds());
                };
                // `Value::clone` stays an out-of-line call; the
                // local-value clone inlines its primitive cases, which
                // is what a constant pool of numbers actually needs.
                execute::store(
                    &mut window[dst as usize],
                    crate::bytecode::vm_bindings::clone_local_value(value),
                );
            }
            CompactOp::Move { dst, src } => {
                let value = crate::bytecode::vm_bindings::clone_local_value(&window[src as usize]);
                execute::store(&mut window[dst as usize], value);
            }
            CompactOp::Binary {
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
                if let Err(error) = binary_slow(activation, window, dst, op, left, right) {
                    break 'operation Err(error);
                }
            }
            CompactOp::Drop { src } => {
                execute::store(&mut window[src as usize], Value::Undefined);
            }
            _ => unreachable!("operand family"),
        }
        Ok(())
    }
}
