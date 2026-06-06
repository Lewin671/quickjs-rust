use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, ensure_constructor,
};

use super::{
    PROMISE_OBJECT_PROTOTYPE, initialize_promise, promise_object_from_function, resolving_function,
};

pub(crate) fn native_promise_with_resolvers(
    function: &Function,
    this_value: Value,
) -> Result<Value, RuntimeError> {
    ensure_constructor(&this_value)?;
    let promise = promise_object_from_function(function);
    initialize_promise(&promise);
    let result = ObjectRef::with_prototype(
        HashMap::from([
            ("promise".to_owned(), Value::Object(promise.clone())),
            (
                "resolve".to_owned(),
                unnamed_resolving_function(NativeFunction::PromiseResolveFunction, promise.clone()),
            ),
            (
                "reject".to_owned(),
                unnamed_resolving_function(NativeFunction::PromiseRejectFunction, promise),
            ),
        ]),
        object_prototype(function),
    );
    Ok(Value::Object(result))
}

fn object_prototype(function: &Function) -> Option<ObjectRef> {
    match function.env.get(PROMISE_OBJECT_PROTOTYPE).cloned() {
        Some(Value::Object(prototype)) => Some(prototype),
        _ => None,
    }
}

fn unnamed_resolving_function(native: NativeFunction, promise: ObjectRef) -> Value {
    let Value::Function(mut function) = resolving_function("", native, Value::Object(promise))
    else {
        unreachable!("resolving_function returns a function");
    };
    function.name = None;
    function.properties.borrow_mut().insert(
        "name".to_owned(),
        Property::data(Value::String(String::new()), false, false, true),
    );
    Value::Function(function)
}
