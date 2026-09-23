//! The positions a match can begin at.
//!
//! A search tries every start position in turn, and each attempt interprets
//! the pattern from its first atom. When every alternative must begin by
//! consuming one code point through a simple atom (a literal, a class, an
//! escape such as `\d`), or with a non-multiline `^`, a position where none
//! of those atoms steps cannot begin a match, and the search skips it without
//! entering the matcher. `/"[^"]*"|true|-?\d+/` begins only at `"`, `t`, `-`
//! or a digit; `/^[a-z]+$/` begins only at position 0.
//!
//! The analysis is conservative: anything it does not model -- an alternative
//! that can match empty, a lookaround, a backreference, `$`, `\b`, a group
//! that may repeat zero times, a property escape -- leaves every position a
//! candidate.

use super::escapes::PropertyCache;
use super::fast_scan::{SimpleAtom, simple_atom_matcher};
use super::groups::{GroupKind, closing_group, group_alternatives, group_kind};
use super::{MatchOptions, atom_end, quantifier};

/// What the first step of every match must be.
pub(super) struct StartFilter {
    /// Pattern offsets of the atoms one of which a match consumes first.
    atom_pcs: Vec<usize>,
    /// Whether a match may begin with a non-multiline `^`, that is, at 0.
    at_input_start: bool,
    /// The ASCII code points one of the atoms steps over, one bit each. An
    /// atom's answer for an ASCII code point depends on nothing else, so
    /// the search tests those against this set and steps atoms only for the
    /// rest.
    ascii: u128,
}

impl StartFilter {
    /// The filter for a pattern whose top-level alternatives are
    /// `alternatives`, or `None` when every position stays a candidate.
    pub(super) fn analyze(
        pattern: &[char],
        alternatives: &[(usize, usize)],
        properties: &PropertyCache,
        options: MatchOptions,
    ) -> Option<Self> {
        let mut filter = Self {
            atom_pcs: Vec::new(),
            at_input_start: false,
            ascii: 0,
        };
        for &(start, end) in alternatives {
            if !filter.add_sequence(pattern, start, end, properties, options) {
                return None;
            }
        }
        let atoms = filter.atoms(pattern, properties, options);
        for code in 0..128_u8 {
            let text = [char::from(code)];
            if atoms
                .iter()
                .any(|atom| atom.step(&text, 0, properties, options).is_some())
            {
                filter.ascii |= 1 << code;
            }
        }
        Some(filter)
    }

    /// Adds the first steps of `pattern[pc..end]`; `false` when the sequence
    /// can match without consuming or starts with something not modelled.
    fn add_sequence(
        &mut self,
        pattern: &[char],
        mut pc: usize,
        end: usize,
        properties: &PropertyCache,
        options: MatchOptions,
    ) -> bool {
        while pc < end {
            match pattern[pc] {
                '^' => {
                    if options.multiline {
                        return false;
                    }
                    self.at_input_start = true;
                    return true;
                }
                '(' => {
                    let body = match group_kind(pattern, pc) {
                        GroupKind::Capturing => pc + 1,
                        GroupKind::NonCapturing => pc + 3,
                        GroupKind::Named { body_offset } => pc + body_offset,
                        GroupKind::Lookahead { .. } | GroupKind::Lookbehind { .. } => {
                            return false;
                        }
                    };
                    let Some(close) = closing_group(pattern, pc) else {
                        return false;
                    };
                    if quantifier(pattern, close + 1).min == 0 {
                        return false;
                    }
                    return group_alternatives(pattern, body, close).all(|(start, end)| {
                        self.add_sequence(pattern, start, end, properties, options)
                    });
                }
                '\\' if matches!(pattern.get(pc + 1), Some('b' | 'B')) => return false,
                _ => {
                    let Some(next) = atom_end(pattern, pc, properties, options.unicode) else {
                        return false;
                    };
                    match simple_atom_matcher(pattern, pc, properties, options) {
                        None | Some(SimpleAtom::Property(_)) => return false,
                        Some(_) => {}
                    }
                    self.atom_pcs.push(pc);
                    let quantifier = quantifier(pattern, next);
                    if quantifier.min > 0 {
                        return true;
                    }
                    pc = quantifier.next_pc;
                }
            }
        }
        false
    }

    /// The atoms, resolved against the pattern they were analyzed from.
    pub(super) fn atoms<'p>(
        &self,
        pattern: &'p [char],
        properties: &PropertyCache,
        options: MatchOptions,
    ) -> Vec<SimpleAtom<'p>> {
        self.atom_pcs
            .iter()
            .filter_map(|&pc| simple_atom_matcher(pattern, pc, properties, options))
            .collect()
    }

    /// Whether a match may begin at `start`, given the resolved `atoms`.
    /// With no atoms only position 0 is admitted, so a search can stop at
    /// the first position refused.
    #[inline]
    pub(super) fn admits(
        &self,
        atoms: &[SimpleAtom<'_>],
        text: &[char],
        start: usize,
        properties: &PropertyCache,
        options: MatchOptions,
    ) -> bool {
        if self.at_input_start && start == 0 {
            return true;
        }
        match text.get(start) {
            Some(&value) if value.is_ascii() => self.ascii & (1 << value as u32) != 0,
            Some(_) => atoms
                .iter()
                .any(|atom| atom.step(text, start, properties, options).is_some()),
            None => false,
        }
    }
}
