use std::collections::HashMap;

use crate::regexp::unicode::{self, PropertySet};
use crate::string::{string_from_code_unit, surrogate_escape_code_unit};

pub(super) fn is_trailing_surrogate_position(text: &[char], index: usize) -> bool {
    if index == 0 || index >= text.len() {
        return false;
    }
    matches!(
        (char_code_unit(text[index - 1]), char_code_unit(text[index])),
        (Some(0xD800..=0xDBFF), Some(0xDC00..=0xDFFF))
    )
}

pub(super) fn char_code_unit(value: char) -> Option<u16> {
    if let Some(code_unit) = surrogate_escape_code_unit(value) {
        return Some(code_unit);
    }
    let code_point = value as u32;
    if code_point <= 0xFFFF {
        return Some(code_point as u16);
    }
    let mut buffer = [0u16; 2];
    value.encode_utf16(&mut buffer).first().copied()
}

/// Resolved `\p{...}` / `\P{...}` escapes for one pattern, keyed by the absolute
/// position of the leading backslash in the pattern's `char` slice.
///
/// Property resolution allocates a body `String` and walks the generated alias
/// and range tables, which is far too slow to repeat for every character of a
/// property-escape match (those Test262 cases scan ~1M code points). The cache
/// resolves each escape exactly once when matching begins, so the hot loop only
/// performs an O(1) map lookup plus a binary search over the static range slice.
pub(super) struct PropertyCache {
    escapes: HashMap<usize, ParsedPropertyEscape>,
}

impl PropertyCache {
    /// Resolve every property escape in `pattern` up front.
    ///
    /// Tracks character-class nesting so a `\p`/`\P` is treated the same way the
    /// matcher does inside `[...]` and outside it; all other `\p`/`\P`-shaped
    /// runs that fail to resolve are simply absent from the cache (validation has
    /// already rejected genuinely malformed patterns).
    pub(super) fn build(pattern: &[char]) -> Self {
        let mut escapes = HashMap::new();
        let mut index = 0;
        while index < pattern.len() {
            if pattern[index] == '\\' {
                if matches!(pattern.get(index + 1), Some('p' | 'P'))
                    && let Some(escape) = property_escape(pattern, index)
                {
                    let next = escape.next_pc;
                    escapes.insert(index, escape);
                    index = next;
                    continue;
                }
                // Skip the escaped character so an escaped brace or `p` inside a
                // literal does not desynchronize the scan.
                index += 2;
                continue;
            }
            index += 1;
        }
        PropertyCache { escapes }
    }

    /// Look up the escape whose backslash sits at absolute position `pc`.
    pub(super) fn get(&self, pc: usize) -> Option<ParsedPropertyEscape> {
        self.escapes.get(&pc).copied()
    }
}

/// A parsed `\p{...}` / `\P{...}` Unicode property escape.
///
/// `Copy` so a resolved escape can be cached once per match (keyed by pattern
/// position) and looked up in the hot per-character loop without re-parsing the
/// body or re-resolving the property table.
#[derive(Clone, Copy)]
pub(super) struct ParsedPropertyEscape {
    pub(super) set: PropertySet,
    pub(super) negated: bool,
    pub(super) next_pc: usize,
}

/// Parse and resolve a property escape at `pc` (pointing at the backslash).
///
/// Returns `None` when the escape is not a `\p{...}`/`\P{...}` form; assumes the
/// pattern has already passed validation so a well-formed body resolves.
///
/// This allocates a `String` for the body and resolves the property table, so
/// callers in the per-character matching loop must go through a
/// [`PropertyCache`] rather than calling this directly each character.
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

pub(super) fn control_letter_escape(pattern: &[char], pc: usize) -> Option<ParsedEscape> {
    if pattern.get(pc + 1) != Some(&'c') {
        return None;
    }
    let escaped = *pattern.get(pc + 2)?;
    if !escaped.is_ascii_alphabetic() {
        return None;
    }
    char::from_u32(u32::from(escaped) % 32).map(|value| ParsedEscape {
        value,
        next_pc: pc + 3,
    })
}

/// Parse a `\xHH` hex escape at `pc` (pointing at the backslash).
///
/// Requires exactly two hexadecimal digits after `\x`; otherwise the escape is
/// not a hex escape (in non-unicode mode the matcher then treats `\x` as the
/// literal `x`, matching Annex B `IdentityEscape` semantics).
pub(super) fn hex_escape(pattern: &[char], pc: usize) -> Option<ParsedEscape> {
    if pattern.get(pc + 1) != Some(&'x') {
        return None;
    }
    let high = pattern.get(pc + 2)?.to_digit(16)?;
    let low = pattern.get(pc + 3)?.to_digit(16)?;
    let value = (high * 16 + low) as u16;
    Some(ParsedEscape {
        value: char_from_code_unit(value),
        next_pc: pc + 4,
    })
}

pub(super) fn legacy_octal_escape(pattern: &[char], pc: usize) -> Option<ParsedEscape> {
    let mut next_pc = pc + 1;
    let first = pattern.get(next_pc)?.to_digit(8)?;
    let max_digits = if first <= 3 { 3 } else { 2 };
    let mut value = 0;
    let mut digit_count = 0;
    while digit_count < max_digits {
        let Some(digit) = pattern.get(next_pc).and_then(|value| value.to_digit(8)) else {
            break;
        };
        value = value * 8 + digit;
        digit_count += 1;
        next_pc += 1;
    }

    char::from_u32(value).map(|value| ParsedEscape { value, next_pc })
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
