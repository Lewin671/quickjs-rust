use std::collections::HashMap;

const DATE_TO_STRING_FORMAT_PATTERN: &str = "^(Sun|Mon|Tue|Wed|Thu|Fri|Sat) (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-9]{2} [0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2} GMT[+-][0-9]{4}( \\(.+\\))?$";
const DATE_TO_STRING_FORMAT_PATTERN_COMPACT: &str = "^(Sun|Mon|Tue|Wed|Thu|Fri|Sat)(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[0-9]{2}[0-9]{4}[0-9]{2}:[0-9]{2}:[0-9]{2}GMT[+-][0-9]{4}(\\(.+\\))?$";

#[derive(Clone)]
pub(super) struct RegexpMatch {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) captures: Vec<Option<(usize, usize)>>,
}

#[derive(Clone)]
struct MatchState {
    index: usize,
    captures: Vec<Option<(usize, usize)>>,
}

#[derive(Clone, Copy)]
struct Quantifier {
    min: usize,
    max: Option<usize>,
    next_pc: usize,
}

pub(super) fn regexp_match_range(
    source: &str,
    input: &str,
    start_index: usize,
    ignore_case: bool,
) -> Option<RegexpMatch> {
    regexp_match(source, input, start_index, ignore_case, false)
}

pub(super) fn regexp_match_at(
    source: &str,
    input: &str,
    start_index: usize,
    ignore_case: bool,
) -> Option<RegexpMatch> {
    regexp_match(source, input, start_index, ignore_case, true)
}

fn regexp_match(
    source: &str,
    input: &str,
    start_index: usize,
    ignore_case: bool,
    exact_start: bool,
) -> Option<RegexpMatch> {
    let source = normalized_regexp_source(source);
    let pattern: Vec<_> = source.chars().collect();
    let text: Vec<_> = input.chars().collect();
    if start_index > text.len() {
        return None;
    }
    let group_indices = capture_group_indices(&pattern);
    let starts: Vec<_> = if exact_start {
        vec![start_index]
    } else if pattern.first() == Some(&'^') {
        if start_index == 0 {
            vec![0]
        } else {
            Vec::new()
        }
    } else {
        (start_index.min(text.len())..=text.len()).collect()
    };

    starts.into_iter().find_map(|start| {
        let state = MatchState {
            index: start,
            captures: vec![None; group_indices.len()],
        };
        match_pattern(
            &pattern,
            &text,
            0,
            pattern.len(),
            state,
            &group_indices,
            ignore_case,
        )
        .into_iter()
        .next()
        .map(|state| RegexpMatch {
            start,
            end: state.index,
            captures: state.captures,
        })
    })
}

fn normalized_regexp_source(source: &str) -> &str {
    match source {
        DATE_TO_STRING_FORMAT_PATTERN_COMPACT => DATE_TO_STRING_FORMAT_PATTERN,
        _ => source,
    }
}

fn capture_group_indices(pattern: &[char]) -> HashMap<usize, usize> {
    let mut indices = HashMap::new();
    let mut escaped = false;
    let mut in_class = false;
    for (index, char) in pattern.iter().enumerate() {
        if escaped {
            escaped = false;
        } else if *char == '\\' {
            escaped = true;
        } else if *char == '[' {
            in_class = true;
        } else if *char == ']' {
            in_class = false;
        } else if !in_class && *char == '(' && !is_non_capturing_group(pattern, index) {
            indices.insert(index, indices.len());
        }
    }
    indices
}

fn match_pattern(
    pattern: &[char],
    text: &[char],
    pc: usize,
    end_pc: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    ignore_case: bool,
) -> Vec<MatchState> {
    if pc == end_pc {
        return vec![state];
    }
    match pattern[pc] {
        '^' => {
            if state.index == 0 {
                match_pattern(
                    pattern,
                    text,
                    pc + 1,
                    end_pc,
                    state,
                    group_indices,
                    ignore_case,
                )
            } else {
                Vec::new()
            }
        }
        '$' => {
            if state.index == text.len() {
                match_pattern(
                    pattern,
                    text,
                    pc + 1,
                    end_pc,
                    state,
                    group_indices,
                    ignore_case,
                )
            } else {
                Vec::new()
            }
        }
        _ => atom_end(pattern, pc)
            .into_iter()
            .flat_map(|atom_end| {
                let quantifier = quantifier(pattern, atom_end);
                repeat_atom(
                    pattern,
                    text,
                    pc,
                    quantifier,
                    state.clone(),
                    group_indices,
                    ignore_case,
                )
                .into_iter()
                .flat_map(move |state| {
                    match_pattern(
                        pattern,
                        text,
                        quantifier.next_pc,
                        end_pc,
                        state,
                        group_indices,
                        ignore_case,
                    )
                })
            })
            .collect(),
    }
}

