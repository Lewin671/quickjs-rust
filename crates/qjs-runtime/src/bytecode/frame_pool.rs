//! Cleared general-frame storage and the common free-capacity budget.
//! Active frames retain only this data-only handle, never RuntimeServices.
//! Resource teardown precedes bin borrows; pooled capacities contain no cells,
//! Context, code, handlers or pending completion identities.
use std::cell::{Cell, RefCell};

use super::{
    frame_bank::FrameStorage,
    vm::{ColdFrame, FrameState, Slot},
};
use crate::Value;

pub(super) const MAX_RECORDS: usize = 64;
pub(super) const MAX_BYTES: usize = 256 * 1024;
pub(super) const MAX_RECORD_BYTES: usize = 16 * 1024;
const MAX_VECTORS: usize = 32;
const MAX_COLD: usize = 8;
const MAX_CAPACITY: usize = 256;

struct FreeLocals {
    slots: Vec<Slot>,
    poisoned: bool,
}

pub(super) struct GeneralPool {
    bytes: Cell<usize>,
    count: Cell<usize>,
    poison: Cell<bool>,
    operands: RefCell<Vec<Vec<Value>>>,
    locals: RefCell<Vec<FreeLocals>>,
    // Retain the actual box allocation, not another by-value cold record.
    #[allow(clippy::vec_box)]
    cold: RefCell<Vec<Box<ColdFrame>>>,
}

impl GeneralPool {
    // Conservatively charge every directory's maximum allocation, including
    // headers and the Rc allocation. Vec capacities cannot exceed these caps.
    const DIRECTORY_BYTES: usize = MAX_RECORDS * size_of::<FrameStorage>()
        + MAX_VECTORS * (size_of::<Vec<Value>>() + size_of::<FreeLocals>())
        + MAX_COLD * size_of::<Box<ColdFrame>>()
        + size_of::<Self>()
        + 2 * size_of::<usize>()
        + size_of::<super::frame_bank::StackPool>();

    pub(super) fn new(poison: bool) -> Self {
        Self {
            bytes: Cell::new(0),
            count: Cell::new(0),
            poison: Cell::new(poison),
            operands: RefCell::default(),
            locals: RefCell::default(),
            cold: RefCell::default(),
        }
    }
    #[cfg(debug_assertions)]
    pub(super) fn set_poison(&self, poison: bool) {
        self.poison.set(poison);
    }
    pub(super) fn reserve(&self, bytes: usize) -> bool {
        let total = self.bytes.get().saturating_add(bytes);
        if bytes > MAX_RECORD_BYTES
            || self.count.get() >= MAX_RECORDS
            || total.saturating_add(Self::DIRECTORY_BYTES) > MAX_BYTES
        {
            return false;
        }
        self.bytes.set(total);
        self.count.set(self.count.get() + 1);
        true
    }
    pub(super) fn release(&self, bytes: usize) {
        self.bytes.set(
            self.bytes
                .get()
                .checked_sub(bytes)
                .expect("retained byte charge"),
        );
        self.count.set(
            self.count
                .get()
                .checked_sub(1)
                .expect("retained record charge"),
        );
    }
    pub(super) fn take_operands(&self) -> Vec<Value> {
        crate::diagnostics::count!(operand_stack_pool_requests);
        let storage = self.operands.borrow_mut().pop();
        storage.map_or_else(
            || {
                crate::diagnostics::count!(operand_stack_pool_allocations);
                Vec::with_capacity(64)
            },
            |values| {
                self.release(values.capacity() * size_of::<Value>());
                values
            },
        )
    }
    pub(super) fn put_operands(&self, mut values: Vec<Value>) {
        values.clear();
        if values.capacity() == 0 || values.capacity() > MAX_CAPACITY {
            return;
        }
        let mut bin = self.operands.borrow_mut();
        if bin.len() < MAX_VECTORS && self.reserve(values.capacity() * size_of::<Value>()) {
            bin.push(values);
        }
    }
    pub(super) fn take_locals(&self, len: usize) -> Vec<Slot> {
        crate::diagnostics::count!(local_slot_pool_requests);
        let storage = self.locals.borrow_mut().pop();
        let Some(FreeLocals {
            mut slots,
            poisoned,
        }) = storage
        else {
            if len != 0 {
                crate::diagnostics::count!(local_slot_allocations);
            }
            return vec![None; len];
        };
        self.release(slots.capacity() * size_of::<Slot>());
        if poisoned {
            for slot in &mut slots {
                assert!(
                    matches!(slot, Some(Value::Number(value)) if value.is_nan()),
                    "released local poison changed"
                );
                *slot = None;
            }
        }
        if slots.len() != len {
            if slots.capacity() < len {
                crate::diagnostics::count!(local_slot_allocations);
            }
            slots.resize(len, None);
        }
        slots
    }
    pub(super) fn put_locals(&self, mut slots: Vec<Slot>) {
        if slots.capacity() == 0 || slots.capacity() > MAX_CAPACITY {
            return;
        }
        // A released Value can tear down another suspended frame using this
        // pool. Finish that work before borrowing any free-bin directory.
        let poison = cfg!(debug_assertions) && self.poison.get();
        for slot in &mut slots {
            *slot = poison.then_some(Value::Number(f64::NAN));
        }
        let mut bin = self.locals.borrow_mut();
        if bin.len() < MAX_VECTORS && self.reserve(slots.capacity() * size_of::<Slot>()) {
            bin.push(FreeLocals {
                slots,
                poisoned: poison,
            });
        }
    }
    pub(super) fn take_cold(&self) -> Box<ColdFrame> {
        crate::diagnostics::count!(cold_frame_pool_requests);
        let storage = self.cold.borrow_mut().pop();
        storage.map_or_else(
            || {
                crate::diagnostics::count!(cold_frame_allocations);
                Box::default()
            },
            |cold| {
                self.release(cold.retained_bytes());
                cold
            },
        )
    }
    pub(super) fn put_cold(&self, mut cold: Box<ColdFrame>) {
        cold.reset();
        let bytes = cold.retained_bytes();
        let mut bin = self.cold.borrow_mut();
        if bin.len() < MAX_COLD && self.reserve(bytes) {
            bin.push(cold);
        }
    }
    pub(super) fn drain(&self) {
        let operands = std::mem::take(&mut *self.operands.borrow_mut());
        let locals = std::mem::take(&mut *self.locals.borrow_mut());
        let cold = std::mem::take(&mut *self.cold.borrow_mut());
        for values in operands {
            self.release(values.capacity() * size_of::<Value>());
        }
        for values in locals {
            self.release(values.slots.capacity() * size_of::<Slot>());
        }
        for value in cold {
            self.release(value.retained_bytes());
        }
    }
    #[cfg(test)]
    pub(super) fn retained_bytes(&self) -> usize {
        self.bytes.get() + Self::DIRECTORY_BYTES
    }
}

impl Drop for FrameState<'_> {
    #[inline(never)]
    fn drop(&mut self) {
        self.stack
            .pool()
            .put_locals(std::mem::take(&mut self.locals));
        if let Some(cold) = self.cold.take() {
            self.stack.pool().put_cold(cold);
        }
    }
}

#[cfg(test)]
mod tests;
