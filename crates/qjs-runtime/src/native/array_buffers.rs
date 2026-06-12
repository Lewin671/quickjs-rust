use crate::{Function, NativeFunction, Value, array_buffer};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_array_buffer_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::ArrayBuffer => array_buffer::native_array_buffer(
            function,
            this_value,
            argument_values,
            is_construct,
            env,
        )?,
        NativeFunction::ArrayBufferIsView => {
            array_buffer::native_array_buffer_is_view(argument_values)?
        }
        NativeFunction::ArrayBufferPrototypeByteLength => {
            array_buffer::native_array_buffer_prototype_byte_length(this_value)?
        }
        NativeFunction::ArrayBufferPrototypeSlice => {
            array_buffer::native_array_buffer_prototype_slice(this_value, argument_values, env)?
        }
        NativeFunction::DetachArrayBuffer => {
            array_buffer::native_detach_array_buffer(argument_values)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
