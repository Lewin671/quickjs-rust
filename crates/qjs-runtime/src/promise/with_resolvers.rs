use crate::{Function, ObjectRef, RuntimeError, Value};

use super::capability::new_promise_capability;

/// `Promise.withResolvers` (ES2024 27.2.4.8): builds a capability from the
/// `this` constructor and returns `{ promise, resolve, reject }`.
pub(crate) fn native_promise_with_resolvers(
    function: &Function,
    this_value: Value,
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    let capability = new_promise_capability(&this_value, env)?;
    let result = ObjectRef::with_ordered_prototype(
        vec![
            ("promise".to_owned(), capability.promise),
            ("resolve".to_owned(), capability.resolve),
            ("reject".to_owned(), capability.reject),
        ],
        object_prototype(function),
        env.realm().atoms(),
    );
    Ok(Value::Object(result))
}

fn object_prototype(function: &Function) -> Option<ObjectRef> {
    function
        .native_captures()
        .promise_prototypes()
        .and_then(|captures| captures.object.clone())
}
