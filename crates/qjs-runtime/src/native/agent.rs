//! Operation entries for agent; bound once when a native is created.

use super::CallInfo;
use crate::{RuntimeError, Value, agent};

pub(super) fn agent_start(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    agent::native_agent_start(argument_values, env)
}

pub(super) fn agent_broadcast(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    agent::native_agent_broadcast(argument_values, env)
}

pub(super) fn agent_get_report(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    agent::native_agent_get_report(env)
}

pub(super) fn agent_report(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    agent::native_agent_report(argument_values, env)
}

pub(super) fn agent_sleep(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    agent::native_agent_sleep(argument_values, env)
}

pub(super) fn agent_monotonic_now(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    agent::native_agent_monotonic_now(env)
}

pub(super) fn agent_receive_broadcast(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    agent::native_agent_receive_broadcast(argument_values, env)
}

pub(super) fn agent_leaving(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    agent::native_agent_leaving()
}
