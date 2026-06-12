use crate::{NativeFunction, Value, json};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_json_native(
    native: NativeFunction,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::JsonIsRawJson => json::native_json_is_raw_json(argument_values)?,
        NativeFunction::JsonParse => json::native_json_parse(argument_values, env)?,
        NativeFunction::JsonRawJson => json::native_json_raw_json(argument_values, env)?,
        NativeFunction::JsonStringify => json::native_json_stringify(argument_values, env)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
