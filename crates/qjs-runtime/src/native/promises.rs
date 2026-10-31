//! Operation entries for promises; bound once when a native is created.

use super::CallInfo;
use crate::{RuntimeError, Value, promise};

pub(super) fn promise(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn promise_all(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_all(info.this_value.clone(), argument_values, env)
}

pub(super) fn promise_any(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::any::native_promise_any(info.callee, info.this_value.clone(), argument_values, env)
}

pub(super) fn promise_any_reject_element(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::any::native_promise_any_reject_element(info.callee, argument_values, env)
}

pub(super) fn promise_all_settled(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::all_settled::native_promise_all_settled(info.this_value.clone(), argument_values, env)
}

pub(super) fn promise_all_settled_reject_element(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::all_settled::native_promise_all_settled_reject_element(
        info.callee,
        argument_values,
        env,
    )
}

pub(super) fn promise_all_settled_resolve_element(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::all_settled::native_promise_all_settled_resolve_element(
        info.callee,
        argument_values,
        env,
    )
}

pub(super) fn promise_all_resolve_element(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_all_resolve_element(info.callee, argument_values, env)
}

pub(super) fn promise_get_capabilities_executor(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_get_capabilities_executor(info.callee, argument_values, env)
}

pub(super) fn promise_prototype_catch(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_catch(info.callee, info.this_value.clone(), argument_values, env)
}

pub(super) fn promise_prototype_finally(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_finally(info.callee, info.this_value.clone(), argument_values, env)
}

pub(super) fn promise_prototype_finally_fulfilled(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_finally_fulfilled(info.callee, argument_values, env)
}

pub(super) fn promise_prototype_finally_rejected(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_finally_rejected(info.callee, argument_values, env)
}

pub(super) fn promise_prototype_finally_value_thunk(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_finally_value_thunk(info.callee)
}

pub(super) fn promise_prototype_finally_thrower_thunk(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_finally_thrower_thunk(info.callee)
}

pub(super) fn promise_prototype_then(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_then(info.callee, info.this_value.clone(), argument_values, env)
}

pub(super) fn promise_race(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_race(info.this_value.clone(), argument_values, env)
}

pub(super) fn promise_reject(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_reject(info.this_value.clone(), argument_values, env)
}

pub(super) fn promise_resolve(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_resolve(info.this_value.clone(), argument_values, env)
}

pub(super) fn promise_try(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::r#try::native_promise_try(info.callee, info.this_value.clone(), argument_values, env)
}

pub(super) fn promise_with_resolvers(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::with_resolvers::native_promise_with_resolvers(
        info.callee,
        info.this_value.clone(),
        env,
    )
}

pub(super) fn promise_reject_function(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_reject_function(info.callee, argument_values, env)
}

pub(super) fn promise_resolve_function(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    promise::native_promise_resolve_function(info.callee, argument_values, env)
}
