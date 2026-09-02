//! Per-body storage pools handed to frames.
//!
//! A `Bytecode` owns the recyclers for the storage every activation of that
//! body needs -- its operand stack, its local slots, and its cold frame state
//! -- so a hot callee reuses the previous activation's allocations instead of
//! paying malloc/free pairs per call.

use crate::Value;

use super::ir::Bytecode;
use super::operand_stack::OperandStackRecycler;
use super::vm::ColdFrame;

impl Bytecode {
    /// A handle on this body's operand-stack recycler.
    ///
    /// Handing out the handle rather than the `Bytecode` is what lets a frame
    /// return its stack to the pool without holding a borrow of the bytecode
    /// for the frame's whole lifetime.
    pub(super) fn operand_stack_recycler(&self) -> OperandStackRecycler {
        self.operand_stack_pool.clone()
    }

    /// Takes `len` cleared local slots from this body's pool, or allocates
    /// them. Unlike the operand stack this is not handed out as a cloned
    /// handle: both call sites already hold the bytecode, and cloning an `Rc`
    /// twice per call to save one allocation is a wash.
    pub(super) fn take_local_slots(&self, len: usize) -> Vec<Option<Value>> {
        self.local_slot_pool.take(len)
    }

    pub(super) fn recycle_local_slots(&self, slots: Vec<Option<Value>>) {
        self.local_slot_pool.recycle(slots);
    }

    /// Takes a cleared cold-frame box from this body's pool, or allocates one.
    pub(super) fn take_cold_frame(&self) -> Box<ColdFrame> {
        self.cold_frame_pool.take()
    }

    /// Hands a completed frame's cold state back to the pool, cleared.
    pub(super) fn recycle_cold_frame(&self, cold: Box<ColdFrame>) {
        self.cold_frame_pool.recycle(cold);
    }
}
