use crate::RuntimeError;
use qjs_unicode as unicode;

pub(crate) fn validate_regexp_init(source: &str, flags: &str) -> Result<(), RuntimeError> {
    validate_regexp_flags(flags)?;
    if flags.is_empty() && is_fast_non_unicode_bmp_literal_source(source) {
        return Ok(());
    }
    let unicode_sets = flags.contains('v');
    validate_regexp_pattern(source, flags.contains('u') || unicode_sets, unicode_sets)
}

fn is_fast_non_unicode_bmp_literal_source(source: &str) -> bool {
    let mut chars = source.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    match (
        first,
        chars.next(),
        chars.next(),
        chars.next(),
        chars.next(),
        chars.next(),
    ) {
        (ch, None, None, None, None, None) => is_fast_single_regexp_atom(ch),
        ('\\', Some(ch), None, None, None, None) => is_fast_escaped_regexp_atom(ch),
        ('a', Some('\\'), Some(ch), None, None, None) => is_fast_escaped_regexp_atom(ch),
        ('n', Some('n'), Some('n'), Some('n'), Some(ch), None) => is_fast_single_regexp_atom(ch),
        _ => false,
    }
}

fn is_fast_single_regexp_atom(ch: char) -> bool {
    !matches!(
        ch,
        '\n' | '\r'
            | '\u{2028}'
            | '\u{2029}'
            | '*'
            | '/'
            | '\\'
            | '+'
            | '?'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
    )
}

