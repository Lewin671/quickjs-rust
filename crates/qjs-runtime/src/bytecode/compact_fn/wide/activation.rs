//! The wide tier's VM-free activation and frame driver.
//!
//! This is the numeric tier's driver (`compact_fn::activation`) with three
//! additions: every frame carries its seeded `this`, a resolved call takes
//! its receiver from the register below the callee, and the dispatch loop
//! answers the named-property operations through the shared cached fast
//! paths in `compact_fn::property`. The numeric driver is deliberately not
//! reused: sharing one loop between the two tiers measurably slowed the
//! recursive bodies the numeric tier exists for.

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use super::{WideOp, WideProgram};
use crate::bytecode::DirectCallSlots;
use crate::bytecode::compact_fn::execute;
use crate::bytecode::compact_fn::property;
use crate::bytecode::ir::Bytecode;
use crate::function::{CallEnv, Function, Upvalue};
use crate::{RuntimeError, Value};

/// Everything an admitted body needs to run.
struct WideActivation<'a> {
    bytecode: &'a Bytecode,
    /// The environment this body runs in, borrowed: `shares_caller_environment`
    /// proves it equals what `direct_leaf_function_env` would build.
    env: &'a CallEnv,
    upvalue_owner: &'a Option<Function>,
    upvalue_slots: u128,
    /// The seeded `this`, mirroring `Vm.direct_this`. A body that reads `this`
    /// is admitted only when this is present.
    this_value: Option<&'a Value>,
}

impl WideActivation<'_> {
    fn upvalue_cell(&self, slot: usize) -> Option<&Upvalue> {
        let owner = self.upvalue_owner.as_ref()?;
        let bit = (slot < u128::BITS as usize).then(|| 1_u128 << slot)?;
        (self.upvalue_slots & bit != 0).then_some(())?;
        let index = self.bytecode.direct_readonly_received_upvalue_index(slot)?;
        owner.upvalues.get(index)
    }

    #[cold]
    #[inline(never)]
    fn uninitialized_upvalue(&self, slot: usize) -> Result<Value, RuntimeError> {
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

    #[inline(never)]
    fn eval_binary(&self, left: Value, op: BinaryOp, right: Value) -> Result<Value, RuntimeError> {
        let mut env = self.env.empty_frame();
        crate::operations::eval_binary(left, op, right, &mut env)
    }
}

/// Why an activation stopped running.
enum Action {
    Return(Value),
    Call {
        dst: u16,
        base: u16,
        argc: u8,
        resume_pc: usize,
    },
    CallResolved {
        dst: u16,
        base: u16,
        argc: u8,
        resume_pc: usize,
    },
}

/// The per-body facts this tier needs proven before it may run a call.
struct WideEntry<'a> {
    program: &'a WideProgram,
    upvalue_owner: Option<Function>,
    upvalue_slots: u128,
}

