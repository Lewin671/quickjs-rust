//! Eager iterator helpers: `reduce`, `toArray`, `forEach`, `some`, `every`, and
//! `find`. Each validates the receiver as an object, reads its `next` method,
//! drives the iterator protocol, and closes the iterator (via IteratorClose
//! with a throw completion) when the callback or a derived step completes
//! abruptly.

use std::collections::HashMap;

use crate::{
    ArrayRef, NativeFunction, RuntimeError, Value, call_function, is_truthy, property_value,
};

use super::protocol::{iterator_close_on_throw, iterator_step, iterator_value};
use crate::CallEnv;

/// Validates the receiver and reads its `next` method (GetIteratorDirect).
fn iterator_direct(
    this_value: &Value,
    method: &str,
    env: &mut CallEnv,
) -> Result<(Value, Value), RuntimeError> {
    if !matches!(this_value, Value::Object(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: Iterator.prototype.{method} called on a non-object"),
        });
    }
    let next = property_value(this_value.clone(), "next", env)?;
    Ok((this_value.clone(), next))
}

fn require_callback(
    argument_values: &[Value],
    iterator: &Value,
    method: &str,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        let err = RuntimeError {
            thrown: None,
            message: format!("TypeError: Iterator.prototype.{method} callback is not a function"),
        };
        return Err(iterator_close_on_throw(iterator, err, env));
    }
    Ok(callback)
}

/// Dispatches the eager helpers.
pub(super) fn native_eager_helper(
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match native {
        NativeFunction::IteratorPrototypeReduce => reduce(this_value, argument_values, env),
        NativeFunction::IteratorPrototypeToArray => to_array(this_value, env),
        NativeFunction::IteratorPrototypeForEach => for_each(this_value, argument_values, env),
        NativeFunction::IteratorPrototypeSome => predicate(
            this_value,
            argument_values,
            env,
            "some",
            PredicateKind::Some,
        ),
        NativeFunction::IteratorPrototypeEvery => predicate(
            this_value,
            argument_values,
            env,
            "every",
            PredicateKind::Every,
        ),
        NativeFunction::IteratorPrototypeFind => predicate(
            this_value,
            argument_values,
            env,
            "find",
            PredicateKind::Find,
        ),
        _ => unreachable!("native_eager_helper received a non-eager native"),
    }
}

fn reduce(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (iterator, next) = iterator_direct(&this_value, "reduce", env)?;
    let reducer = require_callback(argument_values, &iterator, "reduce", env)?;

    let mut counter = 0.0_f64;
    let (mut accumulator, has_initial) = match argument_values.get(1) {
        Some(initial) => (initial.clone(), true),
        None => (Value::Undefined, false),
    };

    if !has_initial {
        // No initial value: the first element seeds the accumulator.
        match iterator_step(&iterator, &next, env)? {
            Some(result) => {
                accumulator = iterator_value(result, env)?;
                counter = 1.0;
            }
            None => {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: Reduce of empty iterator with no initial value".to_owned(),
                });
            }
        }
    }

    while let Some(result) = iterator_step(&iterator, &next, env)? {
        let value = iterator_value(result, env)?;
        let outcome = call_function(
            reducer.clone(),
            Value::Undefined,
            vec![accumulator.clone(), value, Value::Number(counter)],
            env,
            false,
        );
        accumulator = match outcome {
            Ok(value) => value,
            Err(error) => return Err(iterator_close_on_throw(&iterator, error, env)),
        };
        counter += 1.0;
    }
    Ok(accumulator)
}

fn to_array(this_value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let (iterator, next) = iterator_direct(&this_value, "toArray", env)?;
    let mut items = Vec::new();
    while let Some(result) = iterator_step(&iterator, &next, env)? {
        items.push(iterator_value(result, env)?);
    }
    Ok(Value::Array(ArrayRef::new(items)))
}

fn for_each(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (iterator, next) = iterator_direct(&this_value, "forEach", env)?;
    let callback = require_callback(argument_values, &iterator, "forEach", env)?;
    let mut counter = 0.0_f64;
    while let Some(result) = iterator_step(&iterator, &next, env)? {
        let value = iterator_value(result, env)?;
        let outcome = call_function(
            callback.clone(),
            Value::Undefined,
            vec![value, Value::Number(counter)],
            env,
            false,
        );
        if let Err(error) = outcome {
            return Err(iterator_close_on_throw(&iterator, error, env));
        }
        counter += 1.0;
    }
    Ok(Value::Undefined)
}

enum PredicateKind {
    Some,
    Every,
    Find,
}

fn predicate(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
    method: &str,
    kind: PredicateKind,
) -> Result<Value, RuntimeError> {
    let (iterator, next) = iterator_direct(&this_value, method, env)?;
    let callback = require_callback(argument_values, &iterator, method, env)?;
    let mut counter = 0.0_f64;
    while let Some(result) = iterator_step(&iterator, &next, env)? {
        let value = iterator_value(result, env)?;
        let outcome = call_function(
            callback.clone(),
            Value::Undefined,
            vec![value.clone(), Value::Number(counter)],
            env,
            false,
        );
        let truthy = match outcome {
            Ok(value) => is_truthy(&value),
            Err(error) => return Err(iterator_close_on_throw(&iterator, error, env)),
        };
        match kind {
            PredicateKind::Some => {
                if truthy {
                    // A `true` result closes the iterator and returns true.
                    super::protocol::iterator_close(&iterator, env)?;
                    return Ok(Value::Boolean(true));
                }
            }
            PredicateKind::Every => {
                if !truthy {
                    super::protocol::iterator_close(&iterator, env)?;
                    return Ok(Value::Boolean(false));
                }
            }
            PredicateKind::Find => {
                if truthy {
                    super::protocol::iterator_close(&iterator, env)?;
                    return Ok(value);
                }
            }
        }
        counter += 1.0;
    }
    Ok(match kind {
        PredicateKind::Some => Value::Boolean(false),
        PredicateKind::Every => Value::Boolean(true),
        PredicateKind::Find => Value::Undefined,
    })
}
