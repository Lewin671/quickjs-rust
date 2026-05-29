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
        NativeFunction::DatePrototypeGetTime => date::native_date_prototype_get_time(this_value)?,
        NativeFunction::DatePrototypeToISOString => {
            date::native_date_prototype_to_iso_string(this_value)?
        }
        NativeFunction::DatePrototypeValueOf => date::native_date_prototype_value_of(this_value)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
