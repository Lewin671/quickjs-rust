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
    Function, ObjectRef, PropertyKey, RuntimeError, Value, construct_function,
    function::{CallEnv, Realm, Upvalue},
    is_truthy,
    property::try_to_property_key_without_coercion,
    to_property_key_value,
    value::OwnDataPropertyWrite,
};
use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};
pub(super) type Slot = Option<Value>;

mod general_ops;
mod rare_ops;

use super::frame_program::{FrameBytecode, FrameProgramView};
use super::frame_stack::FrameExit;
use super::operand_stack::OperandStack;
use super::vm_call_env::{VmCallEnv, VmCallEnvOrigin};

pub(super) fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode)?;
    let value = vm.run()?;
    vm.persist_global_lexical_bindings();
    vm.drain_promise_jobs()?;
    Ok(value)
}
pub(super) fn eval_function_bytecode<'a>(
    bytecode: &'a Bytecode,
    env: CallEnv,
    upvalues: Vec<Upvalue>,
    with_stack: Vec<Value>,
    persist_global_lexicals: bool,
    direct_call_slots: Option<DirectCallSlots<'_>>,
) -> FunctionBytecodeResult<'a> {
    let direct_eval_with_stack = !env.direct_eval_with_stack().is_empty();
    let mut vm = Vm::new_with_globals_upvalues_with_stack_and_direct_call_slots(
        bytecode,
        env,
        upvalues,
        with_stack,
        direct_call_slots,
    );
    vm.persist_global_lexicals = persist_global_lexicals;
    // Ordinary functions created inside `with` are compiled with explicit
    // with-aware ops for free names; their own slot-indexed locals remain
    // closer than the retained object environment. Only direct-eval bytecode
    // needs generic load/store ops redirected through the caller's with stack.
    vm.direct_eval_with_stack = direct_eval_with_stack;
    let value = vm.run();
    // Move the four fields the caller needs straight out of the frame. Binding
    // the whole `FrameState` first materializes several hundred bytes of it in
    // this function's own frame, which a profile charges to `memmove` on every
    // call.
    let FrameState {
        env,
        locals,
        local_upvalues,
        sloppy_global_names,
        ..
    } = vm.current;
    FunctionBytecodeResult {
        value,
        bytecode,
        env,
        locals,
        local_upvalues,
        sloppy_global_names,
    }
}

/// Runs a guarded direct call and drops its completed frame in place.
pub(super) fn eval_direct_call_bytecode(
    bytecode: &Bytecode,
    env: CallEnv,
    direct_call_slots: DirectCallSlots<'_>,
) -> Result<Value, RuntimeError> {
    // A body the compact tier admits runs without a `Vm` at all: it has no
    // handler, cannot suspend, runs no loop plans, and keeps its operands in
    // registers, so none of `FrameState`'s 704 bytes would be read. Admission
    // is decided before `env` or the slots are consumed, so a declined body
    // builds exactly the frame it always did.
    let mut pending_env = Some(env);
    let mut pending_slots = Some(direct_call_slots);
    if let Some(result) =
        super::compact_fn::try_run_standalone(bytecode, &mut pending_env, &mut pending_slots)
    {
        return result;
    }
    let env = pending_env.expect("a declined standalone activation consumes nothing");
    let direct_call_slots =
        pending_slots.expect("a declined standalone activation consumes nothing");
    let mut vm = Vm::new_with_globals_upvalues_with_stack_and_direct_call_slots(
        bytecode,
        env,
        Vec::new(),
        Vec::new(),
        Some(direct_call_slots),
    );
    let value = vm.run();
    // The direct-slot contract excludes closures over this frame's locals, so
    // nothing outlives the call and the allocation can go back to the body's
    // pool instead of being freed and rebuilt on the next invocation.
    bytecode.recycle_local_slots(std::mem::take(&mut vm.current.locals));
    value
}