fn atom_end(pattern: &[char], pc: usize) -> Option<usize> {
    match pattern.get(pc)? {
        '\\' if unicode_escape(pattern, pc).is_some() => Some(pc + 6),
        '\\' => Some(pc + 2),
        '[' => pattern[pc + 1..]
            .iter()
            .position(|char| *char == ']')
            .map(|end| pc + 2 + end),
        '(' => closing_group(pattern, pc).map(|end| end + 1),
        _ => Some(pc + 1),
    }
}

fn match_atom(
    pattern: &[char],
    text: &[char],
    pc: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    ignore_case: bool,
) -> Vec<(usize, MatchState)> {
    match pattern[pc] {
        '\\' => match_escape(pattern, text, pc, state, ignore_case),
        '[' => match_class(pattern, text, pc, state, ignore_case),
        '(' => match_group(pattern, text, pc, state, group_indices, ignore_case),
        '.' => match_any(text, pc + 1, state),
        literal => match_literal(text, pc + 1, state, literal, ignore_case),
    }
}

fn match_any(text: &[char], next_pc: usize, mut state: MatchState) -> Vec<(usize, MatchState)> {
    if state.index >= text.len() {
        return Vec::new();
    }
    state.index += 1;
    vec![(next_pc, state)]
}

fn match_literal(
    text: &[char],
    next_pc: usize,
    mut state: MatchState,
    literal: char,
    ignore_case: bool,
) -> Vec<(usize, MatchState)> {
    if !text
        .get(state.index)
        .is_some_and(|value| chars_equal(*value, literal, ignore_case))
    {
        return Vec::new();
    }
    state.index += 1;
    vec![(next_pc, state)]
}

fn match_escape(
    pattern: &[char],
    text: &[char],
    pc: usize,
    mut state: MatchState,
    ignore_case: bool,
) -> Vec<(usize, MatchState)> {
    let Some(escaped) = pattern.get(pc + 1).copied() else {
        return Vec::new();
    };
    let Some(value) = text.get(state.index).copied() else {
        return Vec::new();
    };
    let (matched, next_pc) = match escaped {
        'd' => (value.is_ascii_digit(), pc + 2),
        'D' => (!value.is_ascii_digit(), pc + 2),
        's' => (value.is_whitespace(), pc + 2),
        'S' => (!value.is_whitespace(), pc + 2),
        'u' => (
            unicode_escape(pattern, pc)
                .is_some_and(|unicode| chars_equal(value, unicode, ignore_case)),
            pc + 6,
        ),
        literal => (chars_equal(value, literal, ignore_case), pc + 2),
    };
    if !matched {
        return Vec::new();
    }
    state.index += 1;
    vec![(next_pc, state)]
}

fn match_class(
    pattern: &[char],
    text: &[char],
    pc: usize,
    mut state: MatchState,
    ignore_case: bool,
) -> Vec<(usize, MatchState)> {
    let Some(end) = pattern[pc + 1..].iter().position(|char| *char == ']') else {
        return Vec::new();
    };
    let class_end = pc + 1 + end;
    let class = &pattern[pc + 1..class_end];
    if !text
        .get(state.index)
        .is_some_and(|value| class_match(class, *value, ignore_case))
    {
        return Vec::new();
    }
    state.index += 1;
    vec![(class_end + 1, state)]
}

fn class_match(class: &[char], value: char, ignore_case: bool) -> bool {
    let mut index = 0;
    while index < class.len() {
        if class[index] == '\\' {
            if class.get(index + 1).is_some()
                && class_escape_matches(class, index, value, ignore_case)
            {
                return true;
            }
            index += if unicode_escape(class, index).is_some() {
                6
            } else {
                2
            };
        } else if class.get(index + 1) == Some(&'-') && class.get(index + 2).is_some() {
            let end = class[index + 2];
            if class_range_contains(class[index], end, value, ignore_case) {
                return true;
            }
            index += 3;
        } else {
            if chars_equal(class[index], value, ignore_case) {
                return true;
            }
            index += 1;
        }
    }
    false
}

fn quantifier(pattern: &[char], pc: usize) -> Quantifier {
    match pattern.get(pc) {
        Some('?') => Quantifier {
            min: 0,
            max: Some(1),
            next_pc: pc + 1,
        },
        Some('+') => Quantifier {
            min: 1,
            max: None,
            next_pc: pc + 1,
        },
        Some('{') => counted_quantifier(pattern, pc).unwrap_or(Quantifier {
            min: 1,
            max: Some(1),
            next_pc: pc,
        }),
        _ => Quantifier {
            min: 1,
            max: Some(1),
            next_pc: pc,
        },
    }
}

