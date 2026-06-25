use crate::{Function, RuntimeError, Value, call_function};

use super::capability::{self, new_promise_capability};
use crate::CallEnv;

/// `Promise.try` (ES2025): builds a capability from the `this`
/// constructor, runs the callback synchronously, and resolves/rejects the
/// capability with its outcome.
pub(crate) fn native_promise_try(
    _function: &Function,
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let capability = new_promise_capability(&this_value, env)?;
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let arguments = argument_values.get(1..).unwrap_or(&[]).to_vec();
    match call_function(callback, Value::Undefined, arguments, env, false) {
        Ok(value) => {
            capability::capability_resolve(&capability, value, env)?;
        }
        Err(error) => {
            let reason = crate::error::runtime_error_to_value(error, env);
            capability::capability_reject(&capability, reason, env)?;
        }
    }
    Ok(capability.promise)
}
