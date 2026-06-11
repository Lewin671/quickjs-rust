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
        NativeFunction::PromiseAll => {
            promise::native_promise_all(this_value, argument_values, env)?
        }
        NativeFunction::PromiseAny => {
            promise::any::native_promise_any(function, this_value, argument_values, env)?
        }
        NativeFunction::PromiseAnyRejectElement => {
            promise::any::native_promise_any_reject_element(function, argument_values, env)?
        }
        NativeFunction::PromiseAllSettled => {
            promise::all_settled::native_promise_all_settled(this_value, argument_values, env)?
        }
        NativeFunction::PromiseAllSettledRejectElement => {
            promise::all_settled::native_promise_all_settled_reject_element(
                function,
                argument_values,
                env,
            )?
        }
        NativeFunction::PromiseAllSettledResolveElement => {
            promise::all_settled::native_promise_all_settled_resolve_element(
                function,
                argument_values,
                env,
            )?
        }
        NativeFunction::PromiseAllResolveElement => {
            promise::native_promise_all_resolve_element(function, argument_values, env)?
        }
        NativeFunction::PromiseGetCapabilitiesExecutor => {
            promise::native_get_capabilities_executor(function, argument_values, env)?
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
        NativeFunction::PromisePrototypeFinallyValueThunk => {
            promise::native_promise_finally_value_thunk(function)?
        }
        NativeFunction::PromisePrototypeFinallyThrowerThunk => {
            promise::native_promise_finally_thrower_thunk(function)?
        }
        NativeFunction::PromisePrototypeThen => {
            promise::native_promise_then(function, this_value, argument_values, env)?
        }
        NativeFunction::PromiseRace => {
            promise::native_promise_race(this_value, argument_values, env)?
        }
        NativeFunction::PromiseReject => {
            promise::native_promise_reject(this_value, argument_values, env)?
        }
        NativeFunction::PromiseResolve => {
            promise::native_promise_resolve(this_value, argument_values, env)?
        }
        NativeFunction::PromiseTry => {
            promise::r#try::native_promise_try(function, this_value, argument_values, env)?
        }
        NativeFunction::PromiseWithResolvers => {
            promise::with_resolvers::native_promise_with_resolvers(function, this_value, env)?
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
