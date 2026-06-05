use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, promise};

use super::NativeCallResult;

pub(super) fn call_promise_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Promise => {
            promise::native_promise(function, this_value, argument_values, is_construct, env)?
        }
        NativeFunction::PromiseReject => promise::native_promise_reject(function, argument_values)?,
        NativeFunction::PromiseResolve => {
            promise::native_promise_resolve(function, argument_values)?
        }
        NativeFunction::PromiseRejectFunction => {
            promise::native_promise_reject_function(function, argument_values)?
        }
        NativeFunction::PromiseResolveFunction => {
            promise::native_promise_resolve_function(function, argument_values)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
