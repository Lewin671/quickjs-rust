mod case;
mod code_units;
mod padding;
mod search;
mod sequence;
mod trim;

pub(crate) use case::{
    native_string_prototype_to_lower_case, native_string_prototype_to_upper_case,
};
pub(crate) use code_units::{
    native_string_prototype_at, native_string_prototype_char_at,
    native_string_prototype_char_code_at, native_string_prototype_code_point_at,
};
pub(crate) use padding::{StringPadKind, native_string_prototype_pad};
pub(crate) use search::{
    native_string_prototype_ends_with, native_string_prototype_includes,
    native_string_prototype_index_of, native_string_prototype_last_index_of,
    native_string_prototype_starts_with,
};
pub(crate) use sequence::{
    native_string_prototype_concat, native_string_prototype_repeat, native_string_prototype_slice,
    native_string_prototype_split, native_string_prototype_substring,
};
pub(crate) use trim::{
    native_string_prototype_to_string, native_string_prototype_trim,
    native_string_prototype_trim_end, native_string_prototype_trim_start,
};
