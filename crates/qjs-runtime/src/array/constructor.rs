use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    call_function, construct_function, is_truthy,
    object::{
        PropertyDescriptor, array_length_from_descriptor_value, define_array_length_value,
        define_property_descriptor_on_value_key,
    },
    property_value, property_value_key,
    reflect::ordinary_set,
    symbol,
};

use super::array_like::{array_like_length, array_like_values_with_env};
use super::constructor_realm::{array_constructor_prototype_slot, array_with_prototype};
use crate::CallEnv;

const ARRAY_FROM_ASYNC_CAPABILITY: &str = "\0ArrayFromAsyncCapability";
const ARRAY_FROM_ASYNC_CONSTRUCTOR: &str = "\0ArrayFromAsyncConstructor";
const ARRAY_FROM_ASYNC_INDEX: &str = "\0ArrayFromAsyncIndex";
const ARRAY_FROM_ASYNC_ITERATOR: &str = "\0ArrayFromAsyncIterator";
const ARRAY_FROM_ASYNC_LENGTH: &str = "\0ArrayFromAsyncLength";
const ARRAY_FROM_ASYNC_MAPPING: &str = "\0ArrayFromAsyncMapping";
const ARRAY_FROM_ASYNC_NEXT: &str = "\0ArrayFromAsyncNext";
const ARRAY_FROM_ASYNC_RECEIVER: &str = "\0ArrayFromAsyncReceiver";
const ARRAY_FROM_ASYNC_TARGET: &str = "\0ArrayFromAsyncTarget";
const ARRAY_FROM_ASYNC_THIS_ARG: &str = "\0ArrayFromAsyncThisArg";

pub(crate) fn native_array(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let prototype = array_constructor_prototype_slot(function, is_construct, env)?;
    if let [Value::Number(length)] = argument_values {
        let length = array_length_from_descriptor_value(Value::Number(*length), env)?;
        return Ok(Value::Array(array_with_prototype(
            ArrayRef::new_with_length(length),
            prototype,
        )));
    }

    Ok(Value::Array(array_with_prototype(
        ArrayRef::new(argument_values.to_vec()),
        prototype,
    )))
}

pub(crate) fn native_array_from(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let items = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let map_fn = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let this_arg = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let mapping = match map_fn {
        Value::Undefined => None,
        Value::Function(_) => Some(map_fn),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Array.from map function is not callable".to_owned(),
            });
        }
    };

    let constructor = array_from_constructor(this_value);
    array_from_values(items, mapping.as_ref(), this_arg, constructor, env)
}

pub(crate) fn native_array_from_async(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let items = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let map_fn = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let this_arg = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let mapping = match map_fn {
        Value::Undefined => None,
        Value::Function(_) => Some(map_fn),
        _ => {
            return array_from_async_rejected(
                RuntimeError {
                    thrown: None,
                    message: "TypeError: Array.fromAsync map function is not callable".to_owned(),
                },
                env,
            );
        }
    };

    let constructor = array_from_constructor(this_value);
    array_from_async_start(items, mapping, this_arg, constructor, env)
}

struct ArrayFromElements {
    values: Vec<Value>,
    construct_length: Option<usize>,
}

fn array_from_values(
    items: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    constructor: Option<Value>,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(items, Value::Null | Value::Undefined) {
        let elements = array_like_values_with_env(items, "Array.from", env).map(|values| {
            ArrayFromElements {
                construct_length: Some(values.len()),
                values,
            }
        })?;
        return array_from_array_like_result(constructor, elements, env);
    }

    let iterator_method = match symbol::iterator_symbol(env) {
        Some(iterator_symbol) => {
            property_value_key(items.clone(), &PropertyKey::Symbol(iterator_symbol), env)?
        }
        None => Value::Undefined,
    };

    match iterator_method {
        Value::Undefined | Value::Null => {
            let values = map_array_like_values(items, mapping, this_arg, env)?;
            array_from_array_like_result(
                constructor,
                ArrayFromElements {
                    construct_length: Some(values.len()),
                    values,
                },
                env,
            )
        }
        Value::Function(ref function)
            if mapping.is_none()
                && function.native_kind() == Some(NativeFunction::ArrayPrototypeValues)
                && super::iterator::array_iterator_next_is_native(env)
                && is_native_array_constructor(constructor.as_ref()) =>
        {
            if let Value::Array(array) = &items
                && let Some(values) = array.dense_argument_values(env)
            {
                return Ok(Value::Array(ArrayRef::new(values)));
            }
            array_from_iterable_result(items, iterator_method, mapping, this_arg, constructor, env)
        }
        Value::Function(_) => {
            array_from_iterable_result(items, iterator_method, mapping, this_arg, constructor, env)
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "Array.from iterator method is not callable".to_owned(),
        }),
    }
}

