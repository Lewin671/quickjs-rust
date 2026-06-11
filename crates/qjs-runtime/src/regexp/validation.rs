use super::unicode;
use crate::RuntimeError;

pub(super) fn validate_regexp_init(source: &str, flags: &str) -> Result<(), RuntimeError> {
    validate_regexp_flags(flags)?;
    validate_regexp_pattern(source, flags.contains('u'))
}

fn validate_regexp_flags(flags: &str) -> Result<(), RuntimeError> {
    let mut seen = Vec::with_capacity(flags.len());
    for flag in flags.chars() {
        if !"dgimsyu".contains(flag) || seen.contains(&flag) {
            return Err(regexp_syntax_error("invalid regular expression flags"));
        }
        seen.push(flag);
    }
    Ok(())
}

fn validate_regexp_pattern(source: &str, unicode: bool) -> Result<(), RuntimeError> {
    let pattern: Vec<_> = source.chars().collect();
    let capture_count = regexp_capture_count(&pattern);
    let mut index = 0;
    let mut has_atom = false;
    while index < pattern.len() {
        match pattern[index] {
            '\\' => {
                if index + 1 >= pattern.len() {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                if pattern[index + 1] == 'u'
                    && pattern.get(index + 2) == Some(&'{')
                    && let Some(end) = braced_escape_end(&pattern, index + 2)
                {
                    index = end + 1;
                    has_atom = true;
                    continue;
                }
                if unicode && matches!(pattern[index + 1], 'p' | 'P') {
                    let end = validate_property_escape(&pattern, index)?;
                    index = end;
                    has_atom = true;
                    continue;
                }
                if unicode
                    && pattern[index + 1].is_ascii_digit()
                    && pattern[index + 1]
                        .to_digit(10)
                        .is_some_and(|value| value as usize > capture_count)
                {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                index += 2;
                has_atom = true;
            }
            '[' => {
                let Some(end) = class_end(&pattern, index) else {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                };
                validate_class_ranges(&pattern, index + 1, end, unicode)?;
                index = end + 1;
                has_atom = true;
            }
            ']' => return Err(regexp_syntax_error("invalid regular expression pattern")),
            '(' => {
                let Some(end) = group_end(&pattern, index) else {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                };
                index = end + 1;
                has_atom = true;
            }
            ')' => return Err(regexp_syntax_error("invalid regular expression pattern")),
            '?' | '*' | '+' if !has_atom => {
                return Err(regexp_syntax_error("invalid regular expression pattern"));
            }
            '?' | '*' | '+' => {
                index += 1;
                if pattern.get(index) == Some(&'?') {
                    index += 1;
                }
                has_atom = false;
            }
            '{' => match counted_quantifier_bounds(&pattern, index) {
                Some((min, Some(max), _)) if min > max => {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                Some((_min, _max, next)) if has_atom => {
                    index = next;
                    if pattern.get(index) == Some(&'?') {
                        index += 1;
                    }
                    has_atom = false;
                }
                Some(_) | None if unicode => {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                _ => {
                    index += 1;
                    has_atom = true;
                }
            },
            _ => {
                index += 1;
                has_atom = true;
            }
        }
    }
    Ok(())
}

fn regexp_capture_count(pattern: &[char]) -> usize {
    let mut count = 0;
    let mut index = 0;
    while index < pattern.len() {
        match pattern[index] {
            '\\' => index += 2,
            '[' => {
                index = class_end(pattern, index).map_or(pattern.len(), |end| end + 1);
            }
            '(' if !matches!(pattern.get(index + 1..index + 3), Some(['?', ':'])) => {
                count += 1;
                index += 1;
            }
            _ => index += 1,
        }
    }
    count
}

fn class_end(pattern: &[char], start: usize) -> Option<usize> {
    let mut index = start + 1;
    while index < pattern.len() {
        match pattern[index] {
            '\\' => index += 2,
            ']' => return Some(index),
            _ => index += 1,
        }
    }
    None
}

fn validate_class_ranges(
    pattern: &[char],
    start: usize,
    end: usize,
    unicode: bool,
) -> Result<(), RuntimeError> {
    let mut index = start;
    while index < end {
        if pattern[index] == '\\' {
            if unicode && matches!(pattern.get(index + 1), Some('p' | 'P')) {
                index = validate_property_escape(pattern, index)?;
                continue;
            }
            index = class_escape_end(pattern, index, unicode);
            continue;
        }
        if index + 2 < end && pattern[index + 1] == '-' {
            if pattern[index] > pattern[index + 2] {
                return Err(regexp_syntax_error("invalid regular expression pattern"));
            }
            index += 3;
            continue;
        }
        index += 1;
    }
    Ok(())
}

/// Validate a `\p{...}` / `\P{...}` Unicode property escape (unicode mode).
/// `start` points at the backslash. Returns the index just past the closing
/// brace, or a SyntaxError when the body is not a valid property expression.
fn validate_property_escape(pattern: &[char], start: usize) -> Result<usize, RuntimeError> {
    if pattern.get(start + 2) != Some(&'{') {
        return Err(regexp_syntax_error("invalid regular expression pattern"));
    }
    let Some(close) = braced_escape_end(pattern, start + 2) else {
        return Err(regexp_syntax_error("invalid regular expression pattern"));
    };
    let body: String = pattern[start + 3..close].iter().collect();
    if unicode::resolve_property(&body).is_none() {
        return Err(regexp_syntax_error("invalid regular expression pattern"));
    }
    Ok(close + 1)
}

fn class_escape_end(pattern: &[char], start: usize, unicode: bool) -> usize {
    if pattern.get(start + 1) == Some(&'u') {
        if unicode
            && pattern.get(start + 2) == Some(&'{')
            && let Some(end) = braced_escape_end(pattern, start + 2)
        {
            return end + 1;
        }
        return (start + 6).min(pattern.len());
    }
    if !unicode && let Some(first) = pattern.get(start + 1).and_then(|value| value.to_digit(8)) {
        let max_digits = if first <= 3 { 3 } else { 2 };
        let mut index = start + 1;
        let mut digit_count = 0;
        while digit_count < max_digits && pattern.get(index).is_some_and(|value| value.is_digit(8))
        {
            index += 1;
            digit_count += 1;
        }
        return index;
    }
    (start + 2).min(pattern.len())
}

fn braced_escape_end(pattern: &[char], start: usize) -> Option<usize> {
    let mut index = start + 1;
    while index < pattern.len() {
        if pattern[index] == '}' {
            return Some(index);
        }
        index += 1;
    }
    None
}

fn group_end(pattern: &[char], start: usize) -> Option<usize> {
    let mut depth = 1;
    let mut index = start + 1;
    while index < pattern.len() {
        match pattern[index] {
            '\\' => index += 2,
            '[' => index = class_end(pattern, index)? + 1,
            '(' => {
                depth += 1;
                index += 1;
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
                index += 1;
            }
            _ => index += 1,
        }
    }
    None
}

fn counted_quantifier_bounds(
    pattern: &[char],
    start: usize,
) -> Option<(usize, Option<usize>, usize)> {
    let mut index = start + 1;
    let mut min = 0;
    let mut has_min = false;
    while pattern.get(index).is_some_and(|char| char.is_ascii_digit()) {
        min = min * 10 + pattern[index].to_digit(10)? as usize;
        has_min = true;
        index += 1;
    }
    if !has_min {
        return None;
    }
    if pattern.get(index) == Some(&'}') {
        return Some((min, Some(min), index + 1));
    }
    if pattern.get(index) != Some(&',') {
        return None;
    }
    index += 1;
    let mut max = 0;
    let mut has_max = false;
    while pattern.get(index).is_some_and(|char| char.is_ascii_digit()) {
        max = max * 10 + pattern[index].to_digit(10)? as usize;
        has_max = true;
        index += 1;
    }
    if pattern.get(index) != Some(&'}') {
        return None;
    }
    Some((min, has_max.then_some(max), index + 1))
}

fn regexp_syntax_error(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("SyntaxError: {message}"),
    }
}
