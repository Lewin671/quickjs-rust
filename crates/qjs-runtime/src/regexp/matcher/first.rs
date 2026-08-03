//! Failure-atomic first-match traversal.
//!
//! Top-level RegExp execution only observes the first successful state. This
//! path therefore mutates one state in place and restores it before returning
//! failure. Ordinary exact-once groups and lookahead assertions use a capture
//! undo journal; quantified compound atoms and lookbehind still bridge to the
//! all-state matcher until the explicit choice-stack stages replace them.

use std::collections::HashMap;

use super::escapes::PropertyCache;
use super::fast_scan::{simple_atom_boundaries, simple_atom_matcher};
use super::groups::{GroupKind, closing_group, group_alternatives, group_kind};
use super::{
    MatchOptions, MatchState, RepeatScratch, at_line_end, at_line_start, atom_capture_indices,
    atom_end, match_repeated_atom_first, quantifier, regexp_word_char,
};

type Capture = Option<(usize, usize)>;

#[derive(Clone, Copy)]
struct CaptureEnd {
    slot: usize,
    start: usize,
}

#[derive(Clone, Copy)]
struct Continuation {
    parent: Option<usize>,
    pc: usize,
    end_pc: usize,
    capture: Option<CaptureEnd>,
    reject_empty_from: Option<usize>,
}

struct CaptureUndo {
    slot: usize,
    previous: Capture,
}

pub(super) struct FirstMatcher<'a> {
    pattern: &'a [char],
    text: &'a [char],
    group_indices: &'a HashMap<usize, usize>,
    properties: &'a PropertyCache,
    options: MatchOptions,
    continuations: Vec<Continuation>,
    capture_undo: Vec<CaptureUndo>,
    repeat_scratch: RepeatScratch,
}

impl<'a> FirstMatcher<'a> {
    pub(super) fn new(
        pattern: &'a [char],
        text: &'a [char],
        group_indices: &'a HashMap<usize, usize>,
        properties: &'a PropertyCache,
        options: MatchOptions,
    ) -> Self {
        Self {
            pattern,
            text,
            group_indices,
            properties,
            options,
            continuations: Vec::new(),
            capture_undo: Vec::new(),
            repeat_scratch: RepeatScratch::default(),
        }
    }

    /// Find the first match in ECMAScript backtracking priority order.
    ///
    /// Returning `false` leaves `state` exactly as it was on entry and both
    /// scratch journals empty. The invariant lets one matcher reuse its
    /// allocated storage across candidate boundaries and alternatives.
    pub(super) fn match_pattern_first(
        &mut self,
        pc: usize,
        end_pc: usize,
        state: &mut MatchState,
    ) -> bool {
        debug_assert!(self.continuations.is_empty());
        debug_assert!(self.capture_undo.is_empty());
        debug_assert!(self.repeat_scratch.is_empty());
        let matched = self.match_pattern(pc, end_pc, state, None);
        debug_assert!(matched || self.continuations.is_empty());
        debug_assert!(matched || self.capture_undo.is_empty());
        debug_assert!(self.repeat_scratch.is_empty());
        matched
    }

    /// Every recursive entry is a rollback boundary. Continuation frames are
    /// append-only so a nested attempt cannot overwrite a parent frame that a
    /// later alternative still needs.
    fn match_pattern(
        &mut self,
        pc: usize,
        end_pc: usize,
        state: &mut MatchState,
        continuation: Option<usize>,
    ) -> bool {
        let entry_index = state.index;
        let undo_checkpoint = self.capture_undo.len();
        let continuation_checkpoint = self.continuations.len();
        if self.match_pattern_inner(pc, end_pc, state, continuation) {
            return true;
        }
        state.index = entry_index;
        self.rollback_captures(state, undo_checkpoint);
        self.continuations.truncate(continuation_checkpoint);
        false
    }

