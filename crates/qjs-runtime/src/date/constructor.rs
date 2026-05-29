use crate::{
    Function, RuntimeError, Value,
    date::{
        MS_PER_DAY, MS_PER_HOUR, MS_PER_MINUTE, MS_PER_SECOND,
        iso::days_from_civil,
        value::{
            current_time_ms, define_date_value, optional_number, parse_date_string, time_clip,
        },
    },
    to_number,
};

pub(crate) fn native_date(
    _function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Ok(Value::String(super::iso::format_local_string(
            current_time_ms(),
        )));
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
        Some(value) => Ok(Value::Number(parse_date_string(&crate::to_js_string(
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
