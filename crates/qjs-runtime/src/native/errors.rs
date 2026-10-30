//! Operation entries for errors; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, error};

pub(super) fn error(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn aggregate_error(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_aggregate_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn suppressed_error(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_suppressed_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn error_is_error(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    Ok(error::native_error_is_error(argument_values))
}

pub(super) fn error_prototype_to_string(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error_prototype_to_string(info.this_value.clone(), env)
}

pub(super) fn eval_error(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn range_error(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn reference_error(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn syntax_error(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn type_error(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn uri_error(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}
