//! The general interpreter: `Vm`, `FrameState`, and the dispatch loop.
//!
//! Entry points are `eval_bytecode` (scripts), `eval_function_bytecode`
//! (general calls) and `eval_direct_call_bytecode` (slot-seeded calls, which
//! try the compact tiers first). `Vm::run_current_activation` runs one frame
//! until it returns or suspends. It answers inline only the opcodes that
//! touch nothing but the operand stack, an authoritative local slot, or the
//! program counter. Every other opcode is an out-of-line call -- a
//! per-opcode `op_*` body for the hot ones, `run_general_op` and then
//! `run_rare_op` for the rest (`vm/`) -- with `self.ip` authoritative on
//! entry to and exit from that call.
//!
//! `FrameState` is built and dropped once per frame-building call, so state
//! most activations never touch lives in `ColdFrame`, behind one pointer.

use super::util::stack_underflow;
use super::vm_props::{
    array_index_from_number, array_index_from_string, get_property, get_property_key,
};
use super::vm_result::{Completion, FunctionBytecodeResult, ResumeMode};
use super::vm_set::set_property_key;
use super::vm_try::TryFrame;
use super::{
    DirectCallSlots,
    ir::{Bytecode, NamedPropertyCache, Op, decode_index_receiver},
};
use crate::{
    Function, ObjectRef, PropertyKey, RuntimeError, Value,
    function::{BindingCell, Realm},
    is_truthy,
    property::try_to_property_key_without_coercion,
    to_property_key_value,
    value::OwnDataPropertyWrite,
};
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};
pub(super) type Slot = Option<Value>;

mod general_ops;
mod rare_ops;
mod wide_resume;

pub(in crate::bytecode) use wide_resume::{
    ResumeFrom, Resumed, WideRegisters, resume_direct_call_bytecode,
};

use super::frame_program::{FrameBytecode, FrameProgramView};
use super::frame_stack::FrameExit;
use super::operand_stack::OperandStack;

pub(super) fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    let (mut runtime, activation) = crate::context::HostRoot::script().into_parts();
    let mut vm = Vm::new(bytecode, &mut runtime, activation)?;
    let value = vm.run()?;
    vm.drain_promise_jobs()?;
    Ok(value)
}
#[allow(clippy::too_many_arguments)]
pub(super) fn eval_function_bytecode(
    bytecode: &Bytecode,
    env: crate::context::ActivationContext,
    runtime: &mut crate::context::RuntimeServices,
    upvalues: Vec<BindingCell>,
    with_stack: Vec<Value>,
    persist_global_lexicals: bool,
    direct_call_slots: Option<DirectCallSlots<'_>>,
    call_inputs: Option<crate::function::CallInputs<'_>>,
) -> FunctionBytecodeResult {
    let direct_eval_with_stack = false;
    let mut vm = Vm::with_frame_bytecode(
        FrameBytecode::Borrowed(bytecode),
        runtime,
        env,
        upvalues,
        with_stack,
        direct_call_slots,
        call_inputs,
    );
    vm.persist_global_lexicals = persist_global_lexicals;
    // Ordinary functions created inside `with` are compiled with explicit
    // with-aware ops for free names; their own slot-indexed locals remain
    // closer than the retained object environment. Only direct-eval bytecode
    // needs generic load/store ops redirected through the caller's with stack.
    vm.direct_eval_with_stack = direct_eval_with_stack;
    let value = vm.run();
    let bound_this = vm.env.derived_this_cell().and_then(|cell| cell.read().ok());
    vm.recycle_binding_slots();
    if let Some(cold) = vm.current.cold.take() {
        bytecode.recycle_cold_frame(cold);
    }
    FunctionBytecodeResult { value, bound_this }
}

/// An ordinary synchronous caller consumes only the value. Keep completed
/// state in place and recycle cleared storage; escaped bindings remain in
/// their shared cells, whose identities are never pooled.
pub(super) fn eval_call_bytecode(
    bytecode: &Bytecode,
    env: crate::context::ActivationContext,
    runtime: &mut crate::context::RuntimeServices,
    upvalues: Vec<BindingCell>,
    with_stack: Vec<Value>,
    call_inputs: Option<crate::function::CallInputs<'_>>,
) -> Result<Value, RuntimeError> {
    let direct_eval_with_stack = false;
    let mut vm = Vm::with_frame_bytecode(
        FrameBytecode::Borrowed(bytecode),
        runtime,
        env,
        upvalues,
        with_stack,
        None,
        call_inputs,
    );
    vm.persist_global_lexicals = true;
    vm.direct_eval_with_stack = direct_eval_with_stack;
    let value = vm.run();
    vm.recycle_binding_slots();
    if let Some(cold) = vm.current.cold.take() {
        bytecode.recycle_cold_frame(cold);
    }
    value
}

/// Runs a guarded direct call and drops its completed frame in place.
#[inline(always)]
pub(super) fn eval_direct_call_bytecode(
    bytecode: &Bytecode,
    env: &mut Option<crate::context::ActivationContext>,
    runtime: &mut crate::context::RuntimeServices,
    slots: &mut Option<DirectCallSlots<'_>>,
) -> Result<Value, RuntimeError> {
    if let Some(result) = super::compact_fn::try_run_standalone(
        bytecode,
        env.as_mut().expect("prepared callee activation"),
        runtime,
        slots.as_mut().expect("prepared positional inputs"),
    ) {
        return result;
    }
    eval_direct_call_fallback(bytecode, env, runtime, slots)
}

/// Only a declined tier moves the actual callee and inputs into VM storage.
/// The admitted path borrows the owning entry without moving large records
/// through recursive dispatcher stack frames.
#[inline(never)]
fn eval_direct_call_fallback(
    bytecode: &Bytecode,
    entry: &mut Option<crate::context::ActivationContext>,
    runtime: &mut crate::context::RuntimeServices,
    slots: &mut Option<DirectCallSlots<'_>>,
) -> Result<Value, RuntimeError> {
    let env = entry.take().expect("prepared callee activation");
    let direct_call_slots = slots.take().expect("prepared positional inputs");
    let mut vm = Vm::new_with_globals_upvalues_with_stack_and_direct_call_slots(
        bytecode,
        runtime,
        env,
        Vec::new(),
        Vec::new(),
        Some(direct_call_slots),
    );
    let value = vm.run();
    // The direct-slot contract excludes closures over this frame's locals, so
    // nothing outlives the call and the allocation can go back to the body's
    // pool instead of being freed and rebuilt on the next invocation.
    vm.recycle_binding_slots();
    if let Some(cold) = vm.current.cold.take() {
        bytecode.recycle_cold_frame(cold);
    }
    value
}

