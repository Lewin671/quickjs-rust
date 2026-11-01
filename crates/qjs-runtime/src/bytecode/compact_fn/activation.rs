//! A VM-free activation for bodies the compact tier admits.
//!
//! What makes a small recursive call slow on the interpreter is building and
//! tearing down the activation, more than dispatching the body (measurements
//! in `tasks/archive/T021-single-vm-frame-stack-log.md`). A `Vm` carries a
//! `FrameState` holding unwinding state, suspension state, loop-plan decline
//! bitsets, prototype caches and an operand stack, and an admitted compact
//! body can use none of them: it has no handler, cannot suspend, runs no loop
//! plans, and keeps its operands in registers. This module gives such a body
//! only what it needs.
//!
//! The calling convention is not nested. An admitted callee runs on the
//! *same* dispatch loop as its caller: this module owns an explicit frame
//! stack and one contiguous register stack, so a compact-to-compact call is a
//! frame push and a base change rather than a Rust call that reconstructs an
//! activation, takes a pooled register file from behind an `Rc<RefCell<..>>`,
//! and propagates a `Result` back through the native stack. Only a callee this
//! tier cannot admit re-enters the ordinary call path.
//!
//! Two consequences beyond speed. Recursion through admitted bodies consumes
//! no native stack, so its depth is bounded by `MAX_FRAMES` and reports
//! a catchable `RangeError` instead of whatever the native stack does. And the
//! arguments of an inlined call are *moved* into the callee's parameter
//! registers: `Op::Call` pops its operand registers, so nothing else can
//! observe them, and no reference count moves.

use crate::bytecode::frame_bank::{FrameStorage, PREFIX, RootActivation, clear_written};

use qjs_ast::BinaryOp;

use super::CompactOp;
use super::execute;
use super::execute::Action;
use crate::bytecode::DirectCallSlots;
use crate::bytecode::ir::Bytecode;
use crate::function::{BindingCell, Function};
use crate::{RuntimeError, Value};

/// Everything an admitted body needs to run, and nothing a `FrameState`
/// carries for the general interpreter's sake.
pub(super) struct CompactActivation<'a, 's> {
    pub(super) bytecode: &'a Bytecode,
    /// A borrow of services and this callee's actual typed activation.
    pub(super) env: &'a mut crate::context::Context<'s>,
    /// The function whose upvalue vector backs this body's received cells.
    /// Reads resolve through it by bytecode slot, keeping cell identity live.
    /// The frame stack owns it; an activation is rebuilt per frame switch and
    /// borrows it, so switching frames moves no reference count.
    upvalue_owner: Option<&'a Function>,
    upvalue_slots: u128,
}

impl<'a, 's> CompactActivation<'a, 's> {
    /// Returns the live cell for a received-upvalue slot.
    pub(super) fn upvalue_cell(&self, slot: usize) -> Option<&BindingCell> {
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
        &mut self,
        left: Value,
        op: BinaryOp,
        right: Value,
    ) -> Result<Value, RuntimeError> {
        let env = &mut *self.env;
        crate::operations::eval_binary(left, op, right, env)
    }
}

/// The per-body facts this tier needs proven before it may run a call.
struct CompactEntry<'a> {
    program: &'a super::CompactFunctionProgram,
    upvalue_owner: Option<Function>,
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
    })
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

/// What the driver needs to enter an admitted callee without borrowing it from
/// the register that named it.
struct InlineCallee {
    register_count: usize,
}

