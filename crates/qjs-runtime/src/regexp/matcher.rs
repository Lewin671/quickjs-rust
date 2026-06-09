use std::collections::HashMap;

use crate::string::{advance_string_index, string_code_units, string_from_code_unit};

mod classes;
mod escapes;
mod groups;
mod normalization;
#[cfg(test)]
mod tests;

use classes::class_match;
use escapes::{chars_equal, regexp_control_escape, regexp_word_char, unicode_escape};
use groups::{closing_group, group_alternatives, is_non_capturing_group};
use normalization::normalized_regexp_source;

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
    greedy: bool,
}

#[derive(Clone, Copy)]
struct MatchOptions {
    ignore_case: bool,
    unicode: bool,
    dot_all: bool,
}

struct RepeatAtom<'a> {
    pattern: &'a [char],
    text: &'a [char],
    atom_pc: usize,
    quantifier: Quantifier,
    atom_captures: Vec<usize>,
    group_indices: &'a HashMap<usize, usize>,
    options: MatchOptions,
}

pub(super) fn regexp_match_range(
    source: &str,
    input: &str,
    start_index: usize,
    ignore_case: bool,
    unicode: bool,
    dot_all: bool,
) -> Option<RegexpMatch> {
    regexp_match(
        source,
        input,
        start_index,
        ignore_case,
        unicode,
        dot_all,
        false,
    )
}

pub(super) fn regexp_match_at(
    source: &str,
    input: &str,
    start_index: usize,
    ignore_case: bool,
    unicode: bool,
    dot_all: bool,
) -> Option<RegexpMatch> {
    regexp_match(
        source,
        input,
        start_index,
        ignore_case,
        unicode,
        dot_all,
        true,
    )
}

fn regexp_match(
    source: &str,
    input: &str,
    start_index: usize,
    ignore_case: bool,
    unicode: bool,
    dot_all: bool,
    exact_start: bool,
) -> Option<RegexpMatch> {
    let source = normalized_regexp_source(source);
    let pattern: Vec<_> = source.chars().collect();
    let text: Vec<_> = if unicode {
        input.chars().collect()
    } else {
        string_code_units(input)
            .into_iter()
            .filter_map(|code_unit| string_from_code_unit(code_unit).chars().next())
            .collect()
    };
    if start_index > text.len() {
        return None;
    }
    let group_indices = capture_group_indices(&pattern);
    let options = MatchOptions {
        ignore_case,
        unicode,
        dot_all,
    };
    let starts: Vec<_> = if exact_start {
        vec![start_index]
    } else if pattern.first() == Some(&'^') {
        if start_index == 0 {
            vec![0]
        } else {
            Vec::new()
        }
    } else {
        (start_index.min(text.len())..=text.len())
            .filter(|index| !options.unicode || !is_trailing_surrogate_position(&text, *index))
            .collect()
    };

    starts.into_iter().find_map(|start| {
        let state = MatchState {
            index: start,
            captures: vec![None; group_indices.len()],
        };
        group_alternatives(&pattern, 0, pattern.len())
            .into_iter()
            .find_map(|(alternative_start, alternative_end)| {
                match_pattern(
                    &pattern,
                    &text,
                    alternative_start,
                    alternative_end,
                    state.clone(),
                    &group_indices,
                    options,
                )
                .into_iter()
                .next()
            })
            .map(|state| RegexpMatch {
                start,
                end: state.index,
                captures: state.captures,
            })
    })
}

fn is_trailing_surrogate_position(text: &[char], index: usize) -> bool {
    if index == 0 || index >= text.len() {
        return false;
    }
    matches!(
        (char_code_unit(text[index - 1]), char_code_unit(text[index])),
        (Some(0xD800..=0xDBFF), Some(0xDC00..=0xDFFF))
    )
}

fn char_code_unit(value: char) -> Option<u16> {
    crate::string::string_code_units(&value.to_string())
        .first()
        .copied()
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
    options: MatchOptions,
) -> Vec<MatchState> {
    if pc == end_pc {
        return vec![state];
    }
    match pattern[pc] {
        '^' => {
            if state.index == 0 {
                match_pattern(pattern, text, pc + 1, end_pc, state, group_indices, options)
            } else {
                Vec::new()
            }
        }
        '$' => {
            if state.index == text.len() {
                match_pattern(pattern, text, pc + 1, end_pc, state, group_indices, options)
            } else {
                Vec::new()
            }
        }
        _ => atom_end(pattern, pc, options.unicode)
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
                    options,
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
                        options,
                    )
                })
            })
            .collect(),
    }
}

