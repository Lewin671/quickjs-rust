use std::collections::HashMap;

use crate::{ArrayRef, Function, NativeFunction, ObjectRef, RuntimeError, Value, call_function};

use super::{
    PROMISE_AGGREGATE_ERROR, PROMISE_ALL_ALREADY_CALLED, PROMISE_ALL_CAPABILITY_RESOLVE,
    PROMISE_ALL_INDEX, PROMISE_ALL_REMAINING, PROMISE_ALL_VALUES,
    all::already_called,
    capability::{self, PromiseCapability},
    perform::{self, ElementHandler},
};

/// `Promise.any` (ES2023 27.2.4.3): fulfils with the first input to fulfil, or
/// rejects with an `AggregateError` collecting every rejection reason.
pub(crate) fn native_promise_any(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iterable = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let aggregate_error = function.env.get(PROMISE_AGGREGATE_ERROR).cloned();
    let handler = AnyHandler {
        errors: ArrayRef::new(Vec::new()),
        remaining: perform::new_remaining(1),
        aggregate_error,
    };
    perform::run_combinator(this_value, iterable, "Promise.any", handler, env)
}

struct AnyHandler {
    errors: ArrayRef,
    remaining: ObjectRef,
    aggregate_error: Option<Value>,
}

impl ElementHandler for AnyHandler {
    fn on_element(
        &mut self,
        index: usize,
        capability: &PromiseCapability,
        _env: &mut HashMap<String, Value>,
    ) -> Result<(Value, Value), RuntimeError> {
        self.errors.set(index, Value::Undefined);
        perform::increment_remaining(&self.remaining);

        let already_called = ObjectRef::new(HashMap::new());
        let mut on_rejected =
            Function::new_native(None, 1, NativeFunction::PromiseAnyRejectElement, false);
        on_rejected
            .env
            .insert(PROMISE_ALL_INDEX.to_owned(), Value::Number(index as f64));
        on_rejected.env.insert(
            PROMISE_ALL_VALUES.to_owned(),
            Value::Array(self.errors.clone()),
        );
        on_rejected.env.insert(
            PROMISE_ALL_REMAINING.to_owned(),
            Value::Object(self.remaining.clone()),
        );
        on_rejected.env.insert(
            PROMISE_ALL_ALREADY_CALLED.to_owned(),
            Value::Object(already_called),
        );
        on_rejected.env.insert(
            PROMISE_ALL_CAPABILITY_RESOLVE.to_owned(),
            capability.reject.clone(),
        );
        if let Some(aggregate_error) = &self.aggregate_error {
            on_rejected
                .env
                .insert(PROMISE_AGGREGATE_ERROR.to_owned(), aggregate_error.clone());
        }

        // onFulfilled is the capability's resolve: the first fulfilment wins.
        Ok((capability.resolve.clone(), Value::Function(on_rejected)))
    }

    fn on_complete(
        &mut self,
        _count: usize,
        capability: &PromiseCapability,
        env: &mut HashMap<String, Value>,
    ) -> Result<(), RuntimeError> {
        if perform::decrement_remaining(&self.remaining) == 0.0 {
            let error = build_aggregate_error(self.aggregate_error.clone(), &self.errors, env);
            capability::capability_reject(capability, error, env)?;
        }
        Ok(())
    }
}

/// Promise.any Reject Element function (ES2023 27.2.4.3.3): records a rejection
/// reason and, once every element has rejected, rejects the result capability
/// with an `AggregateError`. Guarded so it runs at most once.
pub(crate) fn native_promise_any_reject_element(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if already_called(function) {
        return Ok(Value::Undefined);
    }
    let index = match function.env.get(PROMISE_ALL_INDEX) {
        Some(Value::Number(index)) if *index >= 0.0 => *index as usize,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Promise.any reject element is missing its index".to_owned(),
            });
        }
    };
    let errors = match function.env.get(PROMISE_ALL_VALUES).cloned() {
        Some(Value::Array(errors)) => errors,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Promise.any reject element is missing errors".to_owned(),
            });
        }
    };
    let remaining = match function.env.get(PROMISE_ALL_REMAINING).cloned() {
        Some(Value::Object(remaining)) => remaining,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Promise.any reject element is missing remaining count".to_owned(),
            });
        }
    };

    let reason = argument_values.first().cloned().unwrap_or(Value::Undefined);
    errors.set(index, reason);
    if perform::decrement_remaining(&remaining) == 0.0 {
        let aggregate = function.env.get(PROMISE_AGGREGATE_ERROR).cloned();
        let error = build_aggregate_error(aggregate, &errors, env);
        let reject = function
            .env
            .get(PROMISE_ALL_CAPABILITY_RESOLVE)
            .cloned()
            .unwrap_or(Value::Undefined);
        call_function(reject, Value::Undefined, vec![error], env, false)?;
    }
    Ok(Value::Undefined)
}

fn build_aggregate_error(
    constructor: Option<Value>,
    errors: &ArrayRef,
    env: &mut HashMap<String, Value>,
) -> Value {
    let errors_value = Value::Array(errors.clone());
    let message = Value::String("All promises were rejected".to_owned());
    let constructor = constructor.or_else(|| env.get("AggregateError").cloned());
    if let Some(constructor) = constructor {
        if let Ok(value) = crate::construct_function(
            constructor.clone(),
            constructor,
            vec![errors_value.clone(), message.clone()],
            env,
        ) {
            return value;
        }
    }

    let object = ObjectRef::new(HashMap::from([
        (
            "name".to_owned(),
            Value::String("AggregateError".to_owned()),
        ),
        ("message".to_owned(), message),
        ("errors".to_owned(), errors_value),
    ]));
    Value::Object(object)
}