/// `FrameState::slow_flags` bits, set by every writer that changes what the
/// dispatch loop must check before an op: a promotion (authority, and scope
/// tracking once it starts) or a scope update that changes authority. The
/// fast loop runs only while scope tracking and reference hooks are off and
/// leaves its interval as soon as an op sets a bit.
pub(super) const SLOW_SCOPES: u8 = 1;
pub(super) const SLOW_AUTHORITY: u8 = 1 << 1;

pub(super) struct FrameState<'a> {
    pub(super) slow_flags: u8,
    pub(super) reference_hooks: bool,
    pub(super) active_reference_count: usize,
    pub(super) binding_scope: Option<crate::binding::ScopeId>,
    pub(super) scope_instances: RefCell<Option<Box<super::vm_scopes::ScopeInstances>>>,
    pub(super) bytecode: FrameBytecode<'a>,
    /// Whether this frame may run the lowered stream that also virtualizes
    /// function literals. It is a selection input rather than a selection:
    /// the stream itself is derived per activation, from a bytecode owner the
    /// frame owns rather than borrows.
    pub(super) virtual_function_context_safe: bool,
    pub(super) ip: usize,
    /// One bit per typed loop program this frame has already declined, so a
    /// region the frame cannot run natively is not re-examined per iteration.
    pub(super) declined_typed_loop_programs: u128,
    pub(super) virtual_values: Vec<Value>,
    pub(super) stack: OperandStack,
    pub(super) locals: Vec<Slot>,
    pub(super) local_upvalues: Vec<Option<BindingCell>>,
    /// Direct frames that only read received cells retain the source function
    /// once and resolve cells by bytecode slot. This avoids rebuilding an
    /// owned local-sized option vector for every direct invocation while
    /// preserving the live cell identity.
    pub(super) direct_readonly_upvalue_owner: Option<Function>,
    pub(super) direct_readonly_upvalue_slots: u128,
    /// Inline per-slot cache for frames where indexed storage is the sole
    /// binding authority. The common first 128 slots require no allocation;
    /// larger slot indices conservatively use the full binding path.
    pub(super) authoritative_slots: u128,
    /// Inline per-slot cache for locals backed by this realm's shared binding
    /// cells. This turns ordinary global-var reads into a direct cell load;
    /// the cell's uninitialized marker still deoptimizes deleted/accessor
    /// globals through the observable global-object path.
    pub(super) realm_binding_slots: u128,
    /// The realm binding-table generation `realm_binding_slots` was derived
    /// at: while it is current, each such slot's cell is still the realm's
    /// cell for its name (`binding::table::DynamicBindings::generation`).
    pub(super) realm_binding_generation: u64,
    pub(super) upvalues: Vec<BindingCell>,
    /// Shared realm plus this frame's internal/caller-scope bindings.
    pub(super) env: crate::context::ActivationContext,
    /// Indexed-entry eligibility, never receiver storage. Receiver fast paths
    /// borrow the activation's canonical call owner through `indexed_receiver`.
    pub(super) indexed_call: bool,
    pub(super) realm: Realm,
    /// Dynamic-import host copied into every `crate::context::ActivationContext` this VM creates.
    pub(super) module_host: Option<crate::module::ModuleHostRef>,
    /// Test262 `$262.agent` context stamped onto every `crate::context::ActivationContext` this VM builds
    /// (via `attach_host`), so native `Atomics`/`$262.agent` hooks reach it.
    /// Cached realm Array.prototype for the `a[i] = x` fast path.
    pub(super) array_prototype_cache: Option<ObjectRef>,
    /// Cached intrinsic Object.prototype for object-literal construction.
    /// Mutable `Object` global rebinding does not invalidate the realm slot.
    pub(super) object_prototype_cache: Option<ObjectRef>,
    /// Makes generators run parameter prologues before first suspension.
    pub(super) stop_at_prologue: bool,
    /// State most activations never touch: `try`/`finally` bookkeeping,
    /// `with` scopes, generator resumes, `using` scopes, sloppy global names,
    /// and per-frame loop-plan overrides. Kept out of line so an ordinary
    /// call neither initializes nor drops it; see [`ColdFrame`].
    pub(super) cold: Option<Box<ColdFrame>>,
    /// True only for direct eval VMs that inherited an active with-chain from
    /// their caller. Ordinary functions created inside `with` also retain the
    /// chain, but their own local/global opcodes must not be dynamically
    /// re-resolved through it.
    pub(super) direct_eval_with_stack: bool,
    /// Whether global-scope lexical declarations should become persistent
    /// global lexical bindings. Indirect eval uses global-scope bytecode, but
    /// its lexical environment is ephemeral.
    pub(super) persist_global_lexicals: bool,
}

/// The part of a frame that an ordinary activation never reads or writes.
///
/// `FrameState` is built and dropped once per general-path call, and per-call
/// cost tracks its size, empty fields included. Do not add a field to
/// `FrameState` that an ordinary call never touches; put it here.
/// Everything here is reached only by `try`/`finally`,
/// `with`, generator suspension, `using`, sloppy-global recording, or a
/// deoptimized loop plan, so it lives behind one pointer that stays `None`
/// until the first such operation materializes it.
#[derive(Default)]
pub(super) struct ColdFrame {
    pub(super) class_regions: Vec<super::vm_class_region::ClassKeyContinuation>,
    pub(super) active_reference_lifetimes: Vec<usize>,
    pub(super) resolved_bindings: Vec<Option<crate::binding::ResolvedReference>>,
    /// Frame-local override of the shared numeric mutation loop plans,
    /// materialized only when a deoptimization suppresses or rewrites one for
    /// this invocation.
    pub(super) numeric_mutation_loop_plans:
        Option<Vec<super::vm_numeric_mutation_loop::NumericMutationLoopPlan>>,
    pub(super) sloppy_global_names: Vec<String>,
    pub(super) try_stack: Vec<TryFrame>,
    pub(super) pending_throw: Option<Value>,
    pub(super) pending_return: Option<Value>,
    /// Target IP for a break/continue routed through a finally block.
    pub(super) pending_jump: Option<usize>,
    /// Staged resume for a generator body suspended inside `yield*`.
    pub(super) resume_mode: Option<ResumeMode>,
    /// Explicit prototype required only for arrays created in a synthetic
    /// cross-realm VM, so ordinary `[]` stays on a cheap `None` branch.
    pub(super) array_literal_prototype_override: Option<ObjectRef>,
    /// Enclosing `with` object-environment records, innermost last.
    pub(super) with_stack: Vec<Value>,
    /// Active `using` disposal scopes (innermost last); each block's resources,
    /// disposed LIFO when the scope exits via the block's implicit finally.
    pub(super) disposable_scopes: Vec<Vec<super::vm_dispose::DisposeResource>>,
    /// Set for a frame continuing a wide activation from a probed backedge:
    /// `Some(None)` while it may hand a loop back to the tier, and
    /// `Some(Some(backedge))` once it has (`vm/wide_resume.rs`).
    pub(super) wide_handback: Option<Option<usize>>,
}

