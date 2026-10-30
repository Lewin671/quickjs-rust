//! Operation entries for finalization registries; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, finalization_registry};

pub(super) fn finalization_registry(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    finalization_registry::native_finalization_registry(
        info.callee,
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn finalization_registry_prototype_register(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    finalization_registry::native_finalization_registry_prototype_register(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn finalization_registry_prototype_unregister(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    finalization_registry::native_finalization_registry_prototype_unregister(
        info.this_value.clone(),
        argument_values,
        env,
    )
}
