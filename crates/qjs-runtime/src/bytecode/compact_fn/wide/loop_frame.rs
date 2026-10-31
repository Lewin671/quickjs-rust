//! A wide activation as the frame a typed loop program runs against.
//!
//! At a probed backedge the typed loop program runs against the wide
//! activation's own registers, rather than the tier handing the whole
//! activation to an interpreter frame for the typed loop tier to claim the
//! loop there. For a function called many times around a short loop -- a bit
//! count, a vector kernel -- building that frame and interpreting the rest of
//! the call costs more than the loop. The activation's locals are the frame's
//! own bindings, received cells are read through their cells, and anything
//! else declines.

use crate::bytecode::ir::Bytecode;
use crate::bytecode::typed_loop::LoopFrame;
use crate::bytecode::vm_bindings::TypedLoopSloppyGlobalWrite;
use crate::function::BindingCell;
use crate::{ArrayRef, ObjectRef, Property, RuntimeError, Value};

pub(super) struct WideLoopFrame<'a, 's> {
    pub(super) bytecode: &'a Bytecode,
    pub(super) env: &'a mut crate::context::Context<'s>,
    /// Registers `0..locals.len()` of the activation.
    pub(super) locals: &'a mut [Value],
    pub(super) own_locals: u128,
    /// The cells the activation received, indexed through the bytecode.
    pub(super) upvalues: &'a [BindingCell],
    pub(super) upvalue_slots: u128,
    /// The activation's receiver, which a program reads as the `this` name.
    pub(super) this_value: Option<&'a Value>,
    /// Where the program left the loop and the operand stack it left.
    pub(super) resume_ip: Option<usize>,
    pub(super) deoptimized: bool,
    /// The activation's operand-stack registers above the loop's base, which
    /// a leaving program rebuilds in place (`push_stack`), and how many it
    /// has pushed; `stack_overflowed` if more than fit.
    stack: &'a mut [Value],
    pub(super) pushed: usize,
    pub(super) stack_overflowed: bool,
    /// Backedges the program took before its loop exited normally.
    pub(super) iterations: Option<u64>,
    declined: u128,
    /// The realm's `Array.prototype` and whether its chain has an indexed
    /// property, resolved on the first element access that needs them: a
    /// program runs no user code, so neither changes while it runs.
    array_prototype: Option<Option<(ObjectRef, bool)>>,
}

impl<'a, 's> WideLoopFrame<'a, 's> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        bytecode: &'a Bytecode,
        env: &'a mut crate::context::Context<'s>,
        locals: &'a mut [Value],
        own_locals: u128,
        upvalues: &'a [BindingCell],
        upvalue_slots: u128,
        this_value: Option<&'a Value>,
        stack: &'a mut [Value],
    ) -> Self {
        Self {
            bytecode,
            env,
            locals,
            own_locals,
            upvalues,
            upvalue_slots,
            this_value,
            resume_ip: None,
            deoptimized: false,
            stack,
            pushed: 0,
            stack_overflowed: false,
            iterations: None,
            declined: 0,
            array_prototype: None,
        }
    }

    fn own(&self, slot: usize) -> bool {
        slot < self.locals.len()
            && slot < u128::BITS as usize
            && self.own_locals & (1_u128 << slot) != 0
    }

    fn cell(&self, slot: usize) -> Option<&BindingCell> {
        let bit = (slot < u128::BITS as usize).then(|| 1_u128 << slot)?;
        (self.upvalue_slots & bit != 0).then_some(())?;
        let index = self.bytecode.cell_received_upvalue_index(slot)?;
        self.upvalues.get(index)
    }
}

impl LoopFrame for WideLoopFrame<'_, '_> {
    fn loop_context(&mut self) -> crate::context::Context<'_> {
        self.env.reborrow()
    }
    fn loop_activation(&self) -> &crate::context::ActivationContext {
        self.env.activation
    }

    fn direct_eval_with_stack(&self) -> bool {
        false
    }

    fn declined_typed_loop_programs(&self) -> u128 {
        self.declined
    }

    fn decline_typed_loop_program(&mut self, bit: u128) {
        self.declined |= bit;
    }

    fn can_seed_slot(&self, slot: usize) -> bool {
        self.own(slot) || self.cell(slot).is_some()
    }

    fn local_slot_value(&self, slot: usize) -> Option<Value> {
        if self.own(slot) {
            let value = &self.locals[slot];
            return (!value.is_uninitialized_lexical_marker()).then(|| value.clone());
        }
        self.cell(slot).map(BindingCell::get)
    }

    fn slot_accepts_typed_loop_write(&self, slot: usize) -> bool {
        self.own(slot)
            && self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.mutable && !local.sloppy_global_fallback)
    }

    fn write_typed_loop_slot(&mut self, slot: usize, value: Value) {
        if self.own(slot) {
            self.locals[slot] = value;
        }
    }

    fn load_global(&mut self, name: &str) -> Result<Value, RuntimeError> {
        // The receiver is compiled as a read of the name `this`, which the
        // tier answers from the activation (`WideOp::LoadThis`).
        if name == "this" {
            return self.this_value.cloned().ok_or_else(|| RuntimeError {
                thrown: None,
                message: "compact activation has no receiver".to_owned(),
            });
        }
        crate::bytecode::compact_fn::property::load_global(name, self.env)
    }

    fn global_this_own_property(&self, name: &str) -> Option<Property> {
        match self.env.global_this() {
            Some(Value::Object(global_this)) => global_this.own_property(name),
            _ => None,
        }
    }

    fn array_access_is_plain(&mut self, array: &ArrayRef) -> bool {
        let env = &mut *self.env;
        let resolved = self.array_prototype.get_or_insert_with(|| {
            let prototype = crate::property::array_prototype(env)?;
            let hazard = crate::bytecode::vm_props::prototype_chain_has_index_hazard(Some(
                crate::Prototype::Object(prototype.clone()),
            ));
            Some((prototype, hazard))
        });
        let Some((prototype, hazard)) = resolved else {
            return false;
        };
        !*hazard && (array.uses_prototype_object(prototype))
    }

    // Admitted bodies write no sloppy globals, so a program that would is
    // left to the interpreter.
    fn prepare_typed_loop_sloppy_global_write(
        &self,
        _slot: usize,
        _name: &str,
    ) -> Option<TypedLoopSloppyGlobalWrite> {
        None
    }

    fn write_typed_loop_sloppy_global(
        &mut self,
        _target: &TypedLoopSloppyGlobalWrite,
        _value: Value,
    ) -> bool {
        false
    }

    fn record_sloppy_global_name(&mut self, _name: &str) {}

    fn push_stack(&mut self, value: Value) {
        match self.stack.get_mut(self.pushed) {
            Some(register) => crate::bytecode::compact_fn::execute::store(register, value),
            None => self.stack_overflowed = true,
        }
        self.pushed += 1;
    }

    fn ran_iterations(&mut self, iterations: u64) {
        self.iterations = Some(iterations);
    }

    fn resume_at(&mut self, ip: usize, deoptimized: bool) {
        self.resume_ip = Some(ip);
        self.deoptimized = deoptimized;
    }

    #[cfg(feature = "perf-counters")]
    fn bytecode_op(&self, ip: usize) -> Option<&crate::bytecode::ir::Op> {
        self.bytecode.code.get(ip)
    }
}
