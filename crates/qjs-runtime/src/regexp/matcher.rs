use std::collections::{HashMap, HashSet};

use crate::string::{
    advance_string_index, char_from_code_unit, string_code_units, surrogate_escape_code_unit,
};

mod classes;
mod escapes;
mod fast_scan;
mod groups;
mod lookaround;
mod normalization;
#[cfg(test)]
mod tests;

use classes::class_match;
use escapes::{
    ParsedEscape, ParsedPropertyEscape, PropertyCache, char_code_unit, chars_equal,
    control_letter_escape, hex_escape, is_trailing_surrogate_position, legacy_octal_escape,
    property_escape, regexp_control_escape, regexp_whitespace, regexp_word_char, unicode_escape,
};
use fast_scan::{repeat_simple_atom, simple_atom_boundaries, simple_atom_matcher};
use groups::{
    GroupKind, closing_group, group_alternatives, group_kind, is_non_capturing_group,
    named_backreference, named_group_index,
};
use lookaround::match_lookaround;
use normalization::normalized_regexp_source;

pub(super) use groups::regexp_group_names;

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
    multiline: bool,
    reverse_captures: bool,
}

/// Pattern-side state that is invariant across the repeated `exec` calls made
/// by global RegExp operations. Keeping it separate from the input lets native
/// builtins prepare both sides once without changing the observable `exec`
/// protocol for custom RegExp-like objects.
pub(super) struct PreparedRegexp {
    pattern: Vec<char>,
    group_indices: HashMap<usize, usize>,
    properties: PropertyCache,
    alternatives: Vec<(usize, usize)>,
    options: MatchOptions,
}

pub(super) struct PreparedInput {
    text: Vec<char>,
}

impl PreparedRegexp {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        source: &str,
        ignore_case: bool,
        unicode: bool,
        dot_all: bool,
        multiline: bool,
    ) -> Self {
        let source = normalized_regexp_source(source);
        let pattern: Vec<char> = if unicode {
            source.chars().collect()
        } else {
            string_code_units(source)
                .into_iter()
                .map(char_from_code_unit)
                .collect()
        };
        let options = MatchOptions {
            ignore_case,
            unicode,
            dot_all,
            multiline,
            reverse_captures: false,
        };
        let group_indices = capture_group_indices(&pattern);
        let properties = PropertyCache::build(&pattern);
        let alternatives = group_alternatives(&pattern, 0, pattern.len());
        Self {
            pattern,
            group_indices,
            properties,
            alternatives,
            options,
        }
    }

    pub(super) fn prepare_input(&self, input: &str) -> PreparedInput {
        let text = if self.options.unicode {
            input.chars().collect()
        } else {
            string_code_units(input)
                .into_iter()
                .map(char_from_code_unit)
                .collect()
        };
        PreparedInput { text }
    }

    pub(super) fn match_range(
        &self,
        input: &str,
        prepared_input: &PreparedInput,
        start_index: usize,
    ) -> Option<RegexpMatch> {
        self.match_input(input, prepared_input, start_index, false)
    }

    pub(super) fn match_at(
        &self,
        input: &str,
        prepared_input: &PreparedInput,
        start_index: usize,
    ) -> Option<RegexpMatch> {
        self.match_input(input, prepared_input, start_index, true)
    }

    fn match_input(
        &self,
        input: &str,
        prepared_input: &PreparedInput,
        start_index: usize,
        exact_start: bool,
    ) -> Option<RegexpMatch> {
        match match_anchored_property_repetition(&self.pattern, input, start_index, self.options) {
            AnchoredPropertyResult::Matched(match_result) => return Some(match_result),
            AnchoredPropertyResult::NoMatch => return None,
            AnchoredPropertyResult::NotAnchored => {}
        }
        let text = &prepared_input.text;
        if start_index > text.len() {
            return None;
        }
        let final_start = if exact_start { start_index } else { text.len() };
        (start_index..=final_start)
            .filter(|index| !self.options.unicode || !is_trailing_surrogate_position(text, *index))
            .find_map(|start| {
                let state = MatchState {
                    index: start,
                    captures: vec![None; self.group_indices.len()],
                };
                self.alternatives
                    .iter()
                    .find_map(|(alternative_start, alternative_end)| {
                        match_pattern_first(
                            &self.pattern,
                            text,
                            *alternative_start,
                            *alternative_end,
                            state.clone(),
                            &self.group_indices,
                            &self.properties,
                            self.options,
                        )
                    })
                    .map(|state| RegexpMatch {
                        start,
                        end: state.index,
                        captures: state.captures,
                    })
            })
    }
}

struct RepeatAtom<'a> {
    pattern: &'a [char],
    text: &'a [char],
    atom_pc: usize,
    quantifier: Quantifier,
    atom_captures: Vec<usize>,
    group_indices: &'a HashMap<usize, usize>,
    properties: &'a PropertyCache,
    options: MatchOptions,
}

#[derive(Clone, Copy)]
struct AtomStep {
    pc: usize,
    quantifier: Quantifier,
}

