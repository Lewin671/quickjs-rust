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
        NativeFunction::SetPrototypeDifference => {
            set::native_set_prototype_difference(this_value, argument_values, env)?
        }
        NativeFunction::SetPrototypeEntries => set::native_set_prototype_entries(this_value, env)?,
        NativeFunction::SetPrototypeForEach => {
            set::native_set_prototype_for_each(this_value, argument_values, env)?
        }
        NativeFunction::SetPrototypeHas => {
            set::native_set_prototype_has(this_value, argument_values)?
        }
        NativeFunction::SetPrototypeIntersection => {
            set::native_set_prototype_intersection(this_value, argument_values, env)?
        }
        NativeFunction::SetPrototypeIsDisjointFrom => {
            set::native_set_prototype_is_disjoint_from(this_value, argument_values, env)?
        }
        NativeFunction::SetPrototypeIsSubsetOf => {
            set::native_set_prototype_is_subset_of(this_value, argument_values, env)?
        }
        NativeFunction::SetPrototypeIsSupersetOf => {
            set::native_set_prototype_is_superset_of(this_value, argument_values, env)?
        }
        NativeFunction::SetPrototypeSize => set::native_set_prototype_size(this_value)?,
        NativeFunction::SetPrototypeSymmetricDifference => {
            set::native_set_prototype_symmetric_difference(this_value, argument_values, env)?
        }
        NativeFunction::SetPrototypeUnion => {
            set::native_set_prototype_union(this_value, argument_values, env)?
        }
        NativeFunction::SetPrototypeValues => set::native_set_prototype_values(this_value, env)?,
        NativeFunction::SetIteratorPrototypeNext => set::native_set_iterator_next(this_value)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