/// A per-body pool of cleared [`ColdFrame`] boxes, shared like the operand
/// stack recycler so a cloned `Bytecode` keeps handing out the same storage.
// The pool exists to hand the same `Box` allocation back to the next frame,
// so the boxes are the point, not an oversight.
#[allow(clippy::vec_box)]
#[derive(Clone, Default)]
pub(super) struct ColdFramePool(Rc<RefCell<Vec<Box<ColdFrame>>>>);

impl std::fmt::Debug for ColdFramePool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ColdFramePool")
    }
}

impl ColdFramePool {
    const MAX_POOLED: usize = 8;

    pub(super) fn take(&self) -> Box<ColdFrame> {
        crate::diagnostics::count!(cold_frame_pool_requests);
        self.0.borrow_mut().pop().unwrap_or_else(|| {
            crate::diagnostics::count!(cold_frame_allocations);
            Box::default()
        })
    }

    pub(super) fn recycle(&self, mut cold: Box<ColdFrame>) {
        cold.reset();
        let mut pooled = self.0.borrow_mut();
        if pooled.len() < Self::MAX_POOLED {
            pooled.push(cold);
        }
    }
}

impl ColdFrame {
    /// Clears every field so a pooled box starts the next frame empty. A
    /// frame that completed normally has already emptied its stacks, so this
    /// is a handful of length checks.
    pub(super) fn reset(&mut self) {
        self.class_regions.clear();
        self.resolved_bindings.clear();
        self.active_reference_lifetimes.clear();
        self.numeric_mutation_loop_plans = None;
        self.sloppy_global_names.clear();
        self.try_stack.clear();
        self.pending_throw = None;
        self.pending_return = None;
        self.pending_jump = None;
        self.resume_mode = None;
        self.array_literal_prototype_override = None;
        self.with_stack.clear();
        self.disposable_scopes.clear();
        self.wide_handback = None;
    }
}

impl<'a> FrameState<'a> {
    #[inline]
    pub(super) fn cold(&self) -> Option<&ColdFrame> {
        self.cold.as_deref()
    }

    #[inline]
    pub(super) fn cold_mut(&mut self) -> &mut ColdFrame {
        if self.cold.is_none() {
            self.materialize_cold();
        }
        self.cold
            .as_deref_mut()
            .unwrap_or_else(|| unreachable!("cold frame was just materialized"))
    }

    /// The pool round trip stays out of line: `cold_mut` is reached from the
    /// loop-plan dispatch the typed executor inlines, and letting the pool
    /// code into that executor measurably perturbs its register allocation.
    #[cold]
    #[inline(never)]
    fn materialize_cold(&mut self) {
        let bytecode: &Bytecode = &self.bytecode;
        self.cold = Some(bytecode.take_cold_frame());
    }

    #[inline]
    pub(super) fn with_stack(&self) -> &[Value] {
        self.cold
            .as_deref()
            .map_or(&[], |cold| cold.with_stack.as_slice())
    }

    #[inline]
    pub(super) fn sloppy_global_names(&self) -> &[String] {
        self.cold
            .as_deref()
            .map_or(&[], |cold| cold.sloppy_global_names.as_slice())
    }

    #[inline]
    pub(super) fn try_stack_is_empty(&self) -> bool {
        self.cold
            .as_deref()
            .is_none_or(|cold| cold.try_stack.is_empty())
    }

    #[inline]
    pub(super) fn take_pending_throw(&mut self) -> Option<Value> {
        self.cold
            .as_deref_mut()
            .and_then(|cold| cold.pending_throw.take())
    }

    #[inline]
    pub(super) fn take_pending_return(&mut self) -> Option<Value> {
        self.cold
            .as_deref_mut()
            .and_then(|cold| cold.pending_return.take())
    }

    #[inline]
    pub(super) fn take_pending_jump(&mut self) -> Option<usize> {
        self.cold
            .as_deref_mut()
            .and_then(|cold| cold.pending_jump.take())
    }

    #[inline]
    pub(super) fn take_resume_mode(&mut self) -> Option<ResumeMode> {
        self.cold
            .as_deref_mut()
            .and_then(|cold| cold.resume_mode.take())
    }

    /// Drops every staged abrupt completion without materializing cold state.
    #[inline]
    pub(super) fn clear_pending_abrupt(&mut self) {
        if let Some(cold) = self.cold.as_deref_mut() {
            cold.pending_throw = None;
            cold.pending_return = None;
            cold.pending_jump = None;
        }
    }
}

pub(super) struct Vm<'a, 's> {
    pub(super) runtime: &'s mut crate::context::RuntimeServices,
    pub(super) current: FrameState<'a>,
    /// Frames waiting for the one above them to finish. Empty until ordinary
    /// calls are routed onto this VM; the driver in `frame_stack` is what
    /// makes a non-empty stack meaningful.
    pub(super) callers: Vec<super::frame_stack::SuspendedFrame<'a>>,
    /// A frame a handler asked to enter, waiting for the driver to install it
    /// once the current activation's program view has been dropped.
    pub(super) pending_frame_entry: Option<(FrameState<'a>, super::frame_stack::FrameContinuation)>,
}

impl<'a, 's> Vm<'a, 's> {
    pub(super) fn into_frame(self) -> FrameState<'a> {
        self.current
    }
}

impl<'a, 's> Deref for Vm<'a, 's> {
    type Target = FrameState<'a>;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl DerefMut for Vm<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current
    }
}

impl<'a, 's> Vm<'a, 's> {
    pub(super) fn coerce_property_key(
        &mut self,
        value: Value,
    ) -> Result<PropertyKey, RuntimeError> {
        let value = match try_to_property_key_without_coercion(value) {
            Ok(key) => return Ok(key),
            Err(value) => value,
        };
        match value {
            Value::Object(_)
            | Value::Function(_)
            | Value::Array(_)
            | Value::Map(_)
            | Value::Set(_) => {
                let mut key_env = self.context();
                let key = to_property_key_value(value, &mut key_env)?;
                Ok(key)
            }
            value => {
                let mut key_env = self.context();
                to_property_key_value(value, &mut key_env)
            }
        }
    }