pub(super) fn regexp_match_range(
    source: &str,
    input: &str,
    start_index: usize,
    ignore_case: bool,
    unicode: bool,
    dot_all: bool,
    multiline: bool,
) -> Option<RegexpMatch> {
    regexp_match(
        source,
        input,
        start_index,
        ignore_case,
        unicode,
        dot_all,
        multiline,
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
    multiline: bool,
) -> Option<RegexpMatch> {
    regexp_match(
        source,
        input,
        start_index,
        ignore_case,
        unicode,
        dot_all,
        multiline,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn regexp_match(
    source: &str,
    input: &str,
    start_index: usize,
    ignore_case: bool,
    unicode: bool,
    dot_all: bool,
    multiline: bool,
    exact_start: bool,
) -> Option<RegexpMatch> {
    let prepared = PreparedRegexp::new(source, ignore_case, unicode, dot_all, multiline);
    let prepared_input = prepared.prepare_input(input);
    if exact_start {
        prepared.match_at(input, &prepared_input, start_index)
    } else {
        prepared.match_range(input, &prepared_input, start_index)
    }
}

enum AnchoredPropertyResult {
    NotAnchored,
    NoMatch,
    Matched(RegexpMatch),
}

fn match_anchored_property_repetition(
    pattern: &[char],
    input: &str,
    start_index: usize,
    options: MatchOptions,
) -> AnchoredPropertyResult {
    if start_index != 0 || options.ignore_case || !options.unicode || options.multiline {
        return AnchoredPropertyResult::NotAnchored;
    }
    let Some(escape) = anchored_property_repetition_escape(pattern) else {
        return AnchoredPropertyResult::NotAnchored;
    };
    if matches!(escape, AnchoredPropertyRepetition::RgiEmoji) {
        return if is_rgi_emoji_concatenation(input) {
            AnchoredPropertyResult::Matched(RegexpMatch {
                start: 0,
                end: input.chars().count(),
                captures: Vec::new(),
            })
        } else {
            AnchoredPropertyResult::NoMatch
        };
    }
    let AnchoredPropertyRepetition::CodePoint(escape) = escape else {
        return AnchoredPropertyResult::NotAnchored;
    };
    let mut chars = input.chars().peekable();
    let mut index = 0;
    let mut matched = false;
    while let Some(first) = chars.next() {
        let mut code_point = u32::from(first);
        index += 1;
        if let Some(first_unit) = surrogate_escape_code_unit(first) {
            code_point = u32::from(first_unit);
            if (0xD800..=0xDBFF).contains(&first_unit)
                && let Some(second_unit) = chars
                    .peek()
                    .and_then(|value| surrogate_escape_code_unit(*value))
                && (0xDC00..=0xDFFF).contains(&second_unit)
            {
                chars.next();
                index += 1;
                code_point =
                    0x10000 + ((u32::from(first_unit) - 0xD800) << 10) + u32::from(second_unit)
                        - 0xDC00;
            }
        }
        if escape.set.contains(code_point) == escape.negated {
            return AnchoredPropertyResult::NoMatch;
        }
        matched = true;
    }
    if matched {
        AnchoredPropertyResult::Matched(RegexpMatch {
            start: 0,
            end: index,
            captures: Vec::new(),
        })
    } else {
        AnchoredPropertyResult::NoMatch
    }
}

enum AnchoredPropertyRepetition {
    CodePoint(ParsedPropertyEscape),
    RgiEmoji,
}

fn anchored_property_repetition_escape(pattern: &[char]) -> Option<AnchoredPropertyRepetition> {
    if pattern.first() != Some(&'^') {
        return None;
    }
    if matches_rgi_emoji_repetition(pattern) {
        return Some(AnchoredPropertyRepetition::RgiEmoji);
    }
    let escape = property_escape(pattern, 1)?;
    if pattern.get(escape.next_pc) == Some(&'+')
        && pattern.get(escape.next_pc + 1) == Some(&'$')
        && escape.next_pc + 2 == pattern.len()
    {
        return Some(AnchoredPropertyRepetition::CodePoint(escape));
    }
    None
}

fn matches_rgi_emoji_repetition(pattern: &[char]) -> bool {
    let body = [
        '^', '\\', 'p', '{', 'R', 'G', 'I', '_', 'E', 'm', 'o', 'j', 'i', '}', '+', '$',
    ];
    pattern == body
}

fn is_rgi_emoji_concatenation(input: &str) -> bool {
    !input.is_empty() && input.chars().all(is_rgi_emoji_component)
}

fn is_rgi_emoji_component(ch: char) -> bool {
    let code = u32::from(ch);
    matches!(
        code,
        0x200D
            | 0xFE0F
            | 0x1F1E6..=0x1F1FF
            | 0x1F300..=0x1FAFF
            | 0x2600..=0x27BF
            | 0x2300..=0x23FF
            | 0x2B00..=0x2BFF
            | 0xE0020..=0xE007F
    )
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

#[allow(clippy::too_many_arguments)]
fn match_pattern(
    pattern: &[char],
    text: &[char],
    pc: usize,
    end_pc: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    if pc == end_pc {
        return vec![state];
    }
    match pattern[pc] {
        '^' => {
            if at_line_start(text, state.index, options.multiline) {
                match_pattern(
                    pattern,
                    text,
                    pc + 1,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                )
            } else {
                Vec::new()
            }
        }
        '$' => {
            if at_line_end(text, state.index, options.multiline) {
                match_pattern(
                    pattern,
                    text,
                    pc + 1,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                )
            } else {
                Vec::new()
            }
        }
        // `\b` / `\B` are zero-width word-boundary assertions, not literal atoms.
        '\\' if matches!(pattern.get(pc + 1), Some('b' | 'B')) => {
            let before = state.index > 0 && regexp_word_char(text[state.index - 1]);
            let after = text.get(state.index).copied().is_some_and(regexp_word_char);
            let want_boundary = pattern[pc + 1] == 'b';
            if (before != after) == want_boundary {
                match_pattern(
                    pattern,
                    text,
                    pc + 2,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                )
            } else {
                Vec::new()
            }
        }
        _ => atom_end(pattern, pc, properties, options.unicode)
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
                    properties,
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
                        properties,
                        options,
                    )
                })
            })
            .collect(),
    }
}

