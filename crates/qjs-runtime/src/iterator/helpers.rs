//! Lazy iterator helpers (`map`/`filter`/`take`/`drop`/`flatMap`) and the
//! `%IteratorHelperPrototype%` `next`/`return` methods.
//!
//! Each helper validates the receiver, coerces its arguments, and returns an
//! Iterator Helper object whose internal state lives in `\0`-prefixed own
//! properties: the underlying iterator and its `next` method, the helper kind,
//! the callback or remaining count, a per-element counter, and (for `flatMap`)
//! the in-progress inner iterator. The helper's `next` advances the underlying
//! iterator, applying the transform, and closes the underlying iterator on any
//! abrupt completion; `return` closes the underlying iterator and marks the
//! helper done.

use std::collections::HashMap;

use crate::{
    NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value, call_function,
    is_truthy, property_value, property_value_key, symbol, to_number_with_env,
};

use super::protocol::{iterator_close_on_throw, iterator_step, iterator_value};
use crate::CallEnv;

const HELPER_KIND: &str = "\0iterator_helper_kind";
const HELPER_UNDERLYING: &str = "\0iterator_helper_underlying";
const HELPER_NEXT: &str = "\0iterator_helper_next";
const HELPER_CALLBACK: &str = "\0iterator_helper_callback";
const HELPER_REMAINING: &str = "\0iterator_helper_remaining";
const HELPER_COUNTER: &str = "\0iterator_helper_counter";
const HELPER_DONE: &str = "\0iterator_helper_done";
const HELPER_INNER: &str = "\0iterator_helper_inner_alive";
const HELPER_EXECUTING: &str = "\0iterator_helper_executing";

#[derive(Clone, Copy, PartialEq, Eq)]
enum HelperKind {
    Map,
    Filter,
    Take,
    Drop,
    FlatMap,
}

impl HelperKind {
    fn tag(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Filter => "filter",
            Self::Take => "take",
            Self::Drop => "drop",
            Self::FlatMap => "flatMap",
        }
    }

    fn from_tag(tag: &str) -> Option<Self> {
        Some(match tag {
            "map" => Self::Map,
            "filter" => Self::Filter,
            "take" => Self::Take,
            "drop" => Self::Drop,
            "flatMap" => Self::FlatMap,
            _ => return None,
        })
    }
}

fn iterator_receiver(this_value: &Value, method: &str) -> Result<Value, RuntimeError> {
    if !matches!(this_value, Value::Object(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: Iterator.prototype.{method} called on a non-object"),
        });
    }
    Ok(this_value.clone())
}

