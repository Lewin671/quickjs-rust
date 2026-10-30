//! Operation entries for numbers; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, number};

pub(super) fn number(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.new_target.is_some(),
        env,
    )
}

pub(super) fn number_is_finite(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_is_finite(argument_values)
}

pub(super) fn number_is_integer(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_is_integer(argument_values)
}

pub(super) fn number_is_na_n(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_is_nan(argument_values)
}

pub(super) fn number_is_safe_integer(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_is_safe_integer(argument_values)
}

pub(super) fn number_prototype_to_exponential(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_to_exponential(info.this_value.clone(), argument_values, env)
}

pub(super) fn number_prototype_to_fixed(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_to_fixed(info.this_value.clone(), argument_values, env)
}

pub(super) fn number_prototype_to_precision(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_to_precision(info.this_value.clone(), argument_values, env)
}

pub(super) fn number_prototype_to_string(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_to_string(info.this_value.clone(), argument_values, env)
}

pub(super) fn number_prototype_to_locale_string(
    env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_to_string(info.this_value.clone(), &[], env)
}

pub(super) fn number_prototype_value_of(
    _env: &mut CallEnv,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_value_of(info.this_value.clone())
}

pub(super) fn parse_float(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_parse_float(argument_values, env)
}

pub(super) fn parse_int(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_parse_int(argument_values, env)
}