fn is_native_array_constructor(constructor: Option<&Value>) -> bool {
    matches!(
        constructor,
        Some(Value::Function(function)) if function.native_kind() == Some(NativeFunction::Array)
    )
}

fn array_from_array_like_result(
    constructor: Option<Value>,
    elements: ArrayFromElements,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let length = elements.values.len();
    let Some(constructor) = constructor else {
        return Ok(Value::Array(ArrayRef::new(elements.values)));
    };

    let arguments = elements
        .construct_length
        .map(|length| vec![Value::Number(length as f64)])
        .unwrap_or_default();
    let target = construct_function(constructor.clone(), constructor, arguments, env)?;
    for (index, value) in elements.values.into_iter().enumerate() {
        create_data_property_or_throw(target.clone(), index.to_string(), value, env)?;
    }
    set_array_from_length(target.clone(), length, env)?;
    Ok(target)
}

fn array_from_async_start(
    items: Value,
    mapping: Option<Value>,
    this_arg: Value,
    constructor: Option<Value>,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let capability = crate::promise::new_pending_promise(env);
    let state = ObjectRef::new(HashMap::new());
    state.define_non_enumerable(
        ARRAY_FROM_ASYNC_CAPABILITY.to_owned(),
        Value::Object(capability.clone()),
    );
    if let Some(mapping) = mapping {
        state.define_non_enumerable(ARRAY_FROM_ASYNC_MAPPING.to_owned(), mapping);
        state.define_non_enumerable(ARRAY_FROM_ASYNC_THIS_ARG.to_owned(), this_arg);
    }
    if let Some(constructor) = constructor.clone() {
        state.define_non_enumerable(ARRAY_FROM_ASYNC_CONSTRUCTOR.to_owned(), constructor);
    }

    let start_result = array_from_async_start_result(items, constructor, &state, env);
    if let Err(error) = start_result {
        array_from_async_reject_error(&state, error, env);
    }
    Ok(Value::Object(capability))
}

fn array_from_async_start_result(
    items: Value,
    constructor: Option<Value>,
    state: &ObjectRef,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    if matches!(items, Value::Null | Value::Undefined) {
        array_like_values_with_env(items.clone(), "Array.fromAsync", env)?;
    }

    if let Some(iterator) = array_from_async_iterator(items.clone(), env)? {
        let target = match constructor {
            Some(constructor) => {
                construct_function(constructor.clone(), constructor, Vec::new(), env)?
            }
            None => Value::Array(ArrayRef::new(Vec::new())),
        };
        let next = property_value(iterator.clone(), "next", env)?;
        if !matches!(next, Value::Function(_)) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Array.fromAsync iterator next method is not callable"
                    .to_owned(),
            });
        }
        state.define_non_enumerable(ARRAY_FROM_ASYNC_TARGET.to_owned(), target);
        state.define_non_enumerable(ARRAY_FROM_ASYNC_ITERATOR.to_owned(), iterator);
        state.define_non_enumerable(ARRAY_FROM_ASYNC_NEXT.to_owned(), next);
        state.define_non_enumerable(ARRAY_FROM_ASYNC_INDEX.to_owned(), Value::Number(0.0));
        array_from_async_continue_iterator(state, env);
        return Ok(());
    }

    let source = array_like_length(items, "Array.fromAsync", env)?;
    if constructor.is_none() && source.length > u32::MAX as usize {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    let target = match constructor {
        Some(constructor) => construct_function(
            constructor.clone(),
            constructor,
            vec![Value::Number(source.length as f64)],
            env,
        )?,
        None => Value::Array(ArrayRef::new(Vec::new())),
    };
    state.define_non_enumerable(ARRAY_FROM_ASYNC_TARGET.to_owned(), target);
    state.define_non_enumerable(ARRAY_FROM_ASYNC_RECEIVER.to_owned(), source.receiver);
    state.define_non_enumerable(
        ARRAY_FROM_ASYNC_LENGTH.to_owned(),
        Value::Number(source.length as f64),
    );
    state.define_non_enumerable(ARRAY_FROM_ASYNC_INDEX.to_owned(), Value::Number(0.0));
    array_from_async_continue_array_like(state, env);
    Ok(())
}

