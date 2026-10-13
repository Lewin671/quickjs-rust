//! The frame a typed loop program runs against.
//!
//! A program seeds its registers from the frame's local slots and writes them
//! back when it leaves; everything else it needs from the frame is a handful
//! of realm facts. The interpreter's `Vm` is one such frame. The wide compact
//! tier is the other: its activation holds the same locals in registers and
//! runs a loop's program at the loop's backedge without building an
//! interpreter frame first.

use std::rc::Rc;

use super::super::vm::Vm;
use super::super::vm_bindings::TypedLoopSloppyGlobalWrite;
use crate::function::CallEnv;
use crate::{ArrayRef, ObjectRef, Property, RuntimeError, Value};

pub(in crate::bytecode) trait LoopFrame {
    /// The environment calls and realm lookups made by the program use.
    fn loop_env(&self) -> &CallEnv;
    /// Whether the frame evaluates direct eval code with a live operand
    /// stack, which no program may enter.
    fn direct_eval_with_stack(&self) -> bool;
    /// The programs this frame has stopped entering, one bit per program.
    fn declined_typed_loop_programs(&self) -> u128;
    fn decline_typed_loop_program(&mut self, bit: u128);
    /// Whether this frame can answer a read of `slot` at all. A slot it
    /// cannot read declines the program rather than reading as `undefined`.
    #[inline]
    fn can_seed_slot(&self, _slot: usize) -> bool {
        true
    }
    /// A local slot's value; `None` for an uninitialized binding.
    fn local_slot_value(&self, slot: usize) -> Option<Value>;
    /// Whether a plain store to `slot` is the whole observable effect.
    fn slot_accepts_typed_loop_write(&self, slot: usize) -> bool;
    fn write_typed_loop_slot(&mut self, slot: usize, value: Value);
    fn load_global(&mut self, name: &str) -> Result<Value, RuntimeError>;
    fn global_this_own_property(&self, name: &str) -> Option<Property>;
    /// Whether `array` uses the realm's `Array.prototype` and nothing on
    /// that chain has an own indexed property an element access could meet.
    fn array_access_is_plain(&mut self, array: &ArrayRef) -> bool;
    fn try_create_ordinary_own_data_property(
        &self,
        object: &ObjectRef,
        key: Rc<str>,
        value: &Value,
    ) -> bool;
    fn prepare_typed_loop_sloppy_global_write(
        &self,
        slot: usize,
        name: &str,
    ) -> Option<TypedLoopSloppyGlobalWrite>;
    fn write_typed_loop_sloppy_global(
        &mut self,
        target: &TypedLoopSloppyGlobalWrite,
        value: Value,
    ) -> bool;
    fn record_sloppy_global_name(&mut self, name: &str);
    /// Leaves the program: the operand stack the bytecode at `ip` expects,
    /// value by value from the bottom, then the instruction to resume at.
    fn push_stack(&mut self, value: Value);
    fn resume_at(&mut self, ip: usize, deoptimized: bool);
    /// How many times the program went round its backedge before leaving the
    /// loop normally; a frame that chooses between tiers per loop keeps it.
    fn ran_iterations(&mut self, _iterations: u64) {}
    /// The bytecode instruction at `ip`, for diagnostics.
    #[cfg(feature = "perf-counters")]
    fn bytecode_op(&self, ip: usize) -> Option<&super::super::ir::Op>;
}

impl LoopFrame for Vm<'_> {
    #[inline]
    fn loop_env(&self) -> &CallEnv {
        &self.env
    }

    #[inline]
    fn direct_eval_with_stack(&self) -> bool {
        self.direct_eval_with_stack
    }

    #[inline]
    fn declined_typed_loop_programs(&self) -> u128 {
        self.declined_typed_loop_programs
    }

    #[inline]
    fn decline_typed_loop_program(&mut self, bit: u128) {
        self.declined_typed_loop_programs |= bit;
    }

    #[inline]
    fn local_slot_value(&self, slot: usize) -> Option<Value> {
        Vm::local_slot_value(self, slot)
    }

    #[inline]
    fn slot_accepts_typed_loop_write(&self, slot: usize) -> bool {
        Vm::slot_accepts_typed_loop_write(self, slot)
    }

    #[inline]
    fn write_typed_loop_slot(&mut self, slot: usize, value: Value) {
        Vm::write_typed_loop_slot(self, slot, value);
    }

    #[inline]
    fn load_global(&mut self, name: &str) -> Result<Value, RuntimeError> {
        Vm::load_global(self, name)
    }

    #[inline]
    fn global_this_own_property(&self, name: &str) -> Option<Property> {
        Vm::global_this_own_property(self, name)
    }

    #[inline]
    fn array_access_is_plain(&mut self, array: &ArrayRef) -> bool {
        self.array_uses_realm_prototype(array)
            && !self
                .array_prototype_chain_has_index_hazard()
                .unwrap_or(true)
    }

    #[inline]
    fn try_create_ordinary_own_data_property(
        &self,
        object: &ObjectRef,
        key: Rc<str>,
        value: &Value,
    ) -> bool {
        Vm::try_create_ordinary_own_data_property(self, object, key, value)
    }

    #[inline]
    fn prepare_typed_loop_sloppy_global_write(
        &self,
        slot: usize,
        name: &str,
    ) -> Option<TypedLoopSloppyGlobalWrite> {
        Vm::prepare_typed_loop_sloppy_global_write(self, slot, name)
    }

    #[inline]
    fn write_typed_loop_sloppy_global(
        &mut self,
        target: &TypedLoopSloppyGlobalWrite,
        value: Value,
    ) -> bool {
        Vm::write_typed_loop_sloppy_global(self, target, value)
    }

    #[inline]
    fn record_sloppy_global_name(&mut self, name: &str) {
        Vm::record_sloppy_global_name(self, name);
    }

    #[inline]
    fn push_stack(&mut self, value: Value) {
        self.stack.push(value);
    }

    #[inline]
    fn resume_at(&mut self, ip: usize, _deoptimized: bool) {
        self.ip = ip;
    }

    #[cfg(feature = "perf-counters")]
    fn bytecode_op(&self, ip: usize) -> Option<&super::super::ir::Op> {
        self.current.bytecode.code.get(ip)
    }
}
