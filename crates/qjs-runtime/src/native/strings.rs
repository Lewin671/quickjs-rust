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
            string::native_string(function, this_value, argument_values, is_construct, env)?
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
        NativeFunction::StringPrototypeAnchor => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Anchor,
        )?,
        NativeFunction::StringPrototypeBig => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Big,
        )?,
        NativeFunction::StringPrototypeBlink => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Blink,
        )?,
        NativeFunction::StringPrototypeBold => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Bold,
        )?,
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
        NativeFunction::StringPrototypeFixed => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Fixed,
        )?,
        NativeFunction::StringPrototypeFontcolor => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Fontcolor,
        )?,
        NativeFunction::StringPrototypeFontsize => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Fontsize,
        )?,
        NativeFunction::StringPrototypeIncludes => {
            string::native_string_prototype_includes(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeIndexOf => {
            string::native_string_prototype_index_of(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeItalics => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Italics,
        )?,
        NativeFunction::StringPrototypeIterator => {
            string::native_string_prototype_iterator(this_value, env)?
        }
        NativeFunction::StringIteratorPrototypeNext => {
            string::native_string_iterator_next(this_value)?
        }
        NativeFunction::StringPrototypeIsWellFormed => {
            string::native_string_prototype_is_well_formed(this_value, env)?
        }
        NativeFunction::StringPrototypeLastIndexOf => {
            string::native_string_prototype_last_index_of(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeLink => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Link,
        )?,
        NativeFunction::StringPrototypeLocaleCompare => {
            string::native_string_prototype_locale_compare(this_value, argument_values, env)?
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
        NativeFunction::StringPrototypeReplace => {
            string::native_string_prototype_replace(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeReplaceAll => {
            string::native_string_prototype_replace_all(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeSearch => {
            string::native_string_prototype_search(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeSlice => {
            string::native_string_prototype_slice(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeSmall => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Small,
        )?,
        NativeFunction::StringPrototypeSplit => {
            string::native_string_prototype_split(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeStartsWith => {
            string::native_string_prototype_starts_with(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeStrike => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Strike,
        )?,
        NativeFunction::StringPrototypeSubstr => {
            string::native_string_prototype_substr(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeSubstring => {
            string::native_string_prototype_substring(this_value, argument_values, env)?
        }
        NativeFunction::StringPrototypeSub => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Sub,
        )?,
        NativeFunction::StringPrototypeSup => string::native_string_prototype_html(
            this_value,
            argument_values,
            env,
            string::StringHtmlKind::Sup,
        )?,
        NativeFunction::StringPrototypeToLowerCase => {
            string::native_string_prototype_to_lower_case(this_value, env)?
        }
        NativeFunction::StringPrototypeToLocaleLowerCase => {
            string::native_string_prototype_to_lower_case(this_value, env)?
        }
        NativeFunction::StringPrototypeToLocaleUpperCase => {
            string::native_string_prototype_to_upper_case(this_value, env)?
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
        NativeFunction::StringPrototypeToWellFormed => {
            string::native_string_prototype_to_well_formed(this_value, env)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
