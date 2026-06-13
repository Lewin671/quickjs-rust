use crate::{Function, NativeFunction, Value, weak_ref};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_weak_ref_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::WeakRef => {
            weak_ref::native_weak_ref(function, argument_values, is_construct, env)?
        }
        NativeFunction::WeakRefPrototypeDeref => {
            weak_ref::native_weak_ref_prototype_deref(this_value)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
