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
//! The calling convention is no longer nested. An admitted callee runs on the
//! *same* dispatch loop as its caller: this module owns an explicit frame
//! stack and one contiguous register stack, so a compact-to-compact call is a
//! frame push and a base change rather than a Rust call that reconstructs an
//! activation, takes a pooled register file from behind an `Rc<RefCell<..>>`,
//! and propagates a `Result` back through the native stack. Only a callee this
//! tier cannot admit still re-enters the ordinary call path.
//!
//! Two consequences beyond speed. Recursion through admitted bodies no longer
//! consumes native stack, so its depth is bounded by `MAX_FRAMES` and reports
//! a catchable `RangeError` instead of whatever the native stack does. And the
//! arguments of an inlined call are *moved* into the callee's parameter
//! registers: `Op::Call` pops its operand registers, so nothing else can
//! observe them, and no reference count moves.

use qjs_ast::BinaryOp;

use super::CompactOp;
use super::execute;
use super::execute::Action;
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
    /// The frame stack owns it; an activation is rebuilt per frame switch and
    /// borrows it, so switching frames moves no reference count.
    upvalue_owner: &'a Option<Function>,
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

    /// Answers a received cell that is still in its temporal dead zone.
    ///
    /// Whether a cell is initialized is a runtime fact about the *caller's*
    /// execution, not a property of this body, so no amount of admission
    /// narrowing can rule it out -- a closure can be called before the `let`
    /// it captures is reached. A compiler temporary reads as `undefined`,
    /// matching `Vm::uninitialized_local_value`.
    #[cold]
    #[inline(never)]
    pub(super) fn uninitialized_upvalue(&self, slot: usize) -> Result<Value, RuntimeError> {
        if self.bytecode.local_is_compiler_temporary(slot) {
            return Ok(Value::Undefined);
        }
        Err(RuntimeError {
            thrown: None,
            message: format!(
                "ReferenceError: undefined identifier `{}`",
                self.bytecode.locals[slot].name
            ),
        })
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
        && !env.has_dynamic_function_realm_global()
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
    let host_agrees = match (&function.module_host, env.module_host_ref()) {
        (None, _) => true,
        (Some(callee), Some(caller)) => std::rc::Rc::ptr_eq(callee, caller),
        (Some(_), None) => false,
    };
    host_agrees
        && !bytecode.uses_lexical_this()
        && !function.has_dynamic_function_realm
        && !function.has_dynamic_function_realm_override.get()
        && function.module_imports.is_empty()
        && !function.has_cold_lexical_state()
}

/// One suspended compact activation.
///
/// The *running* frame is not in this list: its fields live in the driver
/// loop's locals, which is what keeps the hot path free of indexed frame
/// reads.
struct CompactFrame {
    /// The function this frame is running, which is what keeps its bytecode
    /// alive. `Op::Call` pops the register that named the callee, so the value
    /// is *moved* here and no reference count is touched; the root's entry is
    /// `undefined`, because the caller keeps the root bytecode alive itself.
    callee: Value,
    upvalue_owner: Option<Function>,
    upvalue_slots: u128,
    /// First register of this frame's window in the shared register stack.
    base: usize,
    /// Width of the window.
    len: usize,
    /// Where this frame resumes once the callee's result is stored.
    resume_pc: usize,
    /// Frame-relative register that receives the callee's result.
    dst: u16,
}

/// Storage the frame stack reuses across root activations.
#[derive(Default)]
struct FrameStorage {
    /// Every live frame's registers, laid out back to back. A frame's window
    /// is `[base, base + len)`, and every register outside a live window is
    /// `undefined` -- the invariant that makes entering a frame free of
    /// initialization beyond a parameter copy.
    registers: Vec<Value>,
    frames: Vec<CompactFrame>,
}

thread_local! {
    static FRAME_STORAGE: std::cell::RefCell<FrameStorage> =
        std::cell::RefCell::new(FrameStorage::default());
}

