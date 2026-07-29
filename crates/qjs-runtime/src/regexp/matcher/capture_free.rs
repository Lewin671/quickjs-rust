//! Prepared ordered programs for the capture-free subset of RegExp.
//!
//! The general matcher remains the semantic authority for every pattern with
//! captures, backreferences, lookarounds, or a nullable unbounded repetition.
//! For the regular capture-free subset, decoding groups, quantifiers, and
//! single-code-point atoms once lets repeated `exec` calls run a compact,
//! ordered backtracking program with one reusable choice stack.

use super::escapes::{is_trailing_surrogate_position, regexp_word_char};
use super::fast_scan::{SimpleAtom, simple_atom_matcher};
use super::groups::{GroupKind, closing_group, group_alternatives, group_kind};
use super::{
    MatchOptions, PropertyCache, Quantifier, at_line_end, at_line_start, atom_end, quantifier,
};

const MAX_PROGRAM_INSTRUCTIONS: usize = 4096;
const MAX_UNROLLED_REPETITIONS: usize = 256;

/// A compiled, capture-free pattern plus the scratch space reused for every
/// candidate start within one prepared RegExp operation.
pub(super) struct CaptureFreeProgram {
    instructions: Vec<Instruction>,
    choices: Vec<Choice>,
}

#[derive(Clone)]
struct Alternatives {
    sequences: Vec<Sequence>,
    nullable: bool,
}

#[derive(Clone)]
struct Sequence {
    pieces: Vec<Piece>,
    nullable: bool,
}

#[derive(Clone)]
struct Piece {
    kind: PieceKind,
    quantifier: Quantifier,
}

#[derive(Clone)]
enum PieceKind {
    Atom(SimpleAtom),
    Alternatives(Alternatives),
    StartAnchor,
    EndAnchor,
    WordBoundary { matches: bool },
}

#[derive(Clone, Copy)]
enum Instruction {
    Atom(SimpleAtom),
    StartAnchor,
    EndAnchor,
    WordBoundary { matches: bool },
    Split { first: usize, second: usize },
    Jump { target: usize },
    Accept,
}

#[derive(Clone, Copy)]
struct Choice {
    pc: usize,
    index: usize,
}

struct Parser<'a> {
    pattern: &'a [char],
    properties: &'a PropertyCache,
    options: MatchOptions,
}

struct Builder {
    instructions: Vec<Instruction>,
}

impl CaptureFreeProgram {
    /// Compile only patterns whose result cannot expose captures. Any syntax
    /// outside this intentionally conservative subset returns `None`, so the
    /// caller can enter the existing complete matcher before any matching work.
    pub(super) fn compile(
        pattern: &[char],
        properties: &PropertyCache,
        options: MatchOptions,
    ) -> Option<Self> {
        let parser = Parser {
            pattern,
            properties,
            options,
        };
        let alternatives = parser.parse_alternatives(0, pattern.len())?;
        let mut builder = Builder {
            instructions: Vec::with_capacity(pattern.len().saturating_add(1)),
        };
        builder.compile_alternatives(&alternatives)?;
        builder.emit(Instruction::Accept)?;
        Some(Self {
            instructions: builder.instructions,
            choices: Vec::new(),
        })
    }

    /// Find the first match from `start_index`, following the same leftmost
    /// search and Unicode surrogate-start rules as the generic matcher.
    pub(super) fn match_input(
        &mut self,
        pattern: &[char],
        text: &[char],
        start_index: usize,
        exact_start: bool,
        properties: &PropertyCache,
        options: MatchOptions,
    ) -> Option<(usize, usize)> {
        if start_index > text.len() {
            return None;
        }
        let final_start = if exact_start { start_index } else { text.len() };
        for start in start_index..=final_start {
            if options.unicode && is_trailing_surrogate_position(text, start) {
                continue;
            }
            if let Some(end) = self.match_at(pattern, text, start, properties, options) {
                return Some((start, end));
            }
        }
        None
    }