fn is_fast_escaped_regexp_atom(ch: char) -> bool {
    !matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn validate_regexp_flags(flags: &str) -> Result<(), RuntimeError> {
    let mut seen = Vec::with_capacity(flags.len());
    for flag in flags.chars() {
        if !"dgimsuvy".contains(flag) || seen.contains(&flag) {
            return Err(regexp_syntax_error("invalid regular expression flags"));
        }
        seen.push(flag);
    }
    if seen.contains(&'u') && seen.contains(&'v') {
        return Err(regexp_syntax_error("invalid regular expression flags"));
    }
    Ok(())
}

fn validate_regexp_pattern(
    source: &str,
    unicode: bool,
    unicode_sets: bool,
) -> Result<(), RuntimeError> {
    let pattern: Vec<_> = source.chars().collect();
    let capture_count = regexp_capture_count(&pattern);
    validate_named_group_definitions(&pattern)?;
    validate_named_group_references(&pattern, unicode)?;
    validate_pattern_range(
        &pattern,
        0,
        pattern.len(),
        unicode,
        unicode_sets,
        capture_count,
    )
}

fn validate_pattern_range(
    pattern: &[char],
    start: usize,
    end: usize,
    unicode: bool,
    unicode_sets: bool,
    capture_count: usize,
) -> Result<(), RuntimeError> {
    let mut index = start;
    let mut has_atom = false;
    while index < end {
        match pattern[index] {
            '\\' => {
                if index + 1 >= end {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                if pattern[index + 1] == 'u' && pattern.get(index + 2) == Some(&'{') {
                    if unicode {
                        // In unicode mode `\u{ CodePoint }` must hold 1+ hex
                        // digits naming a value <= 0x10FFFF.
                        index = validate_braced_unicode_escape(pattern, index + 2)?;
                        has_atom = true;
                        continue;
                    }
                    if let Some(end) = braced_escape_end(pattern, index + 2) {
                        index = end + 1;
                        has_atom = true;
                        continue;
                    }
                }
                if unicode && matches!(pattern[index + 1], 'p' | 'P') {
                    let end = validate_property_escape(pattern, index)?;
                    index = end;
                    has_atom = true;
                    continue;
                }
                if unicode
                    && let Some(next) =
                        validate_unicode_decimal_escape(pattern, index, capture_count, true)?
                {
                    index = next;
                    has_atom = true;
                    continue;
                }
                if unicode {
                    // In unicode mode only specific escapes are legal; an
                    // arbitrary IdentityEscape (`\M`, `\a`, `\c0`) is rejected.
                    index = validate_unicode_escape(pattern, index, false)?;
                    has_atom = true;
                    continue;
                }
                index += 2;
                has_atom = true;
            }
            '[' => {
                let Some(end) = class_end(pattern, index) else {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                };
                validate_class_ranges(pattern, index + 1, end, unicode, unicode_sets)?;
                index = end + 1;
                has_atom = true;
            }
            ']' if unicode => {
                return Err(regexp_syntax_error("invalid regular expression pattern"));
            }
            '(' => {
                let Some(end) = group_end(pattern, index) else {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                };
                let body_start = group_body_start(pattern, index)?;
                validate_pattern_range(
                    pattern,
                    body_start,
                    end,
                    unicode,
                    unicode_sets,
                    capture_count,
                )?;
                // Lookbehind assertions are not `QuantifiableAssertion`s, so a
                // quantifier immediately after `(?<=...)` / `(?<!...)` is a
                // SyntaxError in both Annex-B and non-Annex-B modes.
                if (is_lookbehind_group(pattern, index)
                    || (unicode && is_lookahead_group(pattern, index)))
                    && pattern
                        .get(end + 1)
                        .is_some_and(|next| matches!(next, '*' | '+' | '?' | '{'))
                {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                index = end + 1;
                has_atom = true;
            }
            ')' if unicode => {
                return Err(regexp_syntax_error("invalid regular expression pattern"));
            }
            // In unicode mode `}` is a syntax character that must be escaped; a
            // lone closing brace (one not consumed by a counted quantifier) is a
            // SyntaxError, mirroring the lone `]`/`)` rejections above.
            '}' if unicode => {
                return Err(regexp_syntax_error("invalid regular expression pattern"));
            }
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
            '{' => match counted_quantifier_bounds(pattern, index) {
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

fn group_body_start(pattern: &[char], start: usize) -> Result<usize, RuntimeError> {
    if pattern.get(start + 1) != Some(&'?') {
        return Ok(start + 1);
    }
    match pattern.get(start + 2) {
        Some(':') | Some('=') | Some('!') => Ok(start + 3),
        Some('<') if matches!(pattern.get(start + 3), Some('=') | Some('!')) => Ok(start + 4),
        Some('<') => validate_group_name(pattern, start + 3).map(|(body_start, _)| body_start),
        _ => Ok(start + 1),
    }
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
    unicode_sets: bool,
) -> Result<(), RuntimeError> {
    let mut index = start;
    while index < end {
        if pattern[index] == '\\' {
            if unicode
                && let Some(next) = validate_unicode_decimal_escape(pattern, index, 0, false)?
            {
                index = next;
                continue;
            }
            if unicode && let Some(set_end) = unicode_class_set_escape_end(pattern, index)? {
                if pattern.get(set_end) == Some(&'-') && set_end + 1 < end {
                    return Err(regexp_syntax_error("invalid regular expression pattern"));
                }
                index = set_end;
                continue;
            }
            let escape_end = if unicode {
                validate_unicode_escape(pattern, index, true)?
            } else {
                class_escape_end(pattern, index, unicode)
            };
            if unicode
                && pattern.get(escape_end) == Some(&'-')
                && escape_end + 1 < end
                && pattern.get(escape_end + 1) == Some(&'\\')
                && unicode_class_set_escape_end(pattern, escape_end + 1)?.is_some()
            {
                return Err(regexp_syntax_error("invalid regular expression pattern"));
            }
            index = escape_end;
            continue;
        }
        if unicode_sets && is_unicode_sets_reserved_class_char(pattern[index]) {
            return Err(regexp_syntax_error("invalid regular expression pattern"));
        }
        if unicode_sets
            && index + 1 < end
            && pattern[index] == pattern[index + 1]
            && is_unicode_sets_double_punctuator_char(pattern[index])
        {
            return Err(regexp_syntax_error("invalid regular expression pattern"));
        }
        if index + 2 < end && pattern[index + 1] == '-' {
            if unicode
                && pattern[index + 2] == '\\'
                && unicode_class_set_escape_end(pattern, index + 2)?.is_some()
            {
                return Err(regexp_syntax_error("invalid regular expression pattern"));
            }
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

fn is_unicode_sets_reserved_class_char(ch: char) -> bool {
    matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '/' | '-' | '|')
}

fn is_unicode_sets_double_punctuator_char(ch: char) -> bool {
    matches!(
        ch,
        '&' | '!'
            | '#'
            | '$'
            | '%'
            | '*'
            | '+'
            | ','
            | '.'
            | ':'
            | ';'
            | '<'
            | '='
            | '>'
            | '?'
            | '@'
            | '^'
            | '`'
            | '~'
    )
}

fn validate_unicode_decimal_escape(
    pattern: &[char],
    start: usize,
    capture_count: usize,
    allow_backreference: bool,
) -> Result<Option<usize>, RuntimeError> {
    let Some(&first) = pattern.get(start + 1) else {
        return Ok(None);
    };
    if !first.is_ascii_digit() {
        return Ok(None);
    }
    if first == '0' {
        if pattern
            .get(start + 2)
            .is_some_and(|next| next.is_ascii_digit())
        {
            return Err(regexp_syntax_error("invalid regular expression pattern"));
        }
        return Ok(Some(start + 2));
    }
    let mut index = start + 1;
    let mut value = 0usize;
    while let Some(digit) = pattern
        .get(index)
        .filter(|ch| ch.is_ascii_digit())
        .and_then(|ch| ch.to_digit(10))
    {
        value = value.saturating_mul(10).saturating_add(digit as usize);
        index += 1;
    }
    if allow_backreference && value <= capture_count {
        return Ok(Some(index));
    }
    Err(regexp_syntax_error("invalid regular expression pattern"))
}

fn unicode_class_set_escape_end(
    pattern: &[char],
    start: usize,
) -> Result<Option<usize>, RuntimeError> {
    match pattern.get(start + 1) {
        Some('d' | 'D' | 's' | 'S' | 'w' | 'W') => Ok(Some(start + 2)),
        Some('p' | 'P') => validate_property_escape(pattern, start).map(Some),
        _ => Ok(None),
    }
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

/// Validates a unicode-mode `\u{ … }` code-point escape whose `{` is at
/// `brace_index`, returning the index just past `}`. The body must be one or
/// more hex digits naming a value no greater than 0x10FFFF.
fn validate_braced_unicode_escape(
    pattern: &[char],
    brace_index: usize,
) -> Result<usize, RuntimeError> {
    let mut cursor = brace_index + 1;
    let mut value: u32 = 0;
    let mut digits = 0;
    while let Some(&ch) = pattern.get(cursor) {
        if ch == '}' {
            break;
        }
        let Some(digit) = ch.to_digit(16) else {
            return Err(regexp_syntax_error(
                "invalid Unicode escape in regular expression",
            ));
        };
        value = value.saturating_mul(16).saturating_add(digit);
        if value > 0x10_FFFF {
            return Err(regexp_syntax_error(
                "invalid Unicode escape in regular expression",
            ));
        }
        digits += 1;
        cursor += 1;
    }
    if digits == 0 || pattern.get(cursor) != Some(&'}') {
        return Err(regexp_syntax_error(
            "invalid Unicode escape in regular expression",
        ));
    }
    Ok(cursor + 1)
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

/// `(?=...)` / `(?!...)` lookahead at `index`. In unicode mode lookahead is not
/// a QuantifiableAssertion, so a following quantifier is a SyntaxError (in
/// Annex-B mode it stays quantifiable).
fn is_lookahead_group(pattern: &[char], index: usize) -> bool {
    pattern.get(index + 1) == Some(&'?') && matches!(pattern.get(index + 2), Some('=') | Some('!'))
}

/// Validate a unicode-mode `\X` escape at `index` (where
/// `pattern[index] == '\\'`) and return the index just past it.
///
/// The AtomEscape and ClassEscape grammars overlap but are not identical:
/// `\B` and named backreferences are atoms only, while `\-` is a class-only
/// identity escape. `\p`/`\P` and decimal escapes are validated before this is
/// reached.
fn validate_unicode_escape(
    pattern: &[char],
    index: usize,
    in_class: bool,
) -> Result<usize, RuntimeError> {
    match pattern.get(index + 1) {
        Some(
            '^' | '$' | '\\' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
            | '/',
        ) => Ok(index + 2),
        Some('-') if in_class => Ok(index + 2),
        Some('d' | 'D' | 's' | 'S' | 'w' | 'W' | 'f' | 'n' | 'r' | 't' | 'v' | 'b') => {
            Ok(index + 2)
        }
        Some('B') if !in_class => Ok(index + 2),
        Some('0') if !pattern.get(index + 2).is_some_and(char::is_ascii_digit) => Ok(index + 2),
        Some('c') => pattern
            .get(index + 2)
            .filter(|ch| ch.is_ascii_alphabetic())
            .map(|_| index + 3)
            .ok_or_else(|| regexp_syntax_error("invalid regular expression pattern")),
        Some('x') => {
            if pattern
                .get(index + 2)
                .is_some_and(|ch| ch.is_ascii_hexdigit())
                && pattern
                    .get(index + 3)
                    .is_some_and(|ch| ch.is_ascii_hexdigit())
            {
                Ok(index + 4)
            } else {
                Err(regexp_syntax_error("invalid regular expression pattern"))
            }
        }
        Some('u') if pattern.get(index + 2) == Some(&'{') => {
            validate_braced_unicode_escape(pattern, index + 2)
        }
        Some('u') => {
            if (0..4).all(|offset| {
                pattern
                    .get(index + 2 + offset)
                    .is_some_and(|ch| ch.is_ascii_hexdigit())
            }) {
                Ok(index + 6)
            } else {
                Err(regexp_syntax_error("invalid regular expression pattern"))
            }
        }
        Some('k') if !in_class => Ok(index + 2),
        _ => Err(regexp_syntax_error("invalid regular expression pattern")),
    }
}

/// Validate `\k<name>` named backreferences against the named groups declared
/// in the pattern.
///
/// Validates every `(?<name>` group specifier as a well-formed
/// `RegExpIdentifierName`: a non-empty identifier whose first code point is a
/// RegExp identifier start (`ID_Start`, `$`, or `_`) and whose remaining code
/// points are identifier parts (`ID_Continue`, `$`, `_`, ZWNJ, or ZWJ). A name
/// may spell its code points with `\u` escapes (`\uXXXX`, a `\uXXXX\uXXXX`
/// surrogate pair, or `\u{...}`) regardless of the `u` flag. This runs at parse
/// time for both regex literals and `new RegExp`, so a malformed group name is a
/// SyntaxError rather than a silently accepted pattern.
fn validate_named_group_definitions(pattern: &[char]) -> Result<(), RuntimeError> {
    let mut index = 0;
    let mut in_class = false;
    let mut seen_names: Vec<String> = Vec::new();
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
                let (next, name) = validate_group_name(pattern, index + 3)?;
                // Two GroupSpecifiers may not share a name (QuickJS-NG rejects
                // all duplicates, including across alternatives).
                if seen_names.contains(&name) {
                    return Err(invalid_group_name());
                }
                seen_names.push(name);
                index = next;
            }
            _ => index += 1,
        }
    }
    Ok(())
}

/// Validates the `RegExpIdentifierName` whose first character is at `start`
/// (just past `(?<`), returning the index just past the closing `>`.
fn validate_group_name(pattern: &[char], start: usize) -> Result<(usize, String), RuntimeError> {
    let mut index = start;
    let mut is_first = true;
    let mut name = String::new();
    loop {
        match pattern.get(index) {
            None => return Err(invalid_group_name()),
            Some('>') if is_first => return Err(invalid_group_name()),
            Some('>') => return Ok((index + 1, name)),
            Some('\\') => {
                let (code_point, next) = group_name_unicode_escape(pattern, index)?;
                check_group_name_char(code_point, is_first)?;
                if let Some(ch) = char::from_u32(code_point) {
                    name.push(ch);
                }
                is_first = false;
                index = next;
            }
            Some(&ch) => {
                check_group_name_char(ch as u32, is_first)?;
                name.push(ch);
                is_first = false;
                index += 1;
            }
        }
    }
}

fn check_group_name_char(code_point: u32, is_first: bool) -> Result<(), RuntimeError> {
    const DOLLAR: u32 = 0x24;
    const UNDERSCORE: u32 = 0x5f;
    const ZWNJ: u32 = 0x200c;
    const ZWJ: u32 = 0x200d;
    let ok = if is_first {
        code_point == DOLLAR || code_point == UNDERSCORE || unicode::is_id_start(code_point)
    } else {
        code_point == DOLLAR
            || code_point == UNDERSCORE
            || code_point == ZWNJ
            || code_point == ZWJ
            || unicode::is_id_continue(code_point)
    };
    if ok {
        Ok(())
    } else {
        Err(invalid_group_name())
    }
}

/// Decodes the `\u` escape at `index` (where `pattern[index] == '\\'`) into a
/// single code point, returning it with the index just past the escape. Only
/// `\u` escapes are valid inside a group name; anything else is a SyntaxError.
fn group_name_unicode_escape(pattern: &[char], index: usize) -> Result<(u32, usize), RuntimeError> {
    if pattern.get(index + 1) != Some(&'u') {
        return Err(invalid_group_name());
    }
    if pattern.get(index + 2) == Some(&'{') {
        let mut cursor = index + 3;
        let mut value: u32 = 0;
        let mut digits = 0;
        while let Some(&ch) = pattern.get(cursor) {
            if ch == '}' {
                break;
            }
            let digit = ch.to_digit(16).ok_or_else(invalid_group_name)?;
            value = value
                .checked_mul(16)
                .and_then(|value| value.checked_add(digit))
                .filter(|value| *value <= 0x10_ffff)
                .ok_or_else(invalid_group_name)?;
            digits += 1;
            cursor += 1;
        }
        if digits == 0 || pattern.get(cursor) != Some(&'}') {
            return Err(invalid_group_name());
        }
        return Ok((value, cursor + 1));
    }
    let high = read_four_hex(pattern, index + 2)?;
    if (0xd800..=0xdbff).contains(&high)
        && pattern.get(index + 6) == Some(&'\\')
        && pattern.get(index + 7) == Some(&'u')
        && let Ok(low) = read_four_hex(pattern, index + 8)
        && (0xdc00..=0xdfff).contains(&low)
    {
        let code_point = 0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00);
        return Ok((code_point, index + 12));
    }
    Ok((high, index + 6))
}

fn read_four_hex(pattern: &[char], start: usize) -> Result<u32, RuntimeError> {
    let mut value = 0u32;
    for offset in 0..4 {
        let digit = pattern
            .get(start + offset)
            .and_then(|ch| ch.to_digit(16))
            .ok_or_else(invalid_group_name)?;
        value = value * 16 + digit;
    }
    Ok(value)
}

fn invalid_group_name() -> RuntimeError {
    regexp_syntax_error("invalid regular expression group name")
}

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
        accepts("]", "");
        accepts(")", "");
        rejects("]", "u");
        rejects(")", "u");
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
    fn rejects_invalid_unicode_identity_escapes() {
        // In unicode mode only specific escapes are legal.
        rejects("\\M", "u");
        rejects("\\a", "u");
        rejects("\\c0", "u");
        rejects("\\c", "u");
        rejects("\\x", "u");
        rejects("\\x1", "u");
        rejects("\\u", "u");
        rejects("\\u1", "u");
        rejects("\\u12", "u");
        rejects("\\u123", "u");
        rejects("\\u{", "u");
        rejects("\\u{}", "u");
        rejects("[\\M]", "u");
        rejects("[\\a]", "u");
        rejects("[\\B]", "u");
        rejects("[\\c]", "u");
        rejects("[\\c0]", "u");
        rejects("[\\x]", "u");
        rejects("[\\x1]", "u");
        rejects("[\\u]", "u");
        rejects("[\\u1]", "u");
        rejects("[\\u12]", "u");
        rejects("[\\u123]", "u");
        rejects("[\\u{]", "u");
        rejects("[\\u{}]", "u");
        // Sloppy mode keeps Annex-B IdentityEscape; valid unicode escapes pass.
        accepts("\\M", "");
        accepts("\\cA", "u");
        accepts("\\x41", "u");
        accepts("\\u0041", "u");
        accepts("\\u{41}", "u");
        accepts("\\d\\w\\s", "u");
        accepts("\\n\\t\\b", "u");
        accepts("\\^\\$\\.", "u");
        accepts("[\\-\\]\\b\\x41\\u0042\\u{43}]", "u");
        accepts("(?<a>x)\\k<a>", "u");
    }

    #[test]
    fn rejects_unicode_legacy_octal_and_invalid_decimal_escapes() {
        accepts("\\0", "u");
        accepts("[\\0]", "u");
        accepts("(a)\\1", "u");
        accepts("(a)(b)(c)(d)(e)(f)(g)(h)(i)\\9", "u");
        rejects("\\00", "u");
        rejects("\\01", "u");
        rejects("\\07", "u");
        rejects("\\08", "u");
        rejects("\\1", "u");
        rejects("(a)\\2", "u");
        rejects("[\\00]", "u");
        rejects("[\\01]", "u");
        rejects("[\\1]", "u");
        rejects("[\\9]", "u");
    }

    #[test]
    fn rejects_quantified_lookahead_in_unicode_mode() {
        rejects(".(?=.)?", "u");
        rejects(".(?!.)?", "u");
        rejects(".(?=.){2,3}", "u");
        rejects(".(?!.){2,3}", "u");
        // Annex-B keeps lookahead quantifiable; an unquantified lookahead is ok.
        accepts(".(?=.)?", "");
        accepts(".(?=.)", "u");
    }

    #[test]
    fn rejects_duplicate_named_groups() {
        rejects("(?<a>a)(?<a>a)", "");
        rejects("(?<a>a)(?<a>a)", "u");
        rejects("(?<a>x)|(?<a>y)", "");
        rejects("(?<a>a)(?<b>b)(?<a>c)", "");
        accepts("(?<a>a)(?<b>b)", "");
        accepts("(?<a>a)\\k<a>", "");
    }

    #[test]
    fn validates_braced_unicode_escape_in_unicode_mode() {
        rejects("\\u{110000}", "u");
        rejects("\\u{1,}", "u");
        rejects("\\u{1F_639}", "u");
        rejects("\\u{}", "u");
        accepts("\\u{1F639}", "u");
        accepts("\\u{10FFFF}", "u");
        accepts("\\u{0}", "u");
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