/// Dispatches the lazy helper constructors.
pub(super) fn native_lazy_helper(
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (kind, method) = match native {
        NativeFunction::IteratorPrototypeMap => (HelperKind::Map, "map"),
        NativeFunction::IteratorPrototypeFilter => (HelperKind::Filter, "filter"),
        NativeFunction::IteratorPrototypeTake => (HelperKind::Take, "take"),
        NativeFunction::IteratorPrototypeDrop => (HelperKind::Drop, "drop"),
        NativeFunction::IteratorPrototypeFlatMap => (HelperKind::FlatMap, "flatMap"),
        _ => unreachable!("native_lazy_helper received a non-helper native"),
    };

    let mut checked_limit = None;
    let mut checked_callback = None;
    let (iterator, next) = match kind {
        HelperKind::Map | HelperKind::Filter | HelperKind::FlatMap => {
            let iterator = iterator_receiver(&this_value, method)?;
            let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
            if !matches!(callback, Value::Function(_)) {
                let err = RuntimeError {
                    thrown: None,
                    message: format!(
                        "TypeError: Iterator.prototype.{method} callback is not a function"
                    ),
                };
                return Err(iterator_close_on_throw(&iterator, err, env));
            }
            let next = property_value(iterator.clone(), "next", env)?;
            checked_callback = Some(callback);
            (iterator, next)
        }
        HelperKind::Take | HelperKind::Drop => {
            let iterator = iterator_receiver(&this_value, method)?;
            let raw = argument_values.first().cloned().unwrap_or(Value::Undefined);
            let limit = match number_to_integer_or_infinity(raw, env) {
                Ok(value) => value,
                Err(err) => return Err(iterator_close_on_throw(&iterator, err, env)),
            };
            if limit.is_nan() || limit < 0.0 {
                let err = RuntimeError {
                    thrown: None,
                    message: format!(
                        "RangeError: Iterator.prototype.{method} argument must not be negative or NaN"
                    ),
                };
                return Err(iterator_close_on_throw(&iterator, err, env));
            }
            let next = property_value(iterator.clone(), "next", env)?;
            checked_limit = Some(limit);
            (iterator, next)
        }
    };

    let helper = ObjectRef::with_prototype(HashMap::new(), super::iterator_helper_prototype(env));
    helper.define_non_enumerable(HELPER_KIND.to_owned(), Value::String(kind.tag().to_owned()));
    helper.define_non_enumerable(HELPER_UNDERLYING.to_owned(), iterator.clone());
    helper.define_non_enumerable(HELPER_NEXT.to_owned(), next);
    helper.define_non_enumerable(HELPER_DONE.to_owned(), Value::Boolean(false));
    helper.define_non_enumerable(HELPER_EXECUTING.to_owned(), Value::Boolean(false));
    helper.define_non_enumerable(HELPER_COUNTER.to_owned(), Value::Number(0.0));

    match kind {
        HelperKind::Map | HelperKind::Filter | HelperKind::FlatMap => {
            let callback = checked_callback
                .expect("callback helpers validate callback before helper creation");
            helper.define_non_enumerable(HELPER_CALLBACK.to_owned(), callback);
        }
        HelperKind::Take | HelperKind::Drop => {
            let limit = checked_limit.expect("take/drop limit was checked before helper creation");
            helper.define_non_enumerable(HELPER_REMAINING.to_owned(), Value::Number(limit));
        }
    }

    Ok(Value::Object(helper))
}

/// ToIntegerOrInfinity for the take/drop limit, surfacing NaN as a distinct
/// value so the caller can raise the RangeError the proposal requires.
fn number_to_integer_or_infinity(value: Value, env: &mut CallEnv) -> Result<f64, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    if number.is_nan() {
        return Ok(f64::NAN);
    }
    if number.is_infinite() {
        return Ok(number);
    }
    Ok(number.trunc())
}

fn helper_kind(helper: &ObjectRef) -> Option<HelperKind> {
    match helper.own_property(HELPER_KIND).map(|p| p.value) {
        Some(Value::String(tag)) => HelperKind::from_tag(&tag),
        _ => None,
    }
}

fn helper_slot(helper: &ObjectRef, key: &str) -> Option<Value> {
    helper.own_property(key).map(|property| property.value)
}

fn helper_done(helper: &ObjectRef) -> bool {
    matches!(
        helper.own_property(HELPER_DONE).map(|p| p.value),
        Some(Value::Boolean(true))
    )
}

fn helper_executing(helper: &ObjectRef) -> bool {
    matches!(
        helper.own_property(HELPER_EXECUTING).map(|p| p.value),
        Some(Value::Boolean(true))
    )
}

fn set_executing(helper: &ObjectRef, executing: bool) {
    helper.define_non_enumerable(HELPER_EXECUTING.to_owned(), Value::Boolean(executing));
}

fn set_done(helper: &ObjectRef) {
    helper.define_non_enumerable(HELPER_DONE.to_owned(), Value::Boolean(true));
}

fn iterator_result(value: Value, done: bool) -> Value {
    let object = ObjectRef::new(HashMap::new());
    object.define_property("value".to_owned(), Property::enumerable(value));
    object.define_property(
        "done".to_owned(),
        Property::enumerable(Value::Boolean(done)),
    );
    Value::Object(object)
}

/// `%IteratorHelperPrototype%.next`.
pub(super) fn native_helper_next(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let Value::Object(helper) = this_value else {
        return Err(not_a_helper());
    };
    let Some(kind) = helper_kind(&helper) else {
        return Err(not_a_helper());
    };
    if helper_done(&helper) {
        return Ok(iterator_result(Value::Undefined, true));
    }
    if helper_executing(&helper) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: iterator helper is already executing".to_owned(),
        });
    }
    let Some(iterator) = helper_slot(&helper, HELPER_UNDERLYING) else {
        return Err(not_a_helper());
    };
    let Some(next) = helper_slot(&helper, HELPER_NEXT) else {
        return Err(not_a_helper());
    };

    set_executing(&helper, true);
    let advanced = advance(kind, &helper, &iterator, &next, env);
    set_executing(&helper, false);

    match advanced {
        Ok(Some(value)) => Ok(iterator_result(value, false)),
        Ok(None) => {
            set_done(&helper);
            Ok(iterator_result(Value::Undefined, true))
        }
        Err(error) => {
            set_done(&helper);
            Err(error)
        }
    }
}

