use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    Function, ObjectRef, RuntimeError, Value,
    date::{
        DATE_VALUE_PROPERTY, MS_PER_DAY, MS_PER_HOUR, MS_PER_MINUTE, MS_PER_SECOND,
        iso::{days_from_civil, format_iso_string, parse_iso_string, utc_date_time},
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
        Some(Value::String(source)) => {
            Ok(Value::Number(parse_iso_string(source).unwrap_or(f64::NAN)))
        }
        Some(value) => Ok(Value::Number(
            parse_iso_string(&to_js_string(value.clone())?).unwrap_or(f64::NAN),
        )),
        None => Ok(Value::Number(f64::NAN)),
    }
}

pub(crate) fn native_date_utc(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Number(date_utc_from_arguments(argument_values)?))
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

pub(super) fn define_date_value(object: &ObjectRef, value: f64) {
    object.define_non_enumerable(DATE_VALUE_PROPERTY.to_owned(), Value::Number(value));
}

fn construct_date_value(argument_values: &[Value]) -> Result<f64, RuntimeError> {
    if argument_values.is_empty() {
        return Ok(current_time_ms());
    }
    if argument_values.len() == 1 {
        return match &argument_values[0] {
            Value::String(source) => Ok(parse_iso_string(source).unwrap_or(f64::NAN)),
            value => to_number(value.clone()),
        };
    }
    date_utc_from_arguments(argument_values)
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

fn date_value(this_value: Value) -> Result<f64, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(RuntimeError {
            message: "Date method receiver is not an object".to_owned(),
        });
    };
    match object.get(DATE_VALUE_PROPERTY) {
        Some(Value::Number(value)) => Ok(value),
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

fn current_time_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as f64)
        .unwrap_or_else(|error| -(error.duration().as_millis() as f64))
}
