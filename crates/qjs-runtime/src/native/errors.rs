use crate::{Function, NativeFunction, Value, error};

use super::NativeCallResult;

pub(super) fn call_error_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Error => {
            error::native_error(function, this_value, argument_values, is_construct)?
        }
        NativeFunction::ErrorPrototypeToString => {
            error::native_error_prototype_to_string(this_value)?
        }
        native if error::is_native_error(native) => {
            error::native_error(function, this_value, argument_values, is_construct)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
