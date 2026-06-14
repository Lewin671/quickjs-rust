use crate::{
    PreferredType, RuntimeError, Value, call_function,
    date::iso::{
        format_date_string, format_iso_string, format_local_string, format_time_string,
        format_utc_string,
    },
    ordinary_to_primitive,
};

use super::value::date_value;
use crate::CallEnv;

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
    _key: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let tv = crate::to_primitive_with_hint(
        this_value.clone(),
        crate::conversion::PreferredType::Number,
        env,
    )?;
    if let Value::Number(n) = &tv {
        if n.is_nan() || n.is_infinite() {
            return Ok(Value::Null);
        }
    }

    let to_iso_string = crate::property_value(this_value.clone(), "toISOString", env)?;
    match to_iso_string {
        Value::Function(_) => call_function(to_iso_string, this_value, vec![], env, false),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: toJSON receiver does not have a toISOString method".to_owned(),
        }),
    }
}

pub(crate) fn native_date_prototype_to_primitive(
    this_value: Value,
    hint: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !matches!(
        this_value,
        Value::Object(_) | Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_)
    ) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Date.prototype[Symbol.toPrimitive] receiver must be an object"
                .to_owned(),
        });
    }

    let hint = match hint {
        Value::String(hint) if hint == "string" || hint == "default" => PreferredType::String,
        Value::String(hint) if hint == "number" => PreferredType::Number,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: invalid Date.prototype[Symbol.toPrimitive] hint".to_owned(),
            });
        }
    };
    ordinary_to_primitive(this_value, hint, env)
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
