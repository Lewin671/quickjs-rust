//! A VM-free activation for bodies the compact tier admits.
//!
//! The compact executor removed generic dispatch and bought ~5% on the
//! recursive sentinel, which established that dispatch is not what makes a
//! recursive call slow: at ~212 ns per call against QuickJS-NG's ~35 ns,
//! roughly two thirds of the remaining time is spent building and tearing down
//! the activation itself (`tasks/T021-single-vm-frame-stack.md`).
//!
//! A `Vm` carries a 704-byte `FrameState` with 36 fields -- unwinding state,
//! suspension state, loop-plan decline bitsets, prototype caches, an operand
//! stack -- and an admitted compact body can use none of them. It has no
//! handler, cannot suspend, runs no loop plans, and keeps its operands in
//! registers. This module gives such a body the four things it actually needs
//! and nothing else.
//!
//! The calling convention is still nested: a call from here re-enters the
//! ordinary call path, and a callee that is itself compact builds its own
//! activation. Turning that nesting into one loop is a later unit.

use qjs_ast::BinaryOp;

use super::execute;
use crate::bytecode::DirectCallSlots;
use crate::bytecode::ir::Bytecode;
use crate::bytecode::vm::{Slot, Vm};
use crate::function::{CallEnv, Function, Upvalue};
use crate::{RuntimeError, Value};

/// Everything an admitted body needs to run, and nothing a `FrameState`
/// carries for the general interpreter's sake.
pub(super) struct CompactActivation<'a> {
    pub(super) bytecode: &'a Bytecode,
    /// The callee's own environment, built by `direct_leaf_function_env`. It is
    /// not the caller's: it carries `this` normalization, creation-realm
    /// selection, module-host routing, and the private environment.
    pub(super) env: CallEnv,
    pub(super) locals: Vec<Slot>,
    /// The function whose upvalue vector backs this body's received cells.
    /// Reads resolve through it by bytecode slot, keeping cell identity live.
    upvalue_owner: Option<Function>,
    upvalue_slots: u128,
}

impl<'a> CompactActivation<'a> {
    /// Returns the live cell for a received-upvalue slot.
    pub(super) fn upvalue_cell(&self, slot: usize) -> Option<&Upvalue> {
        let owner = self.upvalue_owner.as_ref()?;
        let bit = (slot < u128::BITS as usize).then(|| 1_u128 << slot)?;
        (self.upvalue_slots & bit != 0).then_some(())?;
        let index = self.bytecode.direct_readonly_received_upvalue_index(slot)?;
        owner.upvalues.get(index)
    }

    /// Evaluates one non-numeric binary operation.
    ///
    /// Binary coercion never runs in the caller's lexical environment -- user
    /// hooks carry their own cells and global effects go through the realm --
    /// so an empty realm frame is both correct and what the interpreter's own
    /// path uses.
    pub(super) fn eval_binary(
        &self,
        left: Value,
        op: BinaryOp,
        right: Value,
    ) -> Result<Value, RuntimeError> {
        let mut env = self.env.empty_frame();
        crate::operations::eval_binary(left, op, right, &mut env)
    }
}

/// Runs `bytecode` without building a `Vm`, or returns `None` having consumed
/// nothing, so the caller can construct the ordinary frame unchanged.
///
/// `env` and `slots` are taken only once admission is certain. Every guard runs
/// before any of them is consumed and before any observable work.
pub(in crate::bytecode) fn try_run_standalone(
    bytecode: &Bytecode,
    env: &mut Option<CallEnv>,
    slots: &mut Option<DirectCallSlots<'_>>,
) -> Option<Result<Value, RuntimeError>> {
    let program = super::program_for(bytecode)?;
    let call_env = env.as_ref()?;
    let call_slots = slots.as_ref()?;

    // This tier resolves every name by slot index. An environment that can
    // still answer a name, that carries deoptimized dynamic bindings, or that
    // overrides the realm global belongs on the general path.
    if !call_env.supplies_no_named_binding()
        || call_env.has_module_imports()
        || call_env.deopt_bindings().is_some()
        || call_env.dynamic_function_realm_global().is_some()
    {
        return None;
    }
    // The body's received cells must be reachable from one retained function,
    // which is what makes an upvalue read a cell load rather than a binding
    // resolution.
    let upvalue_slots = bytecode
        .direct_readonly_received_upvalue_slots()
        .unwrap_or(0);
    let upvalue_owner = if upvalue_slots == 0 {
        None
    } else {
        let owner = call_slots.upvalues.function()?;
        (owner.upvalues.len() == bytecode.received_upvalue_slots().len()
            && call_slots.upvalues.as_slice().len() == owner.upvalues.len())
        .then(|| owner.clone())?
        .into()
    };
    // With no per-slot cells, the authoritative mask is the bytecode's own.
    if bytecode.authoritative_mask_clean() & !upvalue_slots & program.required_authoritative_slots
        != program.required_authoritative_slots
    {
        return None;
    }

    // Admitted. From here on the caller's `env` and `slots` are ours.
    let call_env = env.take()?;
    let call_slots = slots.take()?;
    crate::diagnostics::count!(compact_standalone_activations);
    let mut locals = Vm::initial_direct_call_slots(bytecode);
    // An admitted body never reads `this` -- `LoadThis` is not in the opcode
    // set -- so the seeded receiver is discarded rather than retained.
    let _ = Vm::seed_direct_call_slots(bytecode, &mut locals, call_slots);
    let mut activation = CompactActivation {
        bytecode,
        env: call_env,
        locals,
        upvalue_owner,
        upvalue_slots,
    };

    let mut registers = program.take_registers();
    registers.clear();
    registers.resize(program.register_count, Value::Undefined);
    let result = execute::execute(&mut activation, program, &mut registers);
    program.recycle_registers(registers);
    bytecode.recycle_local_slots(std::mem::take(&mut activation.locals));
    Some(result)
}

/// Runs one call out of an activation.
///
/// A slot-seeded direct-leaf callee -- every call in a recursive admitted body
/// -- reaches `call_direct_leaf_function`, which takes an argument slice
/// directly. Every other callee shape goes through `call_function`, which is
/// the same entry the interpreter's general call path reaches, so native,
/// bound, Proxy, and class-constructor behaviour keeps one implementation.
#[inline(never)]
pub(super) fn call_from_activation(
    activation: &mut CompactActivation<'_>,
    callee: Value,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    if crate::function::is_direct_leaf_function(&callee) {
        return crate::function::call_direct_leaf_function(
            callee,
            Value::Undefined,
            arguments,
            &activation.env,
            activation.env.module_host(),
            #[cfg(feature = "agents")]
            activation.env.agent_context(),
        );
    }
    let mut env = activation.env.empty_frame();
    crate::function::call_function(
        callee,
        Value::Undefined,
        arguments.to_vec(),
        &mut env,
        false,
    )
}
