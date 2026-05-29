use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    ObjectRef, RuntimeError, Value,
    date::{
        DATE_VALUE_PROPERTY, MS_PER_DAY,
        iso::{days_from_civil, parse_iso_string},
    },
    to_number,
};

pub(super) fn define_date_value(object: &ObjectRef, value: f64) {
    object.define_non_enumerable(DATE_VALUE_PROPERTY.to_owned(), Value::Number(value));
}

pub(super) fn optional_number(
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

pub(super) fn parse_date_string(source: &str) -> f64 {
    parse_iso_string(source).map(time_clip).unwrap_or(f64::NAN)
}

pub(super) fn date_value(this_value: Value) -> Result<f64, RuntimeError> {
    let object = date_object(this_value)?;
    date_value_from_object(&object)
}

pub(super) fn date_value_from_object(object: &ObjectRef) -> Result<f64, RuntimeError> {
    match object.get(DATE_VALUE_PROPERTY) {
        Some(Value::Number(value)) => Ok(value),
        _ => Err(RuntimeError {
            message: "Date method receiver is not a Date object".to_owned(),
        }),
    }
}

pub(super) fn date_object(this_value: Value) -> Result<ObjectRef, RuntimeError> {
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

pub(super) fn time_clip(time: f64) -> f64 {
    if !time.is_finite() || time.abs() > 8_640_000_000_000_000.0 {
        f64::NAN
    } else {
        time.trunc() + 0.0
    }
}

pub(super) fn utc_time_from_components(
    year: f64,
    month: f64,
    date: f64,
    time_within_day: f64,
) -> f64 {
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

pub(super) fn time_within_day(time: f64) -> f64 {
    time - (time / MS_PER_DAY).floor() * MS_PER_DAY
}

pub(super) fn current_time_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as f64)
        .unwrap_or_else(|error| -(error.duration().as_millis() as f64))
}
