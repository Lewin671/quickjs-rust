use crate::{CallEnv, NativeFunction, Value, atomics};

use super::NativeCallResult;

pub(super) fn call_atomics_native(
    native: NativeFunction,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::AtomicsAdd => {
            atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::Add, env)?
        }
        NativeFunction::AtomicsAnd => {
            atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::And, env)?
        }
        NativeFunction::AtomicsCompareExchange => {
            atomics::native_atomics_compare_exchange(argument_values, env)?
        }
        NativeFunction::AtomicsExchange => atomics::native_atomics_read_modify_write(
            argument_values,
            atomics::AtomicOp::Exchange,
            env,
        )?,
        NativeFunction::AtomicsIsLockFree => {
            atomics::native_atomics_is_lock_free(argument_values, env)?
        }
        NativeFunction::AtomicsLoad => atomics::native_atomics_load(argument_values, env)?,
        NativeFunction::AtomicsNotify => atomics::native_atomics_notify(argument_values, env)?,
        NativeFunction::AtomicsOr => {
            atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::Or, env)?
        }
        NativeFunction::AtomicsPause => atomics::native_atomics_pause(argument_values)?,
        NativeFunction::AtomicsStore => atomics::native_atomics_store(argument_values, env)?,
        NativeFunction::AtomicsSub => {
            atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::Sub, env)?
        }
        NativeFunction::AtomicsXor => {
            atomics::native_atomics_read_modify_write(argument_values, atomics::AtomicOp::Xor, env)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
