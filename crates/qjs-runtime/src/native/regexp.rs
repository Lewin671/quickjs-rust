use crate::{Function, NativeFunction, Value, regexp};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_regexp_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::RegExp => {
            regexp::native_regexp(function, this_value, argument_values, is_construct, env)?
        }
        NativeFunction::RegExpEscape => regexp::native_regexp_escape(argument_values)?,
        NativeFunction::RegExpPrototypeCompile => {
            regexp::native_regexp_prototype_compile(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeDotAll => {
            regexp::native_regexp_prototype_flag(function, this_value, env, 's')?
        }
        NativeFunction::RegExpPrototypeExec => {
            regexp::native_regexp_prototype_exec(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeFlags => {
            regexp::native_regexp_prototype_flags(function, this_value, env)?
        }
        NativeFunction::RegExpPrototypeGlobal => {
            regexp::native_regexp_prototype_flag(function, this_value, env, 'g')?
        }
        NativeFunction::RegExpPrototypeHasIndices => {
            regexp::native_regexp_prototype_flag(function, this_value, env, 'd')?
        }
        NativeFunction::RegExpPrototypeIgnoreCase => {
            regexp::native_regexp_prototype_flag(function, this_value, env, 'i')?
        }
        NativeFunction::RegExpPrototypeMatch => {
            regexp::native_regexp_prototype_match(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeMatchAll => {
            regexp::native_regexp_prototype_match_all(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeMultiline => {
            regexp::native_regexp_prototype_flag(function, this_value, env, 'm')?
        }
        NativeFunction::RegExpPrototypeReplace => {
            regexp::native_regexp_prototype_replace(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeSearch => {
            regexp::native_regexp_prototype_search(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeSplit => {
            regexp::native_regexp_prototype_split(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeSource => {
            regexp::native_regexp_prototype_source(function, this_value, env)?
        }
        NativeFunction::RegExpPrototypeSticky => {
            regexp::native_regexp_prototype_flag(function, this_value, env, 'y')?
        }
        NativeFunction::RegExpPrototypeTest => {
            regexp::native_regexp_prototype_test(this_value, argument_values, env)?
        }
        NativeFunction::RegExpPrototypeToString => {
            regexp::native_regexp_prototype_to_string(this_value)?
        }
        NativeFunction::RegExpPrototypeUnicode => {
            regexp::native_regexp_prototype_flag(function, this_value, env, 'u')?
        }
        NativeFunction::RegExpPrototypeUnicodeSets => {
            regexp::native_regexp_prototype_flag(function, this_value, env, 'v')?
        }
        NativeFunction::RegExpStringIteratorPrototypeNext => {
            regexp::native_regexp_string_iterator_next(this_value, env)?
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
