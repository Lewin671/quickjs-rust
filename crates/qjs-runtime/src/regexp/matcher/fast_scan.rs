//! Linear-scan fast path for quantified single-code-point regexp atoms.
//!
//! The generic backtracking matcher drives every quantifier through an
//! explicit-stack DFS that clones a [`MatchState`](super::MatchState) per
//! character and per backtrack point. For atoms that consume exactly one code
//! point and capture nothing (`\p{X}`, `\d`, `[a-z]`, `.`, a literal,
//! `\uXXXX`) the run of repetitions can instead be scanned once, recording the
//! repetition boundaries, and accept states emitted in priority order. This
//! keeps property-escape conformance cases such as `/^\p{L}+$/u` over ~1M code
//! points inside the matcher's budget.

use super::escapes::{
    self, chars_equal, regexp_control_escape, regexp_whitespace, regexp_word_char, unicode_escape,
};
use super::groups::named_backreference;
use super::{
    MatchOptions, MatchState, PropertyCache, Quantifier, class_match, code_unit_char,
    is_line_terminator, regexp_code_point_at,
};
use crate::string::advance_string_index;

/// A single-code-point atom whose repetition can be scanned linearly. Each
/// variant tests the code point at one text position and reports the index just
/// past a successful match (so unicode surrogate pairs advance by two units).
pub(super) enum SimpleAtom<'a> {
    /// `.` — any code point except (unless dot-all) a line terminator.
    AnyChar,
    /// A bare literal character (`a`, `1`, ...).
    Literal(char),
    /// A character class `[...]` (the slice excludes the brackets).
    Class { class: &'a [char], base: usize },
    /// A `\` escape that consumes exactly one input code point: `\d`, `\w`,
    /// `\s` (and negations) or a control escape.
    Escape(char),
    /// `\uXXXX` / `\u{...}` resolving to a concrete code point.
    UnicodeEscape(char),
    /// `\p{...}` / `\P{...}` property escape.
    Property(escapes::ParsedPropertyEscape),
}

/// Classify the atom at `atom_pc` as a single-code-point matcher, or return
/// `None` for atoms that need the generic machinery (groups, backreferences,
/// anchors). The classification mirrors [`match_atom`](super::match_atom)
/// exactly so the fast path produces identical matches.
pub(super) fn simple_atom_matcher<'a>(
    pattern: &'a [char],
    atom_pc: usize,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Option<SimpleAtom<'a>> {
    match pattern[atom_pc] {
        '.' => Some(SimpleAtom::AnyChar),
        '[' => {
            let end = pattern[atom_pc + 1..]
                .iter()
                .position(|char| *char == ']')?;
            let class_end = atom_pc + 1 + end;
            Some(SimpleAtom::Class {
                class: &pattern[atom_pc + 1..class_end],
                base: atom_pc + 1,
            })
        }
        '\\' => {
            let escaped = *pattern.get(atom_pc + 1)?;
            // Numeric and named backreferences consume a variable width.
            if escaped.to_digit(10).is_some_and(|value| value >= 1) {
                return None;
            }
            if escaped == 'k' && named_backreference(pattern, atom_pc).is_some() {
                return None;
            }
            if options.unicode
                && matches!(escaped, 'p' | 'P')
                && let Some(escape) = properties.get(atom_pc)
            {
                return Some(SimpleAtom::Property(escape));
            }
            if escaped == 'u'
                && let Some(escape) = unicode_escape(pattern, atom_pc, options.unicode)
            {
                return Some(SimpleAtom::UnicodeEscape(escape.value));
            }
            Some(SimpleAtom::Escape(escaped))
        }
        // Groups and anchors need the generic machinery.
        '(' | ')' | '^' | '$' | '|' => None,
        literal => Some(SimpleAtom::Literal(literal)),
    }
}

impl SimpleAtom<'_> {
    /// Test the code point at `index`, returning the index just past a match.
    fn step(
        &self,
        text: &[char],
        index: usize,
        properties: &PropertyCache,
        options: MatchOptions,
    ) -> Option<usize> {
        match self {
            SimpleAtom::AnyChar => {
                let value = *text.get(index)?;
                if !options.dot_all && is_line_terminator(value) {
                    return None;
                }
                Some(advance_string_index(text, index, options.unicode))
            }
            SimpleAtom::Literal(literal) => {
                let value = *text.get(index)?;
                chars_equal(value, *literal, options.ignore_case).then_some(index + 1)
            }
            SimpleAtom::Class { class, base } => {
                let (value, next_index) = regexp_code_point_at(text, index, options.unicode)?;
                class_match(class, *base, value, properties, options).then_some(next_index)
            }
            SimpleAtom::UnicodeEscape(value) => {
                let current = *text.get(index)?;
                if chars_equal(current, *value, options.ignore_case) {
                    return Some(index + 1);
                }
                let mut buffer = [0u16; 2];
                let code_units = value.encode_utf16(&mut buffer);
                if code_units.len() == 2
                    && text.get(index) == Some(&code_unit_char(code_units[0]))
                    && text.get(index + 1) == Some(&code_unit_char(code_units[1]))
                {
                    return Some(index + 2);
                }
                None
            }
            SimpleAtom::Property(escape) => {
                let (value, next_index) = regexp_code_point_at(text, index, true)?;
                (escape.set.contains(u32::from(value)) != escape.negated).then_some(next_index)
            }
            SimpleAtom::Escape(escaped) => {
                let value = *text.get(index)?;
                let matched = match escaped {
                    'd' => value.is_ascii_digit(),
                    'D' => !value.is_ascii_digit(),
                    's' => regexp_whitespace(value),
                    'S' => !regexp_whitespace(value),
                    'w' => regexp_word_char(value),
                    'W' => !regexp_word_char(value),
                    other => chars_equal(value, regexp_control_escape(*other), options.ignore_case),
                };
                matched.then_some(index + 1)
            }
        }
    }
}

/// Linear-scan repetition of a single-code-point atom. Walks forward recording
/// each repetition boundary, then yields accept states in priority order
/// (greedy: longest first; lazy: shortest first) by mutating one base state's
/// index rather than cloning per character.
pub(super) fn repeat_simple_atom(
    text: &[char],
    matcher: &SimpleAtom<'_>,
    quantifier: Quantifier,
    state: MatchState,
    properties: &PropertyCache,
    options: MatchOptions,
) -> Vec<MatchState> {
    let mut boundaries = vec![state.index];
    let mut index = state.index;
    let max = quantifier.max.unwrap_or(usize::MAX);
    while boundaries.len() - 1 < max {
        let Some(next) = matcher.step(text, index, properties, options) else {
            break;
        };
        if next == index {
            break;
        }
        index = next;
        boundaries.push(index);
    }
    if boundaries.len() - 1 < quantifier.min {
        return Vec::new();
    }
    let lowest = quantifier.min;
    let highest = boundaries.len() - 1;
    let order: Vec<usize> = if quantifier.greedy {
        (lowest..=highest).rev().collect()
    } else {
        (lowest..=highest).collect()
    };
    order
        .into_iter()
        .map(|count| {
            let mut accepted = state.clone();
            accepted.index = boundaries[count];
            accepted
        })
        .collect()
}