fn array_from_async_iterator(
    items: Value,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    if let Some(async_iterator_symbol) = symbol::async_iterator_symbol(env) {
        let method = property_value_key(
            items.clone(),
            &PropertyKey::Symbol(async_iterator_symbol),
            env,
        )?;
        match method {
            Value::Undefined | Value::Null => {}
            Value::Function(_) => {
                let iterator = call_function(method, items, Vec::new(), env, false)?;
                if !matches!(
                    iterator,
                    Value::Object(_) | Value::Array(_) | Value::Function(_)
                ) {
                    return Err(RuntimeError {
                        thrown: None,
                        message: "TypeError: async iterator method must return an object"
                            .to_owned(),
                    });
                }
                return Ok(Some(iterator));
            }
            _ => {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: Array.fromAsync async iterator method is not callable"
                        .to_owned(),
                });
            }
        }
    }

    let iterator_method = match symbol::iterator_symbol(env) {
        Some(iterator_symbol) => {
            property_value_key(items.clone(), &PropertyKey::Symbol(iterator_symbol), env)?
        }
        None => Value::Undefined,
    };
    match iterator_method {
        Value::Undefined | Value::Null => Ok(None),
        Value::Function(_) => {
            let sync_iterator = call_function(iterator_method, items, Vec::new(), env, false)?;
            crate::async_generator::create_async_from_sync_iterator(sync_iterator, env).map(Some)
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: Array.fromAsync iterator method is not callable".to_owned(),
        }),
    }
}

fn array_from_async_rejected(
    error: RuntimeError,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let promise_constructor = promise_constructor(env)?;
    let reason = crate::error::runtime_error_to_value(error, env);
    crate::promise::native_promise_reject(promise_constructor, &[reason], env)
}

fn array_from_async_continue_array_like(state: &ObjectRef, env: &mut CallEnv) {
    let index = array_from_async_index(state);
    let length = array_from_async_length(state);
    if index >= length {
        array_from_async_finish(state, length, env);
        return;
    }
    let Some(receiver) = state_value(state, ARRAY_FROM_ASYNC_RECEIVER) else {
        array_from_async_reject_message(state, "TypeError: Array.fromAsync state is invalid", env);
        return;
    };
    let value = match property_value(receiver, &index.to_string(), env) {
        Ok(value) => value,
        Err(error) => {
            array_from_async_reject_error(state, error, env);
            return;
        }
    };
    let on_fulfilled =
        array_from_async_reaction(NativeFunction::ArrayFromAsyncArrayLikeValueFulfilled, state);
    let on_rejected = array_from_async_reaction(NativeFunction::ArrayFromAsyncRejected, state);
    crate::promise::perform_await(value, on_fulfilled, on_rejected, env);
}

pub(crate) fn native_array_from_async_array_like_value_fulfilled(
    function: &Function,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let state = array_from_async_state(function)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if let Some(mapping) = state_value(&state, ARRAY_FROM_ASYNC_MAPPING) {
        let mapped = call_function(
            mapping,
            state_value(&state, ARRAY_FROM_ASYNC_THIS_ARG).unwrap_or(Value::Undefined),
            vec![value, Value::Number(array_from_async_index(&state) as f64)],
            env,
            false,
        );
        match mapped {
            Ok(mapped) => {
                let on_fulfilled = array_from_async_reaction(
                    NativeFunction::ArrayFromAsyncArrayLikeMappedFulfilled,
                    &state,
                );
                let on_rejected =
                    array_from_async_reaction(NativeFunction::ArrayFromAsyncRejected, &state);
                crate::promise::perform_await(mapped, on_fulfilled, on_rejected, env);
            }
            Err(error) => array_from_async_reject_error(&state, error, env),
        }
        return Ok(Value::Undefined);
    }
    array_from_async_store_and_continue_array_like(&state, value, env);
    Ok(Value::Undefined)
}

