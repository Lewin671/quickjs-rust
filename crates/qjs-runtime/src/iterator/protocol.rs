//! Shared iterator-protocol primitives used by the eager helpers and the lazy
//! Iterator Helper objects: `IteratorStep`, `IteratorValue`, and the two
//! `IteratorClose` variants (swallowing for a normal completion, preserving the
//! pending error for an abrupt one).

use crate::CallEnv;
use crate::{RuntimeError, Value, call_function, is_truthy, property_value};

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}

/// IteratorStep: calls `next` on `iterator`, validates the result object, and
/// returns the result object when not done, or `None` when the iterator is
/// exhausted.
pub(super) fn iterator_step(
    iterator: &Value,
    next: &Value,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let result = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
    if !is_object_value(&result) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: iterator result is not an object".to_owned(),
        });
    }
    if is_truthy(&property_value(result.clone(), "done", env)?) {
        return Ok(None);
    }
    Ok(Some(result))
}

/// IteratorValue: reads `value` off an iterator result object.
pub(super) fn iterator_value(result: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    property_value(result, "value", env)
}

/// IteratorClose for a normal completion: calls `return` if present and
/// propagates any error it raises (but ignores a non-object return result, per
/// the normal-completion branch of IteratorClose, which still requires an
/// object).
pub(super) fn iterator_close(iterator: &Value, env: &mut CallEnv) -> Result<(), RuntimeError> {
    let return_method = property_value(iterator.clone(), "return", env)?;
    if matches!(return_method, Value::Null | Value::Undefined) {
        return Ok(());
    }
    let result = call_function(return_method, iterator.clone(), Vec::new(), env, false)?;
    if is_object_value(&result) {
        return Ok(());
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: iterator return result must be an object".to_owned(),
    })
}

/// IteratorClose for an abrupt (throw) completion: calls `return` for its side
/// effects but always re-raises the original `error`, discarding any error or
/// non-object result from `return` (27.1 IteratorClose with a throw
/// completion).
pub(super) fn iterator_close_on_throw(
    iterator: &Value,
    error: RuntimeError,
    env: &mut CallEnv,
) -> RuntimeError {
    if let Ok(return_method) = property_value(iterator.clone(), "return", env)
        && !matches!(return_method, Value::Null | Value::Undefined)
    {
        let _ = call_function(return_method, iterator.clone(), Vec::new(), env, false);
    }
    error
}
