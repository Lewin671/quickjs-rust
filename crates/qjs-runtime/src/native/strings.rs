use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, string};

use super::NativeCallResult;

pub(super) fn call_string_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::String => {
            string::native_string(function, this_value, argument_values, is_construct)?
        }
        NativeFunction::StringFromCharCode => {
            string::native_string_from_char_code(argument_values)?
        }
        NativeFunction::StringFromCodePoint => {
            string::native_string_from_code_point(argument_values)?
        }
        NativeFunction::StringRaw => string::native_string_raw(argument_values, env)?,
        NativeFunction::StringPrototypeAt => {
            string::native_string_prototype_at(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeCharAt => {
            string::native_string_prototype_char_at(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeCharCodeAt => {
            string::native_string_prototype_char_code_at(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeCodePointAt => {
            string::native_string_prototype_code_point_at(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeConcat => {
            string::native_string_prototype_concat(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeEndsWith => {
            string::native_string_prototype_ends_with(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeIncludes => {
            string::native_string_prototype_includes(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeIndexOf => {
            string::native_string_prototype_index_of(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeLastIndexOf => {
            string::native_string_prototype_last_index_of(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeMatch => {
            string::native_string_prototype_match(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypePadEnd => string::native_string_prototype_pad(
            this_value,
            argument_values,
            env,
            string::StringPadKind::End,
        )?,
        NativeFunction::StringPrototypePadStart => string::native_string_prototype_pad(
            this_value,
            argument_values,
            env,
            string::StringPadKind::Start,
        )?,
        NativeFunction::StringPrototypeRepeat => {
            string::native_string_prototype_repeat(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeSearch => {
            string::native_string_prototype_search(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeSlice => {
            string::native_string_prototype_slice(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeSplit => {
            string::native_string_prototype_split(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeStartsWith => {
            string::native_string_prototype_starts_with(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeSubstr => {
            string::native_string_prototype_substr(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeSubstring => {
            string::native_string_prototype_substring(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeToLowerCase => {
            string::native_string_prototype_to_lower_case(this_value, env)?
        }
        NativeFunction::StringPrototypeTrim => {
            string::native_string_prototype_trim(this_value, env)?
        }
        NativeFunction::StringPrototypeTrimEnd => {
            string::native_string_prototype_trim_end(this_value, env)?
        }
        NativeFunction::StringPrototypeTrimStart => {
            string::native_string_prototype_trim_start(this_value, env)?
        }
        NativeFunction::StringPrototypeToString | NativeFunction::StringPrototypeValueOf => {
            string::native_string_prototype_to_string(this_value, env)?
        }
        NativeFunction::StringPrototypeToUpperCase => {
            string::native_string_prototype_to_upper_case(this_value, env)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
