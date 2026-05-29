use crate::{Function, NativeFunction, Value, date};

use super::NativeCallResult;

pub(super) fn call_date_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Date => {
            date::native_date(function, this_value, argument_values, is_construct)?
        }
        NativeFunction::DateNow => date::native_date_now()?,
        NativeFunction::DateParse => date::native_date_parse(argument_values)?,
        NativeFunction::DateUtc => date::native_date_utc(argument_values)?,
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
        NativeFunction::DatePrototypeToISOString => {
            date::native_date_prototype_to_iso_string(this_value)?
        }
        NativeFunction::DatePrototypeValueOf => date::native_date_prototype_value_of(this_value)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
