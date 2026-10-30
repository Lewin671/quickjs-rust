//! Array logical length and four integrity flags share one scalar word.
//! JS lengths fit below the flags; host overflow is rejected, never truncated.
use std::cell::Cell;
const LENGTH: u64 = (1 << 60) - 1;
const WRITABLE: u64 = 1 << 60;
const EXTENSIBLE: u64 = 1 << 61;
const SEALED: u64 = 1 << 62;
const FROZEN: u64 = 1 << 63;
pub(super) struct PackedState(Cell<u64>);
impl PackedState {
    pub(super) fn new(length: usize) -> Self {
        assert!(
            (length as u64) <= LENGTH,
            "array host length exceeds representation"
        );
        Self(Cell::new(length as u64 | WRITABLE | EXTENSIBLE))
    }
    #[inline(always)]
    pub(super) fn length(&self) -> usize {
        (self.0.get() & LENGTH) as usize
    }
    #[inline(always)]
    pub(super) fn set_length(&self, length: usize) {
        assert!(
            (length as u64) <= LENGTH,
            "array host length exceeds representation"
        );
        self.0.set((self.0.get() & !LENGTH) | length as u64);
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
