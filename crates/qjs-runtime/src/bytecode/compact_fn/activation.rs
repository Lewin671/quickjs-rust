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
use crate::function::{CallEnv, Function, Upvalue};
use crate::{RuntimeError, Value};

/// Everything an admitted body needs to run, and nothing a `FrameState`
/// carries for the general interpreter's sake.
pub(super) struct CompactActivation<'a> {
    pub(super) bytecode: &'a Bytecode,
    /// The environment this body runs in, borrowed rather than owned.
    ///
    /// It is still the callee's own environment in the sense that matters --
    /// see `shares_caller_environment`, which proves that what
    /// `direct_leaf_function_env` would build for an admitted callee is equal
    /// field for field to what its compact caller already holds. Borrowing it
    /// is therefore not "reusing the caller's environment"; it is skipping the
    /// reconstruction of an identical one.
    pub(super) env: &'a CallEnv,
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

/// The per-body facts this tier needs proven before it may run a call.
struct CompactEntry<'a> {
    program: &'a super::CompactFunctionProgram,
    upvalue_owner: Option<Function>,
    upvalue_slots: u128,
}

/// Proves a body may run on this tier with the given upvalue source.
fn admit<'a>(
    bytecode: &'a Bytecode,
    upvalues: crate::bytecode::DirectCallUpvalues<'_>,
) -> Option<CompactEntry<'a>> {
    let program = super::program_for(bytecode)?;
    // The body's received cells must be reachable from one retained function,
    // which is what makes an upvalue read a cell load rather than a binding
    // resolution.
    let upvalue_slots = bytecode
        .direct_readonly_received_upvalue_slots()
        .unwrap_or(0);
    let upvalue_owner = if upvalue_slots == 0 {
        None
    } else {
        let owner = upvalues.function()?;
        (owner.upvalues.len() == bytecode.received_upvalue_slots().len()
            && upvalues.as_slice().len() == owner.upvalues.len())
        .then(|| owner.clone())?
        .into()
    };
    // With no per-slot cells, the authoritative mask is the bytecode's own.
    if bytecode.authoritative_mask_clean() & !upvalue_slots & program.required_authoritative_slots
        != program.required_authoritative_slots
    {
        return None;
    }
    Some(CompactEntry {
        program,
        upvalue_owner,
        upvalue_slots,
    })
}

/// Whether an environment can host an admitted body at all.
fn environment_is_slot_only(env: &CallEnv) -> bool {
    env.supplies_no_named_binding()
        && !env.has_module_imports()
        && env.deopt_bindings().is_none()
        && env.dynamic_function_realm_global().is_none()
}

/// Whether `direct_leaf_function_env` would build, for this callee, an
/// environment equal field for field to the one its compact caller holds.
///
/// `new_direct_leaf_function_frame` derives realm, the global-lexical handles,
/// the immutable bindings, the module host, and the agent context from its
/// parent, and sets everything else to a fresh empty value. So two direct-leaf
/// frames in one realm differ only through what the four remaining steps of
/// `direct_leaf_function_env` add: a `this` binding, a marked call realm, a
/// module host, and a private environment. Excluding all four -- and empty
/// module imports, which the caller's environment is already known to have --
/// leaves the two environments indistinguishable.
fn shares_caller_environment(function: &Function, bytecode: &Bytecode, env: &CallEnv) -> bool {
    // `direct_leaf_function_env` installs the callee's own module host over
    // the one the frame inherited from its parent. When they are the same
    // handle -- which is the ordinary case, since a script's functions all
    // carry the host of the environment that created them -- that install is
    // idempotent and the two environments still agree.
    let host_agrees = match (&function.module_host, env.module_host()) {
        (None, _) => true,
        (Some(callee), Some(caller)) => std::rc::Rc::ptr_eq(callee, &caller),
        (Some(_), None) => false,
    };
    host_agrees
        && !bytecode.uses_lexical_this()
        && !function.has_dynamic_function_realm
        && !function.has_dynamic_function_realm_override.get()
        && function.module_imports.is_empty()
        && function.private_environment().is_none()
        && function.home_object().is_none()
}

/// Runs an admitted body in `env`.
fn run(
    bytecode: &Bytecode,
    env: &CallEnv,
    entry: CompactEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    crate::diagnostics::count!(compact_standalone_activations);
    let mut activation = CompactActivation {
        bytecode,
        env,
        upvalue_owner: entry.upvalue_owner,
        upvalue_slots: entry.upvalue_slots,
    };
    let program = entry.program;
    let mut registers = program.take_registers();
    // A recycled buffer already has the right length and is already all
    // `undefined`; only a fresh one needs growing.
    if registers.len() != program.register_count {
        registers.clear();
    }
    // Locals live in the low registers. Everything starts `undefined`, which
    // is already the correct seed for a hoisted `var`; parameters overwrite
    // theirs below, and a received upvalue is read from its cell rather than
    // from a register. `this` is not in the opcode set, so its slot is left
    // alone. That is the whole frame-setup cost for an admitted body.
    registers.resize(program.register_count, Value::Undefined);
    for (index, &slot) in parameter_slots.iter().enumerate() {
        let Some(target) = registers.get_mut(slot) else {
            continue;
        };
        if let Some(argument) = arguments.get(index) {
            *target = crate::bytecode::vm_bindings::clone_local_value(argument);
        }
    }
    let result = execute::execute(&mut activation, program, &mut registers);
    program.recycle_registers(registers);
    result
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
    // This tier resolves every name by slot index. An environment that can
    // still answer a name, that carries deoptimized dynamic bindings, or that
    // overrides the realm global belongs on the general path.
    if !environment_is_slot_only(env.as_ref()?) {
        return None;
    }
    let entry = admit(bytecode, slots.as_ref()?.upvalues)?;
    // Admitted. From here on the caller's `env` and `slots` are ours.
    let call_env = env.take()?;
    let call_slots = slots.take()?;
    Some(run(
        bytecode,
        &call_env,
        entry,
        call_slots.parameter_slots,
        call_slots.arguments,
    ))
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
    // A compact callee whose environment would be identical to this one runs
    // directly: no `CallEnv` construction, no `eval_direct_call_bytecode`, no
    // frame. `is_direct_leaf_function` stays the outer gate because it is what
    // proves seeding parameters into slots is safe for this callee at all --
    // default-parameter prologues and `arguments` objects are among the shapes
    // it rejects, and the compact program's own admission does not subsume it.
    if crate::function::is_direct_leaf_function(&callee)
        && let Value::Function(function) = &callee
        && let Some(callee_bytecode) = function.bytecode.as_ref()
        && shares_caller_environment(function, callee_bytecode, activation.env)
        && let Some(entry) = admit(
            callee_bytecode,
            crate::bytecode::DirectCallUpvalues::Function(function),
        )
    {
        // The attempt counter's whole job is to prove a workload really
        // performs the calls it claims, so it is raised for every dispatched
        // call whatever tier answers it. The tier attribution is
        // `compact_direct_calls`, not `direct_leaf_frames`: no frame is built.
        crate::diagnostics::count!(ordinary_call_attempts);
        crate::diagnostics::count!(compact_direct_calls);
        return run(
            callee_bytecode,
            activation.env,
            entry,
            callee_bytecode.parameter_slots(),
            arguments,
        );
    }
    if crate::function::is_direct_leaf_function(&callee) {
        return crate::function::call_direct_leaf_function(
            callee,
            Value::Undefined,
            arguments,
            activation.env,
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
