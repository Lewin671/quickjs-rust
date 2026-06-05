use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, set};

use super::NativeCallResult;

pub(super) fn call_set_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Set => set::native_set(function, argument_values, is_construct, env)?,
        NativeFunction::SetPrototypeAdd => {
            set::native_set_prototype_add(this_value, argument_values)?
        }
        NativeFunction::SetPrototypeClear => set::native_set_prototype_clear(this_value)?,
        NativeFunction::SetPrototypeDelete => {
            set::native_set_prototype_delete(this_value, argument_values)?
        }
        NativeFunction::SetPrototypeEntries => set::native_set_prototype_entries(this_value)?,
        NativeFunction::SetPrototypeForEach => {
            set::native_set_prototype_for_each(this_value, argument_values, env)?
        }
        NativeFunction::SetPrototypeHas => {
            set::native_set_prototype_has(this_value, argument_values)?
        }
        NativeFunction::SetPrototypeKeys => set::native_set_prototype_keys(this_value)?,
        NativeFunction::SetPrototypeSize => set::native_set_prototype_size(this_value)?,
        NativeFunction::SetPrototypeValues => set::native_set_prototype_values(this_value)?,
        NativeFunction::SetIteratorPrototypeNext => set::native_set_iterator_next(this_value)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
