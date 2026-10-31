use crate::function::NativeCaptures;
use crate::function::PromiseElementCaptures;
use std::collections::HashMap;

use crate::{ArrayRef, Function, NativeFunction, ObjectRef, RuntimeError, Value, call_function};

use super::{
    capability::PromiseCapability,
    perform::{self, ElementHandler},
};

/// `Promise.all` (ES2023 27.2.4.1): resolves with an array of every fulfilled
/// value once all input promises fulfil, or rejects with the first rejection.
pub(crate) fn native_promise_all(
    this_value: Value,
    argument_values: &[Value],
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    let iterable = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let handler = AllHandler {
        values: ArrayRef::new(env.realm(), Vec::new()),
        remaining: perform::new_remaining(1, env.realm().atoms()),
    };
    perform::run_combinator(this_value, iterable, "Promise.all", handler, env)
}

struct AllHandler {
    values: ArrayRef,
    remaining: ObjectRef,
}

impl ElementHandler for AllHandler {
    fn on_element(
        &mut self,
        index: usize,
        capability: &PromiseCapability,
        _env: &mut crate::context::Context<'_>,
    ) -> Result<(Value, Value), RuntimeError> {
        // Reserve the slot so out-of-order fulfilment writes the right index.
        self.values.set(index, Value::Undefined);
        perform::increment_remaining(&self.remaining);

        let already_called = ObjectRef::new(HashMap::new(), _env.realm().atoms());
        let mut on_fulfilled = Function::new_native(
            None,
            1,
            NativeFunction::PromiseAllResolveElement,
            false,
            _env.realm(),
        );
        on_fulfilled.set_native_captures(NativeCaptures::PromiseElement(PromiseElementCaptures {
            index,
            values: self.values.clone(),
            remaining: self.remaining.clone(),
            already_called,
            settle: capability.resolve.clone(),
            aggregate_error: None,
        }));

        Ok((Value::Function(on_fulfilled), capability.reject.clone()))
    }

    fn on_complete(
        &mut self,
        _count: usize,
        capability: &PromiseCapability,
        env: &mut crate::context::Context<'_>,
    ) -> Result<(), RuntimeError> {
        // Final decrement: if every element already settled, resolve now.
        if perform::decrement_remaining(&self.remaining) == 0.0 {
            super::capability::capability_resolve(
                capability,
                Value::Array(self.values.clone()),
                env,
            )?;
        }
        Ok(())
    }
}

/// Promise.all Resolve Element function (ES2023 27.2.4.1.3): records the
/// fulfilled value at its index and, when the last outstanding element settles,
/// resolves the result capability with the values array. Guarded so it runs at
/// most once.
pub(crate) fn native_promise_all_resolve_element(
    function: &Function,
    argument_values: &[Value],
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    if already_called(function) {
        return Ok(Value::Undefined);
    }
    let captures = function
        .native_captures()
        .promise_element()
        .ok_or_else(|| RuntimeError {
            thrown: None,
            message: "Promise.all element is missing its capture payload".into(),
        })?;
    let index = captures.index;
    let values = captures.values.clone();
    let remaining = captures.remaining.clone();

    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    values.set(index, value);
    if perform::decrement_remaining(&remaining) == 0.0 {
        let resolve = captures.settle.clone();
        call_function(
            resolve,
            Value::Undefined,
            vec![Value::Array(values)],
            env,
            false,
        )?;
    }
    Ok(Value::Undefined)
}

/// Reads-and-sets the single-call guard, returning whether it was already set.
pub(super) fn already_called(function: &Function) -> bool {
    let Some(captures) = function.native_captures().promise_element() else {
        return false;
    };
    let cell = &captures.already_called;
    if cell.own_property("called").is_some() {
        return true;
    }
    cell.define_non_enumerable("called".to_owned(), Value::Boolean(true));
    false
}
