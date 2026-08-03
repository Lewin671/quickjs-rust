//! Failure-atomic first-match traversal.
//!
//! Top-level RegExp execution only observes the first successful state. This
//! path therefore mutates one state in place and restores it before returning
//! failure. Capture-bearing compound atoms still bridge to the all-state
//! matcher for now; later migration stages replace that bridge with choice
//! points and a capture undo trail.

use std::collections::HashMap;

use super::escapes::PropertyCache;
use super::fast_scan::{simple_atom_boundaries, simple_atom_matcher};
use super::{
    MatchOptions, MatchState, at_line_end, at_line_start, atom_capture_indices, atom_end,
    quantifier, regexp_word_char, repeat_atom,
};

/// Find the first match in ECMAScript backtracking priority order.
///
/// Returning `false` leaves `state` exactly as it was on entry. The invariant
/// lets callers reuse one state across candidate boundaries and alternatives
/// without cloning its capture vector at every choice point.
#[allow(clippy::too_many_arguments)]
pub(super) fn match_pattern_first(
    pattern: &[char],
    text: &[char],
    pc: usize,
    end_pc: usize,
    state: &mut MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> bool {
    if pc == end_pc {
        return true;
    }
    match pattern[pc] {
        '^' => {
            at_line_start(text, state.index, options.multiline)
                && match_pattern_first(
                    pattern,
                    text,
                    pc + 1,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                )
        }
        '$' => {
            at_line_end(text, state.index, options.multiline)
                && match_pattern_first(
                    pattern,
                    text,
                    pc + 1,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                )
        }
        '\\' if matches!(pattern.get(pc + 1), Some('b' | 'B')) => {
            let before = state.index > 0 && regexp_word_char(text[state.index - 1]);
            let after = text.get(state.index).copied().is_some_and(regexp_word_char);
            let want_boundary = pattern[pc + 1] == 'b';
            (before != after) == want_boundary
                && match_pattern_first(
                    pattern,
                    text,
                    pc + 2,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                )
        }
        _ => match_atom_and_continuation(
            pattern,
            text,
            pc,
            end_pc,
            state,
            group_indices,
            properties,
            options,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn match_atom_and_continuation(
    pattern: &[char],
    text: &[char],
    pc: usize,
    end_pc: usize,
    state: &mut MatchState,
    group_indices: &HashMap<usize, usize>,
    properties: &PropertyCache,
    options: MatchOptions,
) -> bool {
    let Some(atom_end) = atom_end(pattern, pc, properties, options.unicode) else {
        return false;
    };
    let quantifier = quantifier(pattern, atom_end);
    let atom_captures =
        atom_capture_indices(pattern, pc, group_indices, properties, options.unicode);
    if atom_captures.is_empty()
        && let Some(matcher) = simple_atom_matcher(pattern, pc, properties, options)
    {
        let entry_index = state.index;
        if quantifier.is_exactly_one() {
            let Some(next_index) = matcher.step(text, entry_index, properties, options) else {
                return false;
            };
            state.index = next_index;
            if match_pattern_first(
                pattern,
                text,
                quantifier.next_pc,
                end_pc,
                state,
                group_indices,
                properties,
                options,
            ) {
                return true;
            }
            state.index = entry_index;
            return false;
        }

        let Some(boundaries) =
            simple_atom_boundaries(text, &matcher, quantifier, entry_index, properties, options)
        else {
            return false;
        };
        let lowest = quantifier.min;
        let highest = boundaries.len() - 1;
        if quantifier.greedy {
            for count in (lowest..=highest).rev() {
                state.index = boundaries[count];
                if match_pattern_first(
                    pattern,
                    text,
                    quantifier.next_pc,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                ) {
                    return true;
                }
            }
        } else {
            for boundary in &boundaries[lowest..=highest] {
                state.index = *boundary;
                if match_pattern_first(
                    pattern,
                    text,
                    quantifier.next_pc,
                    end_pc,
                    state,
                    group_indices,
                    properties,
                    options,
                ) {
                    return true;
                }
            }
        }
        state.index = entry_index;
        return false;
    }

    // The migration bridge owns every candidate state, so a failed recursive
    // continuation cannot mutate the caller. A later stage replaces these
    // state graphs with choice points plus capture undo checkpoints.
    for mut candidate in repeat_atom(
        pattern,
        text,
        pc,
        quantifier,
        state.clone(),
        group_indices,
        properties,
        options,
    ) {
        if match_pattern_first(
            pattern,
            text,
            quantifier.next_pc,
            end_pc,
            &mut candidate,
            group_indices,
            properties,
            options,
        ) {
            *state = candidate;
            return true;
        }
    }
    false
}
