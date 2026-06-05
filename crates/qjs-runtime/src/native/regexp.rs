use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, regexp};

use super::NativeCallResult;

pub(super) fn call_regexp_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::RegExp => {
            regexp::native_regexp(function, this_value, argument_values, is_construct, env)?
        }
        NativeFunction::RegExpPrototypeExec => {
            regexp::native_regexp_prototype_exec(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeTest => {
            regexp::native_regexp_prototype_test(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeToString => {
            regexp::native_regexp_prototype_to_string(this_value)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