pub(crate) fn native_array_from_async_array_like_mapped_fulfilled(
    function: &Function,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let state = array_from_async_state(function)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    array_from_async_store_and_continue_array_like(&state, value, env);
    Ok(Value::Undefined)
}

fn array_from_async_store_and_continue_array_like(
    state: &ObjectRef,
    value: Value,
    env: &mut CallEnv,
) {
    if array_from_async_store(state, value, env) {
        array_from_async_increment_index(state);
        array_from_async_continue_array_like(state, env);
    }
}

fn array_from_async_continue_iterator(state: &ObjectRef, env: &mut CallEnv) {
    let (Some(iterator), Some(next)) = (
        state_value(state, ARRAY_FROM_ASYNC_ITERATOR),
        state_value(state, ARRAY_FROM_ASYNC_NEXT),
    ) else {
        array_from_async_reject_message(state, "TypeError: Array.fromAsync state is invalid", env);
        return;
    };
    let step = match call_function(next, iterator, Vec::new(), env, false) {
        Ok(step) => step,
        Err(error) => {
            array_from_async_reject_error(state, error, env);
            return;
        }
    };
    let on_fulfilled =
        array_from_async_reaction(NativeFunction::ArrayFromAsyncIteratorStepFulfilled, state);
    let on_rejected =
        array_from_async_reaction(NativeFunction::ArrayFromAsyncIteratorRejected, state);
    crate::promise::perform_await(step, on_fulfilled, on_rejected, env);
}

pub(crate) fn native_array_from_async_iterator_step_fulfilled(
    function: &Function,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let state = array_from_async_state(function)?;
    let step = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_iterator_result_object(&step) {
        array_from_async_close_and_reject_message(
            &state,
            "TypeError: Array.fromAsync iterator result is not an object",
            env,
        );
        return Ok(Value::Undefined);
    }
    if is_truthy(&property_value(step.clone(), "done", env)?) {
        array_from_async_finish(&state, array_from_async_index(&state), env);
        return Ok(Value::Undefined);
    }
    let value = property_value(step, "value", env)?;
    if let Some(mapping) = state_value(&state, ARRAY_FROM_ASYNC_MAPPING) {
        let mapped = call_function(
            mapping,
            state_value(&state, ARRAY_FROM_ASYNC_THIS_ARG).unwrap_or(Value::Undefined),
            vec![value, Value::Number(array_from_async_index(&state) as f64)],
            env,
            false,
        );
        match mapped {
            Ok(mapped) => {
                let on_fulfilled = array_from_async_reaction(
                    NativeFunction::ArrayFromAsyncIteratorMappedFulfilled,
                    &state,
                );
                let on_rejected = array_from_async_reaction(
                    NativeFunction::ArrayFromAsyncIteratorRejected,
                    &state,
                );
                crate::promise::perform_await(mapped, on_fulfilled, on_rejected, env);
            }
            Err(error) => array_from_async_close_and_reject_error(&state, error, env),
        }
        return Ok(Value::Undefined);
    }
    array_from_async_store_and_continue_iterator(&state, value, env);
    Ok(Value::Undefined)
}

pub(crate) fn native_array_from_async_iterator_mapped_fulfilled(
    function: &Function,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let state = array_from_async_state(function)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    array_from_async_store_and_continue_iterator(&state, value, env);
    Ok(Value::Undefined)
}

pub(crate) fn native_array_from_async_iterator_rejected(
    function: &Function,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let state = array_from_async_state(function)?;
    let reason = argument_values.first().cloned().unwrap_or(Value::Undefined);
    array_from_async_close_iterator(&state, env);
    array_from_async_reject_value(&state, reason, env);
    Ok(Value::Undefined)
}

