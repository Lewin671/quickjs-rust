//! Shared skeleton for the iterator-driven promise combinators
//! (`Promise.all`/`allSettled`/`any`/`race`).
//!
//! Each combinator follows the same outer algorithm (ES2023 27.2.4.1 etc.):
//!
//! 1. `C` is the `this` value; `promiseCapability = NewPromiseCapability(C)`.
//! 2. `promiseResolve = Get(C, "resolve")`; reject if not callable.
//! 3. `iteratorRecord = GetIterator(iterable)`; reject on abrupt.
//! 4. Run `PerformPromiseX(iteratorRecord, C, capability, promiseResolve)`,
//!    which iterates, wraps each value with `promiseResolve.call(C, value)`,
//!    and calls `then`. Abrupt completions close the iterator and reject.
//!
//! The per-element bookkeeping (remaining counter, result array, element
//! functions) differs per combinator and is supplied through `ElementHandler`.

use std::collections::HashMap;

use crate::{ObjectRef, PropertyKey, RuntimeError, Value, call_function, property_value, symbol};

use super::capability::{self, PromiseCapability};

/// The realm constructor `C` plus the resolved `promiseResolve` function used to
/// wrap each iterated value.
pub(super) struct CombinatorContext {
    pub(super) constructor: Value,
    pub(super) promise_resolve: Value,
}

/// Per-element behavior for a specific combinator. Given the per-element wrapped
/// promise (`next_promise`) and its zero-based index, returns the `then`
/// arguments `(on_fulfilled, on_rejected)` and may update shared accounting.
pub(super) trait ElementHandler {
    /// Called once before iteration with the number-of-elements-unknown state;
    /// combinators that need a shared "remaining" record initialise it lazily.
    fn on_element(
        &mut self,
        index: usize,
        capability: &PromiseCapability,
        env: &mut HashMap<String, Value>,
    ) -> Result<(Value, Value), RuntimeError>;

    /// Called after the loop completes with the final element count. Returns an
    /// optional value to settle the capability with directly (used by the
    /// empty-iterable / all-settled-synchronously fast settle). When `None`,
    /// settlement happens through the element functions.
    fn on_complete(
        &mut self,
        count: usize,
        capability: &PromiseCapability,
        env: &mut HashMap<String, Value>,
    ) -> Result<(), RuntimeError>;
}

/// Drives a combinator: builds the capability, reads `promiseResolve`, iterates
/// the argument, and wires each element through `handler`. Returns the
/// capability promise. Abrupt completions before settlement reject the
/// capability instead of throwing (`IfAbruptRejectPromise`).
pub(super) fn run_combinator<H: ElementHandler>(
    this_value: Value,
    iterable: Value,
    context_label: &str,
    mut handler: H,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let capability = capability::new_promise_capability(&this_value, env)?;

    // From here, abrupt completions reject the capability rather than throw.
    match perform(
        &this_value,
        iterable,
        context_label,
        &mut handler,
        &capability,
        env,
    ) {
        Ok(()) => Ok(capability.promise),
        Err(error) => {
            let reason = crate::error::runtime_error_to_value(error, env);
            capability::capability_reject(&capability, reason, env)?;
            Ok(capability.promise)
        }
    }
}

fn perform<H: ElementHandler>(
    this_value: &Value,
    iterable: Value,
    context_label: &str,
    handler: &mut H,
    capability: &PromiseCapability,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    // promiseResolve = ? Get(C, "resolve"); if not callable, throw TypeError.
    let promise_resolve = property_value(this_value.clone(), "resolve", env)?;
    if !matches!(promise_resolve, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: {context_label} resolve is not callable"),
        });
    }
    let context = CombinatorContext {
        constructor: this_value.clone(),
        promise_resolve,
    };

    // iteratorRecord = ? GetIterator(iterable, sync).
    let iterator = get_iterator(iterable, context_label, env)?;
    let next = property_value(iterator.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: {context_label} iterator next is not callable"),
        });
    }

    let mut index = 0usize;
    loop {
        // step = ? IteratorStep(iteratorRecord).
        let step = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
        if !is_object(&step) {
            return Err(RuntimeError {
                thrown: None,
                message: format!("TypeError: {context_label} iterator result is not an object"),
            });
        }
        if is_truthy(&property_value(step.clone(), "done", env)?) {
            // Iterator is exhausted; let the handler finalise (and settle if it
            // is responsible for an all-settled-synchronously case).
            handler.on_complete(index, capability, env)?;
            return Ok(());
        }
        let value = property_value(step, "value", env)?;

        // Element processing whose abrupt completion must close the iterator.
        let element_result = process_element(&context, value, index, handler, capability, env);
        if let Err(error) = element_result {
            return Err(iterator_close_on_error(&iterator, error, env));
        }
        index += 1;
    }
}

