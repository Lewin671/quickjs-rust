use super::MatchOptions;
use super::escapes::{
    chars_equal, class_range_contains, regexp_control_escape, regexp_word_char, unicode_escape,
};

pub(super) fn class_match(class: &[char], value: char, options: MatchOptions) -> bool {
    let (negated, class) = if class.first() == Some(&'^') {
        (true, &class[1..])
    } else {
        (false, class)
    };
    let matched = class_match_positive(class, value, options);
    if negated { !matched } else { matched }
}

fn class_match_positive(class: &[char], value: char, options: MatchOptions) -> bool {
    let mut index = 0;
    while index < class.len() {
        if class[index] == '\\' {
            if class.get(index + 1).is_some() && class_escape_matches(class, index, value, options)
            {
                return true;
            }
            index += if let Some(escape) = unicode_escape(class, index, options.unicode) {
                escape.next_pc - index
            } else {
                2
            };
        } else if class.get(index + 1) == Some(&'-') && class.get(index + 2).is_some() {
            let end = class[index + 2];
            if class_range_contains(class[index], end, value, options.ignore_case) {
                return true;
            }
            index += 3;
        } else {
            if chars_equal(class[index], value, options.ignore_case) {
                return true;
            }
            index += 1;
        }
    }
    false
}

fn class_escape_matches(class: &[char], index: usize, value: char, options: MatchOptions) -> bool {
    match class.get(index + 1).copied() {
        Some('d') => value.is_ascii_digit(),
        Some('D') => !value.is_ascii_digit(),
        Some('s') => value.is_whitespace(),
        Some('S') => !value.is_whitespace(),
        Some('w') => regexp_word_char(value),
        Some('W') => !regexp_word_char(value),
        Some('u') => unicode_escape(class, index, options.unicode)
            .is_some_and(|escape| chars_equal(value, escape.value, options.ignore_case)),
        Some(escaped) => chars_equal(regexp_control_escape(escaped), value, options.ignore_case),
        None => false,
    }
}