/// Proves a body may run on this tier with the given upvalue source.
fn admit<'a>(
    bytecode: &'a Bytecode,
    upvalues: crate::bytecode::DirectCallUpvalues<'_>,
) -> Option<WideEntry<'a>> {
    let program = super::program_for(bytecode)?;
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
    if bytecode.authoritative_mask_clean() & !upvalue_slots & program.required_authoritative_slots
        != program.required_authoritative_slots
    {
        return None;
    }
    Some(WideEntry {
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
/// environment equal field for field to the one its caller holds.
///
/// A body that reads `this` does not break the proof here: the driver seeds
/// `this` from the call site's receiver instead of a frame binding, exactly
/// as the slot-seeded frame does, so it never touches the shared environment.
/// Only a lexical-`this` arrow inherits `this` from its enclosing environment
/// and cannot be seeded from a receiver.
fn shares_caller_environment(function: &Function, bytecode: &Bytecode, env: &CallEnv) -> bool {
    let host_agrees = match (&function.module_host, env.module_host_ref()) {
        (None, _) => true,
        (Some(callee), Some(caller)) => std::rc::Rc::ptr_eq(callee, caller),
        (Some(_), None) => false,
    };
    let inherits_lexical_this = function.lexical_this && bytecode.uses_lexical_this();
    host_agrees
        && !inherits_lexical_this
        && !function.has_dynamic_function_realm
        && !function.has_dynamic_function_realm_override.get()
        && function.module_imports.is_empty()
        && !function.has_cold_lexical_state()
}

/// One suspended wide activation.
struct WideFrame {
    callee: Value,
    upvalue_owner: Option<Function>,
    upvalue_slots: u128,
    this_value: Option<Value>,
    base: usize,
    len: usize,
    resume_pc: usize,
    dst: u16,
}

#[derive(Default)]
struct FrameStorage {
    registers: Vec<Value>,
    frames: Vec<WideFrame>,
}

thread_local! {
    static FRAME_STORAGE: std::cell::RefCell<FrameStorage> =
        std::cell::RefCell::new(FrameStorage::default());
}

const MAX_FRAMES: usize = 200_000;
const MAX_POOLED_REGISTERS: usize = 4096;

#[inline]
fn reserve_registers(registers: &mut Vec<Value>, needed: usize) {
    if registers.len() < needed {
        registers.resize(needed, Value::Undefined);
    }
}

#[inline(always)]
fn release(value: Value) {
    if matches!(
        value,
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined
    ) {
        std::mem::forget(value);
    }
}

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

#[inline]
fn clear_window(window: &mut [Value]) {
    for slot in window {
        execute::store(slot, Value::Undefined);
    }
}

/// What the driver needs to enter an admitted callee in its own window.
struct InlineCallee {
    upvalue_owner: Option<Function>,
    upvalue_slots: u128,
    register_count: usize,
    requires_this: bool,
    is_strict: bool,
}

/// Proves a callee may run on this driver, in a window of its register stack.
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
        requires_this: entry.program.requires_this,
        is_strict: function.is_strict,
    })
}