    fn match_pattern_inner(
        &mut self,
        pc: usize,
        end_pc: usize,
        state: &mut MatchState,
        continuation: Option<usize>,
    ) -> bool {
        if pc == end_pc {
            let Some(continuation) = continuation else {
                return true;
            };
            let continuation = self.continuations[continuation];
            if continuation.reject_empty_from == Some(state.index) {
                return false;
            }
            if let Some(capture) = continuation.capture {
                self.write_capture(state, capture.slot, Some((capture.start, state.index)));
            }
            return self.match_pattern(
                continuation.pc,
                continuation.end_pc,
                state,
                continuation.parent,
            );
        }
        match self.pattern[pc] {
            '^' => {
                at_line_start(self.text, state.index, self.options.multiline)
                    && self.match_pattern(pc + 1, end_pc, state, continuation)
            }
            '$' => {
                at_line_end(self.text, state.index, self.options.multiline)
                    && self.match_pattern(pc + 1, end_pc, state, continuation)
            }
            '\\' if matches!(self.pattern.get(pc + 1), Some('b' | 'B')) => {
                let before = state.index > 0 && regexp_word_char(self.text[state.index - 1]);
                let after = self
                    .text
                    .get(state.index)
                    .copied()
                    .is_some_and(regexp_word_char);
                let want_boundary = self.pattern[pc + 1] == 'b';
                (before != after) == want_boundary
                    && self.match_pattern(pc + 2, end_pc, state, continuation)
            }
            _ => self.match_atom_and_continuation(pc, end_pc, state, continuation),
        }
    }

    fn match_atom_and_continuation(
        &mut self,
        pc: usize,
        end_pc: usize,
        state: &mut MatchState,
        continuation: Option<usize>,
    ) -> bool {
        let Some(atom_end) = atom_end(self.pattern, pc, self.properties, self.options.unicode)
        else {
            return false;
        };
        let quantifier = quantifier(self.pattern, atom_end);
        if let Some(matcher) = simple_atom_matcher(self.pattern, pc, self.properties, self.options)
        {
            if quantifier.is_exactly_one() {
                let Some(next_index) =
                    matcher.step(self.text, state.index, self.properties, self.options)
                else {
                    return false;
                };
                state.index = next_index;
                return self.match_pattern(quantifier.next_pc, end_pc, state, continuation);
            }

            let entry_index = state.index;
            let Some(boundaries) = simple_atom_boundaries(
                self.text,
                &matcher,
                quantifier,
                entry_index,
                self.properties,
                self.options,
            ) else {
                return false;
            };
            let lowest = quantifier.min;
            let highest = boundaries.len() - 1;
            if quantifier.greedy {
                for count in (lowest..=highest).rev() {
                    state.index = boundaries[count];
                    if self.match_pattern(quantifier.next_pc, end_pc, state, continuation) {
                        return true;
                    }
                }
            } else {
                for boundary in &boundaries[lowest..=highest] {
                    state.index = *boundary;
                    if self.match_pattern(quantifier.next_pc, end_pc, state, continuation) {
                        return true;
                    }
                }
            }
            return false;
        }

        if self.pattern[pc] == '('
            && quantifier.max == Some(1)
            && quantifier.min <= 1
            && let Some(matched) = self.match_group(pc, quantifier, end_pc, state, continuation)
        {
            return matched;
        }

        // The migration bridge still owns each candidate state, so a failed
        // continuation cannot mutate the caller. It now streams ordered
        // choices and reuses its work storage rather than building every
        // result before the first continuation attempt.
        let pattern = self.pattern;
        let text = self.text;
        let group_indices = self.group_indices;
        let properties = self.properties;
        let options = self.options;
        let mut scratch = std::mem::take(&mut self.repeat_scratch);
        let matched = match_repeated_atom_first(
            pattern,
            text,
            pc,
            quantifier,
            state.clone(),
            group_indices,
            properties,
            options,
            &mut scratch,
            |candidate| self.match_pattern(quantifier.next_pc, end_pc, candidate, continuation),
        );
        self.repeat_scratch = scratch;
        if let Some(candidate) = matched {
            *state = candidate;
            return true;
        }
        false
    }

    fn match_group(
        &mut self,
        pc: usize,
        quantifier: super::Quantifier,
        end_pc: usize,
        state: &mut MatchState,
        continuation: Option<usize>,
    ) -> Option<bool> {
        if matches!(group_kind(self.pattern, pc), GroupKind::Lookbehind { .. }) {
            return None;
        }
        if quantifier.is_exactly_one() {
            return self.match_group_once(
                pc,
                quantifier.next_pc,
                end_pc,
                state,
                continuation,
                false,
            );
        }

        let captures = atom_capture_indices(
            self.pattern,
            pc,
            self.group_indices,
            self.properties,
            self.options.unicode,
        );
        if quantifier.greedy {
            if self.match_group_once(
                pc,
                quantifier.next_pc,
                end_pc,
                state,
                continuation,
                quantifier.min == 0,
            )? {
                return Some(true);
            }
            return Some(self.match_group_zero(
                &captures,
                quantifier.next_pc,
                end_pc,
                state,
                continuation,
            ));
        }
        if self.match_group_zero(&captures, quantifier.next_pc, end_pc, state, continuation) {
            return Some(true);
        }
        self.match_group_once(
            pc,
            quantifier.next_pc,
            end_pc,
            state,
            continuation,
            quantifier.min == 0,
        )
    }

