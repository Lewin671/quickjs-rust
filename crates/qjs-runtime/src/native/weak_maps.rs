//! Operation entries for weak maps; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, weak_map};

pub(super) fn weak_map(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_map::native_weak_map(info.callee, argument_values, info.new_target.is_some(), env)
}

pub(super) fn weak_map_prototype_delete(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_map::native_weak_map_prototype_delete(info.this_value.clone(), argument_values)
}

pub(super) fn weak_map_prototype_get(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_map::native_weak_map_prototype_get(info.this_value.clone(), argument_values)
}

pub(super) fn weak_map_prototype_get_or_insert(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_map::native_weak_map_prototype_get_or_insert(info.this_value.clone(), argument_values, env)
}

pub(super) fn weak_map_prototype_get_or_insert_computed(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_map::native_weak_map_prototype_get_or_insert_computed(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn weak_map_prototype_has(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_map::native_weak_map_prototype_has(info.this_value.clone(), argument_values)
}

pub(super) fn weak_map_prototype_set(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_map::native_weak_map_prototype_set(info.this_value.clone(), argument_values, env)
}
