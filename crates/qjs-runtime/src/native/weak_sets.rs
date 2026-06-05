use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, weak_set};

use super::NativeCallResult;

pub(super) fn call_weak_set_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::WeakSet => {
            weak_set::native_weak_set(function, argument_values, is_construct, env)?
        }
        NativeFunction::WeakSetPrototypeAdd => {
            weak_set::native_weak_set_prototype_add(this_value, argument_values)?
        }
        NativeFunction::WeakSetPrototypeDelete => {
            weak_set::native_weak_set_prototype_delete(this_value, argument_values)?
        }
        NativeFunction::WeakSetPrototypeHas => {
            weak_set::native_weak_set_prototype_has(this_value, argument_values)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
