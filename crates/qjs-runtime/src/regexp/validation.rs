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
                has_atom = false;
            }
            '{' => match counted_quantifier_bounds(&pattern, index) {
                Some((min, Some(max), _)) if min > max => {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                Some((_min, _max, next)) if has_atom => {
                    index = next;
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
