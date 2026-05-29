use crate::{
    RuntimeError, Value,
    date::{
        iso::utc_date_time,
        value::{
            date_object, date_value_from_object, define_date_value, optional_number, time_clip,
            time_within_day, utc_time_from_components,
        },
    },
    to_number,
};

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