pub(super) struct FrameState<'a> {
    pub(super) bytecode: FrameBytecode<'a>,
    /// Whether this frame may run the lowered stream that also virtualizes
    /// function literals. It is a selection input rather than a selection:
    /// the stream itself is derived per activation, from a bytecode owner the
    /// frame owns rather than borrows.
    pub(super) virtual_function_context_safe: bool,
    pub(super) ip: usize,
    /// Two saturating decline counters per numeric loop plan, for the first
    /// 64 plans. A plan that matched an instruction range but could not run
    /// rebuilds its whole preparation state -- write targets, forbidden cells,
    /// prepared terms -- before discovering that again on the next backedge,
    /// which is every iteration of a loop containing a call. Plans are pure
    /// accelerators, so a frame stops retrying one after it has declined
    /// three times; the retries cover a plan that only becomes admissible once
    /// the loop's values settle.
    pub(super) declined_numeric_loop_plans: u128,
    /// One bit per typed loop program this frame has already declined, so a
    /// region the frame cannot run natively is not re-examined per iteration.
    pub(super) declined_typed_loop_programs: u128,
    /// Frame-local override of the shared numeric mutation loop plans,
    /// materialized only when a deoptimization suppresses or rewrites one for
    /// this invocation. Ordinary frames leave this `None`.
    pub(super) numeric_mutation_loop_plans:
        Option<Vec<super::vm_numeric_mutation_loop::NumericMutationLoopPlan>>,
    pub(super) virtual_values: Vec<Value>,
    pub(super) stack: OperandStack,
    pub(super) locals: Vec<Slot>,
    pub(super) local_upvalues: Vec<Option<Upvalue>>,
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
    pub(super) upvalues: Vec<Upvalue>,
    /// Shared realm plus this frame's internal/caller-scope bindings.
    pub(super) env: CallEnv,
    /// Ordinary leaf calls can keep their receiver here instead of
    /// materializing a name-keyed frame binding. Functions that compile an
    /// own `this` local store it in `locals` and leave this empty.
    pub(super) direct_this: Option<Value>,
    pub(super) realm: Realm,
    /// Dynamic-import host copied into every `CallEnv` this VM creates.
    pub(super) module_host: Option<crate::module::ModuleHostRef>,
    /// Test262 `$262.agent` context stamped onto every `CallEnv` this VM builds
    /// (via `attach_host`), so native `Atomics`/`$262.agent` hooks reach it.
    #[cfg(feature = "agents")]
    pub(super) agent_context: Option<crate::agent::AgentContextRef>,
    pub(super) sloppy_global_names: Vec<String>,
    pub(super) try_stack: Vec<TryFrame>,
    pub(super) pending_throw: Option<Value>,
    pub(super) pending_return: Option<Value>,
    /// Target IP for a break/continue routed through a finally block.
    pub(super) pending_jump: Option<usize>,
    /// Staged resume for a generator body suspended inside `yield*`.
    pub(super) resume_mode: Option<ResumeMode>,
    /// Cached realm Array.prototype for the `a[i] = x` fast path.
    pub(super) array_prototype_cache: Option<ObjectRef>,
    /// Explicit prototype required only for arrays created in a synthetic
    /// cross-realm VM. Precomputed once so ordinary `[]` stays on a cheap
    /// `None` branch instead of consulting realm metadata per allocation.
    pub(super) array_literal_prototype_override: Option<ObjectRef>,
    /// Cached intrinsic Object.prototype for object-literal construction.
    /// Mutable `Object` global rebinding does not invalidate the realm slot.
    pub(super) object_prototype_cache: Option<ObjectRef>,
    /// Makes generators run parameter prologues before first suspension.
    pub(super) stop_at_prologue: bool,
    /// Enclosing `with` object-environment records, innermost last.
    pub(super) with_stack: Vec<Value>,
    /// True only for direct eval VMs that inherited an active with-chain from
    /// their caller. Ordinary functions created inside `with` also retain the
    /// chain, but their own local/global opcodes must not be dynamically
    /// re-resolved through it.
    pub(super) direct_eval_with_stack: bool,
    /// Active `using` disposal scopes (innermost last); each block's resources,
    /// disposed LIFO when the scope exits via the block's implicit finally.
    pub(super) disposable_scopes: Vec<Vec<super::vm_dispose::DisposeResource>>,
    /// Whether global-scope lexical declarations should become persistent
    /// global lexical bindings. Indirect eval uses global-scope bytecode, but
    /// its lexical environment is ephemeral.
    pub(super) persist_global_lexicals: bool,
    /// Only a fresh ordinary script VM may batch realm-global loop writes.
    /// Eval, module, dynamic-function, and cross-realm entry points construct
    /// their frames through `new_with_globals*` and leave this disabled.
    pub(super) transactional_realm_globals: bool,
    /// Dynamic source evaluation can replace global descriptors and binding
    /// identities outside the current bytecode stream. Once observed, guarded
    /// realm-global loop batching stays disabled for the rest of this frame.
    pub(super) dynamic_code_executed: bool,
}

