mod constructor;
mod indexing;
mod install;
mod property;
mod prototype;

pub(super) use constructor::{native_string, native_string_from_char_code};
pub(crate) use install::install_string;
pub(super) use property::{
    string_has_own_property, string_own_property_descriptor, string_own_property_keys,
    string_own_property_names, string_property,
};
pub(super) use prototype::{
    StringPadKind, native_string_prototype_at, native_string_prototype_char_at,
    native_string_prototype_char_code_at, native_string_prototype_code_point_at,
    native_string_prototype_concat, native_string_prototype_ends_with,
    native_string_prototype_includes, native_string_prototype_index_of,
    native_string_prototype_last_index_of, native_string_prototype_pad,
    native_string_prototype_repeat, native_string_prototype_slice, native_string_prototype_split,
    native_string_prototype_starts_with, native_string_prototype_substring,
    native_string_prototype_to_lower_case, native_string_prototype_to_string,
    native_string_prototype_to_upper_case, native_string_prototype_trim,
    native_string_prototype_trim_end, native_string_prototype_trim_start,
};
