use std::collections::HashMap;

use crate::{Function, NativeFunction, RuntimeError, Value, array};

use super::{
    PROMISE_REJECTED, call_promise_then, initialize_promise, is_promise_value,
    promise_object_from_function, resolve_promise, resolving_function, settle_promise,
};

pub(crate) fn native_promise_race(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let promise = promise_object_from_function(function);
    initialize_promise(&promise);
    let items = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let values = match array::array_like_values_with_env(items, "Promise.race", env) {
        Ok(values) => values,
        Err(error) => {
            settle_promise(
                &promise,
                PROMISE_REJECTED,
                error
                    .thrown
                    .map_or(Value::String(error.message), |value| *value),
                env,
            );
            return Ok(Value::Object(promise));
        }
    };

    let resolve = resolving_function(
        "resolve",
        NativeFunction::PromiseResolveFunction,
        Value::Object(promise.clone()),
    );
    let reject = resolving_function(
        "reject",
        NativeFunction::PromiseRejectFunction,
        Value::Object(promise.clone()),
    );

    for value in values {
        let element_promise = if is_promise_value(&value) {
            value
        } else {
            let element_promise = promise_object_from_function(function);
            initialize_promise(&element_promise);
            resolve_promise(&element_promise, value, env);
            Value::Object(element_promise)
        };
        call_promise_then(element_promise, vec![resolve.clone(), reject.clone()], env)?;
    }

    Ok(Value::Object(promise))
}