    pub(super) fn run(&mut self) -> Result<Value, RuntimeError> {
        if let Completion::Return(value) = self.run_completion()? {
            return Ok(value);
        }
        Err(RuntimeError {
            thrown: None,
            message: "yield evaluated outside a generator body".to_owned(),
        })
    }

    /// Runs one frame's bytecode until it returns or suspends.
    ///
    /// This is one *activation*: a generator body re-enters it on each resume,
    /// and the frame-stack driver re-enters it once per frame.
    ///
    /// Kept out of line so its register allocation is decided on its own terms
    /// and its disassembly stays inspectable.
    #[inline(never)]
    pub(super) fn run_current_activation(&mut self) -> Result<FrameExit, RuntimeError> {
        // Scope tracking and reference hooks are fixed for an activation
        // except that a promotion can start scope tracking, which ends the
        // fast loop's interval; the next one runs the checking loop.
        self.current.slow_flags = 0;
        if self.current.binding_scope.is_some() || self.current.reference_hooks {
            self.dispatch::<true>()
        } else {
            self.dispatch::<false>()
        }
    }

    /// The dispatch loop. `SLOW` checks scope changes, reference events and
    /// authority before every op; the fast instance checks nothing per op
    /// and leaves its interval after any out-of-line op that set a
    /// `slow_flags` bit.
    #[inline(never)]
    fn dispatch<const SLOW: bool>(&mut self) -> Result<FrameExit, RuntimeError> {
        // One owner clone per activation, held on this stack frame. The view
        // borrows the owner rather than the VM, which is what lets the current
        // instruction stay borrowed while its handler mutates VM state -- and
        // what lets a frame own its bytecode instead of borrowing it from a
        // lifetime that outlives the whole VM.
        //
        // Promotion ends this dispatch interval with RefreshBindings. The next
        // interval derives its stream from the new authority mask; no JS
        // instruction or observable reference selection is replayed.
        self.verify_binding_authority();
        let owner = self.current.bytecode.clone();
        let authority_at_entry = self.current.authoritative_slots;
        let program = FrameProgramView::new(
            &owner,
            self.current.authoritative_slots,
            self.current.virtual_function_context_safe,
        );
        // The program counter, the code pointer and the code length are
        // locals here rather than fields read through `self`, and `self.ip` is
        // resynchronized around exactly the opcodes that can observe it: the
        // ones answered below cannot, because they touch nothing but the
        // operand stack, an authoritative local slot, or `pc` itself.
        let code = program.execution_code;
        let constants: &[Value] = &program.bytecode.constants;
        let mut pc = self.current.ip;
        let references = &program.bytecode.binding_plan.references;
        let mut event_cursor = references.events.partition_point(|event| event.pc < pc);
        let mut expected_pc = pc;
        // Fixed for the activation (set only at frame entry and resume).
        let reference_hooks = self.current.reference_hooks;
        'dispatch: loop {
            debug_assert!(
                SLOW || (self.current.binding_scope.is_none()
                    && !self.current.reference_hooks
                    && self.current.authoritative_slots == authority_at_entry),
                "the fast dispatch loop outlived a slow-path change"
            );
            if SLOW {
                if let Some(active) = self.current.binding_scope
                    && program
                        .bytecode
                        .binding_plan
                        .pc_scopes
                        .get(pc)
                        .is_some_and(|scope| *scope != active)
                {
                    self.update_binding_scope(pc);
                    if self.current.authoritative_slots != authority_at_entry {
                        self.current.ip = pc;
                        return Ok(FrameExit::RefreshBindings);
                    }
                }
                if reference_hooks {
                    if pc != expected_pc {
                        event_cursor = references.events.partition_point(|event| event.pc < pc);
                    }
                    expected_pc = pc + 1;
                    if self.current.active_reference_count != 0 {
                        self.prune_references(references, pc);
                    }
                    if references
                        .events
                        .get(event_cursor)
                        .is_some_and(|event| event.pc == pc)
                    {
                        let event = &references.events[event_cursor];
                        event_cursor += 1;
                        self.current.ip = pc + 1;
                        let result = self.run_reference_event(references, event, &code[pc]);
                        match self.handle_runtime_result(result)? {
                            Some(false) => {}
                            _ => {
                                pc = self.current.ip;
                                continue;
                            }
                        }
                    }
                }
                // Every authority writer sets `SLOW_AUTHORITY`.
                if self.current.slow_flags & SLOW_AUTHORITY != 0
                    && self.current.authoritative_slots != authority_at_entry
                {
                    self.current.ip = pc;
                    return Ok(FrameExit::RefreshBindings);
                }
            }
            let Some(op) = code.get(pc) else {
                self.current.ip = pc;
                return Err(RuntimeError {
                    thrown: None,
                    message: "bytecode instruction pointer out of bounds".to_owned(),
                });
            };
            pc += 1;
            #[cfg(feature = "perf-counters")]
            crate::diagnostics::update(|c| c.executed_ops += 1);
            'out_of_line: {
                match op {
                    Op::LoadConst(index) => {
                        crate::diagnostics::count!(dispatched_load_const_ops);
                        if let Some(value) = constants.get(*index) {
                            let value = value.clone();
                            self.current.stack.push(value);
                            continue 'dispatch;
                        }
                    }
                    Op::LoadLocal(slot) => {
                        crate::diagnostics::count!(dispatched_local_binding_ops);
                        crate::diagnostics::count!(dispatched_load_local_ops);
                        // The general path answers with `Result<Value,
                        // RuntimeError>` (40 bytes) and `handle_runtime_result`
                        // rewraps it as `Result<Option<Value>, RuntimeError>` (48).
                        // Neither fits the two-register return, so the most
                        // frequently dispatched opcode in the engine paid two
                        // round trips through memory to report a success that
                        // cannot fail. Answer the authoritative-slot case here
                        // instead, and leave everything else out of line.
                        if !self.current.direct_eval_with_stack
                            && *slot < u128::BITS as usize
                            && self.current.authoritative_slots & (1_u128 << *slot) != 0
                            && let Some(Some(value)) = self.current.locals.get(*slot)
                        {
                            let value = super::vm_bindings::clone_local_value(value);
                            self.current.stack.push(value);
                            crate::diagnostics::count!(authoritative_load_local_hits);
                            continue 'dispatch;
                        }
                        self.current.ip = pc;
                        self.op_load_local(*slot)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::StoreLocal(slot) => {
                        crate::diagnostics::count!(dispatched_local_binding_ops);
                        crate::diagnostics::count!(dispatched_store_local_ops);
                        // Same rationale as `LoadLocal`: keep the
                        // authoritative-slot write off the memory-returned
                        // `Result` path. Falling through when the stack is empty is
                        // deliberate -- the general path raises the underflow.
                        let slot = *slot;
                        if slot < u128::BITS as usize
                            && self.current.authoritative_slots & (1_u128 << slot) != 0
                            && self
                                .current
                                .bytecode
                                .locals
                                .get(slot)
                                .is_some_and(|local| local.mutable)
                            && let Some(value) = self.current.stack.pop()
                        {
                            self.current.locals[slot] = Some(value);
                            crate::diagnostics::count!(authoritative_store_local_hits);
                            continue 'dispatch;
                        }
                        self.current.ip = pc;
                        self.op_store_local(slot)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::AssignLocal(slot) => {
                        crate::diagnostics::count!(dispatched_local_binding_ops);
                        crate::diagnostics::count!(dispatched_assign_local_ops);
                        // Mirrors `assign_local`'s own fast path, so an assignment
                        // to an initialized mutable slot never builds a `Result` to
                        // report that it succeeded.
                        let slot = *slot;
                        if !self.current.direct_eval_with_stack
                            && slot < u128::BITS as usize
                            && self.current.authoritative_slots & (1_u128 << slot) != 0
                            && self
                                .current
                                .bytecode
                                .locals
                                .get(slot)
                                .is_some_and(|local| local.mutable)
                            && self.current.locals.get(slot).is_some_and(Option::is_some)
                            && let Some(value) = self.current.stack.pop()
                        {
                            self.current.locals[slot] = Some(value);
                            crate::diagnostics::count!(authoritative_assign_local_hits);
                            continue 'dispatch;
                        }
                        self.current.ip = pc;
                        self.op_assign_local(slot)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::LoadGlobal(name) => {
                        crate::diagnostics::count!(dispatched_global_binding_ops);
                        self.current.ip = pc;
                        self.op_load_global(name)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::StoreGlobalStrict(name) => {
                        crate::diagnostics::count!(dispatched_global_binding_ops);
                        self.current.ip = pc;
                        self.op_store_global_strict(name)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::StoreGlobalSloppy { slot, name } => {
                        crate::diagnostics::count!(dispatched_global_binding_ops);
                        self.current.ip = pc;
                        self.op_store_global_sloppy(*slot, name)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::GetPropNamed { name, cache, .. } => {
                        crate::diagnostics::count!(dispatched_named_property_ops);
                        crate::diagnostics::count!(named_property_reads);
                        self.current.ip = pc;
                        self.op_get_prop_named(program.bytecode, *name, cache)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::GetPropIndex(index) => {
                        crate::diagnostics::count!(dispatched_computed_property_ops);
                        crate::diagnostics::count!(computed_property_reads);
                        self.current.ip = pc;
                        self.op_get_prop_index(*index)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::GetProp => {
                        crate::diagnostics::count!(dispatched_computed_property_ops);
                        crate::diagnostics::count!(computed_property_reads);
                        self.current.ip = pc;
                        self.op_get_prop()?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::SetProp { is_strict } => {
                        crate::diagnostics::count!(dispatched_computed_property_ops);
                        crate::diagnostics::count!(computed_property_writes);
                        self.current.ip = pc;
                        self.op_set_prop(*is_strict)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::SetPropIndex { index, is_strict } => {
                        crate::diagnostics::count!(dispatched_computed_property_ops);
                        crate::diagnostics::count!(computed_property_writes);
                        self.current.ip = pc;
                        self.op_set_prop_index(*index, *is_strict)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::SetPropNamed {
                        name,
                        cache,
                        is_strict,
                        ..
                    } => {
                        crate::diagnostics::count!(dispatched_named_property_ops);
                        crate::diagnostics::count!(named_property_writes);
                        self.current.ip = pc;
                        self.op_set_prop_named(
                            program.bytecode,
                            *name,
                            cache.as_ref(),
                            *is_strict,
                        )?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::Call(argc) => {
                        crate::diagnostics::count!(dispatched_call_construct_ops);
                        self.current.ip = pc;
                        self.call(*argc)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::CallResolved(argc) => {
                        crate::diagnostics::count!(dispatched_call_construct_ops);
                        self.current.ip = pc;
                        self.call_resolved(*argc)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::CallResolvedGuardedMathUnary => {
                        crate::diagnostics::count!(dispatched_call_construct_ops);
                        self.current.ip = pc;
                        self.call_resolved_guarded_math_unary()?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::New(argc) => {
                        crate::diagnostics::count!(dispatched_call_construct_ops);
                        self.current.ip = pc;
                        self.construct(*argc)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::Unary(op) => {
                        crate::diagnostics::count!(dispatched_numeric_ops);
                        self.current.ip = pc;
                        self.op_unary(*op)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::Return => {
                        crate::diagnostics::count!(dispatched_branch_return_ops);
                        self.current.ip = pc;
                        if let Some(value) = self.op_return()? {
                            return Ok(FrameExit::Completed(Completion::Return(value)));
                        }
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::Pop => {
                        crate::diagnostics::count!(dispatched_stack_ops);
                        if self.current.stack.pop().is_some() {
                            continue 'dispatch;
                        }
                        self.current.ip = pc;
                        return Err(stack_underflow());
                    }
                    Op::Dup => {
                        crate::diagnostics::count!(dispatched_stack_ops);
                        let stack = &mut *self.current.stack;
                        let Some(value) = stack.last() else {
                            self.current.ip = pc;
                            return Err(stack_underflow());
                        };
                        let value = super::vm_bindings::clone_local_value(value);
                        stack.push(value);
                        continue 'dispatch;
                    }
                    Op::ToNumeric => {
                        crate::diagnostics::count!(dispatched_numeric_ops);
                        // `eval_to_numeric` is the identity on a number, so the
                        // general path pops and re-pushes the same value through
                        // two memory-sized results to change nothing.
                        if matches!(self.current.stack.last(), Some(Value::Number(_))) {
                            continue 'dispatch;
                        }
                        self.current.ip = pc;
                        self.op_to_numeric()?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::Update(op) => {
                        crate::diagnostics::count!(dispatched_numeric_ops);
                        if let Some(Value::Number(number)) = self.current.stack.last_mut() {
                            *number = match op {
                                qjs_ast::UpdateOp::Increment => *number + 1.0,
                                qjs_ast::UpdateOp::Decrement => *number - 1.0,
                            };
                            continue 'dispatch;
                        }
                        self.current.ip = pc;
                        self.op_update(*op)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::Binary(op) => {
                        crate::diagnostics::count!(dispatched_numeric_ops);
                        // Number-number arithmetic rewrites the operand stack in
                        // place. The general path pops twice, calls
                        // `fast_number_binary`, and returns a memory-sized
                        // `Result`, which `handle_runtime_result` then rewraps --
                        // four values crossing a call boundary to add two floats
                        // that are already adjacent on the stack.
                        let stack = &mut *self.current.stack;
                        let len = stack.len();
                        if len >= 2
                            && let (Value::Number(left), Value::Number(right)) =
                                (&stack[len - 2], &stack[len - 1])
                            && let Some(value) =
                                super::vm_props::fast_number_binary_numbers(*left, *op, *right)
                        {
                            stack.truncate(len - 1);
                            stack[len - 2] = value;
                            continue 'dispatch;
                        }
                        self.current.ip = pc;
                        self.op_binary(*op)?;
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::Jump(target) => {
                        crate::diagnostics::count!(dispatched_branch_return_ops);
                        // A forward jump is the `if`/`else` and loop-exit shape and
                        // carries no accelerator; only a backward edge consults the
                        // loop plans, and that decision is out of line.
                        if *target >= pc - 1 {
                            pc = *target;
                            continue 'dispatch;
                        }
                        self.current.ip = pc;
                        if let Some(exit) = self.op_jump(&program, *target) {
                            return Ok(exit);
                        }
                        pc = self.current.ip;
                        break 'out_of_line;
                    }
                    Op::JumpIfFalse(target) => {
                        crate::diagnostics::count!(dispatched_branch_return_ops);
                        let Some(value) = self.current.stack.last() else {
                            self.current.ip = pc;
                            return Err(stack_underflow());
                        };
                        if !is_truthy(value) {
                            pc = *target;
                        }
                        continue 'dispatch;
                    }
                    Op::JumpIfTrue(target) => {
                        crate::diagnostics::count!(dispatched_branch_return_ops);
                        let Some(value) = self.current.stack.last() else {
                            self.current.ip = pc;
                            return Err(stack_underflow());
                        };
                        if is_truthy(value) {
                            pc = *target;
                        }
                        continue 'dispatch;
                    }
                    Op::JumpIfNotNullish(target) => {
                        crate::diagnostics::count!(dispatched_branch_return_ops);
                        if !matches!(
                            self.current.stack.last(),
                            Some(Value::Null | Value::Undefined)
                        ) {
                            pc = *target;
                        }
                        continue 'dispatch;
                    }
                    _ => {
                        crate::diagnostics::count!(dispatched_general_ops);
                    }
                }
                self.current.ip = pc;
                match self.run_general_op(op, &program)? {
                    Some(exit) => return Ok(exit),
                    None => pc = self.current.ip,
                }
                self.verify_binding_authority();
                if SLOW && self.current.authoritative_slots != authority_at_entry {
                    return Ok(FrameExit::RefreshBindings);
                }
            }
            // Only out-of-line work can promote a binding or start scope
            // tracking; the inline arms continue above. A promotion that
            // leaves authority unchanged and starts no scope tracking needs
            // no new interval.
            if !SLOW && self.current.slow_flags != 0 {
                if self.current.binding_scope.is_some()
                    || self.current.authoritative_slots != authority_at_entry
                {
                    self.current.ip = pc;
                    return Ok(FrameExit::RefreshBindings);
                }
                self.current.slow_flags = 0;
            }
        }
    }

    fn get_prop(&mut self) -> Result<(), RuntimeError> {
        let key_value = self.pop()?;
        let object = self.pop()?;
        if object.is_nullish() {
            let object_name = if object.is_null() {
                "null"
            } else {
                "undefined"
            };
            let key_name = match &key_value {
                Value::String(key) => Some(key.to_string()),
                Value::Number(number) => Some(number.to_string()),
                _ => None,
            };
            let message = match key_name {
                Some(key) => {
                    format!("TypeError: Cannot read properties of {object_name} (reading '{key}')")
                }
                None => format!("TypeError: cannot convert {object_name} to object"),
            };
            return Err(RuntimeError {
                thrown: None,
                message,
            });
        }
        if let Value::Number(number) = &key_value
            && let Some(index) = array_index_from_number(*number)
            && let Value::Array(elements) = &object
            && let Some(value) = elements.direct_dense_index_value(index)
        {
            self.stack.push(value);
            return Ok(());
        }
        // Typed-array integer-index read fast path: a non-negative integer index
        // is owned by the exotic [[Get]], so read it directly from the backing
        // buffer without building a string key or re-parsing it.
        if let Value::Number(number) = &key_value
            && let Some(index) = array_index_from_number(*number)
            && let Value::Object(object) = &object
            && crate::typed_array::is_typed_array_object(object)
        {
            let value = crate::typed_array::integer_indexed_value(object, index);
            self.stack.push(value);
            return Ok(());
        }
        // String integer-index read fast path: `text[i]` used to format the
        // number into a property key and parse it back into an index, which
        // cost a float-to-decimal conversion and two allocations per code unit
        // in a string-building loop. A string's own integer-keyed properties
        // are exactly its code units, so answer them directly.
        if let Value::Number(number) = &key_value
            && let Some(index) = array_index_from_number(*number)
            && let Value::String(text) = &object
            && let Some(code_unit) = crate::string::js_string_code_unit_at(text, index)
        {
            self.stack
                .push(Value::String(crate::string::js_string_from_code_unit(
                    code_unit,
                )));
            return Ok(());
        }
        // Fast path: a string-keyed read that the direct getter answers needs
        // no owned `PropertyKey`. Building one copies the string, so a
        // dictionary loop paid one allocation per access even when the lookup
        // itself is a borrow. `ToPropertyKey` on a string is already
        // side-effect free, so trying the borrow first is observably identical.
        if let Value::String(name) = &key_value
            && let Some(value) = self.try_direct_get_string(&object, name.as_str())
        {
            self.stack.push(value);
            return Ok(());
        }
        let key = self.coerce_property_key(key_value)?;
        let value = if let Some(value) = self.try_direct_get(&object, &key) {
            value
        } else if let PropertyKey::String(key) = &key
            && let Some(result) = self.try_direct_leaf_getter(&object, key)
        {
            result?
        } else {
            let mut env = self.context();

            get_property_key(object, &key, &mut env)?
        };
        self.stack.push(value);
        Ok(())
    }

    fn get_named_prop(
        &mut self,
        name: &crate::property::atom::Atom,
        cache: &NamedPropertyCache,
    ) -> Result<(), RuntimeError> {
        let key = name.as_str();
        let object = if let Some(slot) = cache.local_slot() {
            let direct_eval_lookup =
                self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot);
            if self.slot_is_authoritative(slot) && !direct_eval_lookup {
                let cached = {
                    let frame = &mut self.current;
                    match frame.locals.get(slot).and_then(Option::as_ref) {
                        Some(object) => {
                            if let Some(value) = object.unconditional_named_hook(name.as_str()) {
                                Some(value)
                            } else if let Value::Object(receiver) = object
                                && receiver.is_typed_array_exotic()
                                && name.as_str() == "length"
                            {
                                Self::cached_typed_array_length(object, receiver, cache)
                            } else {
                                let context = crate::context::Context {
                                    runtime: self.runtime,
                                    activation: &mut frame.env,
                                    native: None,
                                };
                                cache
                                    .get_value(object, Some(&context))
                                    .or_else(|| cache.read_value(object, name, Some(&context)))
                            }
                        }
                        _ => None,
                    }
                };
                if let Some(value) = cached {
                    self.stack.push(value);
                    return Ok(());
                }
            }
            self.load_local(slot)?
        } else {
            self.pop()?
        };
        if matches!(object, Value::Null | Value::Undefined) {
            let object_name = if matches!(object, Value::Null) {
                "null"
            } else {
                "undefined"
            };
            return Err(RuntimeError {
                thrown: None,
                message: format!(
                    "TypeError: Cannot read properties of {object_name} (reading '{key}')"
                ),
            });
        }
        let value = if let Some(value) = self.try_cached_get_named(&object, name, cache) {
            value
        } else if let Some(property) = cache.accessor(&object) {
            let mut env = self.context();

            crate::property::property_descriptor_value(Some(property), object, &mut env)?
        } else if let Some(result) = self.try_direct_leaf_getter(&object, key) {
            result?
        } else {
            let mut env = self.context();

            get_property(object, key, &mut env)?
        };
        self.stack.push(value);
        Ok(())
    }

    fn get_index_prop(&mut self, encoded_index: usize) -> Result<(), RuntimeError> {
        let (index, local_slot) = decode_index_receiver(encoded_index);
        let object = if let Some(slot) = local_slot {
            let direct_eval_lookup =
                self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot);
            if self.slot_is_authoritative(slot)
                && !direct_eval_lookup
                && let Some(Some(object)) = self.locals.get(slot)
            {
                let value = match object {
                    Value::Array(elements) => elements.direct_dense_index_value(index),
                    Value::Object(object) if crate::typed_array::is_typed_array_object(object) => {
                        Some(crate::typed_array::integer_indexed_value(object, index))
                    }
                    _ => None,
                };
                if let Some(value) = value {
                    self.stack.push(value);
                    return Ok(());
                }
            }
            self.load_local(slot)?
        } else {
            self.pop()?
        };
        if matches!(object, Value::Null | Value::Undefined) {
            let object_name = if matches!(object, Value::Null) {
                "null"
            } else {
                "undefined"
            };
            return Err(RuntimeError {
                thrown: None,
                message: format!(
                    "TypeError: Cannot read properties of {object_name} (reading '{index}')"
                ),
            });
        }
        if let Value::Array(elements) = &object
            && let Some(value) = elements.direct_dense_index_value(index)
        {
            self.stack.push(value);
            return Ok(());
        }
        if let Value::Object(object) = &object
            && crate::typed_array::is_typed_array_object(object)
        {
            let value = crate::typed_array::integer_indexed_value(object, index);
            self.stack.push(value);
            return Ok(());
        }

        let key = PropertyKey::String(index.to_string());
        let value = if let Some(value) = self.try_direct_get(&object, &key) {
            value
        } else {
            let mut env = self.context();

            get_property_key(object, &key, &mut env)?
        };
        self.stack.push(value);
        Ok(())
    }

    fn set_prop(&mut self, is_strict: bool) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let key_value = self.pop()?;
        // Fast path: writing a real array index to a plain array with the
        // default prototype, no own descriptor at that index, and no exotic
        // inherited index accessor. This is the dominant pattern in tight
        // `a[i] = x` append loops and computed compound updates. The latter
        // arrive as canonical string keys because the compiler must perform
        // observable `ToPropertyKey` exactly once before the read/write pair.
        let array_index = match &key_value {
            Value::Number(number) => array_index_from_number(*number),
            Value::String(key) => array_index_from_string(key),
            _ => None,
        };
        if let Some(index) = array_index
            && let Some(Value::Array(elements)) = self.stack.last()
            && elements.try_overwrite_dense_index(index, &value)
        {
            self.pop()?;
            self.stack.push(value);
            return Ok(());
        }
        if let Some(index) = array_index
            && let Some(Value::Array(elements)) = self.stack.last()
            && elements.dense_index_store_eligible(index)
        {
            let elements = elements.clone();
            // A plain array with the default prototype takes the dense-store fast
            // path when the index has no own special descriptor and the realm's
            // Array.prototype carries no own indexed property that an OrdinarySet
            // would have to honor. Both checks are O(1), so a tight `a[i] = x`
            // loop avoids the string-key allocation and prototype walk of the
            // generic path.
            if self.array_uses_realm_prototype(&elements)
                && !self
                    .array_prototype_chain_has_index_hazard()
                    .unwrap_or(true)
            {
                self.pop()?;
                elements.set(index, value.clone());
                self.stack.push(value);
                return Ok(());
            }
        }
        if let Value::Number(number) = &key_value
            && let Some(index) = array_index_from_number(*number)
            && let Some(Value::Object(object)) = self.stack.last()
            && crate::typed_array::is_typed_array_object(object)
        {
            let object = object.clone();
            self.set_typed_array_index(&object, index, &value, is_strict)?;
            self.pop()?;
            self.stack.push(value);
            return Ok(());
        }
        // Fast path: overwriting an existing own data property under a string
        // key, for the same reason as the read above -- the owned key is only
        // needed to *create* a property, and a dictionary loop overwrites.
        // This mirrors `set_property_value`'s own guard exactly, including
        // excluding the global object, whose writes go through bindings.
        if let Value::String(name) = &key_value
            && let Some(receiver @ Value::Object(_)) = self.stack.last()
            && !self.is_global_object(receiver)
            && let Value::Object(target) = receiver.clone()
            && matches!(
                target.write_existing_own_data_property(name.as_str(), &value),
                OwnDataPropertyWrite::Written
            )
        {
            self.pop()?;
            self.stack.push(value);
            return Ok(());
        }
        let key = self.coerce_property_key(key_value)?;
        let object = self.pop()?;
        self.set_property_value(object, key, value, is_strict)
    }

    fn set_index_prop(&mut self, index: usize, is_strict: bool) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let object = self.pop()?;

        if let Value::Array(elements) = &object
            && elements.try_overwrite_dense_index(index, &value)
        {
            self.stack.push(value);
            return Ok(());
        }
        if let Value::Array(elements) = &object
            && elements.dense_index_store_eligible(index)
        {
            let elements = elements.clone();
            // Mirror `set_prop`'s dense-array eligibility exactly. Custom own
            // descriptors, custom prototypes, and indexed Array.prototype
            // properties must retain the full OrdinarySet path below.
            if self.array_uses_realm_prototype(&elements)
                && !self
                    .array_prototype_chain_has_index_hazard()
                    .unwrap_or(true)
            {
                elements.set(index, value.clone());
                self.stack.push(value);
                return Ok(());
            }
        }

        if let Value::Object(typed_array) = &object
            && crate::typed_array::is_typed_array_object(typed_array)
        {
            let typed_array = typed_array.clone();
            self.set_typed_array_index(&typed_array, index, &value, is_strict)?;
            self.stack.push(value);
            return Ok(());
        }

        self.set_property_value(
            object,
            PropertyKey::String(index.to_string()),
            value,
            is_strict,
        )
    }

    /// Shared IntegerIndexedElementSet path for already-classified numeric
    /// indices. Both computed numeric keys and numeric-literal bytecode call
    /// this helper so primitive conversion, detached-buffer handling, and
    /// object coercion cannot drift between the two fast paths.
    fn set_typed_array_index(
        &mut self,
        object: &ObjectRef,
        index: usize,
        value: &Value,
        is_strict: bool,
    ) -> Result<(), RuntimeError> {
        let written = match crate::typed_array::try_set_integer_indexed_primitive_element(
            object, index, value,
        ) {
            Some(written) => written,
            None => {
                let mut env = self.context();

                crate::typed_array::set_integer_indexed_element(
                    object,
                    index,
                    value.clone(),
                    &mut env,
                )?
            }
        };
        if !written && is_strict {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot set property".to_owned(),
            });
        }
        Ok(())
    }

    pub(super) fn set_property_value(
        &mut self,
        object: Value,
        key: PropertyKey,
        value: Value,
        is_strict: bool,
    ) -> Result<(), RuntimeError> {
        if self.symbol_primitive_set_fails(&object, &key) {
            if is_strict {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: cannot assign property on Symbol primitive".to_owned(),
                });
            }
            self.stack.push(value);
            return Ok(());
        }
        let updates_global_binding = self.is_global_object(&object);
        if !updates_global_binding
            && let (Value::Object(object), PropertyKey::String(key)) = (&object, &key)
        {
            let domain = self.env.realm().atoms();
            let lease = domain.find(key).unwrap_or_else(|| domain.intern(key));
            match object
                .write_or_create_computed_data_key(&lease, &value)
                .result
            {
                OwnDataPropertyWrite::Written => {
                    self.stack.push(value);
                    return Ok(());
                }
                OwnDataPropertyWrite::ReadOnly => {
                    if is_strict {
                        return Err(RuntimeError {
                            thrown: None,
                            message: "TypeError: cannot set property".to_owned(),
                        });
                    }
                    self.stack.push(value);
                    return Ok(());
                }
                OwnDataPropertyWrite::NeedsSlowPath => {}
            }
        }
        let mut env = self.context();
        let wrote_data = set_property_key(object, key.clone(), value.clone(), &mut env)?;
        if !wrote_data && is_strict {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot set property".to_owned(),
            });
        }
        if updates_global_binding
            && wrote_data
            && let crate::PropertyKey::String(key) = key
            && !self.env.is_global_lexical_binding(&key)
        {
            // [[Set]] success also includes calling a setter. Only the
            // resulting own data descriptor can supply a realm cell value;
            // a setter may redefine/delete the property or change its value.
            match self.global_this_own_property(&key) {
                Some(property) if !property.is_accessor() => {
                    self.env.insert_realm(key, property.value);
                }
                _ => {
                    self.env.remove_realm(&key);
                }
            }
        }
        self.stack.push(value);
        Ok(())
    }

    fn construct(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let base = self
            .stack
            .len()
            .checked_sub(argc + 1)
            .ok_or_else(stack_underflow)?;
        let mut block = self.stack.take();
        let callee = std::mem::replace(&mut block[base], Value::Undefined);
        let result = self.invoke_construct_callee(callee, &block[base + 1..]);
        block.truncate(base);
        // Restore the owner without recycling the empty take() placeholder.
        *self.stack = block;
        if let Some(value) = self.handle_call_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    fn construct_spread(&mut self) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("constructor spread")?;
        let callee = self.pop()?;
        self.construct_callee(callee, arguments)
    }

    fn construct_callee(
        &mut self,
        callee: Value,
        arguments: Vec<Value>,
    ) -> Result<(), RuntimeError> {
        let result = self.invoke_construct_callee(callee, &arguments);
        if let Some(value) = self.handle_call_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    fn invoke_construct_callee(
        &mut self,
        callee: Value,
        arguments: &[Value],
    ) -> Result<Value, RuntimeError> {
        if let [Value::Number(date_value)] = arguments
            && matches!(
                &callee,
                Value::Function(function) if function.native_kind() == Some(crate::NativeFunction::Date)
            )
        {
            let mut env = self.context();
            let result =
                crate::date::fast_construct_date_from_number(callee.clone(), *date_value, &mut env);
            return result;
        }
        // An ordinary bytecode constructor admitted by the direct-leaf
        // predicate builds its receiver and runs on a slot-seeded frame, the
        // same activation an ordinary call gets, instead of the general
        // construct path; both borrow the same runtime services.
        if let Some(result) = crate::function::try_construct_direct_leaf_function(
            &callee,
            arguments,
            &mut self.context(),
        ) {
            return result;
        }
        let mut env = self.context();
        crate::function::construct_function_slice(callee.clone(), callee, arguments, &mut env)
    }

    pub(super) fn pop_arguments(&mut self, argc: usize) -> Result<Vec<Value>, RuntimeError> {
        let mut arguments = Vec::with_capacity(argc);
        for _ in 0..argc {
            arguments.push(self.pop()?);
        }
        arguments.reverse();
        Ok(arguments)
    }

    pub(super) fn pop_argument_array(&mut self, context: &str) -> Result<Vec<Value>, RuntimeError> {
        let value = self.pop()?;
        // Spreading walks the iterable's own iterator protocol, so it runs on
        // the callee's behalf rather than in the caller's lexical scope.
        let mut env = self.context();
        let arguments = crate::array::array_like_values_with_env(value, context, &mut env)?;
        Ok(arguments)
    }

    pub(super) fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(stack_underflow)
    }
}
