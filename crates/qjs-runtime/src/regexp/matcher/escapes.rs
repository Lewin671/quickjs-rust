use crate::regexp::unicode::{self, PropertySet};
use crate::string::string_from_code_unit;

/// A parsed `\p{...}` / `\P{...}` Unicode property escape.
pub(super) struct ParsedPropertyEscape {
    pub(super) set: PropertySet,
    pub(super) negated: bool,
    pub(super) next_pc: usize,
}

/// Parse a property escape at `pc` (pointing at the backslash). Returns `None`
/// when the escape is not a `\p{...}`/`\P{...}` form; assumes the pattern has
/// already passed validation so a well-formed body resolves.
pub(super) fn property_escape(pattern: &[char], pc: usize) -> Option<ParsedPropertyEscape> {
    let negated = match pattern.get(pc + 1)? {
        'p' => false,
        'P' => true,
        _ => return None,
    };
    if pattern.get(pc + 2) != Some(&'{') {
        return None;
    }
    let mut index = pc + 3;
    while pattern.get(index).is_some_and(|value| *value != '}') {
        index += 1;
    }
    if pattern.get(index) != Some(&'}') {
        return None;
    }
    let body: String = pattern[pc + 3..index].iter().collect();
    let set = unicode::resolve_property(&body)?;
    Some(ParsedPropertyEscape {
        set,
        negated,
        next_pc: index + 1,
    })
}

pub(super) struct ParsedEscape {
    pub(super) value: char,
    pub(super) next_pc: usize,
}

pub(super) fn regexp_control_escape(escaped: char) -> char {
    match escaped {
        'f' => '\u{000c}',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        'v' => '\u{000b}',
        _ => escaped,
    }
}

pub(super) fn chars_equal(left: char, right: char, ignore_case: bool) -> bool {
    if ignore_case {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

pub(super) fn class_range_contains(start: char, end: char, value: char, ignore_case: bool) -> bool {
    if start <= value && value <= end {
        return true;
    }
    ignore_case && {
        let value = value.to_ascii_lowercase();
        start.to_ascii_lowercase() <= value && value <= end.to_ascii_lowercase()
    }
}

pub(super) fn regexp_word_char(value: char) -> bool {
    value.is_ascii_alphanumeric() || value == '_'
}

pub(super) fn regexp_whitespace(value: char) -> bool {
    matches!(
        value as u32,
        0x0009 | 0x000a | 0x000b | 0x000c | 0x000d | 0x0020 | 0x00a0 | 0x1680 | 0x2000
            ..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff
    )
}

pub(super) fn unicode_escape(pattern: &[char], pc: usize, unicode: bool) -> Option<ParsedEscape> {
    if pattern.get(pc + 1) != Some(&'u') {
        return None;
    }
    if unicode && pattern.get(pc + 2) == Some(&'{') {
        return braced_unicode_escape(pattern, pc);
    }

    let first = fixed_unicode_code_unit(pattern, pc + 2)?;
    if unicode && (0xD800..=0xDBFF).contains(&first) && pattern.get(pc + 6) == Some(&'\\') {
        if let Some(second) = fixed_unicode_code_unit(pattern, pc + 8) {
            if (0xDC00..=0xDFFF).contains(&second) {
                let code_point =
                    (u32::from(first) - 0xD800) * 1024 + (u32::from(second) - 0xDC00) + 0x10000;
                return char::from_u32(code_point).map(|value| ParsedEscape {
                    value,
                    next_pc: pc + 12,
                });
            }
        }
    }

    Some(ParsedEscape {
        value: char_from_code_unit(first),
        next_pc: pc + 6,
    })
}

fn braced_unicode_escape(pattern: &[char], pc: usize) -> Option<ParsedEscape> {
    let mut value = 0u32;
    let mut index = pc + 3;
    let mut has_digit = false;
    while pattern.get(index).is_some_and(|char| *char != '}') {
        value = value.checked_mul(16)? + pattern.get(index)?.to_digit(16)?;
        has_digit = true;
        index += 1;
    }
    if !has_digit || pattern.get(index) != Some(&'}') {
        return None;
    }
    char::from_u32(value).map(|value| ParsedEscape {
        value,
        next_pc: index + 1,
    })
}

fn fixed_unicode_code_unit(pattern: &[char], start: usize) -> Option<u16> {
    let mut value = 0u32;
    for index in start..start + 4 {
        value = value * 16 + pattern.get(index)?.to_digit(16)?;
    }
    u16::try_from(value).ok()
}

fn char_from_code_unit(code_unit: u16) -> char {
    string_from_code_unit(code_unit)
        .chars()
        .next()
        .unwrap_or(char::REPLACEMENT_CHARACTER)
}
