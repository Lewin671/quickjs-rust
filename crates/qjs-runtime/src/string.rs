mod constructor;
mod indexing;
mod install;
mod iterator;
mod property;
mod prototype;

pub(super) use constructor::{
    is_string_object, native_string, native_string_from_char_code, native_string_from_code_point,
    native_string_raw, string_object_value,
};
pub(crate) use install::{install_string, install_string_well_known_symbols};
pub(crate) use iterator::{native_string_iterator_next, native_string_prototype_iterator};
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
    native_string_prototype_match, native_string_prototype_match_all,
    native_string_prototype_normalize, native_string_prototype_pad, native_string_prototype_repeat,
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
            if let Some(code_unit) = surrogate_escape_code_unit(character) {
                vec![code_unit]
            } else {
                let mut buffer = [0; 2];
                character.encode_utf16(&mut buffer).to_vec()
            }
        })
        .collect()
}

pub(crate) fn string_utf16_eq(left: &str, right: &str) -> bool {
    string_code_units(left) == string_code_units(right)
}

pub(crate) fn string_from_code_units(code_units: &[u16]) -> String {
    let mut result = String::new();
    let mut index = 0;
    while index < code_units.len() {
        let first = code_units[index];
        if (0xD800..=0xDBFF).contains(&first) && index + 1 < code_units.len() {
            let second = code_units[index + 1];
            if (0xDC00..=0xDFFF).contains(&second) {
                let code_point =
                    0x10000 + ((u32::from(first) - 0xD800) << 10) + u32::from(second) - 0xDC00;
                if let Some(character) = char::from_u32(code_point) {
                    result.push(character);
                    index += 2;
                    continue;
                }
            }
        }
        result.push_str(&string_from_code_unit(first));
        index += 1;
    }
    result
}

pub(crate) fn surrogate_escape_code_unit(character: char) -> Option<u16> {
    let code = character as u32;
    if (SURROGATE_ESCAPE_SENTINEL_BASE..SURROGATE_ESCAPE_SENTINEL_BASE + 0x800).contains(&code) {
        Some((0xD800 + code - SURROGATE_ESCAPE_SENTINEL_BASE) as u16)
    } else {
        None
    }
}

pub(crate) fn advance_string_index(characters: &[char], index: usize, unicode: bool) -> usize {
    if unicode
        && matches!(
            (
                characters
                    .get(index)
                    .and_then(|ch| surrogate_escape_code_unit(*ch)),
                characters
                    .get(index + 1)
                    .and_then(|ch| surrogate_escape_code_unit(*ch))
            ),
            (Some(0xD800..=0xDBFF), Some(0xDC00..=0xDFFF))
        )
    {
        index + 2
    } else {
        index + 1
    }
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
