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
        NativeFunction::RegExpPrototypeGlobal => {
            regexp::native_regexp_prototype_global(this_value)?
        }
        NativeFunction::RegExpPrototypeExec => {
            regexp::native_regexp_prototype_exec(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeIgnoreCase => {
            regexp::native_regexp_prototype_ignore_case(this_value)?
        }
        NativeFunction::RegExpPrototypeMultiline => {
            regexp::native_regexp_prototype_multiline(this_value)?
        }
        NativeFunction::RegExpPrototypeSource => {
            regexp::native_regexp_prototype_source(this_value)?
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