/// Processes one iterated value: wraps it via `promiseResolve.call(C, value)`,
/// requests the combinator's element functions, and invokes `then`.
fn process_element<H: ElementHandler>(
    context: &CombinatorContext,
    value: Value,
    index: usize,
    handler: &mut H,
    capability: &PromiseCapability,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    // nextPromise = ? Call(promiseResolve, C, « value »).
    let next_promise = call_function(
        context.promise_resolve.clone(),
        context.constructor.clone(),
        vec![value],
        env,
        false,
    )?;
    let (on_fulfilled, on_rejected) = handler.on_element(index, capability, env)?;
    // nextPromise.then(onFulfilled, onRejected).
    let then = property_value(next_promise.clone(), "then", env)?;
    call_function(
        then,
        next_promise,
        vec![on_fulfilled, on_rejected],
        env,
        false,
    )?;
    Ok(())
}

/// `GetIterator(obj, sync)`: reads `obj[Symbol.iterator]`, calls it, and checks
/// the result is an object.
fn get_iterator(
    iterable: Value,
    context_label: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: format!("{context_label} iterator symbol is unavailable"),
        });
    };
    let method =
        crate::property_value_key(iterable.clone(), &PropertyKey::Symbol(iterator_symbol), env)?;
    if !matches!(method, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: {context_label} argument is not iterable"),
        });
    }
    let iterator = call_function(method, iterable, Vec::new(), env, false)?;
    if !is_object(&iterator) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: {context_label} iterator is not an object"),
        });
    }
    Ok(iterator)
}

/// `IteratorClose(iterator, completion)` with a throw completion: calls
/// `iterator.return()` if present, preferring the original error over any
/// thrown by `return`. A non-callable or absent `return` is ignored.
fn iterator_close_on_error(
    iterator: &Value,
    error: RuntimeError,
    env: &mut HashMap<String, Value>,
) -> RuntimeError {
    let return_method = match property_value(iterator.clone(), "return", env) {
        Ok(method) => method,
        // A getter that throws here is swallowed: the original error wins.
        Err(_) => return error,
    };
    if matches!(return_method, Value::Undefined | Value::Null) {
        return error;
    }
    if !matches!(return_method, Value::Function(_)) {
        // Per spec a non-callable, non-undefined return is a TypeError, but the
        // original abrupt completion takes precedence, so it is discarded.
        return error;
    }
    let _ = call_function(return_method, iterator.clone(), Vec::new(), env, false);
    error
}

fn is_object(value: &Value) -> bool {
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

fn is_truthy(value: &Value) -> bool {
    crate::is_truthy(value)
}

/// Helper shared by the element handlers: a mutable "remaining count" record.
/// Combinators initialise it to 1 (PerformPromiseAll step "remainingElementsCount
/// = 1"), increment per element, then decrement once after the loop so the
/// capability is settled only after every element has been queued.
pub(super) fn new_remaining(count: usize) -> ObjectRef {
    ObjectRef::new(HashMap::from([(
        "count".to_owned(),
        Value::Number(count as f64),
    )]))
}

/// Increments a remaining record's count.
pub(super) fn increment_remaining(remaining: &ObjectRef) {
    let current = match remaining
        .own_property("count")
        .map(|property| property.value)
    {
        Some(Value::Number(count)) => count,
        _ => 0.0,
    };
    remaining.set("count".to_owned(), Value::Number(current + 1.0));
}

/// Decrements a remaining record's count and returns the new value.
pub(super) fn decrement_remaining(remaining: &ObjectRef) -> f64 {
    let next = match remaining
        .own_property("count")
        .map(|property| property.value)
    {
        Some(Value::Number(count)) if count > 0.0 => count - 1.0,
        _ => 0.0,
    };
    remaining.set("count".to_owned(), Value::Number(next));
    next
}