/// Produces the next yielded value (or `None` when exhausted), advancing the
/// underlying iterator and closing it on an abrupt completion in the transform.
fn advance(
    kind: HelperKind,
    helper: &ObjectRef,
    iterator: &Value,
    next: &Value,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    match kind {
        HelperKind::Map => advance_map(helper, iterator, next, env),
        HelperKind::Filter => advance_filter(helper, iterator, next, env),
        HelperKind::Take => advance_take(helper, iterator, next, env),
        HelperKind::Drop => advance_drop(helper, iterator, next, env),
        HelperKind::FlatMap => advance_flat_map(helper, iterator, next, env),
    }
}

fn counter(helper: &ObjectRef) -> f64 {
    match helper_slot(helper, HELPER_COUNTER) {
        Some(Value::Number(n)) => n,
        _ => 0.0,
    }
}

fn bump_counter(helper: &ObjectRef) -> f64 {
    let current = counter(helper);
    helper.define_non_enumerable(HELPER_COUNTER.to_owned(), Value::Number(current + 1.0));
    current
}

fn callback(helper: &ObjectRef) -> Value {
    helper_slot(helper, HELPER_CALLBACK).unwrap_or(Value::Undefined)
}

fn advance_map(
    helper: &ObjectRef,
    iterator: &Value,
    next: &Value,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let Some(result) = iterator_step(iterator, next, env)? else {
        return Ok(None);
    };
    let value = iterator_value(result, env)?;
    let index = bump_counter(helper);
    let mapped = call_function(
        callback(helper),
        Value::Undefined,
        vec![value, Value::Number(index)],
        env,
        false,
    );
    match mapped {
        Ok(value) => Ok(Some(value)),
        Err(error) => Err(iterator_close_on_throw(iterator, error, env)),
    }
}

fn advance_filter(
    helper: &ObjectRef,
    iterator: &Value,
    next: &Value,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    loop {
        let Some(result) = iterator_step(iterator, next, env)? else {
            return Ok(None);
        };
        let value = iterator_value(result, env)?;
        let index = bump_counter(helper);
        let selected = call_function(
            callback(helper),
            Value::Undefined,
            vec![value.clone(), Value::Number(index)],
            env,
            false,
        );
        match selected {
            Ok(selected) => {
                if is_truthy(&selected) {
                    return Ok(Some(value));
                }
            }
            Err(error) => return Err(iterator_close_on_throw(iterator, error, env)),
        }
    }
}

fn advance_take(
    helper: &ObjectRef,
    iterator: &Value,
    next: &Value,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let remaining = match helper_slot(helper, HELPER_REMAINING) {
        Some(Value::Number(n)) => n,
        _ => 0.0,
    };
    if remaining <= 0.0 {
        // Close the underlying iterator when the budget is exhausted.
        super::protocol::iterator_close(iterator, env)?;
        return Ok(None);
    }
    if remaining.is_finite() {
        helper.define_non_enumerable(HELPER_REMAINING.to_owned(), Value::Number(remaining - 1.0));
    }
    let Some(result) = iterator_step(iterator, next, env)? else {
        return Ok(None);
    };
    Ok(Some(iterator_value(result, env)?))
}

fn advance_drop(
    helper: &ObjectRef,
    iterator: &Value,
    next: &Value,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let mut remaining = match helper_slot(helper, HELPER_REMAINING) {
        Some(Value::Number(n)) => n,
        _ => 0.0,
    };
    while remaining > 0.0 {
        remaining -= 1.0;
        if iterator_step(iterator, next, env)?.is_none() {
            helper.define_non_enumerable(HELPER_REMAINING.to_owned(), Value::Number(0.0));
            return Ok(None);
        }
    }
    helper.define_non_enumerable(HELPER_REMAINING.to_owned(), Value::Number(0.0));
    let Some(result) = iterator_step(iterator, next, env)? else {
        return Ok(None);
    };
    Ok(Some(iterator_value(result, env)?))
}

