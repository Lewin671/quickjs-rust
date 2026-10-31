//! Operation entries for errors; bound once when a native is created.

use super::CallInfo;
use crate::{RuntimeError, Value, error};

pub(super) fn error(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn aggregate_error(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_aggregate_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn suppressed_error(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_suppressed_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn error_is_error(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    Ok(error::native_error_is_error(argument_values))
}

pub(super) fn error_prototype_to_string(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error_prototype_to_string(info.this_value.clone(), env)
}

pub(super) fn eval_error(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn range_error(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn reference_error(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn syntax_error(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn type_error(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn uri_error(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    error::native_error(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}
