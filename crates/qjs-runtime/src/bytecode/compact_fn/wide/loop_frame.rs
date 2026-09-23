//! A wide activation as the frame a typed loop program runs against.
//!
//! At a probed backedge the tier used to hand the whole activation to an
//! interpreter frame so the typed loop tier could claim the loop there. For a
//! function called many times around a short loop -- a bit count, a vector
//! kernel -- building that frame and interpreting the rest of the call cost
//! more than the loop. The program now runs against the activation's own
//! registers: its locals are the frame's own bindings, received cells are read
//! through their cells, and anything else declines.

use std::rc::Rc;

use crate::bytecode::ir::Bytecode;
use crate::bytecode::typed_loop::LoopFrame;
use crate::bytecode::vm_bindings::TypedLoopSloppyGlobalWrite;
use crate::function::{CallEnv, Upvalue};
use crate::{ArrayRef, ObjectRef, Property, RuntimeError, Value};

pub(super) struct WideLoopFrame<'a> {
    pub(super) bytecode: &'a Bytecode,
    pub(super) env: &'a CallEnv,
    /// Registers `0..locals.len()` of the activation.
    pub(super) locals: &'a mut [Value],
    pub(super) own_locals: u128,
    /// The cells the activation received, indexed through the bytecode.
    pub(super) upvalues: &'a [Upvalue],
    pub(super) upvalue_slots: u128,
    /// The activation's receiver, which a program reads as the `this` name.
    pub(super) this_value: Option<&'a Value>,
    /// Where the program left the loop and the operand stack it left.
    pub(super) resume_ip: Option<usize>,
    pub(super) deoptimized: bool,
    pub(super) stack: Vec<Value>,
    declined: u128,
    /// The realm's `Array.prototype` and whether its chain has an indexed
    /// property, resolved on the first element access that needs them: a
    /// program runs no user code, so neither changes while it runs.
    array_prototype: Option<Option<(ObjectRef, bool)>>,
}

impl<'a> WideLoopFrame<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        bytecode: &'a Bytecode,
        env: &'a CallEnv,
        locals: &'a mut [Value],
        own_locals: u128,
        upvalues: &'a [Upvalue],
        upvalue_slots: u128,
        this_value: Option<&'a Value>,
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
            stack: Vec::new(),
            declined: 0,
            array_prototype: None,
        }
    }

    fn own(&self, slot: usize) -> bool {
        slot < self.locals.len()
            && slot < u128::BITS as usize
            && self.own_locals & (1_u128 << slot) != 0
    }

    fn cell(&self, slot: usize) -> Option<&Upvalue> {
        let bit = (slot < u128::BITS as usize).then(|| 1_u128 << slot)?;
        (self.upvalue_slots & bit != 0).then_some(())?;
        let index = self.bytecode.readonly_received_upvalue_index(slot)?;
        self.upvalues.get(index)
    }
}

impl LoopFrame for WideLoopFrame<'_> {
    fn loop_env(&self) -> &CallEnv {
        self.env
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
        self.cell(slot).map(Upvalue::get)
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
        let env = self.env;
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
        !*hazard && (array.uses_default_prototype() || array.uses_prototype_object(prototype))
    }

    fn try_create_ordinary_own_data_property(
        &self,
        object: &ObjectRef,
        key: Rc<str>,
        value: &Value,
    ) -> bool {
        crate::bytecode::compact_fn::property::try_create_ordinary_own_data_property(
            object, key, value,
        )
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
        self.stack.push(value);
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
