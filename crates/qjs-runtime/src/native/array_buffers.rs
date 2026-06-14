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
        NativeFunction::ArrayBufferPrototypeMaxByteLength => {
            array_buffer::native_array_buffer_prototype_max_byte_length(this_value)?
        }
        NativeFunction::ArrayBufferPrototypeResizable => {
            array_buffer::native_array_buffer_prototype_resizable(this_value)?
        }
        NativeFunction::ArrayBufferPrototypeDetached => {
            array_buffer::native_array_buffer_prototype_detached(this_value)?
        }
        NativeFunction::ArrayBufferPrototypeImmutable => {
            array_buffer::native_array_buffer_prototype_immutable(this_value)?
        }
        NativeFunction::ArrayBufferPrototypeResize => {
            array_buffer::native_array_buffer_prototype_resize(this_value, argument_values, env)?
        }
        NativeFunction::ArrayBufferPrototypeSlice => {
            array_buffer::native_array_buffer_prototype_slice(this_value, argument_values, env)?
        }
        NativeFunction::ArrayBufferPrototypeSliceToImmutable => {
            array_buffer::native_array_buffer_prototype_slice_to_immutable(
                this_value,
                argument_values,
                env,
            )?
        }
        NativeFunction::ArrayBufferPrototypeTransferToImmutable => {
            array_buffer::native_array_buffer_prototype_transfer_to_immutable(
                this_value,
                argument_values,
                env,
            )?
        }
        NativeFunction::SharedArrayBuffer => {
            array_buffer::native_shared_array_buffer(function, argument_values, is_construct, env)?
        }
        NativeFunction::SharedArrayBufferPrototypeByteLength => {
            array_buffer::native_shared_array_buffer_prototype_byte_length(this_value)?
        }
        NativeFunction::DetachArrayBuffer => {
            array_buffer::native_detach_array_buffer(argument_values)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
