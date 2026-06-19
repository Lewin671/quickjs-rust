use super::MatchOptions;
use super::escapes::{
    PropertyCache, chars_equal, class_range_contains, control_letter_escape, hex_escape,
    regexp_control_escape, regexp_whitespace, regexp_word_char, unicode_escape,
};
use crate::string::surrogate_escape_code_unit;

/// Match a single code point against a character class.
///
/// `base` is the absolute position of `class[0]` within the full pattern, so
/// property escapes can be resolved through the per-match [`PropertyCache`]
/// (which is keyed by absolute pattern position) instead of being re-parsed.
pub(super) fn class_match(
    class: &[char],
    base: usize,
    value: char,
    properties: &PropertyCache,
    options: MatchOptions,
) -> bool {
    let (negated, class, base) = if class.first() == Some(&'^') {
        (true, &class[1..], base + 1)
    } else {
        (false, class, base)
    };
    let matched = class_match_positive(class, base, value, properties, options);
    if negated { !matched } else { matched }
}

fn class_match_positive(
    class: &[char],
    base: usize,
    value: char,
    properties: &PropertyCache,
    options: MatchOptions,
) -> bool {
    let mut index = 0;
    while index < class.len() {
        if options.unicode
            && class[index] == '\\'
            && matches!(class.get(index + 1), Some('p' | 'P'))
            && let Some(escape) = properties.get(base + index)
        {
            if escape.set.contains(u32::from(value)) != escape.negated {
                return true;
            }
            // `next_pc` is an absolute pattern position; convert back to an
            // index within this class slice.
            index = escape.next_pc - base;
            continue;
        }
        if let Some(start) = class_atom(class, index, options) {
            if class.get(start.next_index) == Some(&'-')
                && let Some(end) = class_atom(class, start.next_index + 1, options)
            {
                if class_range_contains(start.value, end.value, value, options.ignore_case) {
                    return true;
                }
                index = end.next_index;
                continue;
            }

            if chars_equal(start.value, value, options.ignore_case, options.unicode) {
                return true;
            }
            index = start.next_index;
            continue;
        }

        if class[index] == '\\'
            && class.get(index + 1).is_some()
            && class_escape_matches(class, index, value, options)
        {
            return true;
        }
        index += 2;
    }
    false
}

struct ClassAtom {
    value: char,
    next_index: usize,
}

fn class_atom(class: &[char], index: usize, options: MatchOptions) -> Option<ClassAtom> {
    let current = *class.get(index)?;
    if options.unicode
        && let Some((value, next_index)) = class_code_point_at(class, index)
    {
        return Some(ClassAtom { value, next_index });
    }

    if current != '\\' {
        return Some(ClassAtom {
            value: current,
            next_index: index + 1,
        });
    }

    if let Some(escape) = unicode_escape(class, index, options.unicode) {
        return Some(ClassAtom {
            value: escape.value,
            next_index: escape.next_pc,
        });
    }
    if let Some(escape) = control_letter_escape(class, index) {
        return Some(ClassAtom {
            value: escape.value,
            next_index: escape.next_pc,
        });
    }
    if let Some(escape) = hex_escape(class, index) {
        return Some(ClassAtom {
            value: escape.value,
            next_index: escape.next_pc,
        });
    }
    if !options.unicode
        && let Some(escape) = legacy_octal_escape(class, index)
    {
        return Some(escape);
    }
    if options.unicode
        && class.get(index + 1) == Some(&'0')
        && !class.get(index + 2).is_some_and(char::is_ascii_digit)
    {
        return Some(ClassAtom {
            value: '\u{0000}',
            next_index: index + 2,
        });
    }
    if !options.unicode
        && let Some(escape) = legacy_control_letter_escape(class, index)
    {
        return Some(escape);
    }
    if !options.unicode && class.get(index + 1) == Some(&'c') {
        return Some(ClassAtom {
            value: '\\',
            next_index: index + 1,
        });
    }
    match class.get(index + 1).copied()? {
        'd' | 'D' | 's' | 'S' | 'w' | 'W' => None,
        // Inside a character class `\b` is the backspace U+0008 (ClassEscape),
        // not the word-boundary assertion it denotes outside a class.
        'b' => Some(ClassAtom {
            value: '\u{0008}',
            next_index: index + 2,
        }),
        escaped => Some(ClassAtom {
            value: regexp_control_escape(escaped),
            next_index: index + 2,
        }),
    }
}

fn legacy_octal_escape(class: &[char], index: usize) -> Option<ClassAtom> {
    let mut next_index = index + 1;
    let first = class.get(next_index)?.to_digit(8)?;
    let max_digits = if first <= 3 { 3 } else { 2 };
    let mut value = 0;
    let mut digit_count = 0;
    while digit_count < max_digits {
        let Some(digit) = class.get(next_index).and_then(|value| value.to_digit(8)) else {
            break;
        };
        value = value * 8 + digit;
        digit_count += 1;
        next_index += 1;
    }

    char::from_u32(value).map(|value| ClassAtom { value, next_index })
}

fn legacy_control_letter_escape(class: &[char], index: usize) -> Option<ClassAtom> {
    if class.get(index + 1) != Some(&'c') {
        return None;
    }
    let escaped = *class.get(index + 2)?;
    if !(escaped.is_ascii_digit() || escaped == '_') {
        return None;
    }
    char::from_u32(u32::from(escaped) % 32).map(|value| ClassAtom {
        value,
        next_index: index + 3,
    })
}

fn class_escape_matches(class: &[char], index: usize, value: char, options: MatchOptions) -> bool {
    match class.get(index + 1).copied() {
        Some('d') => value.is_ascii_digit(),
        Some('D') => !value.is_ascii_digit(),
        Some('s') => regexp_whitespace(value),
        Some('S') => !regexp_whitespace(value),
        Some('w') => regexp_word_char(value),
        Some('W') => !regexp_word_char(value),
        Some('u') => unicode_escape(class, index, options.unicode).is_some_and(|escape| {
            chars_equal(value, escape.value, options.ignore_case, options.unicode)
        }),
        Some('x') if hex_escape(class, index).is_some() => {
            hex_escape(class, index).is_some_and(|escape| {
                chars_equal(value, escape.value, options.ignore_case, options.unicode)
            })
        }
        Some(escaped) => chars_equal(
            regexp_control_escape(escaped),
            value,
            options.ignore_case,
            options.unicode,
        ),
        None => false,
    }
}

fn class_code_point_at(class: &[char], index: usize) -> Option<(char, usize)> {
    let high = class
        .get(index)
        .and_then(|value| surrogate_escape_code_unit(*value))?;
    if !(0xD800..=0xDBFF).contains(&high) {
        return None;
    }
    let low = class
        .get(index + 1)
        .and_then(|value| surrogate_escape_code_unit(*value))?;
    if !(0xDC00..=0xDFFF).contains(&low) {
        return None;
    }
    let code_point = 0x10000 + ((u32::from(high) - 0xD800) << 10) + u32::from(low) - 0xDC00;
    char::from_u32(code_point).map(|value| (value, index + 2))
}
