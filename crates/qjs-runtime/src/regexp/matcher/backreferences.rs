//! Backreference matching helpers whose cursor width depends on RegExp mode.

use super::{
    chars_equal, is_trailing_surrogate_position, regexp_code_point_at, regexp_code_point_before,
};

/// Compare a captured Unicode-mode sequence with the input at `candidate`.
///
/// Captures and candidates can contain the same ECMAScript code point in two
/// internal forms: one Rust scalar or a canonical surrogate-sentinel pair. A
/// code-point cursor makes those forms equivalent and ensures neither the
/// capture nor the match can end between the two sentinels of a pair.
pub(super) fn match_unicode_backreference_forward(
    text: &[char],
    capture_start: usize,
    capture_end: usize,
    candidate: usize,
    ignore_case: bool,
) -> Option<usize> {
    if is_trailing_surrogate_position(text, capture_start)
        || is_trailing_surrogate_position(text, capture_end)
        || is_trailing_surrogate_position(text, candidate)
    {
        return None;
    }
    let mut capture_index = capture_start;
    let mut candidate_index = candidate;
    while capture_index < capture_end {
        let (captured, next_capture) = regexp_code_point_at(text, capture_index, true)?;
        if next_capture > capture_end {
            return None;
        }
        let (current, next_candidate) = regexp_code_point_at(text, candidate_index, true)?;
        if !chars_equal(current, captured, ignore_case, true) {
            return None;
        }
        capture_index = next_capture;
        candidate_index = next_candidate;
    }
    (capture_index == capture_end && !is_trailing_surrogate_position(text, candidate_index))
        .then_some(candidate_index)
}

/// Reverse-direction counterpart used by lookbehind matching.
pub(super) fn match_unicode_backreference_reverse(
    text: &[char],
    capture_start: usize,
    capture_end: usize,
    candidate_end: usize,
    ignore_case: bool,
) -> Option<usize> {
    if is_trailing_surrogate_position(text, capture_start)
        || is_trailing_surrogate_position(text, capture_end)
        || is_trailing_surrogate_position(text, candidate_end)
    {
        return None;
    }
    let mut capture_index = capture_end;
    let mut candidate_index = candidate_end;
    while capture_index > capture_start {
        let (captured, previous_capture) = regexp_code_point_before(text, capture_index, true)?;
        if previous_capture < capture_start {
            return None;
        }
        let (current, previous_candidate) = regexp_code_point_before(text, candidate_index, true)?;
        if !chars_equal(current, captured, ignore_case, true) {
            return None;
        }
        capture_index = previous_capture;
        candidate_index = previous_candidate;
    }
    (capture_index == capture_start && !is_trailing_surrogate_position(text, candidate_index))
        .then_some(candidate_index)
}
