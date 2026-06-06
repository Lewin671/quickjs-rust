use std::collections::HashMap;

use crate::{Function, RuntimeError, Value, call_function, ensure_constructor};

use super::{
    PROMISE_REJECTED, initialize_promise, promise_object_from_function, resolve_promise,
    settle_promise,
};

pub(crate) fn native_promise_try(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    ensure_constructor(&this_value)?;
    let promise = promise_object_from_function(function);
    initialize_promise(&promise);
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let arguments = argument_values.get(1..).unwrap_or(&[]).to_vec();
    match call_function(callback, Value::Undefined, arguments, env, false) {
        Ok(value) => resolve_promise(&promise, value, env),
        Err(error) => settle_promise(
            &promise,
            PROMISE_REJECTED,
            error.thrown.map_or(Value::Undefined, |value| *value),
            env,
        ),
    }
    Ok(Value::Object(promise))
}
