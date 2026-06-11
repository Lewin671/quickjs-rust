use std::collections::HashMap;

use crate::{ArrayRef, Function, NativeFunction, ObjectRef, RuntimeError, Value, call_function};

use super::{
    PROMISE_ALL_ALREADY_CALLED, PROMISE_ALL_CAPABILITY_RESOLVE, PROMISE_ALL_INDEX,
    PROMISE_ALL_REMAINING, PROMISE_ALL_VALUES,
    capability::PromiseCapability,
    perform::{self, ElementHandler},
};
use crate::CallEnv;

/// `Promise.all` (ES2023 27.2.4.1): resolves with an array of every fulfilled
/// value once all input promises fulfil, or rejects with the first rejection.
pub(crate) fn native_promise_all(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iterable = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let handler = AllHandler {
        values: ArrayRef::new(Vec::new()),
        remaining: perform::new_remaining(1),
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
        _env: &mut CallEnv,
    ) -> Result<(Value, Value), RuntimeError> {
        // Reserve the slot so out-of-order fulfilment writes the right index.
        self.values.set(index, Value::Undefined);
        perform::increment_remaining(&self.remaining);

        let already_called = ObjectRef::new(HashMap::new());
        let mut on_fulfilled =
            Function::new_native(None, 1, NativeFunction::PromiseAllResolveElement, false);
        on_fulfilled.env.insert(
            PROMISE_ALL_CAPABILITY_RESOLVE.to_owned(),
            capability.resolve.clone(),
        );
        on_fulfilled
            .env
            .insert(PROMISE_ALL_INDEX.to_owned(), Value::Number(index as f64));
        on_fulfilled.env.insert(
            PROMISE_ALL_VALUES.to_owned(),
            Value::Array(self.values.clone()),
        );
        on_fulfilled.env.insert(
            PROMISE_ALL_REMAINING.to_owned(),
            Value::Object(self.remaining.clone()),
        );
        on_fulfilled.env.insert(
            PROMISE_ALL_ALREADY_CALLED.to_owned(),
            Value::Object(already_called),
        );

        Ok((Value::Function(on_fulfilled), capability.reject.clone()))
    }

    fn on_complete(
        &mut self,
        _count: usize,
        capability: &PromiseCapability,
        env: &mut CallEnv,
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if already_called(function) {
        return Ok(Value::Undefined);
    }
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
    if perform::decrement_remaining(&remaining) == 0.0 {
        let resolve = function
            .env
            .get(PROMISE_ALL_CAPABILITY_RESOLVE)
            .cloned()
            .unwrap_or(Value::Undefined);
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
    let Some(Value::Object(cell)) = function.env.get(PROMISE_ALL_ALREADY_CALLED) else {
        return false;
    };
    if cell.own_property("called").is_some() {
        return true;
    }
    cell.define_non_enumerable("called".to_owned(), Value::Boolean(true));
    false
}
