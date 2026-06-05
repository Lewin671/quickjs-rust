use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, RuntimeError, Value, array, call_function,
};

use super::{
    PROMISE_AGGREGATE_ERROR, PROMISE_ALL_INDEX, PROMISE_ALL_REMAINING, PROMISE_ALL_VALUES,
    PROMISE_REJECTED, PROMISE_TARGET, call_promise_then, initialize_promise,
    promise_from_resolving_function, promise_object_from_function, resolve_promise,
    resolving_function, settle_promise,
};

pub(crate) fn native_promise_any(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let promise = promise_object_from_function(function);
    initialize_promise(&promise);
    let items = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let values = match array::array_like_values_with_env(items, "Promise.any", env) {
        Ok(values) => values,
        Err(error) => {
            settle_promise(
                &promise,
                PROMISE_REJECTED,
                error
                    .thrown
                    .map_or(Value::String(error.message), |value| *value),
                env,
            );
            return Ok(Value::Object(promise));
        }
    };

    let errors = ArrayRef::new(vec![Value::Undefined; values.len()]);
    if values.is_empty() {
        let aggregate_error = aggregate_error(function, errors, env);
        settle_promise(&promise, PROMISE_REJECTED, aggregate_error, env);
        return Ok(Value::Object(promise));
    }

    let remaining = ObjectRef::new(HashMap::from([(
        "count".to_owned(),
        Value::Number(values.len() as f64),
    )]));
    let resolve = resolving_function(
        "resolve",
        NativeFunction::PromiseResolveFunction,
        Value::Object(promise.clone()),
    );

    for (index, value) in values.into_iter().enumerate() {
        let element_promise = promise_object_from_function(function);
        initialize_promise(&element_promise);
        resolve_promise(&element_promise, value, env);

        let mut on_rejected = Function::new_native(
            Some("Promise.any reject element"),
            1,
            NativeFunction::PromiseAnyRejectElement,
            false,
        );
        on_rejected
            .env
            .insert(PROMISE_TARGET.to_owned(), Value::Object(promise.clone()));
        on_rejected
            .env
            .insert(PROMISE_ALL_INDEX.to_owned(), Value::Number(index as f64));
        on_rejected
            .env
            .insert(PROMISE_ALL_VALUES.to_owned(), Value::Array(errors.clone()));
        on_rejected.env.insert(
            PROMISE_ALL_REMAINING.to_owned(),
            Value::Object(remaining.clone()),
        );
        if let Some(aggregate_error) = function.env.get(PROMISE_AGGREGATE_ERROR).cloned() {
            on_rejected
                .env
                .insert(PROMISE_AGGREGATE_ERROR.to_owned(), aggregate_error);
        }

        call_promise_then(
            Value::Object(element_promise),
            vec![resolve.clone(), Value::Function(on_rejected)],
            env,
        )?;
    }

    Ok(Value::Object(promise))
}

pub(crate) fn native_promise_any_reject_element(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let promise = promise_from_resolving_function(function)?;
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
    let next_remaining = match remaining
        .own_property("count")
        .map(|property| property.value)
    {
        Some(Value::Number(count)) if count > 0.0 => count - 1.0,
        _ => 0.0,
    };
    remaining.set("count".to_owned(), Value::Number(next_remaining));
    if next_remaining == 0.0 {
        let aggregate_error = aggregate_error(function, errors, env);
        settle_promise(&promise, PROMISE_REJECTED, aggregate_error, env);
    }

    Ok(Value::Undefined)
}

fn aggregate_error(
    function: &Function,
    errors: ArrayRef,
    env: &mut HashMap<String, Value>,
) -> Value {
    let errors = Value::Array(errors);
    let message = Value::String("All promises were rejected".to_owned());
    let constructor = function
        .env
        .get(PROMISE_AGGREGATE_ERROR)
        .cloned()
        .or_else(|| env.get("AggregateError").cloned());
    if let Some(constructor) = constructor {
        if let Ok(value) = call_function(
            constructor,
            Value::Undefined,
            vec![errors.clone(), message.clone()],
            env,
            false,
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
        ("errors".to_owned(), errors),
    ]));
    Value::Object(object)
}
