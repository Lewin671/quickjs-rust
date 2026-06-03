use std::collections::HashMap;

use crate::{
    RuntimeError, Value, call_function,
    date::iso::{
        format_date_string, format_iso_string, format_local_string, format_time_string,
        format_utc_string,
    },
};

use super::value::date_value;

pub(crate) fn native_date_prototype_to_iso_string(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let millis = date_value(this_value)?;
    if !millis.is_finite() {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: Invalid time value".to_owned(),
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

pub(crate) fn native_date_prototype_to_date_string(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    format_date_value(this_value, format_date_string)
}

pub(crate) fn native_date_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    format_date_value(this_value, format_local_string)
}

pub(crate) fn native_date_prototype_to_time_string(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    format_date_value(this_value, format_time_string)
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
        thrown: None,
        message: "Date toJSON receiver does not have a toISOString method".to_owned(),
    })?;
    call_function(to_iso_string, this_value, vec![key], env, false)
}

fn format_date_value(
    this_value: Value,
    formatter: impl FnOnce(f64) -> String,
) -> Result<Value, RuntimeError> {
    let millis = date_value(this_value)?;
    if !millis.is_finite() {
        return Ok(Value::String("Invalid Date".to_owned()));
    }
    Ok(Value::String(formatter(millis)))
}
