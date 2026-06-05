use std::collections::HashMap;

use crate::{ArrayRef, Function, NativeFunction, ObjectRef, RuntimeError, Value, array};

use super::{
    PROMISE_ALL_INDEX, PROMISE_ALL_REMAINING, PROMISE_ALL_VALUES, PROMISE_FULFILLED,
    PROMISE_REJECTED, PROMISE_TARGET, call_promise_then, initialize_promise,
    promise_from_resolving_function, promise_object_from_function, resolve_promise,
    resolving_function, settle_promise,
};

pub(crate) fn native_promise_all(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let promise = promise_object_from_function(function);
    initialize_promise(&promise);
    let items = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let values = match array::array_like_values_with_env(items, "Promise.all", env) {
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

    if values.is_empty() {
        settle_promise(
            &promise,
            PROMISE_FULFILLED,
            Value::Array(ArrayRef::new(Vec::new())),
            env,
        );
        return Ok(Value::Object(promise));
    }

    let result_values = ArrayRef::new(vec![Value::Undefined; values.len()]);
    let remaining = ObjectRef::new(HashMap::from([(
        "count".to_owned(),
        Value::Number(values.len() as f64),
    )]));
    let reject = resolving_function(
        "reject",
        NativeFunction::PromiseRejectFunction,
        Value::Object(promise.clone()),
    );

    for (index, value) in values.into_iter().enumerate() {
        let element_promise = promise_object_from_function(function);
        initialize_promise(&element_promise);
        resolve_promise(&element_promise, value, env);

        let mut on_fulfilled = Function::new_native(
            Some("Promise.all resolve element"),
            1,
            NativeFunction::PromiseAllResolveElement,
            false,
        );
        on_fulfilled
            .env
            .insert(PROMISE_TARGET.to_owned(), Value::Object(promise.clone()));
        on_fulfilled
            .env
            .insert(PROMISE_ALL_INDEX.to_owned(), Value::Number(index as f64));
        on_fulfilled.env.insert(
            PROMISE_ALL_VALUES.to_owned(),
            Value::Array(result_values.clone()),
        );
        on_fulfilled.env.insert(
            PROMISE_ALL_REMAINING.to_owned(),
            Value::Object(remaining.clone()),
        );

        call_promise_then(
            Value::Object(element_promise),
            vec![Value::Function(on_fulfilled), reject.clone()],
            env,
        )?;
    }

    Ok(Value::Object(promise))
}

pub(crate) fn native_promise_all_resolve_element(
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
                message: "Promise.all resolve element is missing its index".to_owned(),
            });
        }
    };
    let values = match function.env.get(PROMISE_ALL_VALUES).cloned() {
        Some(Value::Array(values)) => values,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Promise.all resolve element is missing values".to_owned(),
            });
        }
    };
    let remaining = match function.env.get(PROMISE_ALL_REMAINING).cloned() {
        Some(Value::Object(remaining)) => remaining,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Promise.all resolve element is missing remaining count".to_owned(),
            });
        }
    };

    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    values.set(index, value);
    let next_remaining = match remaining
        .own_property("count")
        .map(|property| property.value)
    {
        Some(Value::Number(count)) if count > 0.0 => count - 1.0,
        _ => 0.0,
    };
    remaining.set("count".to_owned(), Value::Number(next_remaining));
    if next_remaining == 0.0 {
        settle_promise(&promise, PROMISE_FULFILLED, Value::Array(values), env);
    }

    Ok(Value::Undefined)
}