/// Recursion bound for admitted bodies.
///
/// These frames are heap-allocated, so the native stack no longer decides the
/// limit; something still has to, and a program that recurses past this gets
/// the same catchable error QuickJS-NG reports.
const MAX_FRAMES: usize = 200_000;

/// How many registers the pooled stack keeps between root activations. A deep
/// recursion should not retain its high-water mark for the rest of the
/// process.
const MAX_POOLED_REGISTERS: usize = 4096;

/// Widens the register stack, keeping the "clear outside every live window"
/// invariant.
#[inline]
fn reserve_registers(registers: &mut Vec<Value>, needed: usize) {
    if registers.len() < needed {
        registers.resize(needed, Value::Undefined);
    }
}

/// Drops a value the interpreter is discarding, inlining the case where it
/// owns nothing -- the same test `execute::store` performs for a register.
#[inline(always)]
fn release(value: Value) {
    if matches!(
        value,
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined
    ) {
        std::mem::forget(value);
    }
}

/// The bytecode of the body a frame is running.
#[inline(always)]
fn running_bytecode<'a>(callee: &'a Value, root: &'a Bytecode) -> &'a Bytecode {
    match callee {
        Value::Function(function) => match function.bytecode.as_deref() {
            Some(bytecode) => bytecode,
            None => root,
        },
        _ => root,
    }
}

/// The parameter slots of an admitted callee.
#[inline(always)]
fn callee_parameter_slots(callee: &Value) -> &[usize] {
    match callee {
        Value::Function(function) => match function.bytecode.as_deref() {
            Some(bytecode) => bytecode.parameter_slots(),
            None => &[],
        },
        _ => &[],
    }
}

/// Returns a frame's window to `undefined`, releasing whatever it still holds.
#[inline]
fn clear_window(window: &mut [Value]) {
    for slot in window {
        execute::store(slot, Value::Undefined);
    }
}

/// What the driver needs to enter an admitted callee without borrowing it from
/// the register that named it.
struct InlineCallee {
    upvalue_owner: Option<Function>,
    upvalue_slots: u128,
    register_count: usize,
}

/// Proves a callee may run on the caller's own loop, in the caller's window.
///
/// This is the same proof `call_from_activation` performs for the fallback
/// path: `is_direct_leaf_function` is the outer gate because it is what makes
/// seeding parameters into slots safe for this callee at all -- default
/// parameter prologues and `arguments` objects are among the shapes it
/// rejects, and the compact program's own admission does not subsume it.
fn inline_callee(callee: &Value, env: &CallEnv) -> Option<InlineCallee> {
    if !crate::function::is_direct_leaf_function(callee) {
        return None;
    }
    let Value::Function(function) = callee else {
        return None;
    };
    let bytecode = function.bytecode.as_ref()?;
    if !shares_caller_environment(function, bytecode, env) {
        return None;
    }
    let entry = admit(
        bytecode,
        crate::bytecode::DirectCallUpvalues::Function(function),
    )?;
    Some(InlineCallee {
        register_count: entry.program.register_count,
        upvalue_owner: entry.upvalue_owner,
        upvalue_slots: entry.upvalue_slots,
    })
}

/// Runs an admitted body in `env`, together with every admitted body it calls.
fn run(
    bytecode: &Bytecode,
    env: &CallEnv,
    entry: CompactEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    crate::diagnostics::count!(compact_standalone_activations);
    // Taking the storage rather than borrowing it keeps re-entrancy simple: a
    // body that calls out to native code that calls back into this tier finds
    // an empty storage and builds its own, instead of meeting a live borrow.
    let mut storage = FRAME_STORAGE.with(|cell| cell.take());
    let result = run_frames(
        bytecode,
        env,
        entry,
        parameter_slots,
        arguments,
        &mut storage,
    );
    storage.frames.clear();
    if storage.registers.len() > MAX_POOLED_REGISTERS {
        storage.registers.truncate(MAX_POOLED_REGISTERS);
        storage.registers.shrink_to_fit();
    }
    FRAME_STORAGE.with(|cell| cell.replace(storage));
    result
}