/// Proves a callee may run on the caller's own loop, in the caller's window.
///
/// This is the same proof `call_from_activation` performs for the fallback
/// path: `is_direct_leaf_function` is the outer gate because it is what makes
/// seeding parameters into slots safe for this callee at all -- default
/// parameter prologues and `arguments` objects are among the shapes it
/// rejects, and the compact program's own admission does not subsume it.
fn inline_callee(callee: &Value) -> Option<InlineCallee> {
    let Value::Function(function) = callee else {
        return None;
    };
    let facts = fixed_inline_facts(callee, function);
    if facts & FACTS_ELIGIBLE == 0
        || function.has_dynamic_function_realm_override.get()
        || function.has_cold_lexical_state()
    {
        return None;
    }
    // This tier has no reference checkpoints; its fresh indexed activation
    // is materialized only on real Context demand after the call is entered.
    if !function
        .bytecode
        .as_ref()?
        .binding_plan
        .references
        .skips_indexed_entry()
    {
        return None;
    }
    Some(InlineCallee {
        register_count: (facts >> FACTS_REGISTER_SHIFT) as usize,
    })
}

/// Whether a numeric plan may call `function` (`numeric_plan`): the facts
/// that let this tier inline it, less the caller-environment agreement a
/// plan does not need -- its body reads nothing but its cells.
pub(super) fn admits_numeric_callee(callee: &Value, function: &Function) -> bool {
    fixed_inline_facts(callee, function) & FACTS_ELIGIBLE != 0
        && !function.has_dynamic_function_realm_override.get()
        && !function.has_cold_lexical_state()
}

/// `Function::compact_inline_facts`: computed, and admitted as far as the
/// function's fixed facts decide -- direct-leaf shape, no lexical `this`, no
/// dynamic realm, no module imports, a program whose received cells this
/// function owns -- then whether it has received cells, and the register
/// count in the high half. A call reads one word instead of re-deriving each.
const FACTS_KNOWN: u32 = 1;
const FACTS_ELIGIBLE: u32 = 1 << 1;
const FACTS_OWNS_UPVALUES: u32 = 1 << 2;
const FACTS_REGISTER_SHIFT: u32 = 16;

#[inline]
fn fixed_inline_facts(callee: &Value, function: &Function) -> u32 {
    let facts = function.compact_inline_facts.get();
    if facts & FACTS_KNOWN != 0 {
        return facts;
    }
    let facts = compute_fixed_inline_facts(callee, function);
    function.compact_inline_facts.set(facts);
    facts
}

#[cold]
#[inline(never)]
fn compute_fixed_inline_facts(callee: &Value, function: &Function) -> u32 {
    let Some(bytecode) = function.bytecode.as_ref() else {
        return FACTS_KNOWN;
    };
    if !crate::function::is_direct_leaf_shape(callee)
        || bytecode.uses_lexical_this()
        || function.has_dynamic_function_realm
        || function.has_module_imports()
    {
        return FACTS_KNOWN;
    }
    let Some(entry) = admit(
        bytecode,
        crate::bytecode::DirectCallUpvalues::Function(function),
    ) else {
        return FACTS_KNOWN;
    };
    let Ok(register_count) = u16::try_from(entry.program.register_count) else {
        return FACTS_KNOWN;
    };
    let owns = if entry.upvalue_owner.is_some() {
        FACTS_OWNS_UPVALUES
    } else {
        0
    };
    FACTS_KNOWN | FACTS_ELIGIBLE | owns | (u32::from(register_count) << FACTS_REGISTER_SHIFT)
}

