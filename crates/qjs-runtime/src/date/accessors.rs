use crate::{RuntimeError, Value, date::iso::utc_date_time};

use super::value::date_value;

pub(crate) fn native_date_prototype_get_time(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Number(date_value(this_value)?))
}

pub(crate) fn native_date_prototype_get_timezone_offset(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let millis = date_value(this_value)?;
    if !millis.is_finite() {
        return Ok(Value::Number(f64::NAN));
    }
    Ok(Value::Number(0.0))
}

pub(crate) fn native_date_prototype_get_utc_date(this_value: Value) -> Result<Value, RuntimeError> {
    utc_component(this_value, |components| components.date)
}

pub(crate) fn native_date_prototype_get_utc_day(this_value: Value) -> Result<Value, RuntimeError> {
    utc_component(this_value, |components| components.day)
}

pub(crate) fn native_date_prototype_get_utc_full_year(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    utc_component(this_value, |components| components.year)
}

pub(crate) fn native_date_prototype_get_utc_hours(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    utc_component(this_value, |components| components.hours)
}

pub(crate) fn native_date_prototype_get_utc_milliseconds(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    utc_component(this_value, |components| components.milliseconds)
}

pub(crate) fn native_date_prototype_get_utc_minutes(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    utc_component(this_value, |components| components.minutes)
}

pub(crate) fn native_date_prototype_get_utc_month(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    utc_component(this_value, |components| components.month)
}

pub(crate) fn native_date_prototype_get_utc_seconds(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    utc_component(this_value, |components| components.seconds)
}

pub(crate) fn native_date_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Number(date_value(this_value)?))
}

fn utc_component(
    this_value: Value,
    component: impl FnOnce(super::iso::UtcDateTime) -> i32,
) -> Result<Value, RuntimeError> {
    let millis = date_value(this_value)?;
    if !millis.is_finite() {
        return Ok(Value::Number(f64::NAN));
    }
    Ok(Value::Number(f64::from(component(utc_date_time(millis)))))
}
