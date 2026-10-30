//! Operation entries for json; bound once when a native is created.

use super::CallInfo;
use crate::{CallEnv, RuntimeError, Value, json};

pub(super) fn json_is_raw_json(
    _env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    json::native_json_is_raw_json(argument_values)
}

pub(super) fn json_parse(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    json::native_json_parse(argument_values, env)
}

pub(super) fn json_raw_json(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    json::native_json_raw_json(argument_values, env)
}

pub(super) fn json_stringify(
    env: &mut CallEnv,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    json::native_json_stringify(argument_values, env)
}
