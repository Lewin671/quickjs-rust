//! Operation entries for array buffers; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, array_buffer};

pub(super) fn array_buffer(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn array_buffer_is_view(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_is_view(argument_values)
}

pub(super) fn array_buffer_prototype_byte_length(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_byte_length(info.this_value.clone())
}

pub(super) fn array_buffer_prototype_max_byte_length(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_max_byte_length(info.this_value.clone())
}

pub(super) fn array_buffer_prototype_resizable(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_resizable(info.this_value.clone())
}

pub(super) fn array_buffer_prototype_detached(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_detached(info.this_value.clone())
}

pub(super) fn array_buffer_prototype_immutable(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_immutable(info.this_value.clone())
}

pub(super) fn array_buffer_prototype_resize(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_resize(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn array_buffer_prototype_slice(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_slice(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_buffer_prototype_slice_to_immutable(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_slice_to_immutable(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn array_buffer_prototype_transfer(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_transfer(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn array_buffer_prototype_transfer_to_fixed_length(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_transfer_to_fixed_length(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn array_buffer_prototype_transfer_to_immutable(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_array_buffer_prototype_transfer_to_immutable(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn shared_array_buffer(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_shared_array_buffer(
        info.callee,
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn shared_array_buffer_prototype_byte_length(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_shared_array_buffer_prototype_byte_length(info.this_value.clone())
}

pub(super) fn shared_array_buffer_prototype_max_byte_length(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_shared_array_buffer_prototype_max_byte_length(info.this_value.clone())
}

pub(super) fn shared_array_buffer_prototype_growable(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_shared_array_buffer_prototype_growable(info.this_value.clone())
}

pub(super) fn shared_array_buffer_prototype_grow(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_shared_array_buffer_prototype_grow(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn shared_array_buffer_prototype_slice(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_shared_array_buffer_prototype_slice(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn detach_array_buffer(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array_buffer::native_detach_array_buffer(argument_values)
}
