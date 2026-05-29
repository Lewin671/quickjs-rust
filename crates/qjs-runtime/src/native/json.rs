use std::collections::HashMap;

use crate::{NativeFunction, Value, json};

use super::NativeCallResult;

pub(super) fn call_json_native(
    native: NativeFunction,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::JsonParse => json::native_json_parse(argument_values, env)?,
        NativeFunction::JsonStringify => json::native_json_stringify(argument_values, env)?,
        _ => return Ok(None),
    };

    Ok(Some(value))
}
