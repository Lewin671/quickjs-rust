//! Array logical length and four integrity flags share one scalar word.
//! Low integrity bits leave a shift-only length read; host overflow is rejected.
use std::cell::Cell;
const LENGTH: u64 = (1 << 60) - 1;
const FLAGS: u64 = 15;
const WRITABLE: u64 = 1;
const EXTENSIBLE: u64 = 2;
const SEALED: u64 = 4;
const FROZEN: u64 = 8;
pub(super) struct PackedState(Cell<u64>);
impl PackedState {
    pub(super) fn new(length: usize) -> Self {
        assert!(
            (length as u64) <= LENGTH,
            "array host length exceeds representation"
        );
        Self(Cell::new(((length as u64) << 4) | WRITABLE | EXTENSIBLE))
    }
    #[inline(always)]
    pub(super) fn length(&self) -> usize {
        (self.0.get() >> 4) as usize
    }
    #[inline(always)]
    pub(super) fn set_length(&self, length: usize) {
        assert!(
            (length as u64) <= LENGTH,
            "array host length exceeds representation"
        );
        self.0.set((self.0.get() & FLAGS) | ((length as u64) << 4));
    }
    fn set(&self, bit: u64, value: bool) {
        self.0.set(if value {
            self.0.get() | bit
        } else {
            self.0.get() & !bit
        });
    }
    pub(super) fn length_writable(&self) -> bool {
        self.0.get() & WRITABLE != 0
    }
    pub(super) fn extensible(&self) -> bool {
        self.0.get() & EXTENSIBLE != 0
    }
    pub(super) fn sealed(&self) -> bool {
        self.0.get() & SEALED != 0
    }
    pub(super) fn frozen(&self) -> bool {
        self.0.get() & FROZEN != 0
    }
    pub(super) fn set_length_writable(&self, value: bool) {
        self.set(WRITABLE, value);
    }
    pub(super) fn set_extensible(&self, value: bool) {
        self.set(EXTENSIBLE, value);
    }
    pub(super) fn set_sealed(&self, value: bool) {
        self.set(SEALED, value);
    }
    pub(super) fn set_frozen(&self, value: bool) {
        self.set(FROZEN, value);
    }
}

#[cfg(all(test, target_pointer_width = "64"))]
mod tests {
    use super::*;
    #[test]
    fn host_lengths_and_integrity_flags_are_independent_at_every_boundary() {
        for length in [
            0,
            1,
            15,
            16,
            u32::MAX as usize,
            1_usize << 40,
            LENGTH as usize,
        ] {
            let state = PackedState::new(length);
            for flags in 0..16 {
                state.set_length_writable(flags & 1 != 0);
                state.set_extensible(flags & 2 != 0);
                state.set_sealed(flags & 4 != 0);
                state.set_frozen(flags & 8 != 0);
                assert_eq!(state.length(), length);
                state.set_length(7);
                assert_eq!(state.length(), 7);
                assert_eq!(state.length_writable(), flags & 1 != 0);
                assert_eq!(state.extensible(), flags & 2 != 0);
                assert_eq!(state.sealed(), flags & 4 != 0);
                assert_eq!(state.frozen(), flags & 8 != 0);
                state.set_length(length);
            }
        }
    }
    #[test]
    #[should_panic(expected = "array host length exceeds representation")]
    fn host_overflow_is_rejected_without_truncation() {
        PackedState::new(usize::MAX);
    }
}
