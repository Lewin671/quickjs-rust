use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    Function, ObjectRef, RuntimeError, Value, call_function,
    date::{
        DATE_VALUE_PROPERTY, MS_PER_DAY, MS_PER_HOUR, MS_PER_MINUTE, MS_PER_SECOND,
        iso::{
            days_from_civil, format_iso_string, format_utc_string, parse_iso_string, utc_date_time,
        },
    },
    to_js_string, to_number,
};

pub(crate) fn native_date(
    _function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Ok(Value::String(format_iso_string(current_time_ms())));
    }

    let Value::Object(object) = this_value else {
        return Err(RuntimeError {
            message: "Date constructor requires an object receiver".to_owned(),
        });
    };
    let date_value = construct_date_value(argument_values)?;
    define_date_value(&object, date_value);
    Ok(Value::Object(object))
}

pub(crate) fn native_date_now() -> Result<Value, RuntimeError> {
    Ok(Value::Number(current_time_ms()))
}

pub(crate) fn native_date_parse(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::String(source)) => Ok(Value::Number(parse_date_string(source))),
        Some(value) => Ok(Value::Number(parse_date_string(&to_js_string(
            value.clone(),
        )?))),
        None => Ok(Value::Number(f64::NAN)),
    }
}

pub(crate) fn native_date_utc(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Number(time_clip(date_utc_from_arguments(
        argument_values,
    )?)))
}

pub(crate) fn native_date_prototype_get_time(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Number(date_value(this_value)?))
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

pub(crate) fn native_date_prototype_set_time(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = date_object(this_value)?;
    let time = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let clipped = time_clip(time);
    define_date_value(&object, clipped);
    Ok(Value::Number(clipped))
}

pub(crate) fn native_date_prototype_set_utc_full_year(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = date_object(this_value)?;
    let millis = date_value_from_object(&object)?;
    let base = if millis.is_nan() { 0.0 } else { millis };
    let components = utc_date_time(base);
    let year = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let month = optional_number(argument_values, 1, f64::from(components.month))?;
    let date = optional_number(argument_values, 2, f64::from(components.date))?;
    let updated = time_clip(utc_time_from_components(
        year,
        month,
        date,
        time_within_day(base),
    ));
    define_date_value(&object, updated);
    Ok(Value::Number(updated))
}

pub(crate) fn native_date_prototype_to_iso_string(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let millis = date_value(this_value)?;
    if !millis.is_finite() {
        return Err(RuntimeError {
            message: "Invalid time value".to_owned(),
        });
    }
    Ok(Value::String(format_iso_string(millis)))
}

pub(crate) fn native_date_prototype_to_utc_string(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let millis = date_value(this_value)?;
    if !millis.is_finite() {
        return Ok(Value::String("Invalid Date".to_owned()));
    }
    Ok(Value::String(format_utc_string(millis)))
}

pub(crate) fn native_date_prototype_to_json(
    this_value: Value,
    key: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !date_value(this_value.clone())?.is_finite() {
        return Ok(Value::Null);
    }

    let to_iso_string = match &this_value {
        Value::Object(object) => object.get("toISOString"),
        _ => None,
    }
    .ok_or_else(|| RuntimeError {
        message: "Date toJSON receiver does not have a toISOString method".to_owned(),
    })?;
    call_function(to_iso_string, this_value, vec![key], env, false)
}

pub(super) fn define_date_value(object: &ObjectRef, value: f64) {
    object.define_non_enumerable(DATE_VALUE_PROPERTY.to_owned(), Value::Number(value));
}

fn construct_date_value(argument_values: &[Value]) -> Result<f64, RuntimeError> {
    if argument_values.is_empty() {
        return Ok(current_time_ms());
    }
    if argument_values.len() == 1 {
        return match &argument_values[0] {
            Value::String(source) => Ok(parse_date_string(source)),
            value => to_number(value.clone()).map(time_clip),
        };
    }
    date_utc_from_arguments(argument_values).map(time_clip)
}

fn date_utc_from_arguments(argument_values: &[Value]) -> Result<f64, RuntimeError> {
    let year = to_number(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    let month = to_number(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    if year.is_nan() || month.is_nan() {
        return Ok(f64::NAN);
    }

    let date = optional_number(argument_values, 2, 1.0)?;
    let hours = optional_number(argument_values, 3, 0.0)?;
    let minutes = optional_number(argument_values, 4, 0.0)?;
    let seconds = optional_number(argument_values, 5, 0.0)?;
    let millis = optional_number(argument_values, 6, 0.0)?;
    if [date, hours, minutes, seconds, millis]
        .into_iter()
        .any(f64::is_nan)
    {
        return Ok(f64::NAN);
    }

    let year = if (0.0..=99.0).contains(&year) {
        year + 1900.0
    } else {
        year
    };
    let month_index = month.trunc() as i32;
    let year = year.trunc() as i32 + month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) + 1;
    let days = days_from_civil(year, month, date.trunc() as i32);
    Ok(days as f64 * MS_PER_DAY
        + hours.trunc() * MS_PER_HOUR
        + minutes.trunc() * MS_PER_MINUTE
        + seconds.trunc() * MS_PER_SECOND
        + millis.trunc())
}

fn optional_number(
    argument_values: &[Value],
    index: usize,
    default: f64,
) -> Result<f64, RuntimeError> {
    argument_values
        .get(index)
        .cloned()
        .map(to_number)
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn parse_date_string(source: &str) -> f64 {
    parse_iso_string(source).map(time_clip).unwrap_or(f64::NAN)
}

fn date_value(this_value: Value) -> Result<f64, RuntimeError> {
    let object = date_object(this_value)?;
    date_value_from_object(&object)
}

fn date_value_from_object(object: &ObjectRef) -> Result<f64, RuntimeError> {
    match object.get(DATE_VALUE_PROPERTY) {
        Some(Value::Number(value)) => Ok(value),
        _ => Err(RuntimeError {
            message: "Date method receiver is not a Date object".to_owned(),
        }),
    }
}

fn date_object(this_value: Value) -> Result<ObjectRef, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(RuntimeError {
            message: "Date method receiver is not an object".to_owned(),
        });
    };
    match object.get(DATE_VALUE_PROPERTY) {
        Some(Value::Number(_)) => Ok(object),
        _ => Err(RuntimeError {
            message: "Date method receiver is not a Date object".to_owned(),
        }),
    }
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

fn time_clip(time: f64) -> f64 {
    if !time.is_finite() || time.abs() > 8_640_000_000_000_000.0 {
        f64::NAN
    } else {
        time.trunc() + 0.0
    }
}

fn utc_time_from_components(year: f64, month: f64, date: f64, time_within_day: f64) -> f64 {
    if [year, month, date, time_within_day]
        .into_iter()
        .any(|value| !value.is_finite())
    {
        return f64::NAN;
    }
    let month_index = month.trunc() as i32;
    let year = year.trunc() as i32 + month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) + 1;
    days_from_civil(year, month, date.trunc() as i32) as f64 * MS_PER_DAY + time_within_day
}

fn time_within_day(time: f64) -> f64 {
    time - (time / MS_PER_DAY).floor() * MS_PER_DAY
}

fn current_time_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as f64)
        .unwrap_or_else(|error| -(error.duration().as_millis() as f64))
}
