//! Operation entries for data views; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, NativeFunction, RuntimeError, Value, data_view};

pub(super) fn data_view(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn data_view_prototype_buffer(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_buffer(info.this_value.clone())
}

pub(super) fn data_view_prototype_byte_length(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_byte_length(info.this_value.clone())
}

pub(super) fn data_view_prototype_byte_offset(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_byte_offset(info.this_value.clone())
}

pub(super) fn data_view_prototype_get_int8(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetInt8,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_get_uint8(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetUint8,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_get_int16(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetInt16,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_get_uint16(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetUint16,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_get_int32(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetInt32,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_get_uint32(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetUint32,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_get_float16(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetFloat16,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_get_float32(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetFloat32,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_get_float64(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetFloat64,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_get_big_int64(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetBigInt64,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_get_big_uint64(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_get(
        NativeFunction::DataViewPrototypeGetBigUint64,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_int8(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetInt8,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_uint8(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetUint8,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_int16(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetInt16,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_uint16(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetUint16,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_int32(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetInt32,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_uint32(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetUint32,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_float16(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetFloat16,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_float32(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetFloat32,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_float64(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetFloat64,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_big_int64(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetBigInt64,
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn data_view_prototype_set_big_uint64(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    data_view::native_data_view_prototype_set(
        NativeFunction::DataViewPrototypeSetBigUint64,
        info.this_value.clone(),
        argument_values,
        env,
    )
}
