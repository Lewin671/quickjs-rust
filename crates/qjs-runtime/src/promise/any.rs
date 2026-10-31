use crate::function::NativeCaptures;
use crate::function::PromiseElementCaptures;
use std::collections::HashMap;

use crate::{ArrayRef, Function, NativeFunction, ObjectRef, RuntimeError, Value, call_function};

use super::{
    all::already_called,
    capability::{self, PromiseCapability},
    perform::{self, ElementHandler},
};
use crate::CallEnv;

/// `Promise.any` (ES2023 27.2.4.3): fulfils with the first input to fulfil, or
/// rejects with an `AggregateError` collecting every rejection reason.
pub(crate) fn native_promise_any(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iterable = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let aggregate_error = function
        .native_captures()
        .promise_prototypes()
        .and_then(|captures| captures.aggregate_error.clone());
    let handler = AnyHandler {
        errors: ArrayRef::new(env.realm(), Vec::new()),
        remaining: perform::new_remaining(1, env.realm().atoms()),
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
        _env: &mut CallEnv,
    ) -> Result<(Value, Value), RuntimeError> {
        self.errors.set(index, Value::Undefined);
        perform::increment_remaining(&self.remaining);

        let already_called = ObjectRef::new(HashMap::new(), _env.realm().atoms());
        let mut on_rejected = Function::new_native(
            None,
            1,
            NativeFunction::PromiseAnyRejectElement,
            false,
            _env.realm(),
        );
        on_rejected.set_native_captures(NativeCaptures::PromiseElement(PromiseElementCaptures {
            index,
            values: self.errors.clone(),
            remaining: self.remaining.clone(),
            already_called,
            settle: capability.reject.clone(),
            aggregate_error: self.aggregate_error.clone(),
        }));

        // onFulfilled is the capability's resolve: the first fulfilment wins.
        Ok((capability.resolve.clone(), Value::Function(on_rejected)))
    }

    fn on_complete(
        &mut self,
        _count: usize,
        capability: &PromiseCapability,
        env: &mut CallEnv,
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if already_called(function) {
        return Ok(Value::Undefined);
    }
    let captures = function
        .native_captures()
        .promise_element()
        .ok_or_else(|| RuntimeError {
            thrown: None,
            message: "Promise.any element is missing its capture payload".into(),
        })?;
    let index = captures.index;
    let errors = captures.values.clone();
    let remaining = captures.remaining.clone();

    let reason = argument_values.first().cloned().unwrap_or(Value::Undefined);
    errors.set(index, reason);
    if perform::decrement_remaining(&remaining) == 0.0 {
        let aggregate = captures.aggregate_error.clone();
        let error = build_aggregate_error(aggregate, &errors, env);
        let reject = captures.settle.clone();
        call_function(reject, Value::Undefined, vec![error], env, false)?;
    }
    Ok(Value::Undefined)
}

fn build_aggregate_error(
    constructor: Option<Value>,
    errors: &ArrayRef,
    env: &mut CallEnv,
) -> Value {
    let errors_value = Value::Array(errors.clone());
    let message = Value::String("All promises were rejected".to_owned().into());
    let constructor = constructor.or_else(|| env.get("AggregateError"));
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

    let object = ObjectRef::new_ordered(
        vec![
            (
                "name".to_owned(),
                Value::String("AggregateError".to_owned().into()),
            ),
            ("message".to_owned(), message),
            ("errors".to_owned(), errors_value),
        ],
        env.realm().atoms(),
    );
    Value::Object(object)
}
