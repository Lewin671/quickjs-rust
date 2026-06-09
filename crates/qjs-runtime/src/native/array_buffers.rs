use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, array_buffer};

use super::NativeCallResult;

pub(super) fn call_array_buffer_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::ArrayBuffer => array_buffer::native_array_buffer(
            function,
            this_value,
            argument_values,
            is_construct,
            env,
        )?,
        NativeFunction::ArrayBufferPrototypeByteLength => {
            array_buffer::native_array_buffer_prototype_byte_length(this_value)?
        }
        NativeFunction::ArrayBufferPrototypeSlice => {
            array_buffer::native_array_buffer_prototype_slice(this_value, argument_values, env)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
