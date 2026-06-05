use std::collections::HashMap;

use crate::{ArrayRef, Function, NativeFunction, ObjectRef, RuntimeError, Value, array};

use super::{
    PROMISE_ALL_INDEX, PROMISE_ALL_REMAINING, PROMISE_ALL_VALUES, PROMISE_FULFILLED,
    PROMISE_REJECTED, PROMISE_TARGET, call_promise_then, initialize_promise,
    promise_from_resolving_function, promise_object_from_function, resolve_promise, settle_promise,
};

pub(crate) fn native_promise_all_settled(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let promise = promise_object_from_function(function);
    initialize_promise(&promise);
    let items = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let values = match array::array_like_values_with_env(items, "Promise.allSettled", env) {
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

    for (index, value) in values.into_iter().enumerate() {
        let element_promise = promise_object_from_function(function);
        initialize_promise(&element_promise);
        resolve_promise(&element_promise, value, env);

        let on_fulfilled = settlement_element_function(
            "Promise.allSettled resolve element",
            NativeFunction::PromiseAllSettledResolveElement,
            promise.clone(),
            index,
            result_values.clone(),
            remaining.clone(),
        );
        let on_rejected = settlement_element_function(
            "Promise.allSettled reject element",
            NativeFunction::PromiseAllSettledRejectElement,
            promise.clone(),
            index,
            result_values.clone(),
            remaining.clone(),
        );

        call_promise_then(
            Value::Object(element_promise),
            vec![Value::Function(on_fulfilled), Value::Function(on_rejected)],
            env,
        )?;
    }

    Ok(Value::Object(promise))
}

pub(crate) fn native_promise_all_settled_resolve_element(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    settle_element(function, argument_values, env, PROMISE_FULFILLED)
}

pub(crate) fn native_promise_all_settled_reject_element(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    settle_element(function, argument_values, env, PROMISE_REJECTED)
}

fn settlement_element_function(
    name: &str,
    native: NativeFunction,
    promise: ObjectRef,
    index: usize,
    values: ArrayRef,
    remaining: ObjectRef,
) -> Function {
    let mut function = Function::new_native(Some(name), 1, native, false);
    function
        .env
        .insert(PROMISE_TARGET.to_owned(), Value::Object(promise));
    function
        .env
        .insert(PROMISE_ALL_INDEX.to_owned(), Value::Number(index as f64));
    function
        .env
        .insert(PROMISE_ALL_VALUES.to_owned(), Value::Array(values));
    function
        .env
        .insert(PROMISE_ALL_REMAINING.to_owned(), Value::Object(remaining));
    function
}

fn settle_element(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
    state: &str,
) -> Result<Value, RuntimeError> {
    let promise = promise_from_resolving_function(function)?;
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
    values.set(index, settled_result_object(state, settled_value));
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

fn settled_result_object(state: &str, value: Value) -> Value {
    let object = if state == PROMISE_FULFILLED {
        ObjectRef::new(HashMap::from([
            (
                "status".to_owned(),
                Value::String(PROMISE_FULFILLED.to_owned()),
            ),
            ("value".to_owned(), value),
        ]))
    } else {
        ObjectRef::new(HashMap::from([
            (
                "status".to_owned(),
                Value::String(PROMISE_REJECTED.to_owned()),
            ),
            ("reason".to_owned(), value),
        ]))
    };
    Value::Object(object)
}
