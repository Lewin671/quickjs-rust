//! Operation entries for sets; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, set};

pub(super) fn set(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set(info.callee, argument_values, info.new_target.is_some(), env)
}

pub(super) fn set_prototype_add(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_add(info.this_value.clone(), argument_values)
}

pub(super) fn set_prototype_clear(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_clear(info.this_value.clone())
}

pub(super) fn set_prototype_delete(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_delete(info.this_value.clone(), argument_values)
}

pub(super) fn set_prototype_difference(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_difference(info.this_value.clone(), argument_values, env)
}

pub(super) fn set_prototype_entries(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_entries(info.this_value.clone(), env)
}

pub(super) fn set_prototype_for_each(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_for_each(info.this_value.clone(), argument_values, env)
}

pub(super) fn set_prototype_has(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_has(info.this_value.clone(), argument_values)
}

pub(super) fn set_prototype_intersection(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_intersection(info.this_value.clone(), argument_values, env)
}

pub(super) fn set_prototype_is_disjoint_from(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_is_disjoint_from(info.this_value.clone(), argument_values, env)
}

pub(super) fn set_prototype_is_subset_of(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_is_subset_of(info.this_value.clone(), argument_values, env)
}

pub(super) fn set_prototype_is_superset_of(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_is_superset_of(info.this_value.clone(), argument_values, env)
}

pub(super) fn set_prototype_size(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_size(info.this_value.clone())
}

pub(super) fn set_prototype_symmetric_difference(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_symmetric_difference(info.this_value.clone(), argument_values, env)
}

pub(super) fn set_prototype_union(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_union(info.this_value.clone(), argument_values, env)
}

pub(super) fn set_prototype_values(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_prototype_values(info.this_value.clone(), env)
}

pub(super) fn set_iterator_prototype_next(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    set::native_set_iterator_next(info.this_value.clone())
}
