use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, call_function,
};

use super::{
    PROMISE_ALL_ALREADY_CALLED, PROMISE_ALL_CAPABILITY_RESOLVE, PROMISE_ALL_INDEX,
    PROMISE_ALL_REMAINING, PROMISE_ALL_VALUES, PROMISE_FULFILLED, PROMISE_REJECTED,
    all::already_called,
    capability::PromiseCapability,
    perform::{self, ElementHandler},
};
use crate::CallEnv;

/// `Promise.allSettled` (ES2023 27.2.4.2): always fulfils, with an array of
/// `{ status, value }` / `{ status, reason }` records once every input settles.
pub(crate) fn native_promise_all_settled(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iterable = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let handler = AllSettledHandler {
        values: ArrayRef::new(Vec::new()),
        remaining: perform::new_remaining(1),
    };
    perform::run_combinator(this_value, iterable, "Promise.allSettled", handler, env)
}

struct AllSettledHandler {
    values: ArrayRef,
    remaining: ObjectRef,
}

impl ElementHandler for AllSettledHandler {
    fn on_element(
        &mut self,
        index: usize,
        capability: &PromiseCapability,
        _env: &mut CallEnv,
    ) -> Result<(Value, Value), RuntimeError> {
        self.values.set(index, Value::Undefined);
        perform::increment_remaining(&self.remaining);

        // A single alreadyCalled record is shared by the resolve and reject
        // element functions for this index, so only the first observed
        // settlement is recorded.
        let already_called = ObjectRef::new(HashMap::new());
        let on_fulfilled = element_function(
            NativeFunction::PromiseAllSettledResolveElement,
            index,
            &self.values,
            &self.remaining,
            &already_called,
            capability,
        );
        let on_rejected = element_function(
            NativeFunction::PromiseAllSettledRejectElement,
            index,
            &self.values,
            &self.remaining,
            &already_called,
            capability,
        );
        Ok((Value::Function(on_fulfilled), Value::Function(on_rejected)))
    }

    fn on_complete(
        &mut self,
        _count: usize,
        capability: &PromiseCapability,
        env: &mut CallEnv,
    ) -> Result<(), RuntimeError> {
        if perform::decrement_remaining(&self.remaining) == 0.0 {
            super::capability::capability_resolve(
                capability,
                Value::Array(self.values.clone()),
                env,
            )?;
        }
        Ok(())
    }
}

pub(crate) fn native_promise_all_settled_resolve_element(
    function: &Function,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    settle_element(function, argument_values, env, PROMISE_FULFILLED)
}

pub(crate) fn native_promise_all_settled_reject_element(
    function: &Function,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    settle_element(function, argument_values, env, PROMISE_REJECTED)
}

fn element_function(
    native: NativeFunction,
    index: usize,
    values: &ArrayRef,
    remaining: &ObjectRef,
    already_called: &ObjectRef,
    capability: &PromiseCapability,
) -> Function {
    let mut function = Function::new_native(None, 1, native, false);
    function.insert_env(PROMISE_ALL_INDEX.to_owned(), Value::Number(index as f64));
    function.insert_env(PROMISE_ALL_VALUES.to_owned(), Value::Array(values.clone()));
    function.insert_env(
        PROMISE_ALL_REMAINING.to_owned(),
        Value::Object(remaining.clone()),
    );
    function.insert_env(
        PROMISE_ALL_ALREADY_CALLED.to_owned(),
        Value::Object(already_called.clone()),
    );
    function.insert_env(
        PROMISE_ALL_CAPABILITY_RESOLVE.to_owned(),
        capability.resolve.clone(),
    );
    function
}

fn settle_element(
    function: &Function,
    argument_values: &[Value],
    env: &mut CallEnv,
    state: &str,
) -> Result<Value, RuntimeError> {
    if already_called(function) {
        return Ok(Value::Undefined);
    }
    let index = match function.env.get(PROMISE_ALL_INDEX) {
        Some(Value::Number(index)) if *index >= 0.0 => *index as usize,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Promise.allSettled element is missing its index".to_owned(),
            });
        }
    };
    let values = match function.env.get(PROMISE_ALL_VALUES).cloned() {
        Some(Value::Array(values)) => values,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Promise.allSettled element is missing values".to_owned(),
            });
        }
    };
    let remaining = match function.env.get(PROMISE_ALL_REMAINING).cloned() {
        Some(Value::Object(remaining)) => remaining,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Promise.allSettled element is missing remaining count".to_owned(),
            });
        }
    };

    let settled_value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    values.set(index, settled_result_object(state, settled_value, env));
    if perform::decrement_remaining(&remaining) == 0.0 {
        let resolve = function
            .env
            .get(PROMISE_ALL_CAPABILITY_RESOLVE)
            .cloned()
            .unwrap_or(Value::Undefined);
        call_function(
            resolve,
            Value::Undefined,
            vec![Value::Array(values)],
            env,
            false,
        )?;
    }
    Ok(Value::Undefined)
}

/// Builds an ordinary object `{ status, value }` or `{ status, reason }` with
/// the spec property order, `%Object.prototype%`, and default data attributes
/// (writable, enumerable, configurable).
fn settled_result_object(state: &str, value: Value, env: &CallEnv) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), crate::object_prototype(env));
    object.define_property(
        "status".to_owned(),
        Property::data(Value::String(state.to_owned().into()), true, true, true),
    );
    let (key, value_key) = if state == PROMISE_FULFILLED {
        ("value", value)
    } else {
        ("reason", value)
    };
    object.define_property(key.to_owned(), Property::data(value_key, true, true, true));
    Value::Object(object)
}