    fn match_group_zero(
        &mut self,
        captures: &[usize],
        next_pc: usize,
        end_pc: usize,
        state: &mut MatchState,
        continuation: Option<usize>,
    ) -> bool {
        let entry_index = state.index;
        let undo_checkpoint = self.capture_undo.len();
        let continuation_checkpoint = self.continuations.len();
        for capture in captures {
            self.write_capture(state, *capture, None);
        }
        if self.match_pattern(next_pc, end_pc, state, continuation) {
            return true;
        }
        state.index = entry_index;
        self.rollback_captures(state, undo_checkpoint);
        self.continuations.truncate(continuation_checkpoint);
        false
    }

    /// Match an ordinary group or lookahead without materializing its result
    /// states. Lookbehind retains the reverse matcher until that path can move
    /// as one semantic unit.
    fn match_group_once(
        &mut self,
        pc: usize,
        next_pc: usize,
        end_pc: usize,
        state: &mut MatchState,
        continuation: Option<usize>,
        reject_empty: bool,
    ) -> Option<bool> {
        let end = closing_group(self.pattern, pc)?;
        let kind = group_kind(self.pattern, pc);
        if let GroupKind::Lookahead { negative } = kind {
            return Some(self.match_lookahead(
                pc + 3,
                end,
                next_pc,
                end_pc,
                state,
                continuation,
                negative,
            ));
        }
        if matches!(kind, GroupKind::Lookbehind { .. }) {
            return None;
        }

        let capture = self.group_indices.get(&pc).copied().map(|slot| CaptureEnd {
            slot,
            start: state.index,
        });
        let group_start = match kind {
            GroupKind::Named { body_offset } => pc + body_offset,
            GroupKind::NonCapturing => pc + 3,
            _ => pc + 1,
        };
        for (alternative_start, alternative_end) in
            group_alternatives(self.pattern, group_start, end)
        {
            let continuation_id = self.continuations.len();
            self.continuations.push(Continuation {
                parent: continuation,
                pc: next_pc,
                end_pc,
                capture,
                reject_empty_from: reject_empty.then_some(state.index),
            });
            if self.match_pattern(
                alternative_start,
                alternative_end,
                state,
                Some(continuation_id),
            ) {
                return Some(true);
            }
            self.continuations.truncate(continuation_id);
        }
        Some(false)
    }

    #[allow(clippy::too_many_arguments)]
    fn match_lookahead(
        &mut self,
        body_start: usize,
        body_end: usize,
        next_pc: usize,
        end_pc: usize,
        state: &mut MatchState,
        continuation: Option<usize>,
        negative: bool,
    ) -> bool {
        let assertion_index = state.index;
        let undo_checkpoint = self.capture_undo.len();
        let continuation_checkpoint = self.continuations.len();
        let mut matched = false;
        for (alternative_start, alternative_end) in
            group_alternatives(self.pattern, body_start, body_end)
        {
            if self.match_pattern(alternative_start, alternative_end, state, None) {
                matched = true;
                break;
            }
        }
        self.continuations.truncate(continuation_checkpoint);

        if negative {
            state.index = assertion_index;
            self.rollback_captures(state, undo_checkpoint);
            return !matched && self.match_pattern(next_pc, end_pc, state, continuation);
        }
        if !matched {
            return false;
        }
        // A positive lookahead is atomic: retain the captures from its first
        // body match, restore its zero-width index, and do not revisit the body
        // if the outer continuation later fails.
        state.index = assertion_index;
        self.match_pattern(next_pc, end_pc, state, continuation)
    }

    fn write_capture(&mut self, state: &mut MatchState, slot: usize, value: Capture) {
        if state.captures[slot] == value {
            return;
        }
        self.capture_undo.push(CaptureUndo {
            slot,
            previous: state.captures[slot],
        });
        state.captures[slot] = value;
    }

    fn rollback_captures(&mut self, state: &mut MatchState, checkpoint: usize) {
        while self.capture_undo.len() > checkpoint {
            let undo = self.capture_undo.pop().expect("undo length checked");
            state.captures[undo.slot] = undo.previous;
        }
    }
}