fn array_from_async_store_and_continue_iterator(
    state: &ObjectRef,
    value: Value,
    env: &mut CallEnv,
) {
    if array_from_async_store(state, value, env) {
        array_from_async_increment_index(state);
        array_from_async_continue_iterator(state, env);
    }
}

pub(crate) fn native_array_from_async_rejected(
    function: &Function,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let state = array_from_async_state(function)?;
    let reason = argument_values.first().cloned().unwrap_or(Value::Undefined);
    array_from_async_reject_value(&state, reason, env);
    Ok(Value::Undefined)
}

fn array_from_async_finish(state: &ObjectRef, length: usize, env: &mut CallEnv) {
    let Some(target) = state_value(state, ARRAY_FROM_ASYNC_TARGET) else {
        array_from_async_reject_message(state, "TypeError: Array.fromAsync state is invalid", env);
        return;
    };
    match set_array_from_length(target.clone(), length, env) {
        Ok(()) => array_from_async_resolve_value(state, target, env),
        Err(error) => array_from_async_reject_error(state, error, env),
    }
}

fn array_from_async_store(state: &ObjectRef, value: Value, env: &mut CallEnv) -> bool {
    let Some(target) = state_value(state, ARRAY_FROM_ASYNC_TARGET) else {
        array_from_async_reject_message(state, "TypeError: Array.fromAsync state is invalid", env);
        return false;
    };
    let index = array_from_async_index(state);
    match create_data_property_or_throw(target, index.to_string(), value, env) {
        Ok(()) => true,
        Err(error) => {
            if state.own_property(ARRAY_FROM_ASYNC_ITERATOR).is_some() {
                array_from_async_close_and_reject_error(state, error, env);
            } else {
                array_from_async_reject_error(state, error, env);
            }
            false
        }
    }
}

fn array_from_async_close_and_reject_message(state: &ObjectRef, message: &str, env: &mut CallEnv) {
    array_from_async_close_iterator(state, env);
    array_from_async_reject_message(state, message, env);
}

fn array_from_async_close_and_reject_error(
    state: &ObjectRef,
    error: RuntimeError,
    env: &mut CallEnv,
) {
    array_from_async_close_iterator(state, env);
    array_from_async_reject_error(state, error, env);
}

fn array_from_async_close_iterator(state: &ObjectRef, env: &mut CallEnv) {
    let Some(iterator) = state_value(state, ARRAY_FROM_ASYNC_ITERATOR) else {
        return;
    };
    let Ok(return_method) = property_value(iterator.clone(), "return", env) else {
        return;
    };
    if matches!(return_method, Value::Function(_)) {
        let _ = call_function(return_method, iterator, Vec::new(), env, false);
    }
}

fn array_from_async_resolve_value(state: &ObjectRef, value: Value, env: &mut CallEnv) {
    if let Some(Value::Object(capability)) = state_value(state, ARRAY_FROM_ASYNC_CAPABILITY) {
        crate::promise::resolve_promise_capability(&capability, value, env);
    }
}

fn array_from_async_reject_error(state: &ObjectRef, error: RuntimeError, env: &mut CallEnv) {
    let reason = match error.thrown {
        Some(value) => *value,
        None => crate::error::runtime_error_to_value(error, env),
    };
    array_from_async_reject_value(state, reason, env);
}

fn array_from_async_reject_message(state: &ObjectRef, message: &str, env: &mut CallEnv) {
    let reason = crate::error::runtime_error_to_value(
        RuntimeError {
            thrown: None,
            message: message.to_owned(),
        },
        env,
    );
    array_from_async_reject_value(state, reason, env);
}

fn array_from_async_reject_value(state: &ObjectRef, reason: Value, env: &mut CallEnv) {
    if let Some(Value::Object(capability)) = state_value(state, ARRAY_FROM_ASYNC_CAPABILITY) {
        crate::promise::reject_promise_capability(&capability, reason, env);
    }
}

fn array_from_async_reaction(native: NativeFunction, state: &ObjectRef) -> Value {
    let mut function = Function::new_native(None, 1, native, false);
    function.insert_native_context(
        ARRAY_FROM_ASYNC_STATE.to_owned(),
        Value::Object(state.clone()),
    );
    Value::Function(function)
}

