use std::collections::HashMap;

use crate::{NativeFunction, Value, array};

use super::NativeCallResult;

pub(super) fn call_array_native(
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Array => array::native_array(argument_values)?,
        NativeFunction::ArrayFrom => array::native_array_from(argument_values, env)?,
        NativeFunction::ArrayIsArray => array::native_array_is_array(argument_values)?,
        NativeFunction::ArrayOf => array::native_array_of(argument_values)?,
        NativeFunction::ArrayPrototypeAt => {
            array::native_array_prototype_at(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeConcat => {
            array::native_array_prototype_concat(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeCopyWithin => {
            array::native_array_prototype_copy_within(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeEntries => {
            array::native_array_prototype_entries(this_value, env)?
        }
        NativeFunction::ArrayPrototypeEvery => {
            array::native_array_prototype_every(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeFill => {
            array::native_array_prototype_fill(this_value, argument_values)?
        }
        NativeFunction::ArrayPrototypeFlat => {
            array::native_array_prototype_flat(this_value, argument_values)?
        }
        NativeFunction::ArrayPrototypeFlatMap => {
            array::native_array_prototype_flat_map(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeFilter => {
            array::native_array_prototype_filter(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeFind => {
            array::native_array_prototype_find(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeFindIndex => {
            array::native_array_prototype_find_index(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeFindLast => {
            array::native_array_prototype_find_last(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeFindLastIndex => {
            array::native_array_prototype_find_last_index(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeForEach => {
            array::native_array_prototype_for_each(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeIncludes => {
            array::native_array_prototype_includes(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeIndexOf => {
            array::native_array_prototype_index_of(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeKeys => array::native_array_prototype_keys(this_value, env)?,
        NativeFunction::ArrayPrototypeLastIndexOf => {
            array::native_array_prototype_last_index_of(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeJoin => {
            array::native_array_prototype_join(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeMap => {
            array::native_array_prototype_map(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypePop => array::native_array_prototype_pop(this_value, env)?,
        NativeFunction::ArrayPrototypePush => {
            array::native_array_prototype_push(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeReduce => {
            array::native_array_prototype_reduce(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeReduceRight => {
            array::native_array_prototype_reduce_right(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeReverse => array::native_array_prototype_reverse(this_value)?,
        NativeFunction::ArrayPrototypeShift => {
            array::native_array_prototype_shift(this_value, env)?
        }
        NativeFunction::ArrayPrototypeSlice => {
            array::native_array_prototype_slice(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeSome => {
            array::native_array_prototype_some(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeSort => {
            array::native_array_prototype_sort(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeSplice => {
            array::native_array_prototype_splice(this_value, argument_values)?
        }
        NativeFunction::ArrayPrototypeToString => {
            array::native_array_prototype_to_string(this_value, env)?
        }
        NativeFunction::ArrayPrototypeToLocaleString => {
            array::native_array_prototype_to_string(this_value, env)?
        }
        NativeFunction::ArrayPrototypeToReversed => {
            array::native_array_prototype_to_reversed(this_value, env)?
        }
        NativeFunction::ArrayPrototypeToSpliced => {
            array::native_array_prototype_to_spliced(this_value, argument_values)?
        }
        NativeFunction::ArrayPrototypeToSorted => {
            array::native_array_prototype_to_sorted(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeUnshift => {
            array::native_array_prototype_unshift(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeWith => {
            array::native_array_prototype_with(this_value, argument_values, env)?
        }
        NativeFunction::ArrayPrototypeValues => {
            array::native_array_prototype_values(this_value, env)?
        }
        NativeFunction::ArrayIteratorPrototypeNext => {
            array::native_array_iterator_next(this_value, env)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
