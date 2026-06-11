use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, date};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_date_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Date => {
            date::native_date(function, this_value, argument_values, is_construct, env)?
        }
        NativeFunction::DateNow => date::native_date_now()?,
        NativeFunction::DateParse => date::native_date_parse(argument_values)?,
        NativeFunction::DateUtc => date::native_date_utc(argument_values)?,
        NativeFunction::DatePrototypeGetTimezoneOffset => {
            date::native_date_prototype_get_timezone_offset(this_value)?
        }
        NativeFunction::DatePrototypeGetYear => date::native_date_prototype_get_year(this_value)?,
        NativeFunction::DatePrototypeGetUtcDate => {
            date::native_date_prototype_get_utc_date(this_value)?
        }
        NativeFunction::DatePrototypeGetUtcDay => {
            date::native_date_prototype_get_utc_day(this_value)?
        }
        NativeFunction::DatePrototypeGetUtcFullYear => {
            date::native_date_prototype_get_utc_full_year(this_value)?
        }
        NativeFunction::DatePrototypeGetUtcHours => {
            date::native_date_prototype_get_utc_hours(this_value)?
        }
        NativeFunction::DatePrototypeGetUtcMilliseconds => {
            date::native_date_prototype_get_utc_milliseconds(this_value)?
        }
        NativeFunction::DatePrototypeGetUtcMinutes => {
            date::native_date_prototype_get_utc_minutes(this_value)?
        }
        NativeFunction::DatePrototypeGetUtcMonth => {
            date::native_date_prototype_get_utc_month(this_value)?
        }
        NativeFunction::DatePrototypeGetUtcSeconds => {
            date::native_date_prototype_get_utc_seconds(this_value)?
        }
        NativeFunction::DatePrototypeGetTime => date::native_date_prototype_get_time(this_value)?,
        NativeFunction::DatePrototypeSetYear => {
            date::native_date_prototype_set_year(this_value, argument_values, env)?
        }
        NativeFunction::DatePrototypeSetTime => {
            date::native_date_prototype_set_time(this_value, argument_values)?
        }
        NativeFunction::DatePrototypeSetUtcDate => {
            date::native_date_prototype_set_utc_date(this_value, argument_values)?
        }
        NativeFunction::DatePrototypeSetUtcFullYear => {
            date::native_date_prototype_set_utc_full_year(this_value, argument_values)?
        }
        NativeFunction::DatePrototypeSetUtcHours => {
            date::native_date_prototype_set_utc_hours(this_value, argument_values)?
        }
        NativeFunction::DatePrototypeSetUtcMilliseconds => {
            date::native_date_prototype_set_utc_milliseconds(this_value, argument_values)?
        }
        NativeFunction::DatePrototypeSetUtcMinutes => {
            date::native_date_prototype_set_utc_minutes(this_value, argument_values)?
        }
        NativeFunction::DatePrototypeSetUtcMonth => {
            date::native_date_prototype_set_utc_month(this_value, argument_values)?
        }
        NativeFunction::DatePrototypeSetUtcSeconds => {
            date::native_date_prototype_set_utc_seconds(this_value, argument_values)?
        }
        NativeFunction::DatePrototypeToDateString => {
            date::native_date_prototype_to_date_string(this_value)?
        }
        NativeFunction::DatePrototypeToISOString => {
            date::native_date_prototype_to_iso_string(this_value)?
        }
        NativeFunction::DatePrototypeToJson => date::native_date_prototype_to_json(
            this_value,
            argument_values.first().cloned().unwrap_or(Value::Undefined),
            env,
        )?,
        NativeFunction::DatePrototypeToPrimitive => date::native_date_prototype_to_primitive(
            this_value,
            argument_values.first().cloned().unwrap_or(Value::Undefined),
            env,
        )?,
        NativeFunction::DatePrototypeToString => date::native_date_prototype_to_string(this_value)?,
        NativeFunction::DatePrototypeToTimeString => {
            date::native_date_prototype_to_time_string(this_value)?
        }
        NativeFunction::DatePrototypeToUtcString => {
            date::native_date_prototype_to_utc_string(this_value)?
        }
        NativeFunction::DatePrototypeValueOf => date::native_date_prototype_value_of(this_value)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
