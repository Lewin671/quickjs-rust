//! Operation entries for maps; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, map};

pub(super) fn map(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map(info.callee, argument_values, info.is_construct, env)
}

pub(super) fn map_group_by(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_group_by(argument_values, env)
}

pub(super) fn map_prototype_clear(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_clear(info.this_value.clone())
}

pub(super) fn map_prototype_delete(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_delete(info.this_value.clone(), argument_values)
}

pub(super) fn map_prototype_entries(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_entries(info.this_value.clone(), env)
}

pub(super) fn map_prototype_for_each(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_for_each(info.this_value.clone(), argument_values, env)
}

pub(super) fn map_prototype_get(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_get(info.this_value.clone(), argument_values)
}

pub(super) fn map_prototype_get_or_insert(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_get_or_insert(info.this_value.clone(), argument_values)
}

pub(super) fn map_prototype_get_or_insert_computed(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_get_or_insert_computed(info.this_value.clone(), argument_values, env)
}

pub(super) fn map_prototype_has(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_has(info.this_value.clone(), argument_values)
}

pub(super) fn map_prototype_keys(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_keys(info.this_value.clone(), env)
}

pub(super) fn map_prototype_set(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_set(info.this_value.clone(), argument_values)
}

pub(super) fn map_prototype_size(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_size(info.this_value.clone())
}

pub(super) fn map_prototype_values(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_prototype_values(info.this_value.clone(), env)
}

pub(super) fn map_iterator_prototype_next(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    map::native_map_iterator_next(info.this_value.clone())
}
