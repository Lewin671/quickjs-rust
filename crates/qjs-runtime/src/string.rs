mod constructor;
mod indexing;
mod install;
mod iterator;
mod property;
mod prototype;

pub(super) use constructor::{
    is_string_object, native_string, native_string_from_char_code, native_string_from_code_point,
    native_string_raw, string_from_code_point_numbers, string_object_value,
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
    numeric_string_slice_code_unit_len,
};

pub(crate) const STRING_DATA_PROPERTY: &str = "\0StringData";

const SURROGATE_ESCAPE_SENTINEL_BASE: u32 = 0xF0000;

pub(crate) fn string_code_units(value: &str) -> Vec<u16> {
    if value.is_ascii() {
        return value.bytes().map(u16::from).collect();
    }

    let mut code_units = Vec::with_capacity(string_code_unit_len(value));
    for character in value.chars() {
        if let Some(code_unit) = surrogate_escape_code_unit(character) {
            code_units.push(code_unit);
        } else {
            let mut buffer = [0; 2];
            code_units.extend_from_slice(character.encode_utf16(&mut buffer));
        }
    }
    code_units
}

pub(crate) fn string_code_unit_at(value: &str, index: usize) -> Option<u16> {
    if value.is_ascii() {
        return value.as_bytes().get(index).copied().map(u16::from);
    }

    let mut current_index = 0;
    for character in value.chars() {
        if let Some(code_unit) = surrogate_escape_code_unit(character) {
            if current_index == index {
                return Some(code_unit);
            }
            current_index += 1;
            continue;
        }

        let mut buffer = [0; 2];
        let encoded = character.encode_utf16(&mut buffer);
        if index < current_index + encoded.len() {
            return encoded.get(index - current_index).copied();
        }
        current_index += encoded.len();
    }
    None
}

pub(crate) fn string_code_unit_len(value: &str) -> usize {
    if value.is_ascii() {
        return value.len();
    }
    value
        .chars()
        .map(|character| {
            if surrogate_escape_code_unit(character).is_some() {
                1
            } else {
                character.len_utf16()
            }
        })
        .sum()
}

pub(crate) fn string_utf16_eq(left: &str, right: &str) -> bool {
    string_code_units(left) == string_code_units(right)
}

pub(crate) fn string_from_code_units(code_units: &[u16]) -> String {
    let mut result = String::with_capacity(code_units.len());
    for code_unit in code_units {
        push_code_unit(&mut result, *code_unit);
    }
    result
}

pub(crate) fn string_from_utf8_scalars(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for character in value.chars() {
        push_code_point(&mut result, character as u32);
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
    let mut result = String::new();
    push_code_unit(&mut result, code_unit);
    result
}

pub(crate) fn char_from_code_unit(code_unit: u16) -> char {
    if (0xD800..=0xDFFF).contains(&code_unit) {
        char::from_u32(SURROGATE_ESCAPE_SENTINEL_BASE + u32::from(code_unit) - 0xD800)
            .unwrap_or(char::REPLACEMENT_CHARACTER)
    } else {
        char::from_u32(u32::from(code_unit)).unwrap_or(char::REPLACEMENT_CHARACTER)
    }
}

pub(crate) fn push_code_unit(result: &mut String, code_unit: u16) {
    result.push(char_from_code_unit(code_unit));
}

/// Appends one validated Unicode code point using the runtime's WTF-16 string
/// representation. Real scalar values that overlap the lone-surrogate
/// sentinel range are expanded into their UTF-16 pair so they cannot be
/// mistaken for one surrogate code unit.
pub(crate) fn push_code_point(result: &mut String, code_point: u32) {
    if (0xD800..=0xDFFF).contains(&code_point) {
        push_code_unit(result, code_point as u16);
        return;
    }

    let character = char::from_u32(code_point).expect("caller validates Unicode code points");
    if surrogate_escape_code_unit(character).is_some() {
        let mut buffer = [0; 2];
        for code_unit in character.encode_utf16(&mut buffer) {
            push_code_unit(result, *code_unit);
        }
    } else {
        result.push(character);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        char_from_code_unit, push_code_point, string_code_unit_at, string_code_unit_len,
        string_code_units, string_from_code_units, surrogate_escape_code_unit,
    };

    #[test]
    fn code_unit_character_conversion_preserves_ascii_and_lone_surrogates() {
        assert_eq!(char_from_code_unit(0x41), 'A');
        for code_unit in [0xD800, 0xDC00, 0xDFFF] {
            assert_eq!(
                surrogate_escape_code_unit(char_from_code_unit(code_unit)),
                Some(code_unit)
            );
        }
    }

    #[test]
    fn code_unit_length_matches_materialized_utf16() {
        let escaped_surrogates = string_from_code_units(&[0xD800, 0xDC00, 0xDFFF]);
        for value in ["ascii", "A😀文", escaped_surrogates.as_str()] {
            assert_eq!(string_code_unit_len(value), string_code_units(value).len());
        }
    }

    #[test]
    fn code_unit_at_matches_materialized_utf16() {
        let escaped_surrogates = string_from_code_units(&[0xD800, 0xDC00, 0xDFFF]);
        for (value, expected) in [
            ("ascii", vec![0x61, 0x73, 0x63, 0x69, 0x69]),
            ("A😀文", vec![0x41, 0xD83D, 0xDE00, 0x6587]),
            (escaped_surrogates.as_str(), vec![0xD800, 0xDC00, 0xDFFF]),
        ] {
            assert_eq!(string_code_units(value), expected);
            for (index, code_unit) in expected.iter().enumerate() {
                assert_eq!(string_code_unit_at(value, index), Some(*code_unit));
            }
            assert_eq!(string_code_unit_at(value, expected.len()), None);
        }
    }

    #[test]
    fn code_points_overlapping_the_surrogate_sentinel_expand_to_utf16() {
        for code_point in 0xF0000..0xF0800 {
            let mut value = String::new();
            push_code_point(&mut value, code_point);

            let mut buffer = [0; 2];
            let expected = char::from_u32(code_point)
                .expect("sentinel-range code point is valid")
                .encode_utf16(&mut buffer)
                .to_vec();
            assert_eq!(string_code_units(&value), expected, "U+{code_point:05X}");
            assert_eq!(string_code_unit_len(&value), 2, "U+{code_point:05X}");
        }
    }
}