    fn match_at(
        &mut self,
        pattern: &[char],
        text: &[char],
        start: usize,
        properties: &PropertyCache,
        options: MatchOptions,
    ) -> Option<usize> {
        self.choices.clear();
        let mut pc = 0;
        let mut index = start;
        loop {
            let instruction = *self.instructions.get(pc)?;
            let next = match instruction {
                Instruction::Atom(atom) => atom
                    .step(pattern, text, index, properties, options)
                    .map(|next_index| (pc + 1, next_index)),
                Instruction::StartAnchor => {
                    at_line_start(text, index, options.multiline).then_some((pc + 1, index))
                }
                Instruction::EndAnchor => {
                    at_line_end(text, index, options.multiline).then_some((pc + 1, index))
                }
                Instruction::WordBoundary { matches } => {
                    let before = index > 0 && regexp_word_char(text[index - 1]);
                    let after = text.get(index).copied().is_some_and(regexp_word_char);
                    ((before != after) == matches).then_some((pc + 1, index))
                }
                Instruction::Split { first, second } => {
                    self.choices.push(Choice { pc: second, index });
                    Some((first, index))
                }
                Instruction::Jump { target } => Some((target, index)),
                Instruction::Accept => return Some(index),
            };
            if let Some((next_pc, next_index)) = next {
                pc = next_pc;
                index = next_index;
                continue;
            }
            let Choice {
                pc: retry_pc,
                index: retry_index,
            } = self.choices.pop()?;
            pc = retry_pc;
            index = retry_index;
        }
    }
}

impl<'a> Parser<'a> {
    fn parse_alternatives(&self, start: usize, end: usize) -> Option<Alternatives> {
        let sequences: Vec<_> = group_alternatives(self.pattern, start, end)
            .into_iter()
            .map(|(alternative_start, alternative_end)| {
                self.parse_sequence(alternative_start, alternative_end)
            })
            .collect::<Option<_>>()?;
        let nullable = sequences.iter().any(|sequence| sequence.nullable);
        Some(Alternatives {
            sequences,
            nullable,
        })
    }

    fn parse_sequence(&self, start: usize, end: usize) -> Option<Sequence> {
        let mut pieces = Vec::new();
        let mut pc = start;
        while pc < end {
            let (kind, atom_end) = match self.pattern[pc] {
                '^' => (PieceKind::StartAnchor, pc + 1),
                '$' => (PieceKind::EndAnchor, pc + 1),
                '\\' if matches!(self.pattern.get(pc + 1), Some('b' | 'B')) => (
                    PieceKind::WordBoundary {
                        matches: self.pattern[pc + 1] == 'b',
                    },
                    pc + 2,
                ),
                '(' => {
                    if group_kind(self.pattern, pc) != GroupKind::NonCapturing {
                        return None;
                    }
                    let close = closing_group(self.pattern, pc)?;
                    if close >= end {
                        return None;
                    }
                    let alternatives = self.parse_alternatives(pc + 3, close)?;
                    (PieceKind::Alternatives(alternatives), close + 1)
                }
                ')' | '|' => return None,
                _ => {
                    let atom =
                        simple_atom_matcher(self.pattern, pc, self.properties, self.options)?;
                    (
                        PieceKind::Atom(atom),
                        atom_end(self.pattern, pc, self.properties, self.options.unicode)?,
                    )
                }
            };
            let quantifier = quantifier(self.pattern, atom_end);
            if quantifier.next_pc <= pc
                || quantifier.max.is_some_and(|max| max < quantifier.min)
                || (kind.is_assertion() && quantifier.next_pc != atom_end)
                || (quantifier.max.is_none() && kind.nullable())
            {
                return None;
            }
            pieces.push(Piece { kind, quantifier });
            pc = quantifier.next_pc;
        }
        let nullable = pieces.iter().all(Piece::nullable);
        Some(Sequence { pieces, nullable })
    }
}

impl PieceKind {
    fn nullable(&self) -> bool {
        match self {
            Self::Atom(_) => false,
            Self::Alternatives(alternatives) => alternatives.nullable,
            Self::StartAnchor | Self::EndAnchor | Self::WordBoundary { .. } => true,
        }
    }

    fn is_assertion(&self) -> bool {
        matches!(
            self,
            Self::StartAnchor | Self::EndAnchor | Self::WordBoundary { .. }
        )
    }
}

impl Piece {
    fn nullable(&self) -> bool {
        self.quantifier.min == 0 || self.kind.nullable()
    }
}

