use crate::{Function, NativeFunction, Value, map};

use super::NativeCallResult;

pub(super) fn call_map_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Map => map::native_map(function, argument_values, is_construct)?,
        NativeFunction::MapPrototypeClear => map::native_map_prototype_clear(this_value)?,
        NativeFunction::MapPrototypeDelete => {
            map::native_map_prototype_delete(this_value, argument_values)?
        }
        NativeFunction::MapPrototypeGet => {
            map::native_map_prototype_get(this_value, argument_values)?
        }
        NativeFunction::MapPrototypeHas => {
            map::native_map_prototype_has(this_value, argument_values)?
        }
        NativeFunction::MapPrototypeSet => {
            map::native_map_prototype_set(this_value, argument_values)?
        }
        NativeFunction::MapPrototypeSize => map::native_map_prototype_size(this_value)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
