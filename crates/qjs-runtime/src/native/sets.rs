use crate::{Function, NativeFunction, Value, set};

use super::NativeCallResult;

pub(super) fn call_set_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Set => set::native_set(function, argument_values, is_construct)?,
        NativeFunction::SetPrototypeAdd => {
            set::native_set_prototype_add(this_value, argument_values)?
        }
        NativeFunction::SetPrototypeClear => set::native_set_prototype_clear(this_value)?,
        NativeFunction::SetPrototypeDelete => {
            set::native_set_prototype_delete(this_value, argument_values)?
        }
        NativeFunction::SetPrototypeHas => {
            set::native_set_prototype_has(this_value, argument_values)?
        }
        NativeFunction::SetPrototypeSize => set::native_set_prototype_size(this_value)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
