//! Operation entries for strings; bound once when a native is created.

use super::CallInfo;
use crate::{RuntimeError, Value, string};

pub(super) fn string(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn string_from_char_code(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_from_char_code(argument_values, env)
}

pub(super) fn string_from_code_point(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_from_code_point(argument_values, env)
}

pub(super) fn string_raw(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_raw(argument_values, env)
}

pub(super) fn string_prototype_at(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_at(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_anchor(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Anchor,
    )
}

pub(super) fn string_prototype_big(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Big,
    )
}

pub(super) fn string_prototype_blink(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Blink,
    )
}

pub(super) fn string_prototype_bold(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Bold,
    )
}

pub(super) fn string_prototype_char_at(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_char_at(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_char_code_at(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_char_code_at(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_code_point_at(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_code_point_at(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_concat(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_concat(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_ends_with(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_ends_with(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_fixed(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Fixed,
    )
}

pub(super) fn string_prototype_fontcolor(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Fontcolor,
    )
}

pub(super) fn string_prototype_fontsize(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Fontsize,
    )
}

pub(super) fn string_prototype_includes(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_includes(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_index_of(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_index_of(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_italics(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Italics,
    )
}

pub(super) fn string_prototype_iterator(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_iterator(info.this_value.clone(), env)
}

pub(super) fn string_iterator_prototype_next(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_iterator_next(info.this_value.clone())
}

pub(super) fn string_prototype_is_well_formed(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_is_well_formed(info.this_value.clone(), env)
}

pub(super) fn string_prototype_last_index_of(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_last_index_of(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_link(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Link,
    )
}

pub(super) fn string_prototype_locale_compare(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_locale_compare(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_match(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_match(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_match_all(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_match_all(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_normalize(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_normalize(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_pad_end(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_pad(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringPadKind::End,
    )
}

pub(super) fn string_prototype_pad_start(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_pad(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringPadKind::Start,
    )
}

pub(super) fn string_prototype_repeat(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_repeat(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_replace(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_replace(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_replace_all(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_replace_all(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_search(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_search(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_slice(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_slice(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_small(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Small,
    )
}

pub(super) fn string_prototype_split(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_split(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_starts_with(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_starts_with(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_strike(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Strike,
    )
}

pub(super) fn string_prototype_substr(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_substr(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_substring(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_substring(info.this_value.clone(), argument_values, env)
}

pub(super) fn string_prototype_sub(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Sub,
    )
}

pub(super) fn string_prototype_sup(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_html(
        info.this_value.clone(),
        argument_values,
        env,
        string::StringHtmlKind::Sup,
    )
}

pub(super) fn string_prototype_to_lower_case(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_to_lower_case(info.this_value.clone(), env)
}

pub(super) fn string_prototype_to_locale_lower_case(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_to_lower_case(info.this_value.clone(), env)
}

pub(super) fn string_prototype_to_locale_upper_case(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_to_upper_case(info.this_value.clone(), env)
}

pub(super) fn string_prototype_trim(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_trim(info.this_value.clone(), env)
}

pub(super) fn string_prototype_trim_end(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_trim_end(info.this_value.clone(), env)
}

pub(super) fn string_prototype_trim_start(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_trim_start(info.this_value.clone(), env)
}

pub(super) fn string_prototype_to_string(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_to_string(info.this_value.clone(), env)
}

pub(super) fn string_prototype_value_of(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_to_string(info.this_value.clone(), env)
}

pub(super) fn string_prototype_to_upper_case(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_to_upper_case(info.this_value.clone(), env)
}

pub(super) fn string_prototype_to_well_formed(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    string::native_string_prototype_to_well_formed(info.this_value.clone(), env)
}