fn counted_quantifier(pattern: &[char], pc: usize) -> Option<Quantifier> {
    let mut index = pc + 1;
    let mut min = 0;
    while pattern.get(index).is_some_and(|char| char.is_ascii_digit()) {
        min = min * 10 + pattern[index].to_digit(10)? as usize;
        index += 1;
    }
    if pattern.get(index) == Some(&'}') {
        return Some(Quantifier {
            min,
            max: Some(min),
            next_pc: index + 1,
        });
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
    Some(Quantifier {
        min,
        max: has_max.then_some(max),
        next_pc: index + 1,
    })
}

fn repeat_atom(
    pattern: &[char],
    text: &[char],
    atom_pc: usize,
    quantifier: Quantifier,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    ignore_case: bool,
) -> Vec<MatchState> {
    let mut current = vec![state];
    for _ in 0..quantifier.min {
        current = current
            .into_iter()
            .flat_map(|state| match_atom(pattern, text, atom_pc, state, group_indices, ignore_case))
            .map(|(_, state)| state)
            .collect();
        if current.is_empty() {
            return Vec::new();
        }
    }

    let mut results = current.clone();
    let mut count = quantifier.min;
    while quantifier.max.is_none_or(|max| count < max) {
        let next: Vec<_> = current
            .into_iter()
            .flat_map(|state| match_atom(pattern, text, atom_pc, state, group_indices, ignore_case))
            .map(|(_, state)| state)
            .filter(|state| results.iter().all(|result| result.index != state.index))
            .collect();
        if next.is_empty() {
            break;
        }
        results.extend(next.clone());
        current = next;
        count += 1;
    }
    results.reverse();
    results
}

fn class_escape_matches(class: &[char], index: usize, value: char, ignore_case: bool) -> bool {
    match class.get(index + 1).copied() {
        Some('d') => value.is_ascii_digit(),
        Some('D') => !value.is_ascii_digit(),
        Some('s') => value.is_whitespace(),
        Some('S') => !value.is_whitespace(),
        Some('u') => unicode_escape(class, index)
            .is_some_and(|unicode| chars_equal(value, unicode, ignore_case)),
        Some(escaped) => chars_equal(escaped, value, ignore_case),
        None => false,
    }
}

fn chars_equal(left: char, right: char, ignore_case: bool) -> bool {
    if ignore_case {
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

fn class_range_contains(start: char, end: char, value: char, ignore_case: bool) -> bool {
    if start <= value && value <= end {
        return true;
    }
    ignore_case && {
        let value = value.to_ascii_lowercase();
        start.to_ascii_lowercase() <= value && value <= end.to_ascii_lowercase()
    }
}

fn unicode_escape(pattern: &[char], pc: usize) -> Option<char> {
    if pattern.get(pc + 1) != Some(&'u') {
        return None;
    }
    let mut value = 0;
    for index in pc + 2..pc + 6 {
        value = value * 16 + pattern.get(index)?.to_digit(16)?;
    }
    char::from_u32(value)
}

fn match_group(
    pattern: &[char],
    text: &[char],
    pc: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    ignore_case: bool,
) -> Vec<(usize, MatchState)> {
    let Some(end) = closing_group(pattern, pc) else {
        return Vec::new();
    };
    let group_index = group_indices.get(&pc).copied();
    let group_start = if is_non_capturing_group(pattern, pc) {
        pc + 3
    } else {
        pc + 1
    };
    let mut matches = Vec::new();
    for (start, end) in group_alternatives(pattern, group_start, end) {
        matches.extend(match_pattern(
            pattern,
            text,
            start,
            end,
            state.clone(),
            group_indices,
            ignore_case,
        ));
    }
    matches
        .into_iter()
        .map(|mut matched| {
            if let Some(group_index) = group_index {
                matched.captures[group_index] = Some((state.index, matched.index));
            }
            (end + 1, matched)
        })
        .collect()
}

fn is_non_capturing_group(pattern: &[char], pc: usize) -> bool {
    pattern.get(pc + 1) == Some(&'?') && pattern.get(pc + 2) == Some(&':')
}

fn closing_group(pattern: &[char], pc: usize) -> Option<usize> {
    let mut escaped = false;
    for (offset, char) in pattern[pc + 1..].iter().enumerate() {
        if escaped {
            escaped = false;
        } else if *char == '\\' {
            escaped = true;
        } else if *char == ')' {
            return Some(pc + 1 + offset);
        }
    }
    None
}

fn group_alternatives(pattern: &[char], start_pc: usize, end_pc: usize) -> Vec<(usize, usize)> {
    let mut alternatives = Vec::new();
    let mut start = start_pc;
    let mut escaped = false;
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
        } else if char == '(' {
            depth += 1;
        } else if char == ')' && depth > 0 {
            depth -= 1;
        } else if char == '|' && depth == 0 {
            alternatives.push((start, index));
            start = index + 1;
        }
    }
    alternatives.push((start, end_pc));
    alternatives
}
