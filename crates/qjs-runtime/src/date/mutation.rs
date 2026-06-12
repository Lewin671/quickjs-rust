use crate::CallEnv;
use crate::{
    RuntimeError, Value,
    date::{
        iso::utc_date_time,
        value::{
            date_object, date_value_from_object, define_date_value, optional_number, time_clip,
            time_from_components, time_within_day, utc_time_from_components,
        },
    },
    to_number_with_env,
};

pub(crate) fn native_date_prototype_set_time(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = date_object(this_value)?;
    let time = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let clipped = time_clip(time);
    define_date_value(&object, clipped);
    Ok(Value::Number(clipped))
}

pub(crate) fn native_date_prototype_set_year(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = date_object(this_value)?;
    let millis = date_value_from_object(&object)?;
    let year = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if year.is_nan() {
        define_date_value(&object, f64::NAN);
        return Ok(Value::Number(f64::NAN));
    }

    let base = if millis.is_nan() { 0.0 } else { millis };
    let components = utc_date_time(base);
    let integer_year = year.trunc();
    let full_year = if (0.0..=99.0).contains(&integer_year) {
        integer_year + 1900.0
    } else {
        year
    };
    let updated = time_clip(utc_time_from_components(
        full_year,
        f64::from(components.month),
        f64::from(components.date),
        time_within_day(base),
    ));
    define_date_value(&object, updated);
    Ok(Value::Number(updated))
}

pub(crate) fn native_date_prototype_set_utc_full_year(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = date_object(this_value)?;
    let millis = date_value_from_object(&object)?;
    let base = if millis.is_nan() { 0.0 } else { millis };
    let components = utc_date_time(base);
    let year = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
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

pub(crate) fn native_date_prototype_set_utc_date(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = date_object(this_value)?;
    let millis = date_value_from_object(&object)?;
    let components = if millis.is_finite() {
        utc_date_time(millis)
    } else {
        utc_date_time(0.0)
    };
    let date = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let updated = if millis.is_finite() {
        time_clip(utc_time_from_components(
            f64::from(components.year),
            f64::from(components.month),
            date,
            time_within_day(millis),
        ))
    } else {
        f64::NAN
    };
    define_date_value(&object, updated);
    Ok(Value::Number(updated))
}

pub(crate) fn native_date_prototype_set_utc_hours(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    set_utc_time_fields(this_value, argument_values, 0, env)
}

pub(crate) fn native_date_prototype_set_utc_milliseconds(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    set_utc_time_fields(this_value, argument_values, 3, env)
}

pub(crate) fn native_date_prototype_set_utc_minutes(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    set_utc_time_fields(this_value, argument_values, 1, env)
}

pub(crate) fn native_date_prototype_set_utc_month(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = date_object(this_value)?;
    let millis = date_value_from_object(&object)?;
    let components = if millis.is_finite() {
        utc_date_time(millis)
    } else {
        utc_date_time(0.0)
    };
    let month = to_number_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let date = optional_number(argument_values, 1, f64::from(components.date))?;
    let updated = if millis.is_finite() {
        time_clip(utc_time_from_components(
            f64::from(components.year),
            month,
            date,
            time_within_day(millis),
        ))
    } else {
        f64::NAN
    };
    define_date_value(&object, updated);
    Ok(Value::Number(updated))
}

pub(crate) fn native_date_prototype_set_utc_seconds(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    set_utc_time_fields(this_value, argument_values, 2, env)
}

fn set_utc_time_fields(
    this_value: Value,
    argument_values: &[Value],
    first_field: usize,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = date_object(this_value)?;
    let millis = date_value_from_object(&object)?;
    let components = if millis.is_finite() {
        utc_date_time(millis)
    } else {
        utc_date_time(0.0)
    };
    let mut time_fields = [
        f64::from(components.hours),
        f64::from(components.minutes),
        f64::from(components.seconds),
        f64::from(components.milliseconds),
    ];
    for (offset, argument) in argument_values.iter().take(4 - first_field).enumerate() {
        time_fields[first_field + offset] = to_number_with_env(argument.clone(), env)?.trunc();
    }
    let updated = if argument_values.is_empty() {
        f64::NAN
    } else if millis.is_finite() {
        time_clip(
            millis - time_within_day(millis)
                + time_from_components(
                    time_fields[0],
                    time_fields[1],
                    time_fields[2],
                    time_fields[3],
                ),
        )
    } else {
        f64::NAN
    };
    define_date_value(&object, updated);
    Ok(Value::Number(updated))
}
