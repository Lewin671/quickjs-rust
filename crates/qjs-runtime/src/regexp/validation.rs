use super::unicode;
use crate::RuntimeError;

pub(crate) fn validate_regexp_init(source: &str, flags: &str) -> Result<(), RuntimeError> {
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
    validate_named_group_references(&pattern, unicode)?;
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
                // Lookbehind assertions are not `QuantifiableAssertion`s, so a
                // quantifier immediately after `(?<=...)` / `(?<!...)` is a
                // SyntaxError in both Annex-B and non-Annex-B modes.
                if is_lookbehind_group(&pattern, index)
                    && pattern
                        .get(end + 1)
                        .is_some_and(|next| matches!(next, '*' | '+' | '?' | '{'))
                {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
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
                // A well-formed `{n}`/`{n,}`/`{n,m}` with nothing to quantify is
                // an `InvalidBracedQuantifier`, a SyntaxError in both Annex-B and
                // non-Annex-B modes (the production has higher precedence than
                // treating `{` as a literal `ExtendedPatternCharacter`).
                Some(_) => {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                None if unicode => {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                None => {
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
            '(' if is_capturing_group(pattern, index) => {
                count += 1;
                index += 1;
            }
            _ => index += 1,
        }
    }
    count
}

/// A `(` opens a capturing group unless it is `(?:`, `(?=`, `(?!`, `(?<=`, or
/// `(?<!`. A `(?<name>` named group is capturing.
fn is_capturing_group(pattern: &[char], index: usize) -> bool {
    if pattern.get(index + 1) != Some(&'?') {
        return true;
    }
    match pattern.get(index + 2) {
        Some(':') | Some('=') | Some('!') => false,
        // `(?<=` / `(?<!` are lookbehind (non-capturing); `(?<name>` captures.
        Some('<') => !matches!(pattern.get(index + 3), Some('=') | Some('!')),
        _ => true,
    }
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

/// Does the `(` at `index` open a lookbehind assertion (`(?<=` / `(?<!`)?
fn is_lookbehind_group(pattern: &[char], index: usize) -> bool {
    pattern.get(index + 1) == Some(&'?')
        && pattern.get(index + 2) == Some(&'<')
        && matches!(pattern.get(index + 3), Some('=') | Some('!'))
}

/// Validate `\k<name>` named backreferences against the named groups declared
/// in the pattern.
///
/// A `\k<name>` is a `GroupName` (and so must resolve) whenever the pattern
/// contains any named group, or when the `u` flag is set. In Annex-B mode with
/// no named groups at all, a bare `\k` is an `IdentityEscape` and is left to
/// the matcher, matching upstream behavior.
fn validate_named_group_references(pattern: &[char], unicode: bool) -> Result<(), RuntimeError> {
    let names = collect_named_group_names(pattern);
    let treat_k_as_reference = unicode || !names.is_empty();
    if !treat_k_as_reference {
        return Ok(());
    }

    let mut index = 0;
    let mut in_class = false;
    while index < pattern.len() {
        match pattern[index] {
            '\\' if !in_class && pattern.get(index + 1) == Some(&'k') => {
                let Some((name, next)) = parse_group_name(pattern, index + 2) else {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                };
                if !names.contains(&name) {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                index = next;
            }
            '\\' => index += 2,
            '[' => {
                in_class = true;
                index += 1;
            }
            ']' => {
                in_class = false;
                index += 1;
            }
            _ => index += 1,
        }
    }
    Ok(())
}

/// Collect the names of every `(?<name>` group, rejecting duplicate names.
fn collect_named_group_names(pattern: &[char]) -> Vec<String> {
    let mut names = Vec::new();
    let mut index = 0;
    let mut in_class = false;
    while index < pattern.len() {
        match pattern[index] {
            '\\' => index += 2,
            '[' => {
                in_class = true;
                index += 1;
            }
            ']' => {
                in_class = false;
                index += 1;
            }
            '(' if !in_class
                && pattern.get(index + 1) == Some(&'?')
                && pattern.get(index + 2) == Some(&'<')
                && !matches!(pattern.get(index + 3), Some('=') | Some('!')) =>
            {
                if let Some((name, next)) = parse_group_name(pattern, index + 2) {
                    names.push(name);
                    index = next;
                } else {
                    index += 1;
                }
            }
            _ => index += 1,
        }
    }
    names
}

/// Parse a `<name>` starting at `start` (pointing at `<`). Returns the name and
/// the index just past `>`.
fn parse_group_name(pattern: &[char], start: usize) -> Option<(String, usize)> {
    if pattern.get(start) != Some(&'<') {
        return None;
    }
    let mut index = start + 1;
    let mut name = String::new();
    while let Some(&value) = pattern.get(index) {
        if value == '>' {
            if name.is_empty() {
                return None;
            }
            return Some((name, index + 1));
        }
        name.push(value);
        index += 1;
    }
    None
}

fn regexp_syntax_error(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("SyntaxError: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::validate_regexp_init;

    fn accepts(source: &str, flags: &str) {
        assert!(
            validate_regexp_init(source, flags).is_ok(),
            "expected /{source}/{flags} to be valid"
        );
    }

    fn rejects(source: &str, flags: &str) {
        assert!(
            validate_regexp_init(source, flags).is_err(),
            "expected /{source}/{flags} to be a SyntaxError"
        );
    }

    #[test]
    fn rejects_braced_quantifier_without_atom() {
        rejects("{2}", "");
        rejects("{2,}", "");
        rejects("{2,4}", "");
        rejects("{2}", "u");
        // A malformed brace stays a literal in Annex-B mode.
        accepts("{", "");
        accepts("a{", "");
        accepts("x{2}", "");
    }

    #[test]
    fn rejects_quantified_lookbehind() {
        rejects(".(?<=.)?", "");
        rejects(".(?<=.)*", "");
        rejects(".(?<!.)+", "");
        rejects(".(?<=.){2}", "u");
        // Lookahead remains a QuantifiableAssertion in Annex-B mode.
        accepts(".(?=.)?", "");
        accepts("(?<=a)b", "");
    }

    #[test]
    fn rejects_dangling_named_backreference() {
        rejects("(?<a>.)\\k<b>", "");
        rejects("(?<a>a)\\k<ab>", "");
        rejects("\\k<a>(?<b>x)", "");
        rejects("\\k<a>", "u");
        accepts("(?<a>.)\\k<a>", "");
        // With no named group and no `u` flag, `\k` is an identity escape.
        accepts("\\k<a>", "");
    }
}
