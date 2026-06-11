/// Classification of a `(` group opener.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum GroupKind {
    /// `(?:` non-capturing group.
    NonCapturing,
    /// `(?=` / `(?!` lookahead assertion.
    Lookahead { negative: bool },
    /// `(?<=` / `(?<!` lookbehind assertion.
    Lookbehind { negative: bool },
    /// `(?<name>` named capturing group; the body starts after `>`.
    Named { body_offset: usize },
    /// Plain `(` capturing group.
    Capturing,
}

pub(super) fn group_kind(pattern: &[char], pc: usize) -> GroupKind {
    if pattern.get(pc + 1) != Some(&'?') {
        return GroupKind::Capturing;
    }
    match pattern.get(pc + 2) {
        Some(':') => GroupKind::NonCapturing,
        Some('=') => GroupKind::Lookahead { negative: false },
        Some('!') => GroupKind::Lookahead { negative: true },
        Some('<') => match pattern.get(pc + 3) {
            Some('=') => GroupKind::Lookbehind { negative: false },
            Some('!') => GroupKind::Lookbehind { negative: true },
            _ => named_group_body_offset(pattern, pc)
                .map(|body_offset| GroupKind::Named { body_offset })
                .unwrap_or(GroupKind::Capturing),
        },
        _ => GroupKind::Capturing,
    }
}

/// For a `(?<name>` opener, return the offset (relative to `pc`) of the first
/// character of the group body, i.e. just past `>`.
fn named_group_body_offset(pattern: &[char], pc: usize) -> Option<usize> {
    // pattern[pc+1] == '?', pattern[pc+2] == '<'
    let mut index = pc + 3;
    while pattern.get(index).is_some_and(|value| *value != '>') {
        index += 1;
    }
    if pattern.get(index) == Some(&'>') && index > pc + 3 {
        Some(index + 1 - pc)
    } else {
        None
    }
}

/// Extract the name of a `(?<name>` group, if `pc` opens one.
pub(super) fn named_group_name(pattern: &[char], pc: usize) -> Option<String> {
    if !matches!(group_kind(pattern, pc), GroupKind::Named { .. }) {
        return None;
    }
    let mut index = pc + 3;
    let mut name = String::new();
    while let Some(&value) = pattern.get(index) {
        if value == '>' {
            return Some(name);
        }
        name.push(value);
        index += 1;
    }
    None
}

pub(super) fn is_non_capturing_group(pattern: &[char], pc: usize) -> bool {
    !matches!(
        group_kind(pattern, pc),
        GroupKind::Capturing | GroupKind::Named { .. }
    )
}

/// Capture-group names in source order, one entry per capturing group
/// (`None` for unnamed groups). Returns an empty vector when there are no
/// named groups, so callers can set `groups` to `undefined`.
pub(in crate::regexp) fn regexp_group_names(source: &str) -> Vec<Option<String>> {
    let source = super::normalization::normalized_regexp_source(source);
    let pattern: Vec<char> = source.chars().collect();
    let mut names = Vec::new();
    scan_capturing_groups(&pattern, |pc| {
        names.push(named_group_name(&pattern, pc));
    });
    if names.iter().any(Option::is_some) {
        names
    } else {
        Vec::new()
    }
}

/// Parse a `\k<name>` named backreference at `pc` (pointing at the backslash).
/// Returns the captured name and the index just past `>`.
pub(super) fn named_backreference(pattern: &[char], pc: usize) -> Option<(String, usize)> {
    if pattern.get(pc + 1) != Some(&'k') || pattern.get(pc + 2) != Some(&'<') {
        return None;
    }
    let mut index = pc + 3;
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

/// Resolve a named group to its zero-based capture index by scanning the
/// pattern for capturing-group openers in source order.
pub(super) fn named_group_index(pattern: &[char], name: &str) -> Option<usize> {
    let mut capture_index = 0;
    let mut found = None;
    scan_capturing_groups(pattern, |pc| {
        if found.is_none() {
            if named_group_name(pattern, pc).as_deref() == Some(name) {
                found = Some(capture_index);
            }
            capture_index += 1;
        }
    });
    found
}

/// Invoke `visit` with the `pc` of each capturing-group opener (plain or named)
/// in source order, skipping escapes and character classes.
fn scan_capturing_groups(pattern: &[char], mut visit: impl FnMut(usize)) {
    let mut escaped = false;
    let mut in_class = false;
    for index in 0..pattern.len() {
        let char = pattern[index];
        if escaped {
            escaped = false;
        } else if char == '\\' {
            escaped = true;
        } else if char == '[' {
            in_class = true;
        } else if char == ']' {
            in_class = false;
        } else if !in_class && char == '(' && !is_non_capturing_group(pattern, index) {
            visit(index);
        }
    }
}

pub(super) fn closing_group(pattern: &[char], pc: usize) -> Option<usize> {
    let mut escaped = false;
    let mut in_class = false;
    let mut depth = 0usize;
    for (offset, char) in pattern[pc + 1..].iter().enumerate() {
        if escaped {
            escaped = false;
        } else if *char == '\\' {
            escaped = true;
        } else if *char == '[' {
            in_class = true;
        } else if *char == ']' {
            in_class = false;
        } else if !in_class && *char == '(' {
            depth += 1;
        } else if !in_class && *char == ')' && depth > 0 {
            depth -= 1;
        } else if !in_class && *char == ')' {
            return Some(pc + 1 + offset);
        }
    }
    None
}

pub(super) fn group_alternatives(
    pattern: &[char],
    start_pc: usize,
    end_pc: usize,
) -> Vec<(usize, usize)> {
    let mut alternatives = Vec::new();
    let mut start = start_pc;
    let mut escaped = false;
    let mut in_class = false;
    let mut depth = 0usize;
    for (index, char) in pattern
        .iter()
        .enumerate()
        .take(end_pc)
        .skip(start_pc)
        .map(|(index, char)| (index, *char))
    {
        if escaped {
            escaped = false;
        } else if char == '\\' {
            escaped = true;
        } else if char == '[' {
            in_class = true;
        } else if char == ']' {
            in_class = false;
        } else if !in_class && char == '(' {
            depth += 1;
        } else if !in_class && char == ')' && depth > 0 {
            depth -= 1;
        } else if !in_class && char == '|' && depth == 0 {
            alternatives.push((start, index));
            start = index + 1;
        }
    }
    alternatives.push((start, end_pc));
    alternatives
}