impl Builder {
    fn emit(&mut self, instruction: Instruction) -> Option<usize> {
        if self.instructions.len() >= MAX_PROGRAM_INSTRUCTIONS {
            return None;
        }
        let pc = self.instructions.len();
        self.instructions.push(instruction);
        Some(pc)
    }

    fn compile_alternatives(&mut self, alternatives: &Alternatives) -> Option<()> {
        let (first, remaining) = alternatives.sequences.split_first()?;
        if remaining.is_empty() {
            return self.compile_sequence(first);
        }
        let split = self.emit(Instruction::Split {
            first: 0,
            second: 0,
        })?;
        let first_start = self.instructions.len();
        self.compile_sequence(first)?;
        let join = self.emit(Instruction::Jump { target: 0 })?;
        let remaining_start = self.instructions.len();
        self.compile_alternatives(&Alternatives {
            sequences: remaining.to_vec(),
            nullable: remaining.iter().any(|sequence| sequence.nullable),
        })?;
        let end = self.instructions.len();
        self.patch_split(split, first_start, remaining_start)?;
        self.patch_jump(join, end)?;
        Some(())
    }

    fn compile_sequence(&mut self, sequence: &Sequence) -> Option<()> {
        for piece in &sequence.pieces {
            self.compile_piece(piece)?;
        }
        Some(())
    }

    fn compile_piece(&mut self, piece: &Piece) -> Option<()> {
        let quantifier = piece.quantifier;
        if quantifier.min > MAX_UNROLLED_REPETITIONS
            || quantifier
                .max
                .is_some_and(|max| max > MAX_UNROLLED_REPETITIONS)
        {
            return None;
        }
        for _ in 0..quantifier.min {
            self.compile_kind(&piece.kind)?;
        }
        match quantifier.max {
            Some(max) => {
                for _ in quantifier.min..max {
                    self.compile_optional(&piece.kind, quantifier.greedy)?;
                }
            }
            None => self.compile_star(&piece.kind, quantifier.greedy)?,
        }
        Some(())
    }

    fn compile_kind(&mut self, kind: &PieceKind) -> Option<()> {
        match kind {
            PieceKind::Atom(atom) => self.emit(Instruction::Atom(*atom)).map(|_| ()),
            PieceKind::Alternatives(alternatives) => self.compile_alternatives(alternatives),
            PieceKind::StartAnchor => self.emit(Instruction::StartAnchor).map(|_| ()),
            PieceKind::EndAnchor => self.emit(Instruction::EndAnchor).map(|_| ()),
            PieceKind::WordBoundary { matches } => self
                .emit(Instruction::WordBoundary { matches: *matches })
                .map(|_| ()),
        }
    }

    fn compile_optional(&mut self, kind: &PieceKind, greedy: bool) -> Option<()> {
        let split = self.emit(Instruction::Split {
            first: 0,
            second: 0,
        })?;
        let body_start = self.instructions.len();
        self.compile_kind(kind)?;
        let end = self.instructions.len();
        let (first, second) = if greedy {
            (body_start, end)
        } else {
            (end, body_start)
        };
        self.patch_split(split, first, second)
    }

    fn compile_star(&mut self, kind: &PieceKind, greedy: bool) -> Option<()> {
        let split = self.emit(Instruction::Split {
            first: 0,
            second: 0,
        })?;
        let body_start = self.instructions.len();
        self.compile_kind(kind)?;
        self.emit(Instruction::Jump { target: split })?;
        let end = self.instructions.len();
        let (first, second) = if greedy {
            (body_start, end)
        } else {
            (end, body_start)
        };
        self.patch_split(split, first, second)
    }

    fn patch_split(&mut self, pc: usize, first: usize, second: usize) -> Option<()> {
        let instruction = self.instructions.get_mut(pc)?;
        if !matches!(instruction, Instruction::Split { .. }) {
            return None;
        }
        *instruction = Instruction::Split { first, second };
        Some(())
    }

    fn patch_jump(&mut self, pc: usize, target: usize) -> Option<()> {
        let instruction = self.instructions.get_mut(pc)?;
        if !matches!(instruction, Instruction::Jump { .. }) {
            return None;
        }
        *instruction = Instruction::Jump { target };
        Some(())
    }
}