pub(super) struct Vm<'a> {
    pub(super) current: FrameState<'a>,
    /// Frames waiting for the one above them to finish. Empty until ordinary
    /// calls are routed onto this VM; the driver in `frame_stack` is what
    /// makes a non-empty stack meaningful.
    pub(super) callers: Vec<super::frame_stack::SuspendedFrame<'a>>,
    /// A frame a handler asked to enter, waiting for the driver to install it
    /// once the current activation's program view has been dropped.
    pub(super) pending_frame_entry: Option<(FrameState<'a>, super::frame_stack::FrameContinuation)>,
}

impl<'a> Vm<'a> {
    pub(super) fn into_frame(self) -> FrameState<'a> {
        self.current
    }
}

impl<'a> Deref for Vm<'a> {
    type Target = FrameState<'a>;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl DerefMut for Vm<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current
    }
}

impl<'a> Vm<'a> {
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
                let mut key_env = self.callee_env();
                let key = to_property_key_value(value, &mut key_env)?;
                self.apply_env(key_env);
                Ok(key)
            }
            value => {
                let mut key_env = self.callee_env();
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
    /// -- and stays measurable: the whole point of the split below is what this
    /// function's disassembly does per dispatch.
    #[inline(never)]
    pub(super) fn run_current_activation(&mut self) -> Result<FrameExit, RuntimeError> {
        // One owner clone per activation, held on this stack frame. The view
        // borrows the owner rather than the VM, which is what lets the current
        // instruction stay borrowed while its handler mutates VM state -- and
        // what lets a frame own its bytecode instead of borrowing it from a
        // lifetime that outlives the whole VM.
        //
        // Deriving here is correct because the selection inputs cannot change
        // during an activation: `refresh_virtual_object_execution` runs only
        // during generator setup and resume, before this loop starts.
        let owner = self.current.bytecode.clone();
        let program = FrameProgramView::new(
            &owner,
            self.current.authoritative_slots,
            self.current.virtual_function_context_safe,
        );
        // The three hottest values in the engine -- the program counter, the
        // code pointer and the code length -- lived in `FrameState` and in this
        // function's spill slots, so every dispatch reloaded them before
        // decoding anything. They are locals here, and `self.ip` is
        // resynchronized around exactly the opcodes that can observe it: the
        // ones answered below cannot, because they touch nothing but the
        // operand stack, an authoritative local slot, or `pc` itself.
        let code = program.execution_code;
        let constants: &[Value] = &program.bytecode.constants;
        let mut pc = self.current.ip;
        loop {
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
            match op {
                Op::LoadConst(index) => {
                    crate::diagnostics::count!(dispatched_load_const_ops);
                    if let Some(value) = constants.get(*index) {
                        let value = value.clone();
                        self.current.stack.push(value);
                        continue;
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
                        && !value.is_uninitialized_lexical_marker()
                    {
                        let value = super::vm_bindings::clone_local_value(value);
                        self.current.stack.push(value);
                        crate::diagnostics::count!(authoritative_load_local_hits);
                        continue;
                    }
                    self.current.ip = pc;
                    self.op_load_local(*slot)?;
                    pc = self.current.ip;
                    continue;
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
                        continue;
                    }
                    self.current.ip = pc;
                    self.op_store_local(slot)?;
                    pc = self.current.ip;
                    continue;
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
                        && self.current.locals.get(slot).is_some_and(|local| {
                            local
                                .as_ref()
                                .is_some_and(|local| !local.is_uninitialized_lexical_marker())
                        })
                        && let Some(value) = self.current.stack.pop()
                    {
                        self.current.locals[slot] = Some(value);
                        crate::diagnostics::count!(authoritative_assign_local_hits);
                        continue;
                    }
                    self.current.ip = pc;
                    self.op_assign_local(slot)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::LoadGlobal(name) => {
                    crate::diagnostics::count!(dispatched_global_binding_ops);
                    self.current.ip = pc;
                    self.op_load_global(name)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::StoreGlobalStrict(name) => {
                    crate::diagnostics::count!(dispatched_global_binding_ops);
                    self.current.ip = pc;
                    self.op_store_global_strict(name)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::StoreGlobalSloppy { slot, name } => {
                    crate::diagnostics::count!(dispatched_global_binding_ops);
                    self.current.ip = pc;
                    self.op_store_global_sloppy(*slot, name)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::GetPropNamed { key, cache } => {
                    crate::diagnostics::count!(dispatched_named_property_ops);
                    crate::diagnostics::count!(named_property_reads);
                    self.current.ip = pc;
                    self.op_get_prop_named(key, cache)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::GetPropIndex(index) => {
                    crate::diagnostics::count!(dispatched_computed_property_ops);
                    crate::diagnostics::count!(computed_property_reads);
                    self.current.ip = pc;
                    self.op_get_prop_index(*index)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::GetProp => {
                    crate::diagnostics::count!(dispatched_computed_property_ops);
                    crate::diagnostics::count!(computed_property_reads);
                    self.current.ip = pc;
                    self.op_get_prop()?;
                    pc = self.current.ip;
                    continue;
                }
                Op::SetProp { is_strict } => {
                    crate::diagnostics::count!(dispatched_computed_property_ops);
                    crate::diagnostics::count!(computed_property_writes);
                    self.current.ip = pc;
                    self.op_set_prop(*is_strict)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::SetPropIndex { index, is_strict } => {
                    crate::diagnostics::count!(dispatched_computed_property_ops);
                    crate::diagnostics::count!(computed_property_writes);
                    self.current.ip = pc;
                    self.op_set_prop_index(*index, *is_strict)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::SetPropNamed {
                    key,
                    cache,
                    is_strict,
                } => {
                    crate::diagnostics::count!(dispatched_named_property_ops);
                    crate::diagnostics::count!(named_property_writes);
                    self.current.ip = pc;
                    self.op_set_prop_named(key, cache.as_ref(), *is_strict)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::Call(argc) => {
                    crate::diagnostics::count!(dispatched_call_construct_ops);
                    self.current.ip = pc;
                    self.call(*argc)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::CallResolved(argc) => {
                    crate::diagnostics::count!(dispatched_call_construct_ops);
                    self.current.ip = pc;
                    self.call_resolved(*argc)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::CallResolvedGuardedMathUnary => {
                    crate::diagnostics::count!(dispatched_call_construct_ops);
                    self.current.ip = pc;
                    self.call_resolved_guarded_math_unary()?;
                    pc = self.current.ip;
                    continue;
                }
                Op::New(argc) => {
                    crate::diagnostics::count!(dispatched_call_construct_ops);
                    self.current.ip = pc;
                    self.construct(*argc)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::Unary(op) => {
                    crate::diagnostics::count!(dispatched_numeric_ops);
                    self.current.ip = pc;
                    self.op_unary(*op)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::Return => {
                    crate::diagnostics::count!(dispatched_branch_return_ops);
                    self.current.ip = pc;
                    if let Some(value) = self.op_return()? {
                        return Ok(FrameExit::Completed(Completion::Return(value)));
                    }
                    pc = self.current.ip;
                    continue;
                }
                Op::Pop => {
                    crate::diagnostics::count!(dispatched_stack_ops);
                    if self.current.stack.pop().is_some() {
                        continue;
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
                    continue;
                }
                Op::ToNumeric => {
                    crate::diagnostics::count!(dispatched_numeric_ops);
                    // `eval_to_numeric` is the identity on a number, so the
                    // general path pops and re-pushes the same value through
                    // two memory-sized results to change nothing.
                    if matches!(self.current.stack.last(), Some(Value::Number(_))) {
                        continue;
                    }
                    self.current.ip = pc;
                    self.op_to_numeric()?;
                    pc = self.current.ip;
                    continue;
                }
                Op::Update(op) => {
                    crate::diagnostics::count!(dispatched_numeric_ops);
                    if let Some(Value::Number(number)) = self.current.stack.last_mut() {
                        *number = match op {
                            qjs_ast::UpdateOp::Increment => *number + 1.0,
                            qjs_ast::UpdateOp::Decrement => *number - 1.0,
                        };
                        continue;
                    }
                    self.current.ip = pc;
                    self.op_update(*op)?;
                    pc = self.current.ip;
                    continue;
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
                        continue;
                    }
                    self.current.ip = pc;
                    self.op_binary(*op)?;
                    pc = self.current.ip;
                    continue;
                }
                Op::Jump(target) => {
                    crate::diagnostics::count!(dispatched_branch_return_ops);
                    // A forward jump is the `if`/`else` and loop-exit shape and
                    // carries no accelerator; only a backward edge consults the
                    // loop plans, and that decision is out of line.
                    if *target >= pc - 1 {
                        pc = *target;
                        continue;
                    }
                    self.current.ip = pc;
                    self.op_jump(&program, *target);
                    pc = self.current.ip;
                    continue;
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
                    continue;
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
                    continue;
                }
                Op::JumpIfNotNullish(target) => {
                    crate::diagnostics::count!(dispatched_branch_return_ops);
                    if !matches!(
                        self.current.stack.last(),
                        Some(Value::Null | Value::Undefined)
                    ) {
                        pc = *target;
                    }
                    continue;
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
        }
    }

    fn get_prop(&mut self) -> Result<(), RuntimeError> {
        let key_value = self.pop()?;
        let object = self.pop()?;
        if matches!(object, Value::Null | Value::Undefined) {
            let object_name = if matches!(object, Value::Null) {
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
            let mut env = self.callee_env();
            let value = get_property_key(object, &key, &mut env)?;
            self.apply_env(env);
            value
        };
        self.stack.push(value);
        Ok(())
    }

    fn get_named_prop(
        &mut self,
        key: &str,
        cache: &NamedPropertyCache,
    ) -> Result<(), RuntimeError> {
        let object = if let Some(slot) = cache.local_slot() {
            let direct_eval_lookup =
                self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot);
            if self.slot_is_authoritative(slot)
                && !direct_eval_lookup
                && let Some(Some(object)) = self.locals.get(slot)
                && !matches!(
                    object,
                    Value::Function(function) if function.is_uninitialized_lexical_marker()
                )
                && let Some(value) = self.try_cached_get_string(object, key, cache)
            {
                self.stack.push(value);
                return Ok(());
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
        let value = if let Some(value) = self.try_cached_get_string(&object, key, cache) {
            value
        } else if let Some(result) = self.try_direct_leaf_getter(&object, key) {
            result?
        } else {
            let mut env = self.callee_env();
            let value = get_property(object, key, &mut env)?;
            self.apply_env(env);
            value
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
            let mut env = self.callee_env();
            let value = get_property_key(object, &key, &mut env)?;
            self.apply_env(env);
            value
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
                let mut env = self.callee_env();
                let written = crate::typed_array::set_integer_indexed_element(
                    object,
                    index,
                    value.clone(),
                    &mut env,
                )?;
                self.apply_env(env);
                written
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
            match object.write_existing_own_data_property(key, &value) {
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
                OwnDataPropertyWrite::NeedsSlowPath => {
                    if self.try_create_ordinary_own_data_property(
                        object,
                        Rc::from(key.as_str()),
                        &value,
                    ) {
                        self.stack.push(value);
                        return Ok(());
                    }
                }
            }
        }
        let mut env = self.callee_env();
        let wrote_data = set_property_key(object, key.clone(), value.clone(), &mut env)?;
        self.apply_env(env);
        if !wrote_data && is_strict {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot set property".to_owned(),
            });
        }
        if updates_global_binding
            && wrote_data
            && let crate::PropertyKey::String(key) = key
        {
            self.env.insert_realm(key, value.clone());
        }
        self.stack.push(value);
        Ok(())
    }

    fn construct(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        self.construct_callee(callee, arguments)
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
        if let [Value::Number(date_value)] = arguments.as_slice()
            && matches!(
                &callee,
                Value::Function(function) if function.native_kind() == Some(crate::NativeFunction::Date)
            )
        {
            let mut env = self.realm_env();
            let result =
                crate::date::fast_construct_date_from_number(callee.clone(), *date_value, &mut env);
            if let Some(result) = self.handle_call_result(result)? {
                self.stack.push(result);
            }
            return Ok(());
        }
        // A native constructor, like a native call, does not inherit the
        // caller's lexical environment. Any coercion hooks or callbacks it
        // invokes carry their own closure cells, while realm writes are shared
        // directly. Avoid materializing and writing back a dynamic caller
        // frame solely because construction happened beneath direct eval or a
        // closure-creating function.
        let frame_independent_native = matches!(
            &callee,
            Value::Function(function) if function.native.is_some() && function.bound.is_none()
        );
        let mut env = if frame_independent_native {
            VmCallEnv {
                env: self.realm_env(),
                origin: VmCallEnvOrigin::RealmOnly,
            }
        } else {
            self.call_env(&callee)
        };
        let result = construct_function(callee.clone(), callee, arguments, &mut env.env);
        self.apply_call_env(env);
        if let Some(result) = self.handle_call_result(result)? {
            self.stack.push(result);
        }
        Ok(())
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
        let mut env = self.callee_env();
        let arguments = crate::array::array_like_values_with_env(value, context, &mut env)?;
        self.apply_env(env);
        Ok(arguments)
    }

    pub(super) fn refresh_realm_backed_locals_from_realm(&mut self) {
        for index in 0..self.locals.len() {
            if !self.bytecode.local_is_sloppy_global_fallback(index) {
                continue;
            }
            let Some(name) = self.current.bytecode.local_name_at(index) else {
                continue;
            };
            if !self
                .sloppy_global_names
                .iter()
                .any(|candidate| candidate == name)
            {
                continue;
            }
            let value = if let Some(value) = self.realm.get_value(name) {
                value
            } else if let Some(property) = self.global_this_own_property(name)
                && !property.is_accessor()
            {
                property.value
            } else {
                continue;
            };
            self.current.locals[index] = Some(value.clone());
            if let Some(binding) = self.current.env.module_live_binding_cell(name) {
                binding.set(value);
            }
        }
    }

    pub(super) fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(stack_underflow)
    }

    fn captured_immutable_function_name(
        &self,
        bytecode: &Bytecode,
        local_names: &[String],
    ) -> Option<String> {
        let name = self.env.immutable_function_name()?;
        if local_names.iter().any(|local| local == name) {
            return None;
        }
        let references_name = bytecode.local_slot(name).is_some()
            || bytecode.global_names().iter().any(|global| global == name);
        references_name.then(|| name.to_owned())
    }
}