fn atom_end(pattern: &[char], pc: usize, unicode: bool) -> Option<usize> {
    match pattern.get(pc)? {
        '\\' if unicode_escape(pattern, pc, unicode).is_some() => {
            unicode_escape(pattern, pc, unicode).map(|escape| escape.next_pc)
        }
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
    options: MatchOptions,
) -> Vec<(usize, MatchState)> {
    match pattern[pc] {
        '\\' => match_escape(pattern, text, pc, state, options),
        '[' => match_class(pattern, text, pc, state, options),
        '(' => match_group(pattern, text, pc, state, group_indices, options),
        '.' => match_any(text, pc + 1, state, options),
        literal => match_literal(text, pc + 1, state, literal, options.ignore_case),
    }
}

fn match_any(
    text: &[char],
    next_pc: usize,
    mut state: MatchState,
    options: MatchOptions,
) -> Vec<(usize, MatchState)> {
    let Some(value) = text.get(state.index) else {
        return Vec::new();
    };
    if !options.dot_all && is_line_terminator(*value) {
        return Vec::new();
    }
    state.index = advance_string_index(text, state.index, options.unicode);
    vec![(next_pc, state)]
}

fn is_line_terminator(value: char) -> bool {
    matches!(value, '\n' | '\r' | '\u{2028}' | '\u{2029}')
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
    options: MatchOptions,
) -> Vec<(usize, MatchState)> {
    let Some(escaped) = pattern.get(pc + 1).copied() else {
        return Vec::new();
    };
    if let Some(index) = escaped.to_digit(10).map(|value| value as usize)
        && (1..=state.captures.len()).contains(&index)
    {
        let capture = state.captures[index - 1];
        return match_backreference(text, state, capture, pc + 2, options);
    }
    let Some(value) = text.get(state.index).copied() else {
        return Vec::new();
    };
    let (matched, next_pc) = match escaped {
        'd' => (value.is_ascii_digit(), pc + 2),
        'D' => (!value.is_ascii_digit(), pc + 2),
        's' => (value.is_whitespace(), pc + 2),
        'S' => (!value.is_whitespace(), pc + 2),
        'w' => (regexp_word_char(value), pc + 2),
        'W' => (!regexp_word_char(value), pc + 2),
        'u' => {
            let Some(escape) = unicode_escape(pattern, pc, options.unicode) else {
                let matched = chars_equal(value, 'u', options.ignore_case);
                if matched {
                    state.index += 1;
                    return vec![(pc + 2, state)];
                }
                return Vec::new();
            };
            return match_unicode_escape(
                text,
                state,
                escape.value,
                escape.next_pc,
                options.ignore_case,
            );
        }
        literal => (
            chars_equal(value, regexp_control_escape(literal), options.ignore_case),
            pc + 2,
        ),
    };
    if !matched {
        return Vec::new();
    }
    state.index += 1;
    vec![(next_pc, state)]
}

fn match_backreference(
    text: &[char],
    mut state: MatchState,
    capture: Option<(usize, usize)>,
    next_pc: usize,
    options: MatchOptions,
) -> Vec<(usize, MatchState)> {
    let Some((start, end)) = capture else {
        return vec![(next_pc, state)];
    };
    let capture_len = end - start;
    if state.index + capture_len > text.len() {
        return Vec::new();
    }
    let matched = (0..capture_len).all(|offset| {
        chars_equal(
            text[state.index + offset],
            text[start + offset],
            options.ignore_case,
        )
    });
    if !matched {
        return Vec::new();
    }
    state.index += capture_len;
    vec![(next_pc, state)]
}

fn match_unicode_escape(
    text: &[char],
    state: MatchState,
    value: char,
    next_pc: usize,
    ignore_case: bool,
) -> Vec<(usize, MatchState)> {
    let mut matches = Vec::new();
    if text
        .get(state.index)
        .is_some_and(|current| chars_equal(*current, value, ignore_case))
    {
        let mut matched = state.clone();
        matched.index += 1;
        matches.push((next_pc, matched));
    }

    let mut buffer = [0u16; 2];
    let code_units = value.encode_utf16(&mut buffer);
    if code_units.len() == 2
        && text.get(state.index) == Some(&code_unit_char(code_units[0]))
        && text.get(state.index + 1) == Some(&code_unit_char(code_units[1]))
    {
        let mut matched = state;
        matched.index += 2;
        matches.push((next_pc, matched));
    }
    matches
}

fn code_unit_char(code_unit: u16) -> char {
    string_from_code_unit(code_unit)
        .chars()
        .next()
        .unwrap_or(char::REPLACEMENT_CHARACTER)
}

fn match_class(
    pattern: &[char],
    text: &[char],
    pc: usize,
    mut state: MatchState,
    options: MatchOptions,
) -> Vec<(usize, MatchState)> {
    let Some(end) = pattern[pc + 1..].iter().position(|char| *char == ']') else {
        return Vec::new();
    };
    let class_end = pc + 1 + end;
    let class = &pattern[pc + 1..class_end];
    let Some((value, next_index)) = regexp_code_point_at(text, state.index, options.unicode) else {
        return Vec::new();
    };
    if !class_match(class, value, options) {
        return Vec::new();
    }
    state.index = next_index;
    vec![(class_end + 1, state)]
}

fn regexp_code_point_at(text: &[char], index: usize, unicode: bool) -> Option<(char, usize)> {
    let first = *text.get(index)?;
    if unicode
        && let Some(high) = char_code_unit(first)
        && (0xD800..=0xDBFF).contains(&high)
        && let Some(low) = text.get(index + 1).and_then(|value| char_code_unit(*value))
        && (0xDC00..=0xDFFF).contains(&low)
    {
        let code_point = 0x10000 + ((u32::from(high) - 0xD800) << 10) + u32::from(low) - 0xDC00;
        if let Some(value) = char::from_u32(code_point) {
            return Some((value, index + 2));
        }
    }
    Some((first, index + 1))
}

fn quantifier(pattern: &[char], pc: usize) -> Quantifier {
    match pattern.get(pc) {
        Some('?') => Quantifier {
            min: 0,
            max: Some(1),
            next_pc: pc + 1,
            greedy: true,
        }
        .with_lazy_suffix(pattern),
        Some('*') => Quantifier {
            min: 0,
            max: None,
            next_pc: pc + 1,
            greedy: true,
        }
        .with_lazy_suffix(pattern),
        Some('+') => Quantifier {
            min: 1,
            max: None,
            next_pc: pc + 1,
            greedy: true,
        }
        .with_lazy_suffix(pattern),
        Some('{') => counted_quantifier(pattern, pc).unwrap_or(Quantifier {
            min: 1,
            max: Some(1),
            next_pc: pc,
            greedy: true,
        }),
        _ => Quantifier {
            min: 1,
            max: Some(1),
            next_pc: pc,
            greedy: true,
        },
    }
}

impl Quantifier {
    fn with_lazy_suffix(mut self, pattern: &[char]) -> Self {
        if pattern.get(self.next_pc) == Some(&'?') {
            self.greedy = false;
            self.next_pc += 1;
        }
        self
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
        return Some(
            Quantifier {
                min,
                max: Some(min),
                next_pc: index + 1,
                greedy: true,
            }
            .with_lazy_suffix(pattern),
        );
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
    Some(
        Quantifier {
            min,
            max: has_max.then_some(max),
            next_pc: index + 1,
            greedy: true,
        }
        .with_lazy_suffix(pattern),
    )
}

fn repeat_atom(
    pattern: &[char],
    text: &[char],
    atom_pc: usize,
    quantifier: Quantifier,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    options: MatchOptions,
) -> Vec<MatchState> {
    let atom_captures = atom_capture_indices(pattern, atom_pc, group_indices, options.unicode);
    let mut current = vec![state];
    for _ in 0..quantifier.min {
        current = current
            .into_iter()
            .flat_map(|state| match_atom(pattern, text, atom_pc, state, group_indices, options))
            .map(|(_, state)| state)
            .collect();
        if current.is_empty() {
            return Vec::new();
        }
    }

    if quantifier.max == Some(quantifier.min) {
        return current;
    }

    let repeat = RepeatAtom {
        pattern,
        text,
        atom_pc,
        quantifier,
        atom_captures,
        group_indices,
        options,
    };
    let mut results = Vec::new();
    for state in current {
        repeat_atom_from(&repeat, state, quantifier.min, &mut results);
    }
    results
}

fn repeat_atom_from(
    repeat: &RepeatAtom<'_>,
    state: MatchState,
    count: usize,
    results: &mut Vec<MatchState>,
) {
    if !repeat.quantifier.greedy {
        results.push(repeat_accept_state(
            state.clone(),
            count,
            &repeat.atom_captures,
        ));
    }

    if repeat.quantifier.max.is_none_or(|max| count < max) {
        for (_, next_state) in match_atom(
            repeat.pattern,
            repeat.text,
            repeat.atom_pc,
            state.clone(),
            repeat.group_indices,
            repeat.options,
        ) {
            if next_state.index == state.index {
                continue;
            }
            repeat_atom_from(repeat, next_state, count + 1, results);
        }
    }

    if repeat.quantifier.greedy {
        results.push(repeat_accept_state(state, count, &repeat.atom_captures));
    }
}

fn repeat_accept_state(mut state: MatchState, count: usize, atom_captures: &[usize]) -> MatchState {
    if count == 0 {
        for capture in atom_captures {
            state.captures[*capture] = None;
        }
    }
    state
}

fn atom_capture_indices(
    pattern: &[char],
    atom_pc: usize,
    group_indices: &HashMap<usize, usize>,
    unicode: bool,
) -> Vec<usize> {
    let Some(atom_end) = atom_end(pattern, atom_pc, unicode) else {
        return Vec::new();
    };
    let mut indices: Vec<_> = group_indices
        .iter()
        .filter_map(|(group_pc, index)| {
            (atom_pc <= *group_pc && *group_pc < atom_end).then_some(*index)
        })
        .collect();
    indices.sort_unstable();
    indices
}

fn match_group(
    pattern: &[char],
    text: &[char],
    pc: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    options: MatchOptions,
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
            options,
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