/// The frame-stack driver.
///
/// Exactly one activation is "current", and its fields are loop locals. A call
/// pushes the current one and overwrites them; a return pops. `run_ops` is
/// re-entered per frame switch, so the executor itself never learns that more
/// than one activation exists and keeps its zero-based register window.
fn run_frames(
    root_bytecode: &Bytecode,
    env: &CallEnv,
    entry: CompactEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
    storage: &mut FrameStorage,
) -> Result<Value, RuntimeError> {
    let registers = &mut storage.registers;
    let frames = &mut storage.frames;
    debug_assert!(frames.is_empty(), "a root activation starts its own stack");

    // `Value::Undefined` marks the root, whose bytecode the caller owns.
    let mut current_callee = Value::Undefined;
    let mut current_owner = entry.upvalue_owner;
    let mut current_slots = entry.upvalue_slots;
    let mut current_base = 0_usize;
    let mut current_len = entry.program.register_count;
    let mut pc = 0_usize;

    reserve_registers(registers, current_len);
    // Locals live in the low registers. Everything starts `undefined`, which
    // is already the correct seed for a hoisted `var`; parameters overwrite
    // theirs here, and a received upvalue is read from its cell rather than
    // from a register. `this` is not in the opcode set, so its slot is left
    // alone. That is the whole frame-setup cost for an admitted body.
    //
    // A root's arguments belong to its caller, so they are cloned; an inlined
    // call below moves them instead.
    for (index, &slot) in parameter_slots.iter().enumerate() {
        if slot >= current_len {
            continue;
        }
        let Some(argument) = arguments.get(index) else {
            break;
        };
        registers[slot] = crate::bytecode::vm_bindings::clone_local_value(argument);
    }

    loop {
        // The dispatch loop is *inlined here* rather than called. Handing an
        // action back across a function boundary costs two indirect returns
        // per call -- one for the call action, one for the return action --
        // where the old nested-Rust-call shape paid one; measured, that
        // boundary was worth more than the pooled register file it replaced.
        // Breaking out of the inner loop hands the action over in registers.
        let outcome = {
            let bytecode: &Bytecode = running_bytecode(&current_callee, root_bytecode);
            // Admission proved this body has a program; re-deriving it here is
            // one `OnceCell` read per frame switch and keeps the frame from
            // carrying a borrow of the bytecode it also owns.
            let Some(program) = super::program_for(bytecode) else {
                unwind(registers, frames, current_base + current_len);
                return Err(missing_program());
            };
            let activation = CompactActivation {
                bytecode,
                env,
                upvalue_owner: &current_owner,
                upvalue_slots: current_slots,
            };
            let ops = &program.ops[..];
            let window = &mut registers[current_base..current_base + current_len];
            loop {
                let Some(op) = ops.get(pc) else {
                    // Falling off the end is an implicit `return undefined`.
                    break Ok(Action::Return(Value::Undefined));
                };
                pc += 1;
                #[cfg(feature = "perf-counters")]
                crate::diagnostics::update(|c| c.compact_function_ops += 1);
                match *op {
                    CompactOp::LoadConst { dst, index } => {
                        let Some(value) = activation.bytecode.constants.get(index as usize) else {
                            break Err(execute::constant_out_of_bounds());
                        };
                        // `Value::clone` stays an out-of-line call; the
                        // local-value clone inlines its primitive cases, which
                        // is what a constant pool of numbers actually needs.
                        execute::store(
                            &mut window[dst as usize],
                            crate::bytecode::vm_bindings::clone_local_value(value),
                        );
                    }
                    CompactOp::Move { dst, src } => {
                        let value =
                            crate::bytecode::vm_bindings::clone_local_value(&window[src as usize]);
                        execute::store(&mut window[dst as usize], value);
                    }
                    CompactOp::LoadUpvalueLocal { dst, slot } => {
                        let Some(cell) = activation.upvalue_cell(slot as usize) else {
                            break Err(execute::uninitialized_local());
                        };
                        let value = cell.get();
                        let value = if value.is_uninitialized_lexical_marker() {
                            match activation.uninitialized_upvalue(slot as usize) {
                                Ok(value) => value,
                                Err(error) => break Err(error),
                            }
                        } else {
                            value
                        };
                        execute::store(&mut window[dst as usize], value);
                    }
                    CompactOp::Binary {
                        dst,
                        op,
                        left,
                        right,
                    } => {
                        if let (Value::Number(left), Value::Number(right)) =
                            (&window[left as usize], &window[right as usize])
                            && let Some(value) =
                                crate::bytecode::vm_props::fast_number_binary_numbers(
                                    *left, op, *right,
                                )
                        {
                            execute::store(&mut window[dst as usize], value);
                            continue;
                        }
                        let left = std::mem::replace(&mut window[left as usize], Value::Undefined);
                        let right =
                            std::mem::replace(&mut window[right as usize], Value::Undefined);
                        match activation.eval_binary(left, op, right) {
                            Ok(value) => execute::store(&mut window[dst as usize], value),
                            Err(error) => break Err(error),
                        }
                    }
                    CompactOp::Drop { src } => {
                        execute::store(&mut window[src as usize], Value::Undefined);
                    }
                    CompactOp::JumpIfFalsy { cond, target } => {
                        // The general `is_truthy` consults the `[[IsHTMLDDA]]`
                        // slot, which showed up in the profile for a loop
                        // condition that is always a number. Numbers answer
                        // here; everything else keeps the shared implementation.
                        let falsy = match &window[cond as usize] {
                            Value::Number(number) => *number == 0.0 || number.is_nan(),
                            other => !crate::is_truthy(other),
                        };
                        if falsy {
                            pc = target as usize;
                        }
                    }
                    CompactOp::Jump { target } => pc = target as usize,
                    CompactOp::Call { dst, base, argc } => {
                        break Ok(Action::Call {
                            dst,
                            base,
                            argc,
                            resume_pc: pc,
                        });
                    }
                    CompactOp::Return { src } => {
                        break Ok(Action::Return(std::mem::replace(
                            &mut window[src as usize],
                            Value::Undefined,
                        )));
                    }
                }
            }
        };
        let action = match outcome {
            Ok(action) => action,
            Err(error) => {
                unwind(registers, frames, current_base + current_len);
                return Err(error);
            }
        };
        match action {
            Action::Return(value) => {
                clear_window(&mut registers[current_base..current_base + current_len]);
                let Some(caller) = frames.pop() else {
                    return Ok(value);
                };
                current_callee = caller.callee;
                current_owner = caller.upvalue_owner;
                current_slots = caller.upvalue_slots;
                current_base = caller.base;
                current_len = caller.len;
                pc = caller.resume_pc;
                execute::store(&mut registers[current_base + caller.dst as usize], value);
            }
            Action::Call {
                dst,
                base,
                argc,
                resume_pc,
            } => {
                let callee_index = current_base + base as usize;
                // `Op::Call` pops the callee register and pushes its result
                // there, so taking the callee out costs no reference count.
                let callee = std::mem::replace(&mut registers[callee_index], Value::Undefined);
                let argument_index = callee_index + 1;
                let argument_count = argc as usize;

                if let Some(inline) = inline_callee(&callee, env) {
                    if frames.len() >= MAX_FRAMES {
                        unwind(registers, frames, current_base + current_len);
                        return Err(call_stack_exhausted());
                    }
                    // The attempt counter's whole job is to prove a workload
                    // really performs the calls it claims, so it is raised for
                    // every dispatched call whatever tier answers it. The tier
                    // attribution is `compact_direct_calls`, not
                    // `direct_leaf_frames`: no frame is built.
                    crate::diagnostics::count!(ordinary_call_attempts);
                    crate::diagnostics::count!(compact_direct_calls);
                    let callee_base = current_base + current_len;
                    let callee_len = inline.register_count;
                    reserve_registers(registers, callee_base + callee_len);
                    {
                        // Every argument register sits below `callee_base`,
                        // because the operand stack that holds them is part of
                        // the caller's own window. Each argument is dead after
                        // the call, so it is moved rather than cloned; one the
                        // callee does not name is released here, because its
                        // register would otherwise stay live until this frame
                        // returns.
                        let parameter_slots = callee_parameter_slots(&callee);
                        let (caller_side, callee_side) = registers.split_at_mut(callee_base);
                        for index in 0..argument_count {
                            let value = std::mem::replace(
                                &mut caller_side[argument_index + index],
                                Value::Undefined,
                            );
                            match parameter_slots.get(index) {
                                Some(&slot) if slot < callee_len => callee_side[slot] = value,
                                _ => release(value),
                            }
                        }
                    }
                    frames.push(CompactFrame {
                        callee: std::mem::replace(&mut current_callee, callee),
                        upvalue_owner: current_owner,
                        upvalue_slots: current_slots,
                        base: current_base,
                        len: current_len,
                        resume_pc,
                        dst,
                    });
                    current_owner = inline.upvalue_owner;
                    current_slots = inline.upvalue_slots;
                    current_base = callee_base;
                    current_len = callee_len;
                    pc = 0;
                    continue;
                }

                let outcome = {
                    let activation = CompactActivation {
                        bytecode: running_bytecode(&current_callee, root_bytecode),
                        env,
                        upvalue_owner: &current_owner,
                        upvalue_slots: current_slots,
                    };
                    let arguments = &registers[argument_index..argument_index + argument_count];
                    call_from_activation(&activation, callee, arguments)
                };
                match outcome {
                    Ok(value) => {
                        execute::store(&mut registers[current_base + dst as usize], value);
                        pc = resume_pc;
                    }
                    Err(error) => {
                        unwind(registers, frames, current_base + current_len);
                        return Err(error);
                    }
                }
            }
        }
    }
}