fn advance_flat_map(
    helper: &ObjectRef,
    iterator: &Value,
    next: &Value,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    loop {
        // Drain the in-progress inner iterator first.
        if let Some(Value::Object(inner_state)) = helper_slot(helper, HELPER_INNER) {
            let inner = helper_slot(&inner_state, HELPER_UNDERLYING).unwrap_or(Value::Undefined);
            let inner_next = helper_slot(&inner_state, HELPER_NEXT).unwrap_or(Value::Undefined);
            match iterator_step(&inner, &inner_next, env) {
                Ok(Some(result)) => {
                    let value = iterator_value(result, env)?;
                    return Ok(Some(value));
                }
                Ok(None) => {
                    helper.define_non_enumerable(HELPER_INNER.to_owned(), Value::Undefined);
                }
                Err(error) => {
                    // An inner-iterator failure closes the outer iterator too.
                    return Err(iterator_close_on_throw(iterator, error, env));
                }
            }
        }

        let Some(result) = iterator_step(iterator, next, env)? else {
            return Ok(None);
        };
        let value = iterator_value(result, env)?;
        let index = bump_counter(helper);
        let mapped = call_function(
            callback(helper),
            Value::Undefined,
            vec![value, Value::Number(index)],
            env,
            false,
        );
        let mapped = match mapped {
            Ok(mapped) => mapped,
            Err(error) => return Err(iterator_close_on_throw(iterator, error, env)),
        };
        // GetIteratorFlattenable(mapped, reject-primitives): the mapped value
        // must be an object; get its iterator.
        let inner = match get_iterator_flattenable(mapped, env) {
            Ok(inner) => inner,
            Err(error) => return Err(iterator_close_on_throw(iterator, error, env)),
        };
        helper.define_non_enumerable(HELPER_INNER.to_owned(), inner);
    }
}

/// GetIteratorFlattenable(obj, reject-primitives): rejects a primitive, then
/// reads `Symbol.iterator` (falling back to treating `obj` itself as the
/// iterator when absent) and returns an inner record packaged as an object
/// carrying the iterator and its `next` method.
fn get_iterator_flattenable(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    if !matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    ) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: flatMap callback must return an object".to_owned(),
        });
    }
    let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: "iterator symbol is unavailable".to_owned(),
        });
    };
    let method = property_value_key(value.clone(), &PropertyKey::Symbol(iterator_symbol), env)?;
    let iterator = if matches!(method, Value::Undefined | Value::Null) {
        value
    } else if matches!(method, Value::Function(_)) {
        let iterator = call_function(method, value, Vec::new(), env, false)?;
        if !matches!(
            iterator,
            Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
        ) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: iterator method must return an object".to_owned(),
            });
        }
        iterator
    } else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: flatMap mapper result is not iterable".to_owned(),
        });
    };
    let next = property_value(iterator.clone(), "next", env)?;
    let record = ObjectRef::new(HashMap::new());
    record.define_non_enumerable(HELPER_UNDERLYING.to_owned(), iterator);
    record.define_non_enumerable(HELPER_NEXT.to_owned(), next);
    Ok(Value::Object(record))
}

/// `%IteratorHelperPrototype%.return`: closes the underlying iterator (and any
/// in-progress inner iterator for `flatMap`) and marks the helper done.
pub(super) fn native_helper_return(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let Value::Object(helper) = this_value else {
        return Err(not_a_helper());
    };
    if helper_kind(&helper).is_none() {
        return Err(not_a_helper());
    }
    if !helper_done(&helper) {
        set_done(&helper);
        if let Some(Value::Object(inner_state)) = helper_slot(&helper, HELPER_INNER)
            && let Some(inner) = helper_slot(&inner_state, HELPER_UNDERLYING)
        {
            super::protocol::iterator_close(&inner, env)?;
        }
        if let Some(iterator) = helper_slot(&helper, HELPER_UNDERLYING) {
            super::protocol::iterator_close(&iterator, env)?;
        }
    }
    Ok(iterator_result(Value::Undefined, true))
}

fn not_a_helper() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: method called on a non-iterator-helper object".to_owned(),
    }
}
