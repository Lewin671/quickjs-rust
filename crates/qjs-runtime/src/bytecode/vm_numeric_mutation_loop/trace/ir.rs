//! Immutable schedules shared by Numeric Trace compilation and execution.

use super::super::dense::{LocalWrite, MAX_DENSE_LOCALS, NumberInstruction};

#[derive(Clone, Copy, Debug)]
pub(super) struct CountedLoop {
    pub(super) header: usize,
    pub(super) backedge: usize,
    pub(super) exit: usize,
    pub(super) body_start: usize,
    pub(super) counter_slot: usize,
    pub(super) limit_slot: usize,
}

#[derive(Clone, Debug)]
pub(super) struct NumericProgram {
    pub(super) operations: Vec<NumberInstruction>,
    pub(super) writes: Vec<LocalWrite>,
    pub(super) invalidations: Vec<usize>,
}

/// Static proof descriptor for a radix-2 paired-index loop nest.
#[derive(Clone, Copy, Debug)]
pub(super) struct Radix2NestProof {
    pub(super) span: usize,
    pub(super) bound: usize,
    pub(super) lane: usize,
    pub(super) index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReceiverRole {
    Writable(usize),
    Readable(usize),
}

#[derive(Clone, Copy)]
pub(super) struct LocalBank {
    values: [f64; MAX_DENSE_LOCALS],
    states: [SlotState; MAX_DENSE_LOCALS],
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SlotState {
    Cleared,
    Undefined,
    Number,
}

impl LocalBank {
    pub(super) fn empty() -> Self {
        Self {
            values: [0.0; MAX_DENSE_LOCALS],
            states: [SlotState::Cleared; MAX_DENSE_LOCALS],
        }
    }

    #[inline(always)]
    pub(super) fn number(&self, local: usize) -> Option<f64> {
        matches!(self.states.get(local), Some(SlotState::Number)).then(|| self.values[local])
    }

    #[inline(always)]
    pub(super) fn write_number(&mut self, local: usize, value: f64) {
        self.values[local] = value;
        self.states[local] = SlotState::Number;
    }

    #[inline(always)]
    pub(super) fn write_undefined(&mut self, local: usize) {
        self.states[local] = SlotState::Undefined;
    }

    pub(super) fn state(&self, local: usize) -> Option<SlotState> {
        self.states.get(local).copied()
    }

    pub(super) fn valid_mask(&self) -> u64 {
        self.states
            .iter()
            .enumerate()
            .fold(0_u64, |mask, (local, state)| {
                mask | (u64::from(*state == SlotState::Number) << local)
            })
    }
}
