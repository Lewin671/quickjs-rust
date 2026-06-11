//! `NewPromiseCapability(C)` (ES2023 27.2.1.5) and its GetCapabilitiesExecutor.
//!
//! A promise capability bundles a promise object with the `resolve`/`reject`
//! functions that settle it. The native `%Promise%` fast path builds these
//! directly; an arbitrary constructor `C` (e.g. a subclass or a plain function
//! passed via `Promise.all.call(C, ...)`) is invoked as `Construct(C, «executor»)`
//! where `executor` is a fresh GetCapabilitiesExecutor function that records the
//! resolve/reject arguments it is called with.

use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, RuntimeError, Value, call_function, construct_function,
    ensure_constructor,
};

use super::{PROMISE_PROTOTYPE, new_pending_promise, resolving_function_pair};
use crate::CallEnv;

/// Internal slot on a GetCapabilitiesExecutor holding the shared capability
/// record it writes `resolve`/`reject` into.
const CAPABILITY_RECORD: &str = "\0PromiseCapabilityRecord";
const CAPABILITY_RESOLVE: &str = "resolve";
const CAPABILITY_REJECT: &str = "reject";

/// A resolved promise capability: the constructed promise plus its resolve and
/// reject functions.
pub(crate) struct PromiseCapability {
    pub(crate) promise: Value,
    pub(crate) resolve: Value,
    pub(crate) reject: Value,
}

/// `NewPromiseCapability(C)`.
///
/// When `c` is the realm's native `%Promise%` constructor we build the
/// capability directly (the common, fast path). Otherwise we run the generic
/// algorithm: construct `C` with a GetCapabilitiesExecutor and read back the
/// resolve/reject it captured, validating that both are callable.
pub(crate) fn new_promise_capability(
    c: &Value,
    env: &mut CallEnv,
) -> Result<PromiseCapability, RuntimeError> {
    ensure_constructor(c)?;

    if is_native_promise_constructor(c, env) {
        let promise = new_pending_promise(env);
        let (resolve, reject) = resolving_function_pair(Value::Object(promise.clone()));
        return Ok(PromiseCapability {
            promise: Value::Object(promise),
            resolve,
            reject,
        });
    }

    // Generic path: the executor stashes resolve/reject into a shared record
    // object that we read after Construct(C, «executor»).
    let record = ObjectRef::new(HashMap::new());
    let mut executor = Function::new_native(
        None,
        2,
        NativeFunction::PromiseGetCapabilitiesExecutor,
        false,
    );
    executor
        .env
        .insert(CAPABILITY_RECORD.to_owned(), Value::Object(record.clone()));

    let promise = construct_function(c.clone(), c.clone(), vec![Value::Function(executor)], env)?;

    let resolve = record
        .own_property(CAPABILITY_RESOLVE)
        .map(|property| property.value)
        .unwrap_or(Value::Undefined);
    let reject = record
        .own_property(CAPABILITY_REJECT)
        .map(|property| property.value)
        .unwrap_or(Value::Undefined);
    if !matches!(resolve, Value::Function(_)) || !matches!(reject, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Promise resolve or reject function is not callable".to_owned(),
        });
    }

    Ok(PromiseCapability {
        promise,
        resolve,
        reject,
    })
}

/// GetCapabilitiesExecutor function (ES2023 27.2.1.5.1). Throws if its
/// capability record already has resolve/reject set; otherwise stores them.
pub(crate) fn native_get_capabilities_executor(
    function: &Function,
    argument_values: &[Value],
    _env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let Some(Value::Object(record)) = function.env.get(CAPABILITY_RECORD).cloned() else {
        return Err(RuntimeError {
            thrown: None,
            message: "Promise capability executor is missing its record".to_owned(),
        });
    };
    // Per spec, the executor throws only if a previous call already stored a
    // *non-undefined* resolve or reject. A first call with undefined arguments
    // still "uses up" the slots but leaves them undefined, so a later call with
    // real functions is permitted.
    let stored_resolve = record
        .own_property(CAPABILITY_RESOLVE)
        .map(|property| property.value);
    let stored_reject = record
        .own_property(CAPABILITY_REJECT)
        .map(|property| property.value);
    if matches!(stored_resolve, Some(value) if !matches!(value, Value::Undefined))
        || matches!(stored_reject, Some(value) if !matches!(value, Value::Undefined))
    {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Promise capability executor already invoked".to_owned(),
        });
    }
    let resolve = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let reject = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    record.define_non_enumerable(CAPABILITY_RESOLVE.to_owned(), resolve);
    record.define_non_enumerable(CAPABILITY_REJECT.to_owned(), reject);
    Ok(Value::Undefined)
}

/// Returns true when `c` is the realm's own `%Promise%` constructor, so the
/// capability can be built without observable user code.
fn is_native_promise_constructor(c: &Value, env: &CallEnv) -> bool {
    let Value::Function(function) = c else {
        return false;
    };
    if function.native != Some(NativeFunction::Promise) {
        return false;
    }
    // A native Promise constructor whose `prototype` is the realm prototype is
    // unmodified enough that the fast path is observationally identical.
    let Some(Value::Object(realm_prototype)) = env.get(PROMISE_PROTOTYPE) else {
        return true;
    };
    match function
        .own_property("prototype")
        .map(|property| property.value)
    {
        Some(Value::Object(prototype)) => prototype.ptr_eq(&realm_prototype),
        _ => true,
    }
}

/// `IsPromise`-aware capability resolution helper exposed for combinators:
/// resolves the capability's promise with `value` by calling its resolve fn.
pub(crate) fn capability_resolve(
    capability: &PromiseCapability,
    value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    call_function(
        capability.resolve.clone(),
        Value::Undefined,
        vec![value],
        env,
        false,
    )
}

/// Calls the capability's reject function with `reason`.
pub(crate) fn capability_reject(
    capability: &PromiseCapability,
    reason: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    call_function(
        capability.reject.clone(),
        Value::Undefined,
        vec![reason],
        env,
        false,
    )
}
