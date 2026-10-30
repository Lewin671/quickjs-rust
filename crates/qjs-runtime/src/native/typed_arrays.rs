//! Operation entries for typed arrays; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, NativeFunction, RuntimeError, Value, typed_array};

pub(super) fn typed_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::TypedArray,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn uint8_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::Uint8Array,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn int8_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::Int8Array,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn uint8_clamped_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::Uint8ClampedArray,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn uint16_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::Uint16Array,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn int16_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::Int16Array,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn uint32_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::Uint32Array,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn int32_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::Int32Array,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn float32_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::Float32Array,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn float64_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::Float64Array,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn big_int64_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::BigInt64Array,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn big_uint64_array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array(
        info.callee,
        NativeFunction::BigUint64Array,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn typed_array_from(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_from(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_of(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_of(info.this_value.clone(), argument_values, env)
}

pub(super) fn uint8_array_from_base64(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_uint8_array_from_base64(argument_values, env)
}

pub(super) fn uint8_array_from_hex(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_uint8_array_from_hex(argument_values, env)
}

pub(super) fn typed_array_prototype_buffer(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_buffer(info.this_value.clone())
}

pub(super) fn typed_array_prototype_byte_length(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_byte_length(info.this_value.clone())
}

pub(super) fn typed_array_prototype_byte_offset(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_byte_offset(info.this_value.clone())
}

pub(super) fn typed_array_prototype_length(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_length(info.this_value.clone())
}

pub(super) fn typed_array_prototype_to_string_tag(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_to_string_tag(info.this_value.clone())
}

pub(super) fn typed_array_prototype_at(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_at(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_index_of(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_index_of(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_last_index_of(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_last_index_of(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_includes(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_includes(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_join(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_join(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_keys(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_keys(info.this_value.clone(), env)
}

pub(super) fn typed_array_prototype_values(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_values(info.this_value.clone(), env)
}

pub(super) fn typed_array_prototype_entries(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_entries(info.this_value.clone(), env)
}

pub(super) fn typed_array_prototype_for_each(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_for_each(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_some(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_some(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_every(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_every(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_find(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_find(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_find_index(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_find_index(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_find_last(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_find_last(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_find_last_index(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_find_last_index(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_map(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_map(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_filter(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_filter(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_reduce(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_reduce(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_reduce_right(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_reduce_right(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_slice(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_slice(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_subarray(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_subarray(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_to_string(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_to_string(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_to_locale_string(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_to_locale_string(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_set(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_set(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_fill(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_fill(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_copy_within(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_copy_within(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_reverse(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_reverse(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_sort(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_sort(info.this_value.clone(), argument_values, env)
}

pub(super) fn typed_array_prototype_to_reversed(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_to_reversed(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_to_sorted(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_to_sorted(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn typed_array_prototype_with(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_typed_array_prototype_with(info.this_value.clone(), argument_values, env)
}

pub(super) fn uint8_array_prototype_to_base64(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_uint8_array_prototype_to_base64(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn uint8_array_prototype_to_hex(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_uint8_array_prototype_to_hex(info.this_value.clone())
}

pub(super) fn uint8_array_prototype_set_from_base64(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_uint8_array_prototype_set_from_base64(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn uint8_array_prototype_set_from_hex(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    typed_array::native_uint8_array_prototype_set_from_hex(
        info.this_value.clone(),
        argument_values,
        env,
    )
}
