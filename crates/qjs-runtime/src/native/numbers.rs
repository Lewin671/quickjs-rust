//! Operation entries for numbers; bound once when a native is created.

use super::CallInfo;
use crate::{RuntimeError, Value, number};

pub(super) fn number(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn number_is_finite(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_is_finite(argument_values)
}

pub(super) fn number_is_integer(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_is_integer(argument_values)
}

pub(super) fn number_is_na_n(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_is_nan(argument_values)
}

pub(super) fn number_is_safe_integer(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_is_safe_integer(argument_values)
}

pub(super) fn number_prototype_to_exponential(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_to_exponential(info.this_value.clone(), argument_values, env)
}

pub(super) fn number_prototype_to_fixed(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_to_fixed(info.this_value.clone(), argument_values, env)
}

pub(super) fn number_prototype_to_precision(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_to_precision(info.this_value.clone(), argument_values, env)
}

pub(super) fn number_prototype_to_string(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_to_string(info.this_value.clone(), argument_values, env)
}

pub(super) fn number_prototype_to_locale_string(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_to_string(info.this_value.clone(), &[], env)
}

pub(super) fn number_prototype_value_of(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_number_prototype_value_of(info.this_value.clone())
}

pub(super) fn parse_float(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_parse_float(argument_values, env)
}

pub(super) fn parse_int(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    number::native_parse_int(argument_values, env)
}
