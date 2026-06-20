//! Lookahead and lookbehind assertion matching.
//!
//! Both forms are zero-width: on success the match index is left unchanged.
//! Positive assertions keep any captures their body established; negative
//! assertions capture nothing and succeed exactly when the body fails.

use std::collections::HashMap;

use super::groups::group_alternatives;
use super::{MatchOptions, MatchState, PropertyCache, match_pattern};

/// Match a lookahead (`behind = false`) or lookbehind (`behind = true`)
/// assertion. The assertion is zero-width: on success the index is unchanged.
#[allow(clippy::too_many_arguments)]
pub(super) fn match_lookaround(
    pattern: &[char],
    text: &[char],
    body_start: usize,
    body_end: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
    negative: bool,
    behind: bool,
) -> Vec<(usize, MatchState)> {
    let close_pc = body_end + 1;
    let inner = if behind {
        match_lookbehind_body(
            pattern,
            text,
            body_start,
            body_end,
            state.clone(),
            group_indices,
            properties,
            options,
        )
    } else {
        let mut results = Vec::new();
        for (start, end) in group_alternatives(pattern, body_start, body_end) {
            results.extend(match_pattern(
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
        results
    };

    if negative {
        // Negative assertions succeed when the body fails to match, and they
        // never capture (captures inside are reset to None on success).
        if inner.is_empty() {
            vec![(close_pc, state)]
        } else {
            Vec::new()
        }
    } else {
        // Positive assertions keep the captures established by the body but
        // restore the index to its position before the assertion.
        let inner: Vec<_> = inner.into_iter().take(1).collect();
        inner
            .into_iter()
            .map(|mut matched| {
                matched.index = state.index;
                (close_pc, matched)
            })
            .collect()
    }
}

/// Match the body of a lookbehind ending at `state.index`. The forward matcher
/// still probes possible body starts from left to right, but capture writes
/// inside the body use lookbehind's reverse-direction priority.
#[allow(clippy::too_many_arguments)]
fn match_lookbehind_body(
    pattern: &[char],
    text: &[char],
    body_start: usize,
    body_end: usize,
    state: MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    let target = state.index;
    let options = MatchOptions {
        reverse_captures: true,
        ..options
    };
    let mut results = Vec::new();
    if !has_body_quantifier(pattern, body_start, body_end) {
        for (start, end) in group_alternatives(pattern, body_start, body_end) {
            for begin in (0..=target).rev() {
                let probe = MatchState {
                    index: begin,
                    captures: state.captures.clone(),
                };
                for matched in match_pattern(
                    pattern,
                    text,
                    start,
                    end,
                    probe,
                    group_indices,
                    properties,
                    options,
                ) {
                    if matched.index == target {
                        results.push(matched);
                    }
                }
            }
        }
        return results;
    }
    for begin in 0..=target {
        for (start, end) in group_alternatives(pattern, body_start, body_end) {
            let probe = MatchState {
                index: begin,
                captures: state.captures.clone(),
            };
            for matched in match_pattern(
                pattern,
                text,
                start,
                end,
                probe,
                group_indices,
                properties,
                options,
            ) {
                if matched.index == target {
                    results.push(matched);
                }
            }
        }
    }
    results
}

fn has_body_quantifier(pattern: &[char], start: usize, end: usize) -> bool {
    let mut escaped = false;
    let mut in_class = false;
    for index in start..end {
        let char = pattern[index];
        if escaped {
            escaped = false;
        } else if char == '\\' {
            escaped = true;
        } else if char == '[' {
            in_class = true;
        } else if char == ']' {
            in_class = false;
        } else if !in_class
            && (matches!(char, '*' | '+' | '{')
                || (char == '?' && index > start && pattern[index - 1] != '('))
        {
            return true;
        }
    }
    false
}