/// Releases every live window after a throw, restoring the storage invariant
/// so the next root activation starts from an all-`undefined` stack.
#[cold]
#[inline(never)]
fn unwind(registers: &mut [Value], frames: &mut Vec<CompactFrame>, live_end: usize) {
    frames.clear();
    let end = live_end.min(registers.len());
    clear_window(&mut registers[..end]);
}

#[cold]
fn missing_program() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "compact function program disappeared after admission".to_owned(),
    }
}

#[cold]
fn call_stack_exhausted() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "RangeError: Maximum call stack size exceeded".to_owned(),
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
    // This tier resolves every name by slot index. An environment that can
    // still answer a name, that carries deoptimized dynamic bindings, or that
    // overrides the realm global belongs on the general path.
    if !environment_is_slot_only(env.as_ref()?) {
        return None;
    }
    // A body outside this tier's numeric set may still be one the wide tier
    // runs; that tier re-checks admission on its own program.
    let Some(entry) = admit(bytecode, slots.as_ref()?.upvalues) else {
        return super::wide::try_run_standalone(bytecode, env, slots);
    };
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

/// Runs one call the driver could not inline.
///
/// A slot-seeded direct-leaf callee reaches `call_direct_leaf_function`, which
/// takes an argument slice directly. Every other callee shape goes through
/// `call_function`, which is the same entry the interpreter's general call
/// path reaches, so native, bound, Proxy, and class-constructor behaviour keeps
/// one implementation.
#[inline(never)]
fn call_from_activation(
    activation: &CompactActivation<'_>,
    callee: Value,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
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
