use crate::CallEnv;
use crate::{
    Function, PreferredType, RuntimeError, Value,
    date::{
        MS_PER_DAY,
        value::{
            current_time_ms, define_date_value, parse_date_string, time_clip, time_from_components,
        },
    },
    to_number_with_env, to_primitive_with_hint,
};

pub(crate) fn native_date(
    _function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Ok(Value::String(super::iso::format_local_string(
            current_time_ms(),
        )));
    }

    let Value::Object(object) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Date constructor requires an object receiver".to_owned(),
        });
    };
    let date_value = construct_date_value(argument_values, env)?;
    define_date_value(&object, date_value);
    Ok(Value::Object(object))
}

pub(crate) fn native_date_now() -> Result<Value, RuntimeError> {
    Ok(Value::Number(current_time_ms()))
}

pub(crate) fn native_date_parse(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::String(source)) => Ok(Value::Number(parse_date_string(source))),
        Some(value) => Ok(Value::Number(parse_date_string(
            &crate::to_js_string_with_env(value.clone(), env)?,
        ))),
        None => Ok(Value::Number(f64::NAN)),
    }
}

pub(crate) fn native_date_utc(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    Ok(Value::Number(time_clip(date_utc_from_arguments(
        argument_values,
        env,
    )?)))
}

fn construct_date_value(argument_values: &[Value], env: &mut CallEnv) -> Result<f64, RuntimeError> {
    if argument_values.is_empty() {
        return Ok(current_time_ms());
    }
    if argument_values.len() == 1 {
        return match &argument_values[0] {
            Value::String(source) => Ok(parse_date_string(source)),
            value => {
                let primitive = to_primitive_with_hint(value.clone(), PreferredType::Default, env)?;
                match primitive {
                    Value::String(source) => Ok(parse_date_string(&source)),
                    value => to_number_with_env(value, env).map(time_clip),
                }
            }
        };
    }
    date_utc_from_arguments(argument_values, env).map(time_clip)
}

fn date_utc_from_arguments(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<f64, RuntimeError> {
    let year = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let month = optional_utc_number(argument_values, 1, 0.0, env)?;
    let date = optional_utc_number(argument_values, 2, 1.0, env)?;
    let hours = optional_utc_number(argument_values, 3, 0.0, env)?;
    let minutes = optional_utc_number(argument_values, 4, 0.0, env)?;
    let seconds = optional_utc_number(argument_values, 5, 0.0, env)?;
    let millis = optional_utc_number(argument_values, 6, 0.0, env)?;
    if [year, month, date, hours, minutes, seconds, millis]
        .into_iter()
        .any(|value| !value.is_finite())
    {
        return Ok(f64::NAN);
    }

    let year_integer = year.trunc();
    let year = if (0.0..=99.0).contains(&year_integer) {
        year_integer + 1900.0
    } else {
        year
    };
    Ok(make_day(year, month, date) * MS_PER_DAY
        + time_from_components(hours, minutes, seconds, millis))
}

fn optional_utc_number(
    argument_values: &[Value],
    index: usize,
    default: f64,
    env: &mut CallEnv,
) -> Result<f64, RuntimeError> {
    argument_values
        .get(index)
        .cloned()
        .map(|value| to_number_with_env(value, env))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn make_day(year: f64, month: f64, date: f64) -> f64 {
    let month = month.trunc();
    let month_years = (month / 12.0).floor();
    let year = year.trunc() + month_years;
    let month = month - month_years * 12.0;
    days_from_civil_number(year, month + 1.0, 1.0) + date.trunc() - 1.0
}

fn days_from_civil_number(year: f64, month: f64, day: f64) -> f64 {
    let year = year - if month <= 2.0 { 1.0 } else { 0.0 };
    let era = (year / 400.0).floor();
    let year_of_era = year - era * 400.0;
    let month_for_year = month + if month > 2.0 { -3.0 } else { 9.0 };
    let day_of_year = ((153.0 * month_for_year + 2.0) / 5.0).floor() + day - 1.0;
    let day_of_era = year_of_era * 365.0 + (year_of_era / 4.0).floor()
        - (year_of_era / 100.0).floor()
        + day_of_year;
    era * 146_097.0 + day_of_era - 719_468.0
}
