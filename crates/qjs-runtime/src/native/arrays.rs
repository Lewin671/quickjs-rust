//! Operation entries for arrays; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, array};

pub(super) fn array(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array(info.callee, argument_values, info.new_target.is_some(), env)
}

pub(super) fn array_from(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_from(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_from_async(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_from_async(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_from_async_array_like_mapped_fulfilled(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_from_async_array_like_mapped_fulfilled(info.callee, argument_values, env)
}

pub(super) fn array_from_async_array_like_value_fulfilled(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_from_async_array_like_value_fulfilled(info.callee, argument_values, env)
}

pub(super) fn array_from_async_iterator_mapped_fulfilled(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_from_async_iterator_mapped_fulfilled(info.callee, argument_values, env)
}

pub(super) fn array_from_async_iterator_rejected(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_from_async_iterator_rejected(info.callee, argument_values, env)
}

pub(super) fn array_from_async_iterator_step_fulfilled(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_from_async_iterator_step_fulfilled(info.callee, argument_values, env)
}

pub(super) fn array_from_async_rejected(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_from_async_rejected(info.callee, argument_values, env)
}

pub(super) fn array_is_array(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_is_array(argument_values, env)
}

pub(super) fn array_of(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_of(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_at(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_at(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_concat(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_concat(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_copy_within(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_copy_within(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_entries(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_entries(info.this_value.clone(), env)
}

pub(super) fn array_prototype_every(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_every(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_fill(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_fill(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_flat(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_flat(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_flat_map(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_flat_map(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_filter(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_filter(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_find(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_find(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_find_index(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_find_index(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_find_last(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_find_last(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_find_last_index(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_find_last_index(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_for_each(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_for_each(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_includes(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_includes(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_index_of(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_index_of(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_keys(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_keys(info.this_value.clone(), env)
}

pub(super) fn array_prototype_last_index_of(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_last_index_of(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_join(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_join(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_map(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_map(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_pop(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_pop(info.this_value.clone(), env)
}

pub(super) fn array_prototype_push(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_push(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_reduce(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_reduce(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_reduce_right(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_reduce_right(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_reverse(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_reverse(info.this_value.clone(), env)
}

pub(super) fn array_prototype_shift(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_shift(info.this_value.clone(), env)
}

pub(super) fn array_prototype_slice(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_slice(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_some(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_some(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_sort(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_sort(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_splice(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_splice(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_to_string(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_to_string(info.this_value.clone(), env)
}

pub(super) fn array_prototype_to_locale_string(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_to_locale_string(info.this_value.clone(), env)
}

pub(super) fn array_prototype_to_reversed(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_to_reversed(info.this_value.clone(), env)
}

pub(super) fn array_prototype_to_spliced(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_to_spliced(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_to_sorted(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_to_sorted(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_unshift(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_unshift(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_with(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_with(info.this_value.clone(), argument_values, env)
}

pub(super) fn array_prototype_values(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_prototype_values(info.this_value.clone(), env)
}

pub(super) fn array_iterator_prototype_next(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    array::native_array_iterator_next(info.this_value.clone(), env)
}