/// Runs an admitted body in `env`, together with every admitted body it calls.
fn run(
    bytecode: &Bytecode,
    env: &CallEnv,
    entry: WideEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
    this_value: Option<Value>,
) -> Result<Value, RuntimeError> {
    crate::diagnostics::count!(compact_standalone_activations);
    let mut storage = FRAME_STORAGE.with(|cell| cell.take());
    let result = run_frames(
        bytecode,
        env,
        entry,
        parameter_slots,
        arguments,
        this_value,
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

/// The frame-stack driver. Exactly one activation is "current", and its
/// fields are loop locals; a call pushes it and a return pops.
fn run_frames(
    root_bytecode: &Bytecode,
    env: &CallEnv,
    entry: WideEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
    root_this: Option<Value>,
    storage: &mut FrameStorage,
) -> Result<Value, RuntimeError> {
    let registers = &mut storage.registers;
    let frames = &mut storage.frames;
    debug_assert!(frames.is_empty(), "a root activation starts its own stack");

    let mut current_callee = Value::Undefined;
    let mut current_owner = entry.upvalue_owner;
    let mut current_slots = entry.upvalue_slots;
    let mut current_this = root_this;
    let mut current_base = 0_usize;
    let mut current_len = entry.program.register_count;
    let mut pc = 0_usize;

    reserve_registers(registers, current_len);
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
        let outcome = {
            let bytecode: &Bytecode = running_bytecode(&current_callee, root_bytecode);
            let Some(program) = super::program_for(bytecode) else {
                unwind(registers, frames, current_base + current_len);
                return Err(missing_program());
            };
            let activation = WideActivation {
                bytecode,
                env,
                upvalue_owner: &current_owner,
                upvalue_slots: current_slots,
                this_value: current_this.as_ref(),
            };
            let ops = &program.ops[..];
            let window = &mut registers[current_base..current_base + current_len];
            loop {
                let Some(op) = ops.get(pc) else {
                    break Ok(Action::Return(Value::Undefined));
                };
                pc += 1;
                #[cfg(feature = "perf-counters")]
                crate::diagnostics::update(|c| c.compact_function_ops += 1);
                match *op {
                    WideOp::LoadConst { dst, index } => {
                        let Some(value) = activation.bytecode.constants.get(index as usize) else {
                            break Err(execute::constant_out_of_bounds());
                        };
                        execute::store(
                            &mut window[dst as usize],
                            crate::bytecode::vm_bindings::clone_local_value(value),
                        );
                    }
                    WideOp::Move { dst, src } | WideOp::Dup { src, dst } => {
                        let value =
                            crate::bytecode::vm_bindings::clone_local_value(&window[src as usize]);
                        execute::store(&mut window[dst as usize], value);
                    }
                    WideOp::LoadUpvalueLocal { dst, slot } => {
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
                    WideOp::LoadThis { dst } => {
                        // Admission required a seeded receiver, so this never
                        // fabricates one.
                        let Some(value) = activation.this_value else {
                            break Err(execute::uninitialized_local());
                        };
                        execute::store(&mut window[dst as usize], value.clone());
                    }
                    WideOp::GetPropNamed { dst, obj, index } => {
                        let Some(site) = program.named_reads.get(index as usize) else {
                            break Err(execute::constant_out_of_bounds());
                        };
                        // A fused site peeks its receiver local, so the
                        // register survives; the plain form replaces its own.
                        let object = if dst == obj {
                            std::mem::replace(&mut window[obj as usize], Value::Undefined)
                        } else {
                            crate::bytecode::vm_bindings::clone_local_value(&window[obj as usize])
                        };
                        match property::get_prop_named(object, &site.key, &site.cache, env) {
                            Ok(value) => execute::store(&mut window[dst as usize], value),
                            Err(error) => break Err(error),
                        }
                    }
                    WideOp::SetPropNamed { obj, value, index } => {
                        let Some(site) = program.named_writes.get(index as usize) else {
                            break Err(execute::constant_out_of_bounds());
                        };
                        let object = std::mem::replace(&mut window[obj as usize], Value::Undefined);
                        let assigned =
                            std::mem::replace(&mut window[value as usize], Value::Undefined);
                        match property::set_prop_named(
                            object,
                            &site.key,
                            site.cache.as_ref(),
                            site.is_strict,
                            assigned,
                            env,
                        ) {
                            // The assigned value stays in the object's
                            // register, matching `SetPropNamed`'s stack effect.
                            Ok(value) => execute::store(&mut window[obj as usize], value),
                            Err(error) => break Err(error),
                        }
                    }
                    WideOp::Binary {
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
                    WideOp::Drop { src } => {
                        execute::store(&mut window[src as usize], Value::Undefined);
                    }
                    WideOp::JumpIfFalsy { cond, target } => {
                        let falsy = match &window[cond as usize] {
                            Value::Number(number) => *number == 0.0 || number.is_nan(),
                            other => !crate::is_truthy(other),
                        };
                        if falsy {
                            pc = target as usize;
                        }
                    }
                    WideOp::JumpIfTruthy { cond, target } => {
                        let truthy = match &window[cond as usize] {
                            Value::Number(number) => !(*number == 0.0 || number.is_nan()),
                            other => crate::is_truthy(other),
                        };
                        if truthy {
                            pc = target as usize;
                        }
                    }
                    WideOp::Jump { target } => pc = target as usize,
                    WideOp::Unary { dst, op, src } => {
                        let value = std::mem::replace(&mut window[src as usize], Value::Undefined);
                        match eval_unary(op, value, env) {
                            Ok(value) => execute::store(&mut window[dst as usize], value),
                            Err(error) => break Err(error),
                        }
                    }
                    WideOp::Typeof { dst, src } => {
                        let value = std::mem::replace(&mut window[src as usize], Value::Undefined);
                        let name = crate::bytecode::util::typeof_value(value);
                        execute::store(&mut window[dst as usize], Value::String(name.into()));
                    }
                    WideOp::ToNumeric { dst } => {
                        if !matches!(window[dst as usize], Value::Number(_)) {
                            let value =
                                std::mem::replace(&mut window[dst as usize], Value::Undefined);
                            match to_numeric(value, env) {
                                Ok(value) => execute::store(&mut window[dst as usize], value),
                                Err(error) => break Err(error),
                            }
                        }
                    }
                    WideOp::Update { dst, op } => {
                        if let Value::Number(number) = &mut window[dst as usize] {
                            *number = match op {
                                UpdateOp::Increment => *number + 1.0,
                                UpdateOp::Decrement => *number - 1.0,
                            };
                        } else {
                            let value =
                                std::mem::replace(&mut window[dst as usize], Value::Undefined);
                            match update(op, value, env) {
                                Ok(value) => execute::store(&mut window[dst as usize], value),
                                Err(error) => break Err(error),
                            }
                        }
                    }
                    WideOp::GetProp { dst, obj, key } => {
                        let object = std::mem::replace(&mut window[obj as usize], Value::Undefined);
                        let key = std::mem::replace(&mut window[key as usize], Value::Undefined);
                        match property::get_prop_computed(object, key, env) {
                            Ok(value) => execute::store(&mut window[dst as usize], value),
                            Err(error) => break Err(error),
                        }
                    }
                    WideOp::Call { dst, base, argc } => {
                        break Ok(Action::Call {
                            dst,
                            base,
                            argc,
                            resume_pc: pc,
                        });
                    }
                    WideOp::CallResolved { dst, base, argc } => {
                        break Ok(Action::CallResolved {
                            dst,
                            base,
                            argc,
                            resume_pc: pc,
                        });
                    }
                    WideOp::Return { src } => {
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
        let (dst, base, argc, resume_pc, resolved) = match action {
            Action::Return(value) => {
                clear_window(&mut registers[current_base..current_base + current_len]);
                let Some(caller) = frames.pop() else {
                    return Ok(value);
                };
                current_callee = caller.callee;
                current_owner = caller.upvalue_owner;
                current_slots = caller.upvalue_slots;
                current_this = caller.this_value;
                current_base = caller.base;
                current_len = caller.len;
                pc = caller.resume_pc;
                execute::store(&mut registers[current_base + caller.dst as usize], value);
                continue;
            }
            Action::Call {
                dst,
                base,
                argc,
                resume_pc,
            } => (dst, base, argc, resume_pc, false),
            Action::CallResolved {
                dst,
                base,
                argc,
                resume_pc,
            } => (dst, base, argc, resume_pc, true),
        };
        let callee_index = current_base + base as usize;
        // Both call forms pop the callee register (and a resolved call its
        // receiver register) and push the result at `dst`, so taking the
        // values out costs no reference count.
        let callee = std::mem::replace(&mut registers[callee_index], Value::Undefined);
        let receiver = if resolved {
            Some(std::mem::replace(
                &mut registers[callee_index - 1],
                Value::Undefined,
            ))
        } else {
            None
        };
        let argument_index = callee_index + 1;
        let argument_count = argc as usize;

        if let Some(inline) = inline_callee(&callee, env) {
            if frames.len() >= MAX_FRAMES {
                unwind(registers, frames, current_base + current_len);
                return Err(call_stack_exhausted());
            }
            crate::diagnostics::count!(ordinary_call_attempts);
            crate::diagnostics::count!(compact_direct_calls);
            let callee_base = current_base + current_len;
            let callee_len = inline.register_count;
            reserve_registers(registers, callee_base + callee_len);
            {
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
            // A callee that reads `this` receives the same coercion the
            // general path applies: the resolved receiver, or `undefined`
            // for a plain call.
            let callee_this = if inline.requires_this {
                Some(crate::function::function_call_this(
                    receiver,
                    env,
                    inline.is_strict,
                ))
            } else {
                if let Some(receiver) = receiver {
                    release(receiver);
                }
                None
            };
            frames.push(WideFrame {
                callee: std::mem::replace(&mut current_callee, callee),
                upvalue_owner: current_owner,
                upvalue_slots: current_slots,
                this_value: std::mem::replace(&mut current_this, callee_this),
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
            let activation = WideActivation {
                bytecode: running_bytecode(&current_callee, root_bytecode),
                env,
                upvalue_owner: &current_owner,
                upvalue_slots: current_slots,
                this_value: current_this.as_ref(),
            };
            let arguments = &registers[argument_index..argument_index + argument_count];
            call_from_activation(
                &activation,
                callee,
                receiver.unwrap_or(Value::Undefined),
                arguments,
            )
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

/// A unary operator, with the interpreter's own fast paths: `!` and `void`
/// coerce nothing, a number answers inline, everything else runs the general
/// operator in an empty realm frame like the tier's binary path.
#[inline(never)]
fn eval_unary(op: UnaryOp, value: Value, env: &CallEnv) -> Result<Value, RuntimeError> {
    match op {
        UnaryOp::Not => return Ok(Value::Boolean(!crate::is_truthy(&value))),
        UnaryOp::Void => return Ok(Value::Undefined),
        _ => {}
    }
    if let Some(value) = crate::bytecode::vm_props::fast_number_unary(op, &value) {
        return Ok(value);
    }
    let mut env = env.empty_frame();
    crate::operations::eval_unary(op, value, &mut env)
}

/// `ToNumeric` on a non-number, mirroring `Vm::eval_to_numeric`.
#[cold]
#[inline(never)]
fn to_numeric(value: Value, env: &CallEnv) -> Result<Value, RuntimeError> {
    if matches!(value, Value::BigInt(_)) {
        return Ok(value);
    }
    let mut env = env.empty_frame();
    let primitive = crate::to_primitive_with_hint(value, crate::PreferredType::Number, &mut env)?;
    Ok(match primitive {
        Value::BigInt(_) => primitive,
        value => Value::Number(crate::to_number_with_env(value, &mut env)?),
    })
}

/// `++`/`--` on a non-number, mirroring `Vm::eval_update`.
#[cold]
#[inline(never)]
fn update(op: UpdateOp, value: Value, env: &CallEnv) -> Result<Value, RuntimeError> {
    let one = num_bigint::BigInt::from(1);
    let step = |value: num_bigint::BigInt| match op {
        UpdateOp::Increment => Value::bigint(value + one.clone()),
        UpdateOp::Decrement => Value::bigint(value - one.clone()),
    };
    let value = match value {
        Value::BigInt(value) => return Ok(step(std::rc::Rc::unwrap_or_clone(value))),
        value => value,
    };
    let mut env = env.empty_frame();
    let primitive = crate::to_primitive_with_hint(value, crate::PreferredType::Number, &mut env)?;
    Ok(match primitive {
        Value::BigInt(value) => step(std::rc::Rc::unwrap_or_clone(value)),
        value => {
            let number = crate::to_number_with_env(value, &mut env)?;
            match op {
                UpdateOp::Increment => Value::Number(number + 1.0),
                UpdateOp::Decrement => Value::Number(number - 1.0),
            }
        }
    })
}

#[cold]
#[inline(never)]
fn unwind(registers: &mut [Value], frames: &mut Vec<WideFrame>, live_end: usize) {
    frames.clear();
    let end = live_end.min(registers.len());
    clear_window(&mut registers[..end]);
}

#[cold]
fn missing_program() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "wide compact program disappeared after admission".to_owned(),
    }
}

#[cold]
fn call_stack_exhausted() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "RangeError: Maximum call stack size exceeded".to_owned(),
    }
}

/// Runs `bytecode` on the wide tier without building a `Vm`, or returns
/// `None` having consumed nothing. Every guard runs before `env` or `slots`
/// is taken and before any observable work.
pub(in crate::bytecode) fn try_run_standalone(
    bytecode: &Bytecode,
    env: &mut Option<CallEnv>,
    slots: &mut Option<DirectCallSlots<'_>>,
) -> Option<Result<Value, RuntimeError>> {
    if !environment_is_slot_only(env.as_ref()?) {
        return None;
    }
    let entry = admit(bytecode, slots.as_ref()?.upvalues)?;
    // A body that reads `this` needs the caller to seed a receiver, exactly
    // like the general path's `Vm.direct_this`.
    if entry.program.requires_this && slots.as_ref()?.this_value.is_none() {
        return None;
    }
    let call_env = env.take()?;
    let call_slots = slots.take()?;
    Some(run(
        bytecode,
        &call_env,
        entry,
        call_slots.parameter_slots,
        call_slots.arguments,
        call_slots.this_value,
    ))
}

/// Runs one call the driver could not inline, through the same entries the
/// interpreter's general call path reaches.
#[inline(never)]
fn call_from_activation(
    activation: &WideActivation<'_>,
    callee: Value,
    this_value: Value,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    if crate::function::is_direct_leaf_function(&callee) {
        return crate::function::call_direct_leaf_function(
            callee,
            this_value,
            arguments,
            activation.env,
            activation.env.module_host(),
            #[cfg(feature = "agents")]
            activation.env.agent_context(),
        );
    }
    let mut env = activation.env.empty_frame();
    crate::function::call_function(callee, this_value, arguments.to_vec(), &mut env, false)
}
