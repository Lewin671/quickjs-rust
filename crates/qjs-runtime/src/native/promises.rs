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
        NativeFunction::PromisePrototypeCatch => {
            promise::native_promise_catch(function, this_value, argument_values, env)?
        }
        NativeFunction::PromisePrototypeFinally => {
            promise::native_promise_finally(function, this_value, argument_values, env)?
        }
        NativeFunction::PromisePrototypeFinallyFulfilled => {
            promise::native_promise_finally_fulfilled(function, argument_values, env)?
        }
        NativeFunction::PromisePrototypeFinallyRejected => {
            promise::native_promise_finally_rejected(function, argument_values, env)?
        }
        NativeFunction::PromisePrototypeThen => {
            promise::native_promise_then(function, this_value, argument_values, env)?
        }
        NativeFunction::PromiseReject => {
            promise::native_promise_reject(function, argument_values, env)?
        }
        NativeFunction::PromiseResolve => {
            promise::native_promise_resolve(function, argument_values, env)?
        }
        NativeFunction::PromiseRejectFunction => {
            promise::native_promise_reject_function(function, argument_values, env)?
        }
        NativeFunction::PromiseResolveFunction => {
            promise::native_promise_resolve_function(function, argument_values, env)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