const ARRAY_FROM_ASYNC_STATE: &str = "\0ArrayFromAsyncState";

fn array_from_async_state(function: &Function) -> Result<ObjectRef, RuntimeError> {
    match function.native_context.get(ARRAY_FROM_ASYNC_STATE) {
        Some(Value::Object(state)) => Ok(state.clone()),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: Array.fromAsync reaction state is missing".to_owned(),
        }),
    }
}

fn array_from_async_index(state: &ObjectRef) -> usize {
    match state
        .own_property(ARRAY_FROM_ASYNC_INDEX)
        .map(|property| property.value)
    {
        Some(Value::Number(index)) if index.is_finite() && index >= 0.0 => index as usize,
        _ => 0,
    }
}

fn array_from_async_length(state: &ObjectRef) -> usize {
    match state
        .own_property(ARRAY_FROM_ASYNC_LENGTH)
        .map(|property| property.value)
    {
        Some(Value::Number(length)) if length.is_finite() && length >= 0.0 => length as usize,
        _ => 0,
    }
}

fn array_from_async_increment_index(state: &ObjectRef) {
    let next = array_from_async_index(state) + 1;
    state.define_non_enumerable(
        ARRAY_FROM_ASYNC_INDEX.to_owned(),
        Value::Number(next as f64),
    );
}

fn state_value(state: &ObjectRef, key: &str) -> Option<Value> {
    state.own_property(key).map(|property| property.value)
}

fn promise_constructor(env: &CallEnv) -> Result<Value, RuntimeError> {
    env.get("Promise").ok_or_else(|| RuntimeError {
        thrown: None,
        message: "TypeError: Promise constructor is unavailable".to_owned(),
    })
}

fn array_from_constructor(value: Value) -> Option<Value> {
    match &value {
        Value::Function(function) if function.constructable => Some(value),
        _ => None,
    }
}

fn create_data_property_or_throw(
    target: Value,
    key: String,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let key = PropertyKey::String(key);
    let descriptor = PropertyDescriptor::data(value, true, true, true);
    if define_property_descriptor_on_value_key(target, key, descriptor, env)? {
        Ok(())
    } else {
        Err(create_data_property_error())
    }
}

fn create_data_property_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.from cannot create result property".to_owned(),
    }
}

fn map_array_like_values(
    items: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    let values = array_like_values_with_env(items, "Array.from", env)?;
    map_array_from_values(values, mapping, this_arg, env)
}

fn map_array_from_values(
    values: Vec<Value>,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    let mut result = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        result.push(array_from_mapped_value(
            value,
            index,
            mapping,
            this_arg.clone(),
            env,
        )?);
    }
    Ok(result)
}

fn array_from_iterable_result(
    items: Value,
    iterator_method: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    constructor: Option<Value>,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = match constructor {
        Some(constructor) => construct_function(constructor.clone(), constructor, Vec::new(), env)?,
        None => Value::Array(ArrayRef::new(Vec::new())),
    };
    let iterator = call_function(iterator_method, items, Vec::new(), env, false)?;
    let next = property_value(iterator.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.from iterator next method is not callable".to_owned(),
        });
    }

    let mut index = 0usize;
    loop {
        let step = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
        if !is_iterator_result_object(&step) {
            return Err(RuntimeError {
                thrown: None,
                message: "Array.from iterator result is not an object".to_owned(),
            });
        }
        if is_truthy(&property_value(step.clone(), "done", env)?) {
            set_array_from_length(target.clone(), index, env)?;
            break;
        }
        let value = property_value(step, "value", env)?;
        let value = match array_from_mapped_value(value, index, mapping, this_arg.clone(), env) {
            Ok(value) => value,
            Err(error) => {
                let _ = close_array_from_iterator(iterator, env);
                return Err(error);
            }
        };
        if let Err(error) =
            create_data_property_or_throw(target.clone(), index.to_string(), value, env)
        {
            let _ = close_array_from_iterator(iterator, env);
            return Err(error);
        }
        index += 1;
    }
    Ok(target)
}