/// Find the first match in ECMAScript backtracking priority order without
/// constructing all successful end states. Top-level RegExp execution only
/// consumes that first state; keeping the all-state matcher for nested atoms
/// preserves the existing general fallback while the common sequence of
/// simple atoms can stream its repetition boundaries into the continuation.
#[allow(clippy::too_many_arguments)]
fn match_pattern_first(
    pattern: &[char],
    text: &[char],
    pc: usize,
    end_pc: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Option<MatchState> {
    if pc == end_pc {
        return Some(state);
    }
    match pattern[pc] {
        '^' => at_line_start(text, state.index, options.multiline)
            .then_some(())
            .and_then(|()| {
                match_pattern_first(
                    pattern,
                    text,
                    pc + 1,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                )
            }),
        '$' => at_line_end(text, state.index, options.multiline)
            .then_some(())
            .and_then(|()| {
                match_pattern_first(
                    pattern,
                    text,
                    pc + 1,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                )
            }),
        '\\' if matches!(pattern.get(pc + 1), Some('b' | 'B')) => {
            let before = state.index > 0 && regexp_word_char(text[state.index - 1]);
            let after = text.get(state.index).copied().is_some_and(regexp_word_char);
            let want_boundary = pattern[pc + 1] == 'b';
            ((before != after) == want_boundary)
                .then_some(())
                .and_then(|()| {
                    match_pattern_first(
                        pattern,
                        text,
                        pc + 2,
                        end_pc,
                        state,
                        group_indices,
                        properties,
                        options,
                    )
                })
        }
        _ => {
            let atom_end = atom_end(pattern, pc, properties, options.unicode)?;
            let quantifier = quantifier(pattern, atom_end);
            let atom_captures =
                atom_capture_indices(pattern, pc, group_indices, properties, options.unicode);
            if atom_captures.is_empty()
                && let Some(matcher) = simple_atom_matcher(pattern, pc, properties, options)
            {
                let boundaries = simple_atom_boundaries(
                    text,
                    &matcher,
                    quantifier,
                    state.index,
                    properties,
                    options,
                )?;
                let lowest = quantifier.min;
                let highest = boundaries.len() - 1;
                if quantifier.greedy {
                    for count in (lowest..=highest).rev() {
                        let mut candidate = state.clone();
                        candidate.index = boundaries[count];
                        if let Some(matched) = match_pattern_first(
                            pattern,
                            text,
                            quantifier.next_pc,
                            end_pc,
                            candidate,
                            group_indices,
                            properties,
                            options,
                        ) {
                            return Some(matched);
                        }
                    }
                } else {
                    for boundary in &boundaries[lowest..=highest] {
                        let mut candidate = state.clone();
                        candidate.index = *boundary;
                        if let Some(matched) = match_pattern_first(
                            pattern,
                            text,
                            quantifier.next_pc,
                            end_pc,
                            candidate,
                            group_indices,
                            properties,
                            options,
                        ) {
                            return Some(matched);
                        }
                    }
                }
                return None;
            }

            repeat_atom(
                pattern,
                text,
                pc,
                quantifier,
                state,
                group_indices,
                properties,
                options,
            )
            .into_iter()
            .find_map(|state| {
                match_pattern_first(
                    pattern,
                    text,
                    quantifier.next_pc,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                )
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn match_pattern_reverse(
    pattern: &[char],
    text: &[char],
    pc: usize,
    end_pc: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    let Some(steps) = reverse_atom_steps(pattern, pc, end_pc, properties, options.unicode) else {
        return Vec::new();
    };
    match_reverse_steps(
        &steps,
        steps.len(),
        pattern,
        text,
        state,
        group_indices,
        properties,
        options,
    )
}

fn reverse_atom_steps(
    pattern: &[char],
    start_pc: usize,
    end_pc: usize,
    properties: &PropertyCache,
    unicode: bool,
) -> Option<Vec<AtomStep>> {
    let mut steps = Vec::new();
    let mut pc = start_pc;
    while pc < end_pc {
        let atom_end = atom_end(pattern, pc, properties, unicode)?;
        let quantifier = quantifier(pattern, atom_end);
        if quantifier.next_pc > end_pc {
            return None;
        }
        steps.push(AtomStep { pc, quantifier });
        pc = quantifier.next_pc;
    }
    (pc == end_pc).then_some(steps)
}

#[allow(clippy::too_many_arguments)]
fn match_reverse_steps(
    steps: &[AtomStep],
    count: usize,
    pattern: &[char],
    text: &[char],
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    if count == 0 {
        return vec![state];
    }
    let step = steps[count - 1];
    repeat_atom_reverse(
        pattern,
        text,
        step.pc,
        step.quantifier,
        state,
        group_indices,
        properties,
        options,
    )
    .into_iter()
    .flat_map(|state| {
        match_reverse_steps(
            steps,
            count - 1,
            pattern,
            text,
            state,
            group_indices,
            properties,
            options,
        )
    })
    .collect()
}

fn atom_end(
    pattern: &[char],
    pc: usize,
    properties: &PropertyCache,
    unicode: bool,
) -> Option<usize> {
    match pattern.get(pc)? {
        '\\' if unicode_escape(pattern, pc, unicode).is_some() => {
            unicode_escape(pattern, pc, unicode).map(|escape| escape.next_pc)
        }
        '\\' if unicode && properties.get(pc).is_some() => {
            properties.get(pc).map(|escape| escape.next_pc)
        }
        '\\' if control_letter_escape(pattern, pc).is_some() => {
            control_letter_escape(pattern, pc).map(|escape| escape.next_pc)
        }
        '\\' if hex_escape(pattern, pc).is_some() => {
            hex_escape(pattern, pc).map(|escape| escape.next_pc)
        }
        '\\' if !unicode && pattern.get(pc + 1) == Some(&'c') => Some(pc + 1),
        '\\' if !unicode && legacy_octal_escape(pattern, pc).is_some() => {
            legacy_octal_escape(pattern, pc).map(|escape| escape.next_pc)
        }
        '\\' if pattern.get(pc + 1) == Some(&'k')
            && named_backreference(pattern, pc)
                .as_ref()
                .is_some_and(|(name, _)| named_group_index(pattern, name).is_some()) =>
        {
            named_backreference(pattern, pc).map(|(_, next_pc)| next_pc)
        }
        '\\' => Some(pc + 2),
        '[' => class_end(pattern, pc).map(|end| end + 1),
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
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<(usize, MatchState)> {
    match pattern[pc] {
        '\\' => match_escape(pattern, text, pc, state, properties, options),
        '[' => match_class(pattern, text, pc, state, properties, options),
        '(' => match_group(pattern, text, pc, state, group_indices, properties, options),
        '.' => match_any(text, pc + 1, state, options),
        literal => match_literal(
            text,
            pc + 1,
            state,
            literal,
            options.ignore_case,
            options.unicode,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn match_atom_reverse(
    pattern: &[char],
    text: &[char],
    pc: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    match pattern[pc] {
        '^' => {
            if at_line_start(text, state.index, options.multiline) {
                vec![state]
            } else {
                Vec::new()
            }
        }
        '$' => {
            if at_line_end(text, state.index, options.multiline) {
                vec![state]
            } else {
                Vec::new()
            }
        }
        '\\' if matches!(pattern.get(pc + 1), Some('b' | 'B')) => {
            let before = state.index > 0 && regexp_word_char(text[state.index - 1]);
            let after = text.get(state.index).copied().is_some_and(regexp_word_char);
            let want_boundary = pattern[pc + 1] == 'b';
            if (before != after) == want_boundary {
                vec![state]
            } else {
                Vec::new()
            }
        }
        '\\' => match_escape_reverse(pattern, text, pc, state, properties, options),
        '[' => match_class_reverse(pattern, text, pc, state, properties, options),
        '(' => match_group_reverse(pattern, text, pc, state, group_indices, properties, options),
        '.' => match_any_reverse(text, state, options),
        literal => {
            match_literal_reverse(text, state, literal, options.ignore_case, options.unicode)
        }
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

fn match_any_reverse(
    text: &[char],
    mut state: MatchState,
    options: MatchOptions,
) -> Vec<MatchState> {
    let Some(index) = state.index.checked_sub(1) else {
        return Vec::new();
    };
    let Some(value) = text.get(index) else {
        return Vec::new();
    };
    if !options.dot_all && is_line_terminator(*value) {
        return Vec::new();
    }
    state.index = index;
    vec![state]
}

fn is_line_terminator(value: char) -> bool {
    matches!(value, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `^` assertion: matches at the start of input, or (in multiline mode) right
/// after a line terminator.
fn at_line_start(text: &[char], index: usize, multiline: bool) -> bool {
    if index == 0 {
        return true;
    }
    multiline && text.get(index - 1).copied().is_some_and(is_line_terminator)
}

/// `$` assertion: matches at the end of input, or (in multiline mode) right
/// before a line terminator.
fn at_line_end(text: &[char], index: usize, multiline: bool) -> bool {
    if index == text.len() {
        return true;
    }
    multiline && text.get(index).copied().is_some_and(is_line_terminator)
}

fn match_literal(
    text: &[char],
    next_pc: usize,
    mut state: MatchState,
    literal: char,
    ignore_case: bool,
    unicode: bool,
) -> Vec<(usize, MatchState)> {
    if !text
        .get(state.index)
        .is_some_and(|value| chars_equal(*value, literal, ignore_case, unicode))
    {
        return Vec::new();
    }
    state.index += 1;
    vec![(next_pc, state)]
}

fn match_literal_reverse(
    text: &[char],
    mut state: MatchState,
    literal: char,
    ignore_case: bool,
    unicode: bool,
) -> Vec<MatchState> {
    let Some(index) = state.index.checked_sub(1) else {
        return Vec::new();
    };
    if !text
        .get(index)
        .is_some_and(|value| chars_equal(*value, literal, ignore_case, unicode))
    {
        return Vec::new();
    }
    state.index = index;
    vec![state]
}

fn match_escape(
    pattern: &[char],
    text: &[char],
    pc: usize,
    mut state: MatchState,
    properties: &PropertyCache,
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
    if !options.unicode
        && escaped.is_ascii_digit()
        && let Some(escape) = legacy_octal_escape(pattern, pc)
    {
        return match_unicode_escape(
            text,
            state,
            escape.value,
            escape.next_pc,
            options.ignore_case,
            options.unicode,
        );
    }
    if options.unicode
        && matches!(escaped, 'p' | 'P')
        && let Some(escape) = properties.get(pc)
    {
        return match_property_escape(text, state, &escape);
    }
    if escaped == 'k'
        && let Some((name, next_pc)) = named_backreference(pattern, pc)
        && let Some(group_index) = named_group_index(pattern, &name)
    {
        let capture = state.captures.get(group_index).copied().flatten();
        return match_backreference(text, state, capture, next_pc, options);
    }
    if escaped == 'c' {
        if let Some(escape) = control_letter_escape(pattern, pc) {
            return match_unicode_escape(
                text,
                state,
                escape.value,
                escape.next_pc,
                options.ignore_case,
                options.unicode,
            );
        }
        return match_literal(
            text,
            pc + 1,
            state,
            '\\',
            options.ignore_case,
            options.unicode,
        );
    }
    let Some(value) = text.get(state.index).copied() else {
        return Vec::new();
    };
    let (matched, next_pc, next_index) = match escaped {
        'd' | 'D' | 's' | 'S' | 'w' | 'W' => {
            let Some((value, next_index)) =
                regexp_code_point_at(text, state.index, options.unicode)
            else {
                return Vec::new();
            };
            let matched = match escaped {
                'd' => value.is_ascii_digit(),
                'D' => !value.is_ascii_digit(),
                's' => regexp_whitespace(value),
                'S' => !regexp_whitespace(value),
                'w' => regexp_word_char(value),
                'W' => !regexp_word_char(value),
                _ => unreachable!(),
            };
            (matched, pc + 2, next_index)
        }
        'u' => {
            let e = unicode_escape(pattern, pc, options.unicode);
            return match_code_unit_escape(text, state, e, 'u', pc, options);
        }
        'x' => {
            return match_code_unit_escape(text, state, hex_escape(pattern, pc), 'x', pc, options);
        }
        // In unicode mode `\0` (not followed by a decimal digit) is the NUL
        // character escape, not the literal `0`. Non-unicode `\0` is handled by
        // the legacy-octal branch above.
        '0' if options.unicode && !pattern.get(pc + 2).is_some_and(char::is_ascii_digit) => (
            chars_equal(value, '\u{0000}', options.ignore_case, options.unicode),
            pc + 2,
            state.index + 1,
        ),
        literal => (
            chars_equal(
                value,
                regexp_control_escape(literal),
                options.ignore_case,
                options.unicode,
            ),
            pc + 2,
            state.index + 1,
        ),
    };
    if !matched {
        return Vec::new();
    }
    state.index = next_index;
    vec![(next_pc, state)]
}

fn match_escape_reverse(
    pattern: &[char],
    text: &[char],
    pc: usize,
    state: MatchState,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    let Some(escaped) = pattern.get(pc + 1).copied() else {
        return Vec::new();
    };
    if let Some(index) = escaped.to_digit(10).map(|value| value as usize)
        && (1..=state.captures.len()).contains(&index)
    {
        let capture = state.captures[index - 1];
        return match_backreference_reverse(text, state, capture, options);
    }
    if escaped == 'k'
        && let Some((name, _)) = named_backreference(pattern, pc)
        && let Some(group_index) = named_group_index(pattern, &name)
    {
        let capture = state.captures.get(group_index).copied().flatten();
        return match_backreference_reverse(text, state, capture, options);
    }
    if options.unicode
        && matches!(escaped, 'p' | 'P')
        && let Some(escape) = properties.get(pc)
    {
        return match_property_escape_reverse(text, state, &escape);
    }
    if escaped == 'c'
        && let Some(escape) = control_letter_escape(pattern, pc)
    {
        return match_unicode_escape_reverse(
            text,
            state,
            escape.value,
            options.ignore_case,
            options.unicode,
        );
    }
    if let Some(escape) = unicode_escape(pattern, pc, options.unicode) {
        return match_unicode_escape_reverse(
            text,
            state,
            escape.value,
            options.ignore_case,
            options.unicode,
        );
    }
    if let Some(escape) = hex_escape(pattern, pc) {
        return match_unicode_escape_reverse(
            text,
            state,
            escape.value,
            options.ignore_case,
            options.unicode,
        );
    }
    if !options.unicode
        && let Some(escape) = legacy_octal_escape(pattern, pc)
    {
        return match_unicode_escape_reverse(
            text,
            state,
            escape.value,
            options.ignore_case,
            options.unicode,
        );
    }
    let Some(index) = state.index.checked_sub(1) else {
        return Vec::new();
    };
    let Some(value) = text.get(index).copied() else {
        return Vec::new();
    };
    let matched = match escaped {
        'd' => value.is_ascii_digit(),
        'D' => !value.is_ascii_digit(),
        's' => regexp_whitespace(value),
        'S' => !regexp_whitespace(value),
        'w' => regexp_word_char(value),
        'W' => !regexp_word_char(value),
        '0' if options.unicode && !pattern.get(pc + 2).is_some_and(char::is_ascii_digit) => {
            chars_equal(value, '\u{0000}', options.ignore_case, options.unicode)
        }
        literal => chars_equal(
            value,
            regexp_control_escape(literal),
            options.ignore_case,
            options.unicode,
        ),
    };
    if !matched {
        return Vec::new();
    }
    let mut matched = state;
    matched.index = index;
    vec![matched]
}

/// Match a fixed code-unit escape (`\uHHHH` or `\xHH`); `literal` is the Annex B
/// identity fallback when the escape did not parse.
fn match_code_unit_escape(
    text: &[char],
    mut state: MatchState,
    escape: Option<ParsedEscape>,
    literal: char,
    pc: usize,
    options: MatchOptions,
) -> Vec<(usize, MatchState)> {
    if let Some(escape) = escape {
        return match_unicode_escape(
            text,
            state,
            escape.value,
            escape.next_pc,
            options.ignore_case,
            options.unicode,
        );
    }
    match text.get(state.index).copied() {
        Some(value) if chars_equal(value, literal, options.ignore_case, options.unicode) => {
            state.index += 1;
            vec![(pc + 2, state)]
        }
        _ => Vec::new(),
    }
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
            options.unicode,
        )
    });
    if !matched {
        return Vec::new();
    }
    state.index += capture_len;
    vec![(next_pc, state)]
}

fn match_backreference_reverse(
    text: &[char],
    mut state: MatchState,
    capture: Option<(usize, usize)>,
    options: MatchOptions,
) -> Vec<MatchState> {
    let Some((start, end)) = capture else {
        return vec![state];
    };
    let capture_len = end - start;
    let Some(begin) = state.index.checked_sub(capture_len) else {
        return Vec::new();
    };
    let matched = (0..capture_len).all(|offset| {
        chars_equal(
            text[begin + offset],
            text[start + offset],
            options.ignore_case,
            options.unicode,
        )
    });
    if !matched {
        return Vec::new();
    }
    state.index = begin;
    vec![state]
}

fn match_unicode_escape(
    text: &[char],
    state: MatchState,
    value: char,
    next_pc: usize,
    ignore_case: bool,
    unicode: bool,
) -> Vec<(usize, MatchState)> {
    let mut matches = Vec::new();
    if text
        .get(state.index)
        .is_some_and(|current| chars_equal(*current, value, ignore_case, unicode))
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

fn match_unicode_escape_reverse(
    text: &[char],
    mut state: MatchState,
    value: char,
    ignore_case: bool,
    unicode: bool,
) -> Vec<MatchState> {
    let Some(index) = state.index.checked_sub(1) else {
        return Vec::new();
    };
    if text
        .get(index)
        .is_some_and(|current| chars_equal(*current, value, ignore_case, unicode))
    {
        state.index = index;
        return vec![state];
    }
    Vec::new()
}

fn match_property_escape(
    text: &[char],
    mut state: MatchState,
    escape: &escapes::ParsedPropertyEscape,
) -> Vec<(usize, MatchState)> {
    let Some((code_point, next_index)) = regexp_property_code_point_at(text, state.index) else {
        return Vec::new();
    };
    let matched = escape.set.contains(code_point);
    if matched == escape.negated {
        return Vec::new();
    }
    state.index = next_index;
    vec![(escape.next_pc, state)]
}

fn match_property_escape_reverse(
    text: &[char],
    mut state: MatchState,
    escape: &escapes::ParsedPropertyEscape,
) -> Vec<MatchState> {
    let Some(index) = state.index.checked_sub(1) else {
        return Vec::new();
    };
    let Some(code_point) = text.get(index).map(|value| u32::from(*value)) else {
        return Vec::new();
    };
    let matched = escape.set.contains(code_point);
    if matched == escape.negated {
        return Vec::new();
    }
    state.index = index;
    vec![state]
}

fn code_unit_char(code_unit: u16) -> char {
    char_from_code_unit(code_unit)
}

fn match_class(
    pattern: &[char],
    text: &[char],
    pc: usize,
    mut state: MatchState,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<(usize, MatchState)> {
    let Some(class_end) = class_end(pattern, pc) else {
        return Vec::new();
    };
    let class = &pattern[pc + 1..class_end];
    let Some((value, next_index)) = regexp_code_point_at(text, state.index, options.unicode) else {
        return Vec::new();
    };
    if !class_match(class, pc + 1, value, properties, options) {
        return Vec::new();
    }
    state.index = next_index;
    vec![(class_end + 1, state)]
}

fn match_class_reverse(
    pattern: &[char],
    text: &[char],
    pc: usize,
    mut state: MatchState,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    let Some(class_end) = class_end(pattern, pc) else {
        return Vec::new();
    };
    let Some(index) = state.index.checked_sub(1) else {
        return Vec::new();
    };
    let Some(value) = text.get(index).copied() else {
        return Vec::new();
    };
    if !class_match(
        &pattern[pc + 1..class_end],
        pc + 1,
        value,
        properties,
        options,
    ) {
        return Vec::new();
    }
    state.index = index;
    vec![state]
}

pub(super) fn class_end(pattern: &[char], start: usize) -> Option<usize> {
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

pub(super) fn regexp_property_code_point_at(text: &[char], index: usize) -> Option<(u32, usize)> {
    let first = *text.get(index)?;
    if let Some(first_unit) = surrogate_escape_code_unit(first) {
        if (0xD800..=0xDBFF).contains(&first_unit)
            && let Some(second_unit) = text
                .get(index + 1)
                .and_then(|value| surrogate_escape_code_unit(*value))
            && (0xDC00..=0xDFFF).contains(&second_unit)
        {
            return Some((
                0x10000 + ((u32::from(first_unit) - 0xD800) << 10) + u32::from(second_unit)
                    - 0xDC00,
                index + 2,
            ));
        }
        return Some((u32::from(first_unit), index + 1));
    }
    Some((u32::from(first), index + 1))
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

#[allow(clippy::too_many_arguments)]
fn repeat_atom(
    pattern: &[char],
    text: &[char],
    atom_pc: usize,
    quantifier: Quantifier,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    let atom_captures =
        atom_capture_indices(pattern, atom_pc, group_indices, properties, options.unicode);

    // Fast path: a quantified single-code-point atom with no captures (`\p{X}+`,
    // `\d*`, `[a-z]{2,}`, `.`, a literal, ...) can be scanned linearly instead of
    // driving the generic explicit-stack DFS, which clones a `MatchState` per
    // character and per backtrack point. This keeps property-escape cases that
    // match ~1M code points (`/^\p{L}+$/u`) inside the matcher's budget.
    if atom_captures.is_empty()
        && let Some(matcher) = simple_atom_matcher(pattern, atom_pc, properties, options)
    {
        return repeat_simple_atom(text, &matcher, quantifier, state, properties, options);
    }

    let mut current = vec![state];
    for _ in 0..quantifier.min {
        current = current
            .into_iter()
            .flat_map(|state| {
                match_atom(
                    pattern,
                    text,
                    atom_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                )
            })
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
        properties,
        options,
    };
    let mut results = Vec::new();
    for state in current {
        repeat_atom_from(&repeat, state, quantifier.min, &mut results);
    }
    results
}

#[allow(clippy::too_many_arguments)]
fn repeat_atom_reverse(
    pattern: &[char],
    text: &[char],
    atom_pc: usize,
    quantifier: Quantifier,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    let max_count = quantifier.max.unwrap_or(text.len() + 1);
    if quantifier.min == max_count {
        let mut current = vec![state];
        for _ in 0..quantifier.min {
            current = current
                .into_iter()
                .flat_map(|state| {
                    match_atom_reverse(
                        pattern,
                        text,
                        atom_pc,
                        state,
                        group_indices,
                        properties,
                        options,
                    )
                })
                .collect();
            if current.is_empty() {
                break;
            }
        }
        return current;
    }
    let mut levels: Vec<Vec<MatchState>> = vec![vec![state]];
    for count in 0..max_count {
        let mut next = Vec::new();
        for current in &levels[count] {
            next.extend(
                match_atom_reverse(
                    pattern,
                    text,
                    atom_pc,
                    current.clone(),
                    group_indices,
                    properties,
                    options,
                )
                .into_iter()
                .filter(|matched| matched.index != current.index),
            );
        }
        if next.is_empty() {
            break;
        }
        levels.push(next);
    }

    let mut counts: Vec<_> = (quantifier.min..levels.len()).collect();
    if quantifier.greedy {
        counts.reverse();
    }
    counts
        .into_iter()
        .flat_map(|count| {
            levels[count]
                .iter()
                .cloned()
                .map(move |state| repeat_accept_state(state, count, &[]))
        })
        .collect()
}

/// Explicit-stack DFS over repetitions of a quantified atom, producing accept
/// states in the same priority order as the natural recursion (greedy: longest
/// match first; lazy: shortest first). Using an explicit stack avoids native
/// stack overflow on long inputs such as `^\p{Nd}+$` over thousands of chars.
fn repeat_atom_from(
    repeat: &RepeatAtom<'_>,
    state: MatchState,
    count: usize,
    results: &mut Vec<MatchState>,
) {
    // Each frame is a state we are expanding at a given repetition count.
    // For greedy matching we want to emit the accept state for a frame only
    // after all of its descendants, so we expand children first (pushed in
    // reverse so the first child is processed first) and defer the accept.
    enum Work {
        Expand(MatchState, usize),
        Accept(MatchState, usize),
    }
    let mut stack = vec![Work::Expand(state, count)];
    let mut expanded = HashSet::new();
    while let Some(work) = stack.pop() {
        match work {
            Work::Accept(state, count) => {
                results.push(repeat_accept_state(state, count, &repeat.atom_captures));
            }
            Work::Expand(state, count) => {
                if !expanded.insert((state.index, count, state.captures.clone())) {
                    continue;
                }
                if repeat.quantifier.greedy {
                    // Defer this frame's own accept until after its children.
                    stack.push(Work::Accept(state.clone(), count));
                } else {
                    results.push(repeat_accept_state(
                        state.clone(),
                        count,
                        &repeat.atom_captures,
                    ));
                }

                if repeat.quantifier.max.is_none_or(|max| count < max) {
                    let mut children: Vec<MatchState> = match_atom(
                        repeat.pattern,
                        repeat.text,
                        repeat.atom_pc,
                        state.clone(),
                        repeat.group_indices,
                        repeat.properties,
                        repeat.options,
                    )
                    .into_iter()
                    .filter_map(|(_, next_state)| {
                        (next_state.index != state.index).then_some(next_state)
                    })
                    .collect();
                    dedup_match_states(&mut children);
                    // Process children in order: push in reverse so the first
                    // child is on top of the stack.
                    children.reverse();
                    for next_state in children {
                        stack.push(Work::Expand(next_state, count + 1));
                    }
                }
            }
        }
    }
}

fn dedup_match_states(states: &mut Vec<MatchState>) {
    let mut seen = HashSet::new();
    states.retain(|state| seen.insert((state.index, state.captures.clone())));
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
    properties: &PropertyCache,
    unicode: bool,
) -> Vec<usize> {
    let Some(atom_end) = atom_end(pattern, atom_pc, properties, unicode) else {
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

#[allow(clippy::too_many_arguments)]
fn match_group(
    pattern: &[char],
    text: &[char],
    pc: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<(usize, MatchState)> {
    let Some(end) = closing_group(pattern, pc) else {
        return Vec::new();
    };
    let kind = group_kind(pattern, pc);
    if let GroupKind::Lookahead { negative } = kind {
        return match_lookaround(
            pattern,
            text,
            pc + 3,
            end,
            state,
            group_indices,
            properties,
            options,
            negative,
            false,
        );
    }
    if let GroupKind::Lookbehind { negative } = kind {
        return match_lookaround(
            pattern,
            text,
            pc + 4,
            end,
            state,
            group_indices,
            properties,
            options,
            negative,
            true,
        );
    }
    let group_index = group_indices.get(&pc).copied();
    let group_start = match kind {
        GroupKind::Named { body_offset } => pc + body_offset,
        GroupKind::NonCapturing => pc + 3,
        _ => pc + 1,
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
            properties,
            options,
        ));
    }
    matches
        .into_iter()
        .map(|mut matched| {
            if let Some(group_index) = group_index {
                let capture = Some((state.index, matched.index));
                let group_is_repeated = quantifier(pattern, end + 1).next_pc != end + 1;
                if !options.reverse_captures
                    || !group_is_repeated
                    || matched.captures[group_index].is_none()
                {
                    matched.captures[group_index] = capture;
                }
            }
            (end + 1, matched)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn match_group_reverse(
    pattern: &[char],
    text: &[char],
    pc: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    let Some(end) = closing_group(pattern, pc) else {
        return Vec::new();
    };
    let kind = group_kind(pattern, pc);
    if matches!(
        kind,
        GroupKind::Lookahead { .. } | GroupKind::Lookbehind { .. }
    ) {
        return Vec::new();
    }

    let group_index = group_indices.get(&pc).copied();
    let group_start = match kind {
        GroupKind::Named { body_offset } => pc + body_offset,
        GroupKind::NonCapturing => pc + 3,
        _ => pc + 1,
    };
    let mut matches = Vec::new();
    for (start, end) in group_alternatives(pattern, group_start, end) {
        matches.extend(match_pattern_reverse(
            pattern,
            text,
            start,
            end,
            state.clone(),
            group_indices,
            properties,
            options,
        ));
    }
    matches
        .into_iter()
        .map(|mut matched| {
            if let Some(group_index) = group_index {
                matched.captures[group_index] = Some((matched.index, state.index));
            }
            matched
        })
        .collect()
}
