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
        _ => return Ok(None),
    };
    Ok(Some(value))
}
