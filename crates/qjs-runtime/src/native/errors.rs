use crate::{Function, NativeFunction, Value, error};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_error_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Error => {
            error::native_error(function, this_value, argument_values, is_construct, env)?
        }
        NativeFunction::AggregateError => {
            error::native_aggregate_error(function, this_value, argument_values, is_construct, env)?
        }
        NativeFunction::SuppressedError => error::native_suppressed_error(
            function,
            this_value,
            argument_values,
            is_construct,
            env,
        )?,
        NativeFunction::ErrorIsError => error::native_error_is_error(argument_values),
        NativeFunction::ErrorPrototypeToString => {
            error::native_error_prototype_to_string(this_value, env)?
        }
        native if error::is_native_error(native) => {
            error::native_error(function, this_value, argument_values, is_construct, env)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