fn close_array_from_iterator(iterator: Value, env: &mut CallEnv) -> Result<(), RuntimeError> {
    match property_value(iterator.clone(), "return", env)? {
        Value::Undefined | Value::Null => Ok(()),
        return_method @ Value::Function(_) => {
            let result = call_function(return_method, iterator, Vec::new(), env, false)?;
            if is_iterator_result_object(&result) {
                Ok(())
            } else {
                Err(RuntimeError {
                    thrown: None,
                    message: "Array.from iterator return result is not an object".to_owned(),
                })
            }
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "Array.from iterator return method is not callable".to_owned(),
        }),
    }
}

fn set_array_from_length(
    target: Value,
    length: usize,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    if let Value::Array(elements) = &target {
        if define_array_length_value(elements, Value::Number(length as f64), env)? {
            return Ok(());
        }
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Array.from cannot set result length".to_owned(),
        });
    }

    let key = PropertyKey::String("length".to_owned());
    let value = Value::Number(length as f64);
    if ordinary_set(target.clone(), &key, value, target, env)? {
        Ok(())
    } else {
        Err(RuntimeError {
            thrown: None,
            message: "TypeError: Array.from cannot set result length".to_owned(),
        })
    }
}

fn array_from_mapped_value(
    value: Value,
    index: usize,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if let Some(callback) = mapping {
        call_function(
            callback.clone(),
            this_arg,
            vec![value, Value::Number(index as f64)],
            env,
            false,
        )
    } else {
        Ok(value)
    }
}

fn is_iterator_result_object(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}

pub(crate) fn native_array_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let length = argument_values.len();
    let result = if matches!(&this_value, Value::Function(function) if function.constructable) {
        construct_function(
            this_value.clone(),
            this_value,
            vec![Value::Number(length as f64)],
            env,
        )?
    } else {
        Value::Array(ArrayRef::new(Vec::new()))
    };
    for (index, value) in argument_values.iter().enumerate() {
        create_array_of_data_property(&result, index.to_string(), value.clone(), env)?;
    }
    set_array_of_length(&result, Value::Number(length as f64), env)?;
    Ok(result)
}

fn create_array_of_data_property(
    target: &Value,
    key: String,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let key = PropertyKey::String(key);
    let descriptor = PropertyDescriptor::data(value, true, true, true);
    if define_property_descriptor_on_value_key(target.clone(), key, descriptor, env)? {
        Ok(())
    } else {
        Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot create array property".to_owned(),
        })
    }
}

fn set_array_of_length(
    target: &Value,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    match target {
        Value::Object(object) => set_object_array_of_length(
            target,
            object.own_property("length"),
            value,
            |property| {
                object.define_property("length".to_owned(), property);
            },
            env,
        ),
        Value::Function(function) => set_object_array_of_length(
            target,
            function.own_property("length"),
            value,
            |property| {
                function.define_property("length".to_owned(), property);
            },
            env,
        ),
        Value::Map(map) => {
            let object = map.object();
            set_object_array_of_length(
                target,
                object.own_property("length"),
                value,
                |property| {
                    object.define_property("length".to_owned(), property);
                },
                env,
            )
        }
        Value::Set(set) => {
            let object = set.object();
            set_object_array_of_length(
                target,
                object.own_property("length"),
                value,
                |property| {
                    object.define_property("length".to_owned(), property);
                },
                env,
            )
        }
        Value::Array(_) => Ok(()),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot set array length".to_owned(),
        }),
    }
}

fn set_object_array_of_length(
    target: &Value,
    existing: Option<Property>,
    value: Value,
    define: impl FnOnce(Property),
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    if let Some(existing) = existing {
        if existing.is_accessor() {
            let (_, setter) = existing
                .into_accessor_parts()
                .expect("accessor properties have accessor state");
            let Some(setter) = setter else {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: length property has no setter".to_owned(),
                });
            };
            call_function(setter, target.clone(), vec![value], env, false)?;
            return Ok(());
        }
        if !existing.writable {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: length property is not writable".to_owned(),
            });
        }
        define(Property::data(
            value,
            existing.enumerable,
            existing.writable,
            existing.configurable,
        ));
        return Ok(());
    }
    define(Property::data(value, false, true, true));
    Ok(())
}
