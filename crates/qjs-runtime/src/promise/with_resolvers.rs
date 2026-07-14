use std::collections::HashMap;

use crate::{Function, ObjectRef, RuntimeError, Value};

use super::{PROMISE_OBJECT_PROTOTYPE, capability::new_promise_capability};
use crate::CallEnv;

/// `Promise.withResolvers` (ES2024 27.2.4.8): builds a capability from the
/// `this` constructor and returns `{ promise, resolve, reject }`.
pub(crate) fn native_promise_with_resolvers(
    function: &Function,
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let capability = new_promise_capability(&this_value, env)?;
    let result = ObjectRef::with_prototype(
        HashMap::from([
            ("promise".to_owned(), capability.promise),
            ("resolve".to_owned(), capability.resolve),
            ("reject".to_owned(), capability.reject),
        ]),
        object_prototype(function),
    );
    Ok(Value::Object(result))
}

fn object_prototype(function: &Function) -> Option<ObjectRef> {
    match function
        .native_context
        .get(PROMISE_OBJECT_PROTOTYPE)
        .cloned()
    {
        Some(Value::Object(prototype)) => Some(prototype),
        _ => None,
    }
}
