use std::collections::HashMap;

use crate::{
    RuntimeError, Value, call_function,
    date::iso::{format_iso_string, format_utc_string},
};

use super::value::date_value;

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
