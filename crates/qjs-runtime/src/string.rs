mod constructor;
mod indexing;
mod install;
mod property;
mod prototype;

pub(super) use constructor::{
    is_string_object, native_string, native_string_from_char_code, native_string_from_code_point,
    native_string_raw, string_object_value,
};
pub(crate) use install::install_string;
pub(super) use property::{
    string_has_own_property, string_own_property_descriptor, string_own_property_keys,
    string_own_property_names, string_property,
};
pub(super) use prototype::{
    StringHtmlKind, StringPadKind, native_string_prototype_at, native_string_prototype_char_at,
    native_string_prototype_char_code_at, native_string_prototype_code_point_at,
    native_string_prototype_concat, native_string_prototype_ends_with,
    native_string_prototype_html, native_string_prototype_includes,
    native_string_prototype_index_of, native_string_prototype_is_well_formed,
    native_string_prototype_last_index_of, native_string_prototype_locale_compare,
    native_string_prototype_match, native_string_prototype_pad, native_string_prototype_repeat,
    native_string_prototype_replace, native_string_prototype_replace_all,
    native_string_prototype_search, native_string_prototype_slice, native_string_prototype_split,
    native_string_prototype_starts_with, native_string_prototype_substr,
    native_string_prototype_substring, native_string_prototype_to_lower_case,
    native_string_prototype_to_string, native_string_prototype_to_upper_case,
    native_string_prototype_to_well_formed, native_string_prototype_trim,
    native_string_prototype_trim_end, native_string_prototype_trim_start,
};

pub(crate) const STRING_DATA_PROPERTY: &str = "\0StringData";

const SURROGATE_ESCAPE_SENTINEL_BASE: u32 = 0xF0000;

pub(crate) fn string_code_units(value: &str) -> Vec<u16> {
    value
        .chars()
        .flat_map(|character| {
            let code = character as u32;
            if (SURROGATE_ESCAPE_SENTINEL_BASE..SURROGATE_ESCAPE_SENTINEL_BASE + 0x800)
                .contains(&code)
            {
                vec![(0xD800 + code - SURROGATE_ESCAPE_SENTINEL_BASE) as u16]
            } else {
                let mut buffer = [0; 2];
                character.encode_utf16(&mut buffer).to_vec()
            }
        })
        .collect()
}

pub(crate) fn string_from_code_unit(code_unit: u16) -> String {
    if (0xD800..=0xDFFF).contains(&code_unit) {
        char::from_u32(SURROGATE_ESCAPE_SENTINEL_BASE + u32::from(code_unit) - 0xD800)
            .unwrap_or(char::REPLACEMENT_CHARACTER)
            .to_string()
    } else {
        String::from_utf16(&[code_unit]).unwrap_or_else(|_| char::REPLACEMENT_CHARACTER.to_string())
    }
}
