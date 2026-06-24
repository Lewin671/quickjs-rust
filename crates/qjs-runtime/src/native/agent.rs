use crate::{CallEnv, NativeFunction, Value, agent};

use super::NativeCallResult;

/// Dispatches the Test262 `$262.agent` primitives. Returns `Ok(None)` for any
/// other native so the main dispatcher falls through to the next family.
pub(super) fn call_agent_native(
    native: NativeFunction,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::AgentStart => agent::native_agent_start(argument_values, env)?,
        NativeFunction::AgentBroadcast => agent::native_agent_broadcast(argument_values, env)?,
        NativeFunction::AgentGetReport => agent::native_agent_get_report(env)?,
        NativeFunction::AgentReport => agent::native_agent_report(argument_values, env)?,
        NativeFunction::AgentSleep => agent::native_agent_sleep(argument_values, env)?,
        NativeFunction::AgentMonotonicNow => agent::native_agent_monotonic_now(env)?,
        NativeFunction::AgentReceiveBroadcast => {
            agent::native_agent_receive_broadcast(argument_values, env)?
        }
        NativeFunction::AgentLeaving => agent::native_agent_leaving()?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}
