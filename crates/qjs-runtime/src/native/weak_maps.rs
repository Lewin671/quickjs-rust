use crate::{Function, NativeFunction, Value, weak_map};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_weak_map_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::WeakMap => {
            weak_map::native_weak_map(function, argument_values, is_construct, env)?
        }
        NativeFunction::WeakMapPrototypeDelete => {
            weak_map::native_weak_map_prototype_delete(this_value, argument_values)?
        }
        NativeFunction::WeakMapPrototypeGet => {
            weak_map::native_weak_map_prototype_get(this_value, argument_values)?
        }
        NativeFunction::WeakMapPrototypeHas => {
            weak_map::native_weak_map_prototype_has(this_value, argument_values)?
        }
        NativeFunction::WeakMapPrototypeSet => {
            weak_map::native_weak_map_prototype_set(this_value, argument_values, env)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
