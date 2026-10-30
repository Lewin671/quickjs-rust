//! Operation entries for weak sets; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, weak_set};

pub(super) fn weak_set(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_set::native_weak_set(info.callee, argument_values, info.new_target.is_some(), env)
}

pub(super) fn weak_set_prototype_add(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_set::native_weak_set_prototype_add(info.this_value.clone(), argument_values, env)
}

pub(super) fn weak_set_prototype_delete(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_set::native_weak_set_prototype_delete(info.this_value.clone(), argument_values)
}

pub(super) fn weak_set_prototype_has(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_set::native_weak_set_prototype_has(info.this_value.clone(), argument_values)
}
