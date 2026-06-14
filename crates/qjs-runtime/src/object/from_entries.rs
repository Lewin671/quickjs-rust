use std::collections::HashMap;

use crate::{
    ObjectRef, Property, PropertyKey, RuntimeError, Value, call_function, is_truthy,
    object_prototype, property_value, property_value_key, symbol, to_property_key_value,
};

use crate::CallEnv;

/// `Object.fromEntries ( iterable )` -- ES2025 20.1.2.8
///
/// Drives the iterator protocol manually so that:
///
/// * Non-object entries (null, strings, numbers, etc.) cause a TypeError *and*
///   the iterator is closed via its `return` method (IteratorClose).
/// * If reading property `"0"` (key) or `"1"` (value) throws, or if
///   `ToPropertyKey` on the key throws, the iterator is closed before the
///   error propagates.
/// * Evaluation order per entry matches the spec: `next()` -> `Get(entry, "0")`
///   -> `Get(entry, "1")` -> `ToPropertyKey(key)` -> `CreateDataPropertyOrThrow`.
pub(crate) fn native_object_from_entries(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iterable = argument_values.first().cloned().unwrap_or(Value::Undefined);

    // GetIterator(iterable, sync)
    let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.fromEntries iterator symbol is unavailable".to_owned(),
        });
    };
    let iterator_method =
        property_value_key(iterable.clone(), &PropertyKey::Symbol(iterator_symbol), env)?;
    if !matches!(iterator_method, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Object.fromEntries requires an iterable argument".to_owned(),
        });
    }
    let iterator = call_function(iterator_method, iterable, Vec::new(), env, false)?;
    let next = property_value(iterator.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Object.fromEntries iterator next method is not callable"
                .to_owned(),
        });
    }

    let result = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));

    loop {
        // IteratorStep
        let step = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
        if !is_object_like(&step) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Object.fromEntries iterator result is not an object"
                    .to_owned(),
            });
        }
        if is_truthy(&property_value(step.clone(), "done", env)?) {
            break;
        }
        let entry = property_value(step, "value", env)?;

        // AddEntriesFromIterable step 4.d: If Type(nextItem) is not Object,
        // throw a TypeError and close the iterator.
        if !is_object_like(&entry) {
            let error = RuntimeError {
                thrown: None,
                message: "TypeError: Object.fromEntries entry must be an object".to_owned(),
            };
            return Err(iterator_close_on_throw(&iterator, error, env));
        }

        // Get(nextItem, "0") -- key
        let raw_key = match property_value(entry.clone(), "0", env) {
            Ok(value) => value,
            Err(error) => return Err(iterator_close_on_throw(&iterator, error, env)),
        };

        // Get(nextItem, "1") -- value
        let value = match property_value(entry, "1", env) {
            Ok(value) => value,
            Err(error) => return Err(iterator_close_on_throw(&iterator, error, env)),
        };

        // ToPropertyKey(key)
        let key = match to_property_key_value(raw_key, env) {
            Ok(key) => key,
            Err(error) => return Err(iterator_close_on_throw(&iterator, error, env)),
        };

        match key {
            PropertyKey::String(key) => {
                result.define_property(key, Property::enumerable(value));
            }
            PropertyKey::Symbol(symbol) => {
                result.define_symbol_property(symbol, Property::enumerable(value));
            }
        }
    }

    Ok(Value::Object(result))
}

/// Returns `true` when the value is an ECMAScript Object type (Object, Array,
/// Function, Map, Set, or Proxy). Null, undefined, strings, numbers, booleans,
/// bigints, and symbols are *not* objects.
fn is_object_like(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)
    )
}

/// IteratorClose for an abrupt (throw) completion: calls the iterator's
/// `return` method for its side effects but always re-raises the original
/// `error`, discarding any error from `return` (ES2025 7.4.6 with a throw
/// completion).
fn iterator_close_on_throw(
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
