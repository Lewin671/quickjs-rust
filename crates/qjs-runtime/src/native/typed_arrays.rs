use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, typed_array};

use super::NativeCallResult;

pub(super) fn call_typed_array_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::TypedArray
        | NativeFunction::Uint8Array
        | NativeFunction::Int8Array
        | NativeFunction::Uint8ClampedArray
        | NativeFunction::Uint16Array
        | NativeFunction::Int16Array
        | NativeFunction::Uint32Array
        | NativeFunction::Int32Array
        | NativeFunction::Float32Array
        | NativeFunction::Float64Array
        | NativeFunction::BigInt64Array
        | NativeFunction::BigUint64Array => typed_array::native_typed_array(
            function,
            native,
            this_value,
            argument_values,
            is_construct,
            env,
        )?,
        NativeFunction::TypedArrayPrototypeBuffer => {
            typed_array::native_typed_array_prototype_buffer(this_value)?
        }
        NativeFunction::TypedArrayPrototypeByteLength => {
            typed_array::native_typed_array_prototype_byte_length(this_value)?
        }
        NativeFunction::TypedArrayPrototypeByteOffset => {
            typed_array::native_typed_array_prototype_byte_offset(this_value)?
        }
        NativeFunction::TypedArrayPrototypeLength => {
            typed_array::native_typed_array_prototype_length(this_value)?
        }
        NativeFunction::TypedArrayPrototypeToStringTag => {
            typed_array::native_typed_array_prototype_to_string_tag(this_value)?
        }
        NativeFunction::TypedArrayPrototypeAt => {
            typed_array::native_typed_array_prototype_at(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeIndexOf => {
            typed_array::native_typed_array_prototype_index_of(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeLastIndexOf => {
            typed_array::native_typed_array_prototype_last_index_of(
                this_value,
                argument_values,
                env,
            )?
        }
        NativeFunction::TypedArrayPrototypeIncludes => {
            typed_array::native_typed_array_prototype_includes(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeJoin => {
            typed_array::native_typed_array_prototype_join(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeKeys => {
            typed_array::native_typed_array_prototype_keys(this_value, env)?
        }
        NativeFunction::TypedArrayPrototypeValues => {
            typed_array::native_typed_array_prototype_values(this_value, env)?
        }
        NativeFunction::TypedArrayPrototypeEntries => {
            typed_array::native_typed_array_prototype_entries(this_value, env)?
        }
        NativeFunction::TypedArrayIteratorPrototypeNext => {
            typed_array::native_typed_array_iterator_next(this_value)?
        }
        NativeFunction::TypedArrayPrototypeForEach => {
            typed_array::native_typed_array_prototype_for_each(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeSome => {
            typed_array::native_typed_array_prototype_some(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeEvery => {
            typed_array::native_typed_array_prototype_every(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeFind => {
            typed_array::native_typed_array_prototype_find(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeFindIndex => {
            typed_array::native_typed_array_prototype_find_index(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeFindLast => {
            typed_array::native_typed_array_prototype_find_last(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeFindLastIndex => {
            typed_array::native_typed_array_prototype_find_last_index(
                this_value,
                argument_values,
                env,
            )?
        }
        NativeFunction::TypedArrayPrototypeMap => {
            typed_array::native_typed_array_prototype_map(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeFilter => {
            typed_array::native_typed_array_prototype_filter(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeReduce => {
            typed_array::native_typed_array_prototype_reduce(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeReduceRight => {
            typed_array::native_typed_array_prototype_reduce_right(
                this_value,
                argument_values,
                env,
            )?
        }
        NativeFunction::TypedArrayPrototypeSlice => {
            typed_array::native_typed_array_prototype_slice(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeSubarray => {
            typed_array::native_typed_array_prototype_subarray(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeToString => {
            typed_array::native_typed_array_prototype_to_string(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeToLocaleString => {
            typed_array::native_typed_array_prototype_to_locale_string(
                this_value,
                argument_values,
                env,
            )?
        }
        NativeFunction::TypedArrayPrototypeSet => {
            typed_array::native_typed_array_prototype_set(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeFill => {
            typed_array::native_typed_array_prototype_fill(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeCopyWithin => {
            typed_array::native_typed_array_prototype_copy_within(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeReverse => {
            typed_array::native_typed_array_prototype_reverse(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeSort => {
            typed_array::native_typed_array_prototype_sort(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeToReversed => {
            typed_array::native_typed_array_prototype_to_reversed(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeToSorted => {
            typed_array::native_typed_array_prototype_to_sorted(this_value, argument_values, env)?
        }
        NativeFunction::TypedArrayPrototypeWith => {
            typed_array::native_typed_array_prototype_with(this_value, argument_values, env)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
