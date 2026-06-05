use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, map};

use super::NativeCallResult;

pub(super) fn call_map_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Map => map::native_map(function, argument_values, is_construct, env)?,
        NativeFunction::MapPrototypeClear => map::native_map_prototype_clear(this_value)?,
        NativeFunction::MapPrototypeDelete => {
            map::native_map_prototype_delete(this_value, argument_values)?
        }
        NativeFunction::MapPrototypeEntries => map::native_map_prototype_entries(this_value)?,
        NativeFunction::MapPrototypeForEach => {
            map::native_map_prototype_for_each(this_value, argument_values, env)?
        }
        NativeFunction::MapPrototypeGet => {
            map::native_map_prototype_get(this_value, argument_values)?
        }
        NativeFunction::MapPrototypeGetOrInsert => {
            map::native_map_prototype_get_or_insert(this_value, argument_values)?
        }
        NativeFunction::MapPrototypeGetOrInsertComputed => {
            map::native_map_prototype_get_or_insert_computed(this_value, argument_values, env)?
        }
        NativeFunction::MapPrototypeHas => {
            map::native_map_prototype_has(this_value, argument_values)?
        }
        NativeFunction::MapPrototypeKeys => map::native_map_prototype_keys(this_value)?,
        NativeFunction::MapPrototypeSet => {
            map::native_map_prototype_set(this_value, argument_values)?
        }
        NativeFunction::MapPrototypeSize => map::native_map_prototype_size(this_value)?,
        NativeFunction::MapPrototypeValues => map::native_map_prototype_values(this_value)?,
        NativeFunction::MapIteratorPrototypeNext => map::native_map_iterator_next(this_value)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
