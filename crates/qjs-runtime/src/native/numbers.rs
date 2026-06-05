use crate::{Function, NativeFunction, Value, number};

use super::NativeCallResult;

pub(super) fn call_number_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Number => {
            number::native_number(function, this_value, argument_values, is_construct)?
        }
        NativeFunction::NumberIsFinite => number::native_number_is_finite(argument_values)?,
        NativeFunction::NumberIsInteger => number::native_number_is_integer(argument_values)?,
        NativeFunction::NumberIsNaN => number::native_number_is_nan(argument_values)?,
        NativeFunction::NumberIsSafeInteger => {
            number::native_number_is_safe_integer(argument_values)?
        }
        NativeFunction::NumberPrototypeToExponential => {
            number::native_number_prototype_to_exponential(this_value, argument_values)?
        }
        NativeFunction::NumberPrototypeToFixed => {
            number::native_number_prototype_to_fixed(this_value, argument_values)?
        }
        NativeFunction::NumberPrototypeToPrecision => {
            number::native_number_prototype_to_precision(this_value, argument_values)?
        }
        NativeFunction::NumberPrototypeToString => {
            number::native_number_prototype_to_string(this_value, argument_values)?
        }
        NativeFunction::NumberPrototypeValueOf => {
            number::native_number_prototype_value_of(this_value)?
        }
        NativeFunction::ParseFloat => number::native_parse_float(argument_values)?,
        NativeFunction::ParseInt => number::native_parse_int(argument_values)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
