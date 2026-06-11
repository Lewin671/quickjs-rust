use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::{
    capability::PromiseCapability,
    perform::{self, ElementHandler},
};
use crate::CallEnv;

/// `Promise.race` (ES2023 27.2.4.5): settles with the first input promise to
/// settle. Each element's `then` simply forwards to the result capability's
/// resolve/reject; the capability's single-settlement guard keeps only the
/// first.
pub(crate) fn native_promise_race(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iterable = argument_values.first().cloned().unwrap_or(Value::Undefined);
    perform::run_combinator(this_value, iterable, "Promise.race", RaceHandler, env)
}

struct RaceHandler;

impl ElementHandler for RaceHandler {
    fn on_element(
        &mut self,
        _index: usize,
        capability: &PromiseCapability,
        _env: &mut CallEnv,
    ) -> Result<(Value, Value), RuntimeError> {
        Ok((capability.resolve.clone(), capability.reject.clone()))
    }

    fn on_complete(
        &mut self,
        _count: usize,
        _capability: &PromiseCapability,
        _env: &mut CallEnv,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
}
