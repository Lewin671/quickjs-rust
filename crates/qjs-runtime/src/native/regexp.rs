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
        NativeFunction::RegExpEscape => regexp::native_regexp_escape(argument_values)?,
        NativeFunction::RegExpPrototypeExec => {
            regexp::native_regexp_prototype_exec(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeFlags => {
            regexp::native_regexp_prototype_flags(this_value, env)?
        }
        NativeFunction::RegExpPrototypeGlobal => {
            regexp::native_regexp_prototype_flag(this_value, env, 'g')?
        }
        NativeFunction::RegExpPrototypeHasIndices => {
            regexp::native_regexp_prototype_flag(this_value, env, 'd')?
        }
        NativeFunction::RegExpPrototypeIgnoreCase => {
            regexp::native_regexp_prototype_flag(this_value, env, 'i')?
        }
        NativeFunction::RegExpPrototypeMultiline => {
            regexp::native_regexp_prototype_flag(this_value, env, 'm')?
        }
        NativeFunction::RegExpPrototypeSource => {
            regexp::native_regexp_prototype_source(this_value, env)?
        }
        NativeFunction::RegExpPrototypeSticky => {
            regexp::native_regexp_prototype_flag(this_value, env, 'y')?
        }
        NativeFunction::RegExpPrototypeTest => {
            regexp::native_regexp_prototype_test(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeToString => {
            regexp::native_regexp_prototype_to_string(this_value)?
        }
        NativeFunction::RegExpPrototypeUnicode => {
            regexp::native_regexp_prototype_flag(this_value, env, 'u')?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
