//! Operation entries for weak refs; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, weak_ref};

pub(super) fn weak_ref(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_ref::native_weak_ref(info.callee, argument_values, info.new_target.is_some(), env)
}

pub(super) fn weak_ref_prototype_deref(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    weak_ref::native_weak_ref_prototype_deref(info.this_value.clone())
}
