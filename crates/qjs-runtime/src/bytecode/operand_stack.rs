//! Operand-stack storage, and its per-body recycler.
//!
//! A frame's operand stack is a plain `Vec<Value>` whose allocation is worth
//! reusing across calls to the same body. The recycler is a *handle* rather
//! than a method on `Bytecode` for a structural reason: a frame must be able
//! to return its stack when it ends without holding a borrow of the bytecode
//! for the frame's whole lifetime. That borrow is one of the things that
//! stopped a frame from owning its bytecode, and therefore stopped a callee
//! from running on its caller's VM.

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::value::Value;

use super::ir::Bytecode;

/// A shared pool of cleared operand-stack allocations for one compiled body.
///
/// The pool is a handle rather than a method on `Bytecode` because a frame
/// must be able to return its stack when it ends without borrowing the
/// bytecode for the frame's whole lifetime -- which is what stopped a frame
/// from owning its bytecode at all.
#[derive(Clone)]
pub(super) struct OperandStackRecycler(Rc<RefCell<Vec<Vec<Value>>>>);

impl std::fmt::Debug for OperandStackRecycler {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The pooled allocations are cleared scratch storage, so printing them
        // would be noise in a `Bytecode` dump.
        formatter
            .debug_struct("OperandStackRecycler")
            .field("pooled", &self.0.borrow().len())
            .finish()
    }
}

impl OperandStackRecycler {
    const INITIAL_CAPACITY: usize = 64;
    pub(super) const MAX_RECYCLED_CAPACITY: usize = 256;
    /// How many operand stacks one body keeps for reuse. Deep recursion holds
    /// one per active frame, and the bound keeps a runaway depth from
    /// retaining unbounded storage after it unwinds.
    pub(super) const MAX_POOLED: usize = 32;

    pub(super) fn new() -> Self {
        Self(Rc::new(RefCell::new(Vec::new())))
    }

    pub(super) fn take(&self) -> Vec<Value> {
        self.0
            .borrow_mut()
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(Self::INITIAL_CAPACITY))
    }

    pub(super) fn recycle(&self, mut stack: Vec<Value>) {
        stack.clear();
        if stack.capacity() > Self::MAX_RECYCLED_CAPACITY {
            return;
        }
        let mut pooled = self.0.borrow_mut();
        if pooled.len() < Self::MAX_POOLED {
            pooled.push(stack);
        }
    }

    #[cfg(test)]
    pub(super) fn pooled_len(&self) -> usize {
        self.0.borrow().len()
    }
}

pub(super) struct OperandStack {
    recycler: OperandStackRecycler,
    values: Vec<Value>,
}

impl OperandStack {
    pub(super) fn new(bytecode: &Bytecode) -> Self {
        let recycler = bytecode.operand_stack_recycler();
        let values = recycler.take();
        Self { recycler, values }
    }

    pub(super) fn take(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.values)
    }

    pub(super) fn replace(&mut self, values: Vec<Value>) {
        let previous = std::mem::replace(&mut self.values, values);
        self.recycler.recycle(previous);
    }
}

impl Deref for OperandStack {
    type Target = Vec<Value>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl DerefMut for OperandStack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl Drop for OperandStack {
    fn drop(&mut self) {
        self.recycler.recycle(std::mem::take(&mut self.values));
    }
}