fn run(
    bytecode: &Bytecode,
    activation: &mut crate::context::ActivationContext,
    runtime: &mut crate::context::RuntimeServices,
    entry: CompactEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    run_root(
        bytecode,
        RootActivation::Full(activation),
        Value::Undefined,
        None,
        runtime,
        entry,
        parameter_slots,
        arguments,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_root(
    bytecode: &Bytecode,
    mut root: RootActivation<'_>,
    receiver: Value,
    target: Option<Value>,
    runtime: &mut crate::context::RuntimeServices,
    entry: CompactEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    crate::diagnostics::count!(compact_standalone_activations);
    let poison = runtime.binding_debug.poisoning_activations() || runtime.binding_debug.poisoning();
    let mut storage = runtime.frame_stacks.take(poison);
    storage.push(
        Value::Undefined,
        receiver,
        target,
        entry.program.register_count,
        arguments.len(),
        0,
        0,
    )?;
    for (index, &slot) in parameter_slots.iter().enumerate() {
        if slot < entry.program.register_count
            && let Some(value) = arguments.get(index)
        {
            execute::store(
                &mut storage.registers[PREFIX + slot],
                crate::bytecode::vm_bindings::clone_local_value(value),
            );
        }
    }
    let result = run_frames(
        bytecode,
        runtime,
        &mut root,
        entry.upvalue_owner.as_ref(),
        &mut storage,
    );
    runtime.frame_stacks.put(storage);
    result
}

enum Boundary {
    Return(Value),
    Call {
        callee: Value,
        inline: InlineCallee,
        arguments: usize,
        argc: usize,
        dst: u16,
        resume_pc: usize,
    },
}

fn run_frames(
    root_bytecode: &Bytecode,
    runtime: &mut crate::context::RuntimeServices,
    root: &mut RootActivation<'_>,
    root_owner: Option<&Function>,
    storage: &mut FrameStorage,
) -> Result<Value, RuntimeError> {
    loop {
        let depth = storage.frames.len();
        let boundary = {
            let header = storage.frames.last_mut().expect("current indexed header");
            let current_base = header.service.base as usize;
            let current_len = header.width as usize;
            let (previous, registers) = storage.registers.split_at_mut(current_base);
            let prefix = &previous[current_base - PREFIX..current_base];
            let bytecode = running_bytecode(&prefix[0], root_bytecode);
            let owner = match &prefix[0] {
                Value::Function(f) => Some(f),
                _ => root_owner,
            };
            let slots = bytecode
                .direct_readonly_received_upvalue_slots()
                .unwrap_or(0);
            let Some(program) = super::program_for(bytecode) else {
                return Err(missing_program());
            };
            let mut pc = header.pc as usize;
            let mut env = root.view(runtime, header, prefix, depth == 1);
            let mut activation = CompactActivation {
                bytecode,
                env: &mut env,
                upvalue_owner: owner,
                upvalue_slots: slots,
            };
            let window = &mut registers[..current_len];
            let ops = &program.ops[..];
            loop {
                let outcome = loop {
                    let Some(op) = ops.get(pc) else {
                        // Falling off the end is an implicit `return undefined`.
                        break Ok(Action::Return(Value::Undefined));
                    };
                    pc += 1;
                    #[cfg(feature = "perf-counters")]
                    crate::diagnostics::update(|c| c.compact_function_ops += 1);
                    match *op {
                        CompactOp::LoadConst { dst, index } => {
                            let Some(value) = activation.bytecode.constants.get(index as usize)
                            else {
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
                            let value = crate::bytecode::vm_bindings::clone_local_value(
                                &window[src as usize],
                            );
                            execute::store(&mut window[dst as usize], value);
                        }
                        CompactOp::LoadUpvalueLocal { dst, slot } => {
                            let Some(cell) = activation.upvalue_cell(slot as usize) else {
                                break Err(execute::uninitialized_local());
                            };
                            let value = if !cell.is_initialized() {
                                match activation.uninitialized_upvalue(slot as usize) {
                                    Ok(value) => value,
                                    Err(error) => break Err(error),
                                }
                            } else {
                                match cell.read() {
                                    Ok(value) => value,
                                    Err(error) => break Err(error),
                                }
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
                            let left =
                                std::mem::replace(&mut window[left as usize], Value::Undefined);
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
                };
                let action = match outcome {
                    Ok(action) => action,
                    Err(error) => {
                        return Err(crate::error::capture_error_origin(error, activation.env));
                    }
                };
                match action {
                    Action::Return(value) => {
                        clear_written(window, &program.written, storage.poison);
                        break Boundary::Return(value);
                    }
                    Action::Call {
                        dst,
                        base,
                        argc,
                        resume_pc,
                    } => {
                        let index = base as usize;
                        let callee = std::mem::replace(&mut window[index], Value::Undefined);
                        if let Some(inline) = inline_callee(&callee) {
                            break Boundary::Call {
                                callee,
                                inline,
                                arguments: current_base + index + 1,
                                argc: argc as usize,
                                dst,
                                resume_pc,
                            };
                        }
                        let args = &window[index + 1..index + 1 + argc as usize];
                        match call_from_activation(&mut activation, callee, args) {
                            Ok(value) => {
                                execute::store(&mut window[dst as usize], value);
                                pc = resume_pc;
                            }
                            Err(error) => {
                                return Err(crate::error::capture_error_origin(
                                    error,
                                    activation.env,
                                ));
                            }
                        }
                    }
                }
            }
        };
        match boundary {
            Boundary::Return(value) => {
                let dst = storage.pop();
                let Some(caller) = storage.frames.last() else {
                    return Ok(value);
                };
                execute::store(
                    &mut storage.registers[caller.service.base as usize + dst as usize],
                    value,
                );
            }
            Boundary::Call {
                callee,
                inline,
                arguments,
                argc,
                dst,
                resume_pc,
            } => {
                storage.frames.last_mut().expect("caller header").pc = resume_pc as u32;
                storage.push(
                    Value::Undefined,
                    Value::Undefined,
                    None,
                    inline.register_count,
                    argc,
                    dst,
                    0,
                )?;
                storage.move_arguments(&callee, arguments, argc);
                storage.install_callee(callee);
                crate::diagnostics::count!(ordinary_call_attempts);
                crate::diagnostics::count!(compact_direct_calls);
            }
        }
    }
}

#[cold]
fn missing_program() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "compact function program disappeared after admission".to_owned(),
    }
}

/// Runs `bytecode` without building a `Vm`, or returns `None` having consumed
/// nothing, so the caller can construct the ordinary frame unchanged.
///
/// The actual prepared callee is borrowed in place. Every guard precedes
/// receiver consumption or any observable work.
#[inline(always)]
pub(in crate::bytecode) fn try_run_standalone(
    bytecode: &Bytecode,
    env: &mut crate::context::ActivationContext,
    runtime: &mut crate::context::RuntimeServices,
    slots: &mut DirectCallSlots<'_>,
) -> Option<Result<Value, RuntimeError>> {
    // This tier resolves every name by slot index. An environment that can
    // still answer a name, that carries deoptimized dynamic bindings, or that
    // overrides the realm global belongs on the general path.
    if !env.indexed_bindings_only() {
        crate::diagnostics::count!(indexed_binding_admission_rejections);
        trace_reference_decline(bytecode, "dynamic scope or selected global");
        return None;
    }
    // A body outside this tier's numeric set may still be one the wide tier
    // runs; that tier checks its own reference admission and checkpoints.
    let Some(entry) = admit(bytecode, slots.upvalues) else {
        return super::wide::try_run_standalone(bytecode, env, runtime, slots);
    };
    // This tier has no reference checkpoints: every site must be elidable for
    // the whole activation (it compiles no global store, so only dynamic
    // sites can fail here).
    if !bytecode.binding_plan.references.can_skip(env) {
        trace_reference_decline(bytecode, "reference sites need selection");
        return None;
    }
    if let Some(value) = run_numeric_plan(bytecode, slots) {
        return Some(Ok(value));
    }
    // Borrow the actual prepared callee in place; do not move a second
    // root-sized activation through the recursive native call stack.
    let call_env = env;
    let call_slots = slots;
    Some(run(
        bytecode,
        call_env,
        runtime,
        entry,
        call_slots.parameter_slots,
        call_slots.arguments,
    ))
}

/// `QJS_CF_TRACE=1` names a standalone entry declined before any tier ran.
#[cfg_attr(not(feature = "perf-counters"), allow(unused_variables))]
pub(in crate::bytecode) fn trace_reference_decline(bytecode: &Bytecode, reason: &str) {
    #[cfg(feature = "perf-counters")]
    if std::env::var_os("QJS_CF_TRACE").is_some() {
        eprintln!(
            "CFDECLINE params=({}) len={} {reason}",
            bytecode.parameter_names().join(","),
            bytecode.code.len()
        );
    }
}

/// Runs an admitted body's numeric plan (`numeric_plan`), when it has one
/// and every argument is a number; `None` leaves the call to the tier.
#[inline(never)]
fn run_numeric_plan(bytecode: &Bytecode, slots: &DirectCallSlots<'_>) -> Option<Value> {
    let plan = super::numeric_plan::plan_for(bytecode)?;
    let mut numbers = [0.0; super::compile::MAX_CALL_ARITY];
    if slots.arguments.len() > numbers.len() {
        return None;
    }
    for (number, argument) in numbers.iter_mut().zip(slots.arguments) {
        let Value::Number(value) = argument else {
            return None;
        };
        *number = *value;
    }
    super::numeric_plan::run(
        plan,
        bytecode,
        slots.upvalues.as_slice(),
        &numbers[..slots.arguments.len()],
    )
}

/// Runs one call the driver could not inline.
///
/// A slot-seeded direct-leaf callee reaches `call_direct_leaf_function`, which
/// takes an argument slice directly. Every other callee shape goes through
/// `call_function_slice`, which shares the interpreter's general call
/// path reaches, so native, bound, Proxy, and class-constructor behaviour keeps
/// one implementation.
#[inline(never)]
fn call_from_activation(
    activation: &mut CompactActivation<'_, '_>,
    callee: Value,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    if crate::function::is_direct_leaf_function(&callee) {
        return crate::function::call_direct_leaf_function(
            callee,
            Value::Undefined,
            arguments,
            activation.env,
        );
    }
    let env = &mut *activation.env;
    crate::function::call_function_slice(&callee, Value::Undefined, arguments, env)
}

/// Fresh indexed root admission uses immutable creation facts and consumes no
/// inputs on decline. Dynamic creation selection retains the full entry API.
#[inline(always)]
pub(in crate::bytecode) fn try_run_indexed(
    function: &Function,
    receiver: &mut Option<Value>,
    target: &mut Option<Value>,
    arguments: &[Value],
    runtime: &mut crate::context::RuntimeServices,
) -> Option<Result<Value, RuntimeError>> {
    if function.has_dynamic_function_realm
        || function.has_dynamic_function_realm_override.get()
        || !matches!(
            function.creation_context().global,
            crate::context::GlobalSelection::Default
        )
        || !function.creation_context().imports().is_empty()
    {
        return None;
    }
    let bytecode = function.bytecode.as_ref()?;
    let upvalues = crate::bytecode::DirectCallUpvalues::Function(function);
    if bytecode.binding_plan.references.skips_indexed_entry()
        && let Some(entry) = admit(bytecode, upvalues)
    {
        let slots = DirectCallSlots {
            parameter_slots: bytecode.parameter_slots(),
            arguments,
            upvalues,
            realm_upvalue_slots: function.realm_upvalue_slots,
        };
        if let Some(value) = run_numeric_plan(bytecode, &slots) {
            if let Some(value) = receiver.take() {
                crate::bytecode::frame_bank::discard(value);
            }
            target.take();
            return Some(Ok(value));
        }
        if let Some(value) = receiver.take() {
            crate::bytecode::frame_bank::discard(value);
        }
        return Some(run_root(
            bytecode,
            RootActivation::Indexed { function },
            Value::Undefined,
            target.take(),
            runtime,
            entry,
            slots.parameter_slots,
            arguments,
        ));
    }
    super::wide::try_run_indexed(function, receiver, target, arguments, runtime)
}
