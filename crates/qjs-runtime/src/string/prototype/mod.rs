mod case;
mod code_units;
mod html;
mod padding;
mod search;
mod sequence;
mod trim;
mod well_formed;

/// Maximum string length, matching QuickJS-NG's `JS_STRING_LEN_MAX` (2^30 - 1).
/// String-producing builtins throw `RangeError: invalid string length` rather
/// than attempt an allocation beyond this.
pub(super) const MAX_STRING_LENGTH: usize = (1 << 30) - 1;

pub(crate) use case::{
    native_string_prototype_locale_compare, native_string_prototype_normalize,
    native_string_prototype_to_lower_case, native_string_prototype_to_upper_case,
};
pub(crate) use code_units::{
    native_string_prototype_at, native_string_prototype_char_at,
    native_string_prototype_char_code_at, native_string_prototype_code_point_at,
};
pub(crate) use html::{StringHtmlKind, native_string_prototype_html};
pub(crate) use padding::{StringPadKind, native_string_prototype_pad};
pub(crate) use search::{
    native_string_prototype_ends_with, native_string_prototype_includes,
    native_string_prototype_index_of, native_string_prototype_last_index_of,
    native_string_prototype_match, native_string_prototype_match_all,
    native_string_prototype_replace, native_string_prototype_replace_all,
    native_string_prototype_search, native_string_prototype_starts_with,
};
pub(crate) use sequence::{
    native_string_prototype_concat, native_string_prototype_repeat, native_string_prototype_slice,
    native_string_prototype_split, native_string_prototype_substr,
    native_string_prototype_substring,
};
pub(crate) use trim::{
    native_string_prototype_to_string, native_string_prototype_trim,
    native_string_prototype_trim_end, native_string_prototype_trim_start,
};
pub(crate) use well_formed::{
    native_string_prototype_is_well_formed, native_string_prototype_to_well_formed,
};
