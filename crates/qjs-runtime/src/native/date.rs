//! Operation entries for date; bound once when a native is created.

use super::CallInfo;
use crate::{RuntimeError, Value, date};

pub(super) fn date(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn date_now(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_now()
}

pub(super) fn date_parse(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_parse(argument_values, env)
}

pub(super) fn date_utc(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_utc(argument_values, env)
}

pub(super) fn date_prototype_get_timezone_offset(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_timezone_offset(info.this_value.clone())
}

pub(super) fn date_prototype_get_year(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_year(info.this_value.clone())
}

pub(super) fn date_prototype_get_utc_date(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_utc_date(info.this_value.clone())
}

pub(super) fn date_prototype_get_utc_day(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_utc_day(info.this_value.clone())
}

pub(super) fn date_prototype_get_utc_full_year(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_utc_full_year(info.this_value.clone())
}

pub(super) fn date_prototype_get_utc_hours(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_utc_hours(info.this_value.clone())
}

pub(super) fn date_prototype_get_utc_milliseconds(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_utc_milliseconds(info.this_value.clone())
}

pub(super) fn date_prototype_get_utc_minutes(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_utc_minutes(info.this_value.clone())
}

pub(super) fn date_prototype_get_utc_month(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_utc_month(info.this_value.clone())
}

pub(super) fn date_prototype_get_utc_seconds(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_utc_seconds(info.this_value.clone())
}

pub(super) fn date_prototype_get_time(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_get_time(info.this_value.clone())
}

pub(super) fn date_prototype_set_year(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_set_year(info.this_value.clone(), argument_values, env)
}

pub(super) fn date_prototype_set_time(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_set_time(info.this_value.clone(), argument_values, env)
}

pub(super) fn date_prototype_set_utc_date(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_set_utc_date(info.this_value.clone(), argument_values, env)
}

pub(super) fn date_prototype_set_utc_full_year(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_set_utc_full_year(info.this_value.clone(), argument_values, env)
}

pub(super) fn date_prototype_set_utc_hours(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_set_utc_hours(info.this_value.clone(), argument_values, env)
}

pub(super) fn date_prototype_set_utc_milliseconds(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_set_utc_milliseconds(info.this_value.clone(), argument_values, env)
}

pub(super) fn date_prototype_set_utc_minutes(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_set_utc_minutes(info.this_value.clone(), argument_values, env)
}

pub(super) fn date_prototype_set_utc_month(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_set_utc_month(info.this_value.clone(), argument_values, env)
}

pub(super) fn date_prototype_set_utc_seconds(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_set_utc_seconds(info.this_value.clone(), argument_values, env)
}

pub(super) fn date_prototype_to_date_string(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_to_date_string(info.this_value.clone())
}

pub(super) fn date_prototype_to_iso_string(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_to_iso_string(info.this_value.clone())
}

pub(super) fn date_prototype_to_json(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_to_json(
        info.this_value.clone(),
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )
}

pub(super) fn date_prototype_to_primitive(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_to_primitive(
        info.this_value.clone(),
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )
}

pub(super) fn date_prototype_to_string(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_to_string(info.this_value.clone())
}

pub(super) fn date_prototype_to_time_string(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_to_time_string(info.this_value.clone())
}

pub(super) fn date_prototype_to_utc_string(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_to_utc_string(info.this_value.clone())
}

pub(super) fn date_prototype_value_of(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    date::native_date_prototype_value_of(info.this_value.clone())
}
