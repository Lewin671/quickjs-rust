use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, data_view};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_data_view_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::DataView => {
            data_view::native_data_view(function, this_value, argument_values, is_construct, env)?
        }
        NativeFunction::DataViewPrototypeBuffer => {
            data_view::native_data_view_prototype_buffer(this_value)?
        }
        NativeFunction::DataViewPrototypeByteLength => {
            data_view::native_data_view_prototype_byte_length(this_value)?
        }
        NativeFunction::DataViewPrototypeByteOffset => {
            data_view::native_data_view_prototype_byte_offset(this_value)?
        }
        NativeFunction::DataViewPrototypeGetInt8
        | NativeFunction::DataViewPrototypeGetUint8
        | NativeFunction::DataViewPrototypeGetInt16
        | NativeFunction::DataViewPrototypeGetUint16
        | NativeFunction::DataViewPrototypeGetInt32
        | NativeFunction::DataViewPrototypeGetUint32
        | NativeFunction::DataViewPrototypeGetFloat32
        | NativeFunction::DataViewPrototypeGetFloat64
        | NativeFunction::DataViewPrototypeGetBigInt64
        | NativeFunction::DataViewPrototypeGetBigUint64 => {
            data_view::native_data_view_prototype_get(native, this_value, argument_values, env)?
        }
        NativeFunction::DataViewPrototypeSetInt8
        | NativeFunction::DataViewPrototypeSetUint8
        | NativeFunction::DataViewPrototypeSetInt16
        | NativeFunction::DataViewPrototypeSetUint16
        | NativeFunction::DataViewPrototypeSetInt32
        | NativeFunction::DataViewPrototypeSetUint32
        | NativeFunction::DataViewPrototypeSetFloat32
        | NativeFunction::DataViewPrototypeSetFloat64
        | NativeFunction::DataViewPrototypeSetBigInt64
        | NativeFunction::DataViewPrototypeSetBigUint64 => {
            data_view::native_data_view_prototype_set(native, this_value, argument_values, env)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
