use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, call_function};

const PROMISE_STATE: &str = "\0PromiseState";
const PROMISE_RESULT: &str = "\0PromiseResult";
const PROMISE_TARGET: &str = "\0PromiseTarget";
const PROMISE_PROTOTYPE: &str = "\0PromisePrototype";
const PROMISE_PENDING: &str = "pending";
const PROMISE_FULFILLED: &str = "fulfilled";
const PROMISE_REJECTED: &str = "rejected";

pub(crate) fn install_promise(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let promise_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    promise_prototype.set_to_string_tag("Promise");
    let promise_function = Function::new_native(Some("Promise"), 1, NativeFunction::Promise, true);
    promise_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(promise_function.clone()),
    );
    promise_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(promise_prototype.clone())),
    );
    let mut promise_resolve =
        Function::new_native(Some("resolve"), 1, NativeFunction::PromiseResolve, false);
    promise_resolve.env.insert(
        PROMISE_PROTOTYPE.to_owned(),
        Value::Object(promise_prototype.clone()),
    );
    promise_function.properties.borrow_mut().insert(
        "resolve".to_owned(),
        Property::non_enumerable(Value::Function(promise_resolve)),
    );

    let mut promise_reject =
        Function::new_native(Some("reject"), 1, NativeFunction::PromiseReject, false);
    promise_reject.env.insert(
        PROMISE_PROTOTYPE.to_owned(),
        Value::Object(promise_prototype),
    );
    promise_function.properties.borrow_mut().insert(
        "reject".to_owned(),
        Property::non_enumerable(Value::Function(promise_reject)),
    );

    let value = Value::Function(promise_function);
    env.insert("Promise".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Promise".to_owned(), value);
    }
}

pub(crate) fn native_promise(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor Promise requires 'new'".to_owned(),
        });
    }
    let executor = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(executor, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Promise resolver must be callable".to_owned(),
        });
    }
    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), crate::function_prototype(function)),
    };
    initialize_promise(&object);
    let promise = Value::Object(object.clone());
    let resolve = resolving_function(
        "resolve",
        NativeFunction::PromiseResolveFunction,
        promise.clone(),
    );
    let reject = resolving_function("reject", NativeFunction::PromiseRejectFunction, promise);
    if let Err(error) = call_function(
        executor,
        Value::Undefined,
        vec![resolve, reject.clone()],
        env,
        false,
    ) {
        settle_promise(
            &object,
            PROMISE_REJECTED,
            error.thrown.map_or(Value::Undefined, |value| *value),
        );
    }
    Ok(Value::Object(object))
}

pub(crate) fn native_promise_resolve(
    function: &Function,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if is_promise_value(&value) {
        return Ok(value);
    }
    let promise = promise_object_from_function(function);
    initialize_promise(&promise);
    settle_promise(&promise, PROMISE_FULFILLED, value);
    Ok(Value::Object(promise))
}

pub(crate) fn native_promise_reject(
    function: &Function,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let reason = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let promise = promise_object_from_function(function);
    initialize_promise(&promise);
    settle_promise(&promise, PROMISE_REJECTED, reason);
    Ok(Value::Object(promise))
}

pub(crate) fn native_promise_resolve_function(
    function: &Function,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let promise = promise_from_resolving_function(function)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    settle_promise(&promise, PROMISE_FULFILLED, value);
    Ok(Value::Undefined)
}

pub(crate) fn native_promise_reject_function(
    function: &Function,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let promise = promise_from_resolving_function(function)?;
    let reason = argument_values.first().cloned().unwrap_or(Value::Undefined);
    settle_promise(&promise, PROMISE_REJECTED, reason);
    Ok(Value::Undefined)
}

pub(crate) fn is_promise_object(object: &ObjectRef) -> bool {
    object.own_property(PROMISE_STATE).is_some()
}

fn initialize_promise(object: &ObjectRef) {
    object.set_to_string_tag("Promise");
    if object.own_property(PROMISE_STATE).is_none() {
        object.define_non_enumerable(
            PROMISE_STATE.to_owned(),
            Value::String(PROMISE_PENDING.to_owned()),
        );
        object.define_non_enumerable(PROMISE_RESULT.to_owned(), Value::Undefined);
    }
}

fn settle_promise(object: &ObjectRef, state: &str, result: Value) {
    if !matches!(
        object.own_property(PROMISE_STATE).map(|property| property.value),
        Some(Value::String(current)) if current == PROMISE_PENDING
    ) {
        return;
    }
    object.define_non_enumerable(PROMISE_STATE.to_owned(), Value::String(state.to_owned()));
    object.define_non_enumerable(PROMISE_RESULT.to_owned(), result);
}

fn resolving_function(name: &str, native: NativeFunction, promise: Value) -> Value {
    let mut function = Function::new_native(Some(name), 1, native, false);
    function.env.insert(PROMISE_TARGET.to_owned(), promise);
    Value::Function(function)
}

fn promise_from_resolving_function(function: &Function) -> Result<ObjectRef, RuntimeError> {
    match function.env.get(PROMISE_TARGET).cloned() {
        Some(Value::Object(object)) if is_promise_object(&object) => Ok(object),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Promise resolving function is missing its promise".to_owned(),
        }),
    }
}

fn promise_object_from_function(function: &Function) -> ObjectRef {
    let prototype = match function.env.get(PROMISE_PROTOTYPE).cloned() {
        Some(Value::Object(prototype)) => Some(prototype),
        _ => crate::function_prototype(function),
    };
    ObjectRef::with_prototype(HashMap::new(), prototype)
}

fn is_promise_value(value: &Value) -> bool {
    matches!(value, Value::Object(object) if is_promise_object(object))
}
