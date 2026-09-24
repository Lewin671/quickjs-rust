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
use crate::bytecode::typed_loop::LoopFrame;
use crate::bytecode::vm::{ResumeFrom, Resumed};
use crate::function::{CallEnv, Function, Upvalue};
use crate::{RuntimeError, Value};

/// Everything an admitted body needs to run.
struct WideActivation<'a> {
    bytecode: &'a Bytecode,
    /// The environment this body runs in, borrowed: `shares_caller_environment`
    /// proves it equals what `direct_leaf_function_env` would build.
    env: &'a CallEnv,
    upvalue_owner: Option<&'a Function>,
    upvalue_slots: u128,
    /// The seeded `this`, mirroring `Vm.direct_this`. A body that reads `this`
    /// is admitted only when this is present.
    this_value: Option<&'a Value>,
}

impl WideActivation<'_> {
    fn upvalue_cell(&self, slot: usize) -> Option<&Upvalue> {
        let owner = self.upvalue_owner?;
        let bit = (slot < u128::BITS as usize).then(|| 1_u128 << slot)?;
        (self.upvalue_slots & bit != 0).then_some(())?;
        let index = self.bytecode.readonly_received_upvalue_index(slot)?;
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

    /// The interpreter's diagnostic for a lexical local read or assigned in
    /// its temporal dead zone.
    #[cold]
    #[inline(never)]
    fn uninitialized_lexical(&self, slot: usize) -> RuntimeError {
        RuntimeError {
            thrown: None,
            message: format!(
                "ReferenceError: undefined identifier `{}`",
                self.bytecode.locals[slot].name
            ),
        }
    }

    /// `CompareJump` on operands that are not both numbers: the comparison
    /// the interpreter's `Binary` would run, on copies, since the operands
    /// may be locals.
    #[inline(never)]
    fn compare(&self, left: &Value, op: BinaryOp, right: &Value) -> Result<bool, RuntimeError> {
        if matches!(op, BinaryOp::StrictEq | BinaryOp::StrictNe)
            && let Some(equal) = crate::bytecode::vm_ops::fast_strict_eq(left, right)
        {
            return Ok(equal == (op == BinaryOp::StrictEq));
        }
        let value = self.eval_binary(left.clone(), op, right.clone())?;
        Ok(crate::is_truthy(&value))
    }

    #[inline(never)]
    fn eval_binary(&self, left: Value, op: BinaryOp, right: Value) -> Result<Value, RuntimeError> {
        if let Some(value) = crate::operations::eval_binary_without_env(&left, op, &right) {
            return Ok(value);
        }
        let (left, right) = if op == BinaryOp::Add {
            match crate::bytecode::vm_string_append::concat_primitives(left, right) {
                Ok(value) => return Ok(value),
                Err(operands) => operands,
            }
        } else {
            (left, right)
        };
        let mut env = self.env.empty_frame();
        crate::operations::eval_binary(left, op, right, &mut env)
    }
}

/// The interpreter's error for `RequireObjectCoercible` on `undefined` or
/// `null`.
#[cold]
#[inline(never)]
fn not_coercible() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: cannot destructure undefined or null".to_owned(),
    }
}

/// `ToPropertyKey` of a register that is not already a key, as the
/// interpreter's `ToPropertyKeyForAccess` converts it.
#[cold]
#[inline(never)]
fn to_property_key(register: &mut Value, env: &CallEnv) -> Result<(), RuntimeError> {
    let value = std::mem::replace(register, Value::Undefined);
    let mut env = env.empty_frame();
    let key = match crate::property::try_to_property_key_without_coercion(value) {
        Ok(key) => key,
        Err(value) => crate::to_property_key_value(value, &mut env)?,
    };
    *register = key.into_value();
    Ok(())
}

/// A relational or equality operator between two numbers; the compiler
/// fuses no other operator into `CompareJump`.
#[inline(always)]
fn compare_numbers(left: f64, op: BinaryOp, right: f64) -> bool {
    match op {
        BinaryOp::Lt => left < right,
        BinaryOp::Le => left <= right,
        BinaryOp::Gt => left > right,
        BinaryOp::Ge => left >= right,
        BinaryOp::Ne | BinaryOp::StrictNe => left != right,
        _ => left == right,
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
    New {
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
    if !program.admit_activation() {
        return None;
    }
    let upvalue_slots = bytecode.readonly_received_upvalue_slots().unwrap_or(0);
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
/// environment equal field for field to the one its caller holds, as far as
/// the caller decides: the module host. The function's own part is in its
/// fixed facts.
///
/// A body that reads `this` does not break the proof here: the driver seeds
/// `this` from the call site's receiver instead of a frame binding, exactly
/// as the slot-seeded frame does, so it never touches the shared environment.
/// Only a lexical-`this` arrow inherits `this` from its enclosing environment
/// and cannot be seeded from a receiver.
fn shares_caller_environment(function: &Function, env: &CallEnv) -> bool {
    let host_agrees = match (&function.module_host, env.module_host_ref()) {
        (None, _) => true,
        (Some(callee), Some(caller)) => std::rc::Rc::ptr_eq(callee, caller),
        (Some(_), None) => false,
    };
    host_agrees && !function.has_dynamic_function_realm_override.get()
}

/// `Function::wide_inline_facts`: computed, and eligible to run on this
/// driver as far as the function's fixed facts decide; then the facts it is
/// entered with, the register count in the high half.
const FACTS_KNOWN: u64 = 1;
const FACTS_ELIGIBLE: u64 = 1 << 1;
const FACTS_REQUIRES_THIS: u64 = 1 << 2;
const FACTS_STRICT: u64 = 1 << 3;
const FACTS_LEXICAL_SLOTS: u64 = 1 << 4;
const FACTS_REGISTER_SHIFT: u32 = 32;

/// The part of the inlining proof that depends only on the function, fixed
/// once it is created -- direct-leaf eligibility, its lexical `this`, its
/// dynamic-realm origin, its module imports, its upvalue layout, its
/// program's slot requirements --
/// memoized on the function object with the facts the driver enters it
/// with, so a call reads one word instead of re-deriving each.
#[inline]
fn fixed_inline_facts(callee: &Value, function: &Function, bytecode: &Bytecode) -> u64 {
    let facts = function.wide_inline_facts.get();
    if facts & FACTS_KNOWN != 0 {
        return facts;
    }
    let facts = compute_fixed_inline_facts(callee, function, bytecode);
    function.wide_inline_facts.set(facts);
    facts
}

#[cold]
#[inline(never)]
fn compute_fixed_inline_facts(callee: &Value, function: &Function, bytecode: &Bytecode) -> u64 {
    let inherits_lexical_this = function.lexical_this && bytecode.uses_lexical_this();
    // A method's home object and a class's private environment are not
    // checked: only `super` and private-name operations observe them, and
    // this tier compiles no body that contains either.
    if !crate::function::is_direct_leaf_function(callee)
        || inherits_lexical_this
        || function.has_dynamic_function_realm
        || !function.module_imports.is_empty()
    {
        return FACTS_KNOWN;
    }
    // `admit`'s checks that do not change once the function exists; its
    // activation count is the caller's to check.
    let Some(program) = super::program_for(bytecode) else {
        return FACTS_KNOWN;
    };
    let upvalue_slots = bytecode.readonly_received_upvalue_slots().unwrap_or(0);
    if upvalue_slots != 0 && function.upvalues.len() != bytecode.received_upvalue_slots().len() {
        return FACTS_KNOWN;
    }
    if bytecode.authoritative_mask_clean() & !upvalue_slots & program.required_authoritative_slots
        != program.required_authoritative_slots
    {
        return FACTS_KNOWN;
    }
    let Ok(register_count) = u32::try_from(program.register_count) else {
        return FACTS_KNOWN;
    };
    let mut facts =
        FACTS_KNOWN | FACTS_ELIGIBLE | (u64::from(register_count) << FACTS_REGISTER_SHIFT);
    if program.requires_this {
        facts |= FACTS_REQUIRES_THIS;
    }
    if function.is_strict {
        facts |= FACTS_STRICT;
    }
    if !program.lexical_slots.is_empty() {
        facts |= FACTS_LEXICAL_SLOTS;
    }
    facts
}

/// One suspended wide activation.
struct WideFrame {
    callee: Value,
    this_value: Option<Value>,
    base: usize,
    len: usize,
    resume_pc: usize,
    dst: u16,
    /// Whether the activation above this one was entered by `new`: its
    /// return value is replaced by its receiver unless it is an object.
    constructs: bool,
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
    // Most of a returning window is already empty -- the operations that
    // consumed its operands moved them out -- and an empty register needs no
    // write at all.
    for slot in window {
        if !matches!(slot, Value::Undefined) {
            execute::store(slot, Value::Undefined);
        }
    }
}

/// What the driver needs to enter an admitted callee in its own window.
struct InlineCallee {
    register_count: usize,
    requires_this: bool,
    is_strict: bool,
    /// Whether the callee's window starts with dead-zone markers to seed.
    has_lexical_slots: bool,
}

/// Proves a callee may run on this driver, in a window of its register stack.
fn inline_callee(callee: &Value, env: &CallEnv) -> Option<InlineCallee> {
    let Value::Function(function) = callee else {
        return None;
    };
    let bytecode = function.bytecode.as_ref()?;
    let facts = fixed_inline_facts(callee, function, bytecode);
    if facts & FACTS_ELIGIBLE == 0 || !shares_caller_environment(function, env) {
        return None;
    }
    if !super::program_for(bytecode)?.admit_activation() {
        return None;
    }
    Some(InlineCallee {
        register_count: (facts >> FACTS_REGISTER_SHIFT) as usize,
        requires_this: facts & FACTS_REQUIRES_THIS != 0,
        is_strict: facts & FACTS_STRICT != 0,
        has_lexical_slots: facts & FACTS_LEXICAL_SLOTS != 0,
    })
}

/// Proves `new callee(...)` may run on this driver: an ordinary (not class,
/// bound or native) constructor the driver would inline as a call, whose
/// `prototype` is an ordinary object. Returns the fresh receiver.
#[inline(never)]
fn inline_constructor(callee: &Value, env: &CallEnv) -> Option<(InlineCallee, crate::ObjectRef)> {
    let Value::Function(function) = callee else {
        return None;
    };
    if function.native.is_some()
        || function.bound.is_some()
        || !function.constructable
        || function.is_class_constructor
    {
        return None;
    }
    let inline = inline_callee(callee, env)?;
    let prototype = match function.own_property("prototype") {
        Some(property) if !property.is_accessor() => match property.value {
            Value::Object(prototype) if !crate::symbol::is_symbol_primitive(&prototype) => {
                prototype
            }
            _ => return None,
        },
        _ => return None,
    };
    let receiver = crate::ObjectRef::with_prototype_slot(
        std::collections::HashMap::new(),
        Some(crate::Prototype::Object(prototype)),
    );
    Some((inline, receiver))
}

/// Enters `new callee(...)` as a frame of this driver when
/// `inline_constructor` admits it, returning `Ok(None)`; otherwise hands the
/// callee back for the general construct path. Out of line, so the driver
/// loop's own code is the same whether or not a body constructs.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn enter_constructor(
    callee: Value,
    root_bytecode: &Bytecode,
    env: &CallEnv,
    registers: &mut Vec<Value>,
    frames: &mut Vec<WideFrame>,
    current_callee: &mut Value,
    current_this: &mut Option<Value>,
    current_base: &mut usize,
    current_len: &mut usize,
    argument_index: usize,
    argc: u8,
    dst: u16,
    resume_pc: usize,
) -> Result<Option<Value>, RuntimeError> {
    let Some((inline, receiver)) = inline_constructor(&callee, env) else {
        return Ok(Some(callee));
    };
    if frames.len() >= MAX_FRAMES {
        return Err(call_stack_exhausted());
    }
    crate::diagnostics::count!(ordinary_call_attempts);
    crate::diagnostics::count!(compact_direct_calls);
    let callee_base = *current_base + *current_len;
    let callee_len = inline.register_count;
    reserve_registers(registers, callee_base + callee_len);
    {
        let parameter_slots = callee_parameter_slots(&callee);
        let (caller_side, callee_side) = registers.split_at_mut(callee_base);
        for index in 0..argc as usize {
            let value =
                std::mem::replace(&mut caller_side[argument_index + index], Value::Undefined);
            match parameter_slots.get(index) {
                Some(&slot) if slot < callee_len => callee_side[slot] = value,
                _ => release(value),
            }
        }
    }
    if inline.has_lexical_slots
        && let Some(program) = super::program_for(running_bytecode(&callee, root_bytecode))
    {
        seed_lexical_markers(
            program,
            &mut registers[callee_base..callee_base + callee_len],
        );
    }
    frames.push(WideFrame {
        callee: std::mem::replace(current_callee, callee),
        this_value: current_this.replace(Value::Object(receiver)),
        base: std::mem::replace(current_base, callee_base),
        len: std::mem::replace(current_len, callee_len),
        resume_pc,
        dst,
        constructs: true,
    });
    Ok(None)
}

/// A constructor's result: the returned value when it is an object, else
/// the receiver, as the general construct path decides.
fn constructed(value: Value, receiver: Option<Value>) -> Value {
    match value {
        Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Object(_)
        | Value::Proxy(_) => value,
        other => {
            release(other);
            receiver.unwrap_or(Value::Undefined)
        }
    }
}

/// Runs an admitted body in `env`, together with every admitted body it calls.
/// The error `Op::Throw` raises when no handler is active, which is always
/// the case in an admitted body. Out of line: the dispatch loop's arms stay
/// one call each (docs/performance-knowledge.md, "Keep hot dispatch arms
/// tiny").
#[cold]
#[inline(never)]
fn thrown(slot: &mut Value) -> RuntimeError {
    let value = std::mem::replace(slot, Value::Undefined);
    RuntimeError {
        thrown: Some(Box::new(value.clone())),
        message: format!(
            "throw statement executed: {}",
            crate::conversion::error_value(value)
        ),
    }
}

/// The arity an exit is spelled with; no call has it (`MAX_CALL_ARITY`).
const EXIT_ARGC: u8 = u8::MAX;

/// Where a root activation's received cells come from, for an exit to hand
/// the same source to the interpreter frame. An inlined callee's comes from
/// its own `Function`.
#[derive(Clone, Copy)]
struct RootExit<'a> {
    upvalues: crate::bytecode::DirectCallUpvalues<'a>,
    realm_upvalue_slots: u128,
}

/// What became of an exit.
enum ExitOutcome {
    /// The interpreter frame finished the activation.
    Finished(Result<Value, RuntimeError>),
    /// The activation continues here at wide instruction `pc`: a loop no
    /// accelerator claims came back, or its exit was already known to.
    Continue { pc: usize },
}

/// Hands the current activation to an interpreter frame that resumes at
/// bytecode instruction `ip`, and returns that frame's completion value. The
/// frame is the one the general call path builds for this call -- the same
/// environment (which admission proved the activation's own), the same
/// received cells and `this` -- brought to this activation's state.
///
/// At a probed backedge the frame may hand the activation back instead
/// (`vm/wide_resume.rs`); that backedge's exit then stays declined, so the
/// loop runs here from then on.
#[cold]
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn exit_to_interpreter(
    callee: &Value,
    root_bytecode: &Bytecode,
    root: RootExit<'_>,
    ip: u32,
    depth: u16,
    resume_pc: usize,
    window: &mut [Value],
    env: &CallEnv,
    this_value: Option<Value>,
) -> ExitOutcome {
    let bytecode = running_bytecode(callee, root_bytecode);
    let Some(program) = super::program_for(bytecode) else {
        return ExitOutcome::Finished(Err(missing_program()));
    };
    let probe = program.probed_backedge(ip);
    if let Some(index) = probe
        && program.backedge_is_native(index)
    {
        return ExitOutcome::Continue {
            pc: program.backedge_jump_pc(index),
        };
    }
    if let Some(crate::bytecode::ir::Op::NewObjectDataLiteral { shape }) =
        bytecode.code.get(ip as usize)
    {
        let top = usize::from(program.local_registers) + usize::from(depth);
        let base = top - shape.input_len();
        let object = property::object_data_literal(shape, &mut window[base..top], env);
        execute::store(&mut window[base], object);
        return ExitOutcome::Continue { pc: resume_pc };
    }
    if let Some(crate::bytecode::ir::Op::SetPropIndex { index, .. }) =
        bytecode.code.get(ip as usize)
    {
        let top = program.local_registers + depth;
        if property::try_plain_set_index(window, top - 2, top - 1, *index, env) {
            return ExitOutcome::Continue { pc: resume_pc };
        }
    }
    // A `for-in`'s key list and its per-key recheck, answered in place so
    // the loop stays on this tier (`compile::is_exit_safe`).
    match bytecode.code.get(ip as usize) {
        Some(crate::bytecode::ir::Op::EnumerateKeys { cache }) => {
            if let Some(pc) = program.resume_pc(ip as usize + 1, usize::from(depth)) {
                let top = usize::from(program.local_registers) + usize::from(depth);
                let target = std::mem::replace(&mut window[top - 1], Value::Undefined);
                let mut call_env = env.empty_frame();
                return match crate::bytecode::vm_ops::enumerate_keys_cached(
                    &target,
                    cache,
                    &mut call_env,
                ) {
                    Ok(keys) => {
                        window[top - 1] = Value::Array(keys);
                        ExitOutcome::Continue { pc }
                    }
                    Err(error) => ExitOutcome::Finished(Err(error)),
                };
            }
        }
        Some(crate::bytecode::ir::Op::ForInKeyIsEnumerable) => {
            let top = usize::from(program.local_registers) + usize::from(depth);
            if let Some(pc) = program.resume_pc(ip as usize + 1, usize::from(depth) - 1)
                && let Value::String(key) = &window[top - 1]
            {
                let key = key.clone();
                let target = std::mem::replace(&mut window[top - 2], Value::Undefined);
                execute::store(&mut window[top - 1], Value::Undefined);
                let mut call_env = env.empty_frame();
                return match crate::bytecode::vm_ops::for_in_property_is_enumerable(
                    target,
                    &key,
                    &mut call_env,
                ) {
                    Ok(enumerable) => {
                        window[top - 2] = Value::Boolean(enumerable);
                        ExitOutcome::Continue { pc }
                    }
                    Err(error) => ExitOutcome::Finished(Err(error)),
                };
            }
        }
        _ => {}
    }
    // `g = g + value` on a global string: appended in place, then the store
    // is skipped (see `compile::appends_to_global`).
    if let (
        Some(crate::bytecode::ir::Op::Binary(qjs_ast::BinaryOp::Add)),
        Some(crate::bytecode::ir::Op::StoreLocalOrGlobalSloppy { name, .. }),
    ) = (
        bytecode.code.get(ip as usize),
        bytecode.code.get(ip as usize + 1),
    ) && let Some(pc) = program.resume_pc(ip as usize + 2, usize::from(depth) - 2)
    {
        let top = usize::from(program.local_registers) + usize::from(depth);
        let (left, right) = window[top - 2..top].split_at_mut(1);
        if property::try_append_global_var(name, &mut left[0], &mut right[0], env) {
            return ExitOutcome::Continue { pc };
        }
    }
    if let Some(crate::bytecode::ir::Op::StoreLocalOrGlobalSloppy { name, .. }) =
        bytecode.code.get(ip as usize)
    {
        let top = usize::from(program.local_registers) + usize::from(depth) - 1;
        if property::try_store_global_var(name, &window[top], env) {
            execute::store(&mut window[top], Value::Undefined);
            return ExitOutcome::Continue { pc: resume_pc };
        }
    }
    if let Some(crate::bytecode::ir::Op::SetProp { .. }) = bytecode.code.get(ip as usize) {
        let operand = |offset: u16| program.local_registers + depth - offset;
        if property::try_plain_set_prop(window, operand(3), operand(2), operand(1), env) {
            return ExitOutcome::Continue { pc: resume_pc };
        }
    }
    let (upvalues, realm_upvalue_slots) = match callee {
        Value::Function(function) => (
            crate::bytecode::DirectCallUpvalues::Function(function),
            function.realm_upvalue_slots,
        ),
        _ => (root.upvalues, root.realm_upvalue_slots),
    };
    let (mut ip, mut depth) = (ip, depth);
    let probe_target = probe.and_then(|_| match bytecode.code.get(ip as usize) {
        Some(crate::bytecode::ir::Op::Jump(target)) => Some(*target),
        _ => None,
    });
    let mut from = match probe_target {
        Some(target) => ResumeFrom::ProbedBackedge(target),
        None => ResumeFrom::Exit,
    };
    if let Some(header) = probe_target {
        match run_typed_loop_here(
            bytecode,
            program,
            env,
            window,
            upvalues.as_slice(),
            this_value.as_ref(),
            header,
            ip as usize,
            depth as usize,
        ) {
            LoopHere::Declined => {}
            LoopHere::Continue { pc } => return ExitOutcome::Continue { pc },
            // The program deoptimized: the interpreter resumes where it
            // stopped, with the stack it rebuilt.
            LoopHere::Deoptimized {
                ip: resume,
                depth: stack_depth,
                declined_typed_loop_programs,
            } => {
                let (Ok(resume), Ok(stack_depth)) =
                    (u32::try_from(resume), u16::try_from(stack_depth))
                else {
                    return ExitOutcome::Finished(Err(missing_program()));
                };
                (ip, depth) = (resume, stack_depth);
                from = ResumeFrom::LoopDeoptimized {
                    declined_typed_loop_programs,
                };
            }
        }
    }
    // `QJS_CF_TRACE=1` names every exit: the body, and the instruction the
    // interpreter resumes at.
    #[cfg(feature = "perf-counters")]
    if std::env::var_os("QJS_CF_TRACE").is_some() {
        eprintln!(
            "CFEXIT params=({}) len={} ip {} op {:?}{}",
            bytecode.parameter_names().join(","),
            bytecode.code.len(),
            ip,
            bytecode.code.get(ip as usize),
            if probe.is_some() { " probed" } else { "" }
        );
    }
    let local_registers = program.local_registers as usize;
    let (locals, stack) = window.split_at_mut(local_registers);
    let slots = DirectCallSlots {
        this_value,
        parameter_slots: bytecode.parameter_slots(),
        arguments: &[],
        upvalues,
        realm_upvalue_slots,
    };
    let registers = crate::bytecode::vm::WideRegisters {
        locals,
        own_locals: program.own_locals,
        stack,
        depth: depth as usize,
        tdz_marker: &program.tdz_marker,
    };
    match crate::bytecode::vm::resume_direct_call_bytecode(
        bytecode,
        env.clone(),
        slots,
        ip as usize,
        registers,
        from,
    ) {
        Resumed::Finished(result) => {
            program.record_exit();
            ExitOutcome::Finished(result)
        }
        // The interpreter frame only ran the accelerated loop, which is the
        // cost the general path would have paid too; such an exit does not
        // count toward judging the body exit-heavy.
        Resumed::LoopFinished { ip: resume, depth } => match program.resume_pc(resume, depth) {
            Some(pc) => ExitOutcome::Continue { pc },
            None => ExitOutcome::Finished(Err(missing_program())),
        },
        Resumed::HandedBack { backedge } => {
            program.record_exit();
            let Some(index) = u32::try_from(backedge)
                .ok()
                .and_then(|backedge| program.probed_backedge(backedge))
            else {
                return ExitOutcome::Finished(Err(missing_program()));
            };
            // `QJS_CF_TRACE=1` names each loop handed back to this tier.
            #[cfg(feature = "perf-counters")]
            if std::env::var_os("QJS_CF_TRACE").is_some() {
                eprintln!(
                    "CFNATIVE params=({}) len={} ip {backedge}",
                    bytecode.parameter_names().join(","),
                    bytecode.code.len()
                );
            }
            program.keep_backedge_native(index);
            ExitOutcome::Continue {
                pc: program.backedge_jump_pc(index),
            }
        }
    }
}

/// What running a loop's typed program from an exit did.
enum LoopHere {
    /// No program ran; the exit proceeds as before.
    Declined,
    /// The loop finished; the activation continues at wide instruction `pc`.
    Continue { pc: usize },
    /// The program stopped at bytecode `ip` with `depth` stack values in the
    /// activation's stack registers, for the interpreter to continue with
    /// the programs it would have declined.
    Deoptimized {
        ip: usize,
        depth: usize,
        declined_typed_loop_programs: u128,
    },
}

/// Runs, against this activation's registers, the typed loop program the
/// interpreter would enter at the probed backedge `backedge` -- unless one of
/// the accelerators the interpreter consults before it has a plan there.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn run_typed_loop_here(
    bytecode: &Bytecode,
    program: &WideProgram,
    env: &CallEnv,
    window: &mut [Value],
    upvalues: &[Upvalue],
    this_value: Option<&Value>,
    header: usize,
    backedge: usize,
    base_depth: usize,
) -> LoopHere {
    let plans = crate::bytecode::vm_loop_dispatch::LoopPlanView::for_bytecode(bytecode);
    // The interpreter consults the other accelerators before the typed tier,
    // at this edge and at the edges of loops inside this one; a region any
    // of them has a plan in stays with the interpreter's order.
    let overlaps = |(plan_header, plan_backedge): (usize, usize)| {
        plan_backedge == backedge || (header..=backedge).contains(&plan_header)
    };
    let consulted_first = plans.numeric.iter().any(|plan| overlaps(plan.region()))
        || plans
            .shared_numeric_mutation
            .iter()
            .any(|plan| overlaps(plan.region()))
        || plans.control.iter().any(|plan| overlaps(plan.region()));
    if consulted_first
        || !plans
            .typed
            .iter()
            .any(|typed| typed.header() == header && typed.backedge() == backedge)
    {
        return LoopHere::Declined;
    }
    let local_registers = program.local_registers as usize;
    let (locals, stack) = window.split_at_mut(local_registers);
    let mut frame = super::loop_frame::WideLoopFrame::new(
        bytecode,
        env,
        locals,
        program.own_locals,
        upvalues,
        bytecode.readonly_received_upvalue_slots().unwrap_or(0),
        this_value,
    );
    if !crate::bytecode::typed_loop::try_run_typed_loop(&mut frame, plans, header, backedge) {
        return LoopHere::Declined;
    }
    // A declined attempt hands the edge to the interpreter, which counts it.
    crate::diagnostics::count!(loop_backedges);
    crate::diagnostics::count!(loop_plan_entries);
    let (resume, deoptimized) = (frame.resume_ip, frame.deoptimized);
    let declined_typed_loop_programs = frame.declined_typed_loop_programs();
    let values = std::mem::take(&mut frame.stack);
    let Some(resume) = resume else {
        return LoopHere::Declined;
    };
    // The program rebuilt the stack above the loop's own base; what lay
    // below it when the loop began is still in its registers.
    let depth = base_depth + values.len();
    if depth > stack.len() {
        return LoopHere::Declined;
    }
    for (register, value) in stack[base_depth..].iter_mut().zip(values) {
        execute::store(register, value);
    }
    // `QJS_CF_TRACE=1` names each loop program run from an exit.
    #[cfg(feature = "perf-counters")]
    if std::env::var_os("QJS_CF_TRACE").is_some() {
        eprintln!(
            "CFLOOP params=({}) len={} ip {backedge} {} at {resume} depth {depth} expects {:?}",
            bytecode.parameter_names().join(","),
            bytecode.code.len(),
            if deoptimized { "deoptimized" } else { "ran" },
            program.ip_depth.get(resume)
        );
    }
    match (!deoptimized)
        .then(|| program.resume_pc(resume, depth))
        .flatten()
    {
        Some(pc) => LoopHere::Continue { pc },
        None => LoopHere::Deoptimized {
            ip: resume,
            depth,
            declined_typed_loop_programs,
        },
    }
}

fn run(
    bytecode: &Bytecode,
    env: &CallEnv,
    entry: WideEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
    this_value: Option<Value>,
    root: RootExit<'_>,
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
        root,
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
/// fields are loop locals; a call pushes it and a return pops. Inlined into
/// `run`, as it always was before exits enlarged it: the split changed the
/// dispatch loop's register allocation (docs/performance-knowledge.md,
/// "Codegen").
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn run_frames(
    root_bytecode: &Bytecode,
    env: &CallEnv,
    entry: WideEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
    root_this: Option<Value>,
    root: RootExit<'_>,
    storage: &mut FrameStorage,
) -> Result<Value, RuntimeError> {
    let registers = &mut storage.registers;
    let frames = &mut storage.frames;
    debug_assert!(frames.is_empty(), "a root activation starts its own stack");

    let mut current_callee = Value::Undefined;
    // The root body's upvalue owner; a callee entered on this driver owns
    // its own upvalues, and its read-only slots are its bytecode's.
    let root_owner = entry.upvalue_owner;
    let root_slots = entry.upvalue_slots;
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
    seed_lexical_markers(entry.program, &mut registers[..current_len]);

    loop {
        let outcome = {
            let bytecode: &Bytecode = running_bytecode(&current_callee, root_bytecode);
            let Some(program) = super::program_for(bytecode) else {
                unwind(registers, frames, current_base + current_len);
                return Err(missing_program());
            };
            let (upvalue_owner, upvalue_slots) = match &current_callee {
                Value::Function(function) => (
                    Some(function),
                    bytecode.readonly_received_upvalue_slots().unwrap_or(0),
                ),
                _ => (root_owner.as_ref(), root_slots),
            };
            let activation = WideActivation {
                bytecode,
                env,
                upvalue_owner,
                upvalue_slots,
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
                    WideOp::GetPropThis { dst, index } => {
                        let (Some(site), Some(receiver)) = (
                            program.named_reads.get(index as usize),
                            activation.this_value,
                        ) else {
                            break Err(execute::uninitialized_local());
                        };
                        match property::get_prop_named(receiver, &site.key, &site.cache, env) {
                            Ok(value) => execute::store(&mut window[dst as usize], value),
                            Err(error) => break Err(error),
                        }
                    }
                    WideOp::GetPropNamed { dst, obj, index } => {
                        let Some(site) = program.named_reads.get(index as usize) else {
                            break Err(execute::constant_out_of_bounds());
                        };
                        // The receiver is borrowed: a fused site peeks its
                        // local, and the plain form's result replaces it.
                        match property::get_prop_named(
                            &window[obj as usize],
                            &site.key,
                            &site.cache,
                            env,
                        ) {
                            Ok(value) => execute::store(&mut window[dst as usize], value),
                            Err(error) => break Err(error),
                        }
                    }
                    WideOp::SetPropNamed { obj, value, index } => {
                        let Some(site) = program.named_writes.get(index as usize) else {
                            break Err(execute::constant_out_of_bounds());
                        };
                        let assigned =
                            std::mem::replace(&mut window[value as usize], Value::Undefined);
                        match property::set_prop_named(
                            &window[obj as usize],
                            &site.key,
                            site.cache.as_ref(),
                            site.is_strict,
                            assigned,
                            env,
                            Some(&site.creation),
                        ) {
                            // The assigned value stays in the object's
                            // register, matching `SetPropNamed`'s stack effect.
                            Ok(value) => execute::store(&mut window[obj as usize], value),
                            Err(error) => break Err(error),
                        }
                    }
                    WideOp::SetPropThis { dst, value, index } => {
                        let (Some(site), Some(receiver)) = (
                            program.named_writes.get(index as usize),
                            activation.this_value,
                        ) else {
                            break Err(execute::uninitialized_local());
                        };
                        let assigned =
                            std::mem::replace(&mut window[value as usize], Value::Undefined);
                        match property::set_prop_named(
                            receiver,
                            &site.key,
                            site.cache.as_ref(),
                            site.is_strict,
                            assigned,
                            env,
                            Some(&site.creation),
                        ) {
                            Ok(value) => execute::store(&mut window[dst as usize], value),
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
                    WideOp::CheckCoercible { src } => {
                        if matches!(window[src as usize], Value::Undefined | Value::Null) {
                            break Err(not_coercible());
                        }
                    }
                    WideOp::ToPropertyKey { dst } => {
                        let is_key = match &window[dst as usize] {
                            Value::String(_) => true,
                            Value::Number(number) => {
                                crate::bytecode::vm_props::array_index_from_number(*number)
                                    .is_some()
                            }
                            _ => false,
                        };
                        if !is_key
                            && let Err(error) = to_property_key(&mut window[dst as usize], env)
                        {
                            break Err(error);
                        }
                    }
                    WideOp::CompareJump {
                        op,
                        left,
                        right,
                        target,
                    } => {
                        let holds = match (&window[left as usize], &window[right as usize]) {
                            (Value::Number(left), Value::Number(right)) => {
                                compare_numbers(*left, op, *right)
                            }
                            (left, right) => match activation.compare(left, op, right) {
                                Ok(holds) => holds,
                                Err(error) => break Err(error),
                            },
                        };
                        if !holds {
                            pc = target as usize;
                        }
                    }
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
                        execute::store(&mut window[dst as usize], Value::String(name));
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
                    WideOp::GetPropIndex { dst, obj, index } => {
                        let object = if dst == obj {
                            std::mem::replace(&mut window[obj as usize], Value::Undefined)
                        } else {
                            crate::bytecode::vm_bindings::clone_local_value(&window[obj as usize])
                        };
                        match property::get_prop_computed(
                            object,
                            Value::Number(f64::from(index)),
                            env,
                        ) {
                            Ok(value) => execute::store(&mut window[dst as usize], value),
                            Err(error) => break Err(error),
                        }
                    }
                    WideOp::LoadGlobal { dst, index } => {
                        let Some(name) = program.global_names.get(index as usize) else {
                            break Err(execute::constant_out_of_bounds());
                        };
                        match property::load_global(name, env) {
                            Ok(value) => execute::store(&mut window[dst as usize], value),
                            Err(error) => break Err(error),
                        }
                    }
                    WideOp::NewArray { dst, base, count } => {
                        let values: Vec<Value> = (0..count as usize)
                            .map(|offset| {
                                std::mem::replace(
                                    &mut window[base as usize + offset],
                                    Value::Undefined,
                                )
                            })
                            .collect();
                        execute::store(
                            &mut window[dst as usize],
                            Value::Array(crate::ArrayRef::new(values)),
                        );
                    }
                    WideOp::ClearLocal { slot } => {
                        execute::store(&mut window[slot as usize], program.tdz_marker.clone());
                    }
                    WideOp::MoveChecked { dst, src } => {
                        if window[src as usize].is_uninitialized_lexical_marker() {
                            break Err(activation.uninitialized_lexical(src as usize));
                        }
                        let value =
                            crate::bytecode::vm_bindings::clone_local_value(&window[src as usize]);
                        execute::store(&mut window[dst as usize], value);
                    }
                    WideOp::AssignChecked { dst, src } => {
                        if window[dst as usize].is_uninitialized_lexical_marker() {
                            break Err(activation.uninitialized_lexical(dst as usize));
                        }
                        let value =
                            crate::bytecode::vm_bindings::clone_local_value(&window[src as usize]);
                        execute::store(&mut window[dst as usize], value);
                    }
                    WideOp::New { dst, base, argc } => {
                        break Ok(Action::New {
                            dst,
                            base,
                            argc,
                            resume_pc: pc,
                        });
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
                    WideOp::Throw { src } => {
                        break Err(thrown(&mut window[src as usize]));
                    }
                    // Spelled as a call with an arity no call has, so the
                    // driver's action type and its match stay exactly as they
                    // were: growing either re-rolls this loop's register
                    // allocation (docs/performance-knowledge.md, "Codegen").
                    WideOp::Exit { ip, depth } => {
                        break Ok(Action::Call {
                            dst: depth,
                            base: 0,
                            argc: EXIT_ARGC,
                            resume_pc: ip as usize,
                        });
                    }
                }
            }
        };
        let action = match outcome {
            Ok(Action::Call {
                dst: depth,
                argc: EXIT_ARGC,
                resume_pc: ip,
                ..
            }) => {
                match exit_to_interpreter(
                    &current_callee,
                    root_bytecode,
                    root,
                    ip as u32,
                    depth,
                    pc,
                    &mut registers[current_base..current_base + current_len],
                    env,
                    current_this.clone(),
                ) {
                    ExitOutcome::Continue { pc: resume } => {
                        pc = resume;
                        continue;
                    }
                    ExitOutcome::Finished(Ok(value)) => Action::Return(value),
                    ExitOutcome::Finished(Err(error)) => {
                        unwind(registers, frames, current_base + current_len);
                        return Err(error);
                    }
                }
            }
            Ok(action) => action,
            Err(error) => {
                unwind(registers, frames, current_base + current_len);
                return Err(error);
            }
        };
        if let Action::New {
            dst,
            base,
            argc,
            resume_pc,
        } = action
        {
            let callee_index = current_base + base as usize;
            let callee = std::mem::replace(&mut registers[callee_index], Value::Undefined);
            // An admitted ordinary constructor runs on this driver like a
            // call, with a fresh receiver as `this`.
            let callee = match enter_constructor(
                callee,
                root_bytecode,
                env,
                registers,
                frames,
                &mut current_callee,
                &mut current_this,
                &mut current_base,
                &mut current_len,
                callee_index + 1,
                argc,
                dst,
                resume_pc,
            ) {
                Ok(None) => {
                    pc = 0;
                    continue;
                }
                Ok(Some(callee)) => callee,
                Err(error) => {
                    unwind(registers, frames, current_base + current_len);
                    return Err(error);
                }
            };
            let arguments = &registers[callee_index + 1..callee_index + 1 + argc as usize];
            match construct_from_activation(env, callee, arguments) {
                Ok(value) => {
                    execute::store(&mut registers[current_base + dst as usize], value);
                    pc = resume_pc;
                    continue;
                }
                Err(error) => {
                    unwind(registers, frames, current_base + current_len);
                    return Err(error);
                }
            }
        }
        let (dst, base, argc, resume_pc, resolved) = match action {
            Action::Return(value) => {
                clear_window(&mut registers[current_base..current_base + current_len]);
                let Some(caller) = frames.pop() else {
                    return Ok(value);
                };
                let value = if caller.constructs {
                    constructed(value, current_this.take())
                } else {
                    value
                };
                current_callee = caller.callee;
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
            Action::New { .. } => unreachable!("construction was handled above"),
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
                        // The window was cleared when it was last returned
                        // from, so the register holds a primitive to forget.
                        Some(&slot) if slot < callee_len => {
                            release(std::mem::replace(&mut callee_side[slot], value));
                        }
                        _ => release(value),
                    }
                }
            }
            // A callee that reads `this` receives the same coercion the
            // general path applies: the resolved receiver, or `undefined`
            // for a plain call.
            let callee_this = if inline.requires_this {
                // An object receiver is `this` unchanged, strict or not;
                // only a primitive (a symbol is an object here) or a missing
                // receiver takes the coercion.
                Some(match receiver {
                    Some(this @ (Value::Array(_) | Value::Function(_))) => this,
                    Some(Value::Object(object)) if !crate::symbol::is_symbol_primitive(&object) => {
                        Value::Object(object)
                    }
                    receiver => {
                        crate::function::function_call_this(receiver, env, inline.is_strict)
                    }
                })
            } else {
                if let Some(receiver) = receiver {
                    release(receiver);
                }
                None
            };
            if inline.has_lexical_slots
                && let Some(program) = super::program_for(running_bytecode(&callee, root_bytecode))
            {
                seed_lexical_markers(
                    program,
                    &mut registers[callee_base..callee_base + callee_len],
                );
            }
            frames.push(WideFrame {
                callee: std::mem::replace(&mut current_callee, callee),
                this_value: std::mem::replace(&mut current_this, callee_this),
                base: current_base,
                len: current_len,
                resume_pc,
                dst,
                constructs: false,
            });
            current_base = callee_base;
            current_len = callee_len;
            pc = 0;
            continue;
        }

        let outcome = {
            let bytecode = running_bytecode(&current_callee, root_bytecode);
            let (upvalue_owner, upvalue_slots) = match &current_callee {
                Value::Function(function) => (
                    Some(function),
                    bytecode.readonly_received_upvalue_slots().unwrap_or(0),
                ),
                _ => (root_owner.as_ref(), root_slots),
            };
            let activation = WideActivation {
                bytecode,
                env,
                upvalue_owner,
                upvalue_slots,
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

/// Starts every own lexical slot of a fresh window in its temporal dead
/// zone, as the interpreter starts such slots uninitialized.
#[inline]
fn seed_lexical_markers(program: &WideProgram, window: &mut [Value]) {
    for &slot in &program.lexical_slots {
        if let Some(register) = window.get_mut(slot as usize) {
            execute::store(register, program.tdz_marker.clone());
        }
    }
}

/// `new callee(arguments)` from an admitted body: the direct-leaf construct
/// path first, then the general `construct_function`, in an empty realm
/// frame exactly as the interpreter's own `construct_callee` does for a
/// callee that cannot observe the caller's frame.
#[inline(never)]
fn construct_from_activation(
    env: &CallEnv,
    callee: Value,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    if let Some(result) = crate::function::try_construct_direct_leaf_function(
        &callee,
        arguments,
        env,
        env.module_host(),
        #[cfg(feature = "agents")]
        env.agent_context(),
    ) {
        return result;
    }
    if let Some(array) = construct_plain_array(env, &callee, arguments) {
        return Ok(array);
    }
    let mut env = env.empty_frame();
    crate::function::construct_function(callee.clone(), callee, arguments.to_vec(), &mut env)
}

/// `new Array(...)` on the realm's own Array constructor, whose `prototype`
/// is non-writable and non-configurable: the array the constructor builds,
/// without first allocating the ordinary receiver the general construct
/// path makes for it. A length that is not a valid array length, or any
/// other constructor, takes the general path and its errors.
fn construct_plain_array(env: &CallEnv, callee: &Value, arguments: &[Value]) -> Option<Value> {
    let Value::Function(function) = callee else {
        return None;
    };
    if function.native != Some(crate::NativeFunction::Array)
        || function.bound.is_some()
        || env.array_prototype_intrinsic_override().is_some()
        || env.has_dynamic_function_realm_global()
    {
        return None;
    }
    let realm_prototype = env.realm().array_prototype()?;
    match function.own_property("prototype") {
        Some(property)
            if !property.is_accessor()
                && matches!(&property.value, Value::Object(prototype) if prototype.ptr_eq(&realm_prototype)) =>
            {}
        _ => return None,
    }
    let array = match arguments {
        [Value::Number(length)] => {
            let valid = length.fract() == 0.0 && (0.0..4_294_967_296.0).contains(length);
            if !valid {
                return None;
            }
            crate::ArrayRef::new_with_length(*length as usize)
        }
        values => crate::ArrayRef::new(values.to_vec()),
    };
    Some(Value::Array(array))
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
    let root = RootExit {
        upvalues: call_slots.upvalues,
        realm_upvalue_slots: call_slots.realm_upvalue_slots,
    };
    Some(run(
        bytecode,
        &call_env,
        entry,
        call_slots.parameter_slots,
        call_slots.arguments,
        call_slots.this_value,
        root,
    ))
}

/// The wide half of `compact_fn::try_run_in_caller_env`: the caller has
/// already proved the environment is shareable and the numeric tier declined.
/// A body that reads `this` is excluded, as it is for a standalone activation
/// whose caller seeded no receiver.
pub(crate) fn try_run_in_caller_env(
    bytecode: &Bytecode,
    upvalues: crate::bytecode::DirectCallUpvalues<'_>,
    arguments: &[Value],
    env: &CallEnv,
) -> Option<Result<Value, RuntimeError>> {
    let entry = admit(bytecode, upvalues)?;
    if entry.program.requires_this {
        return None;
    }
    crate::diagnostics::count!(compact_caller_env_calls);
    let root = RootExit {
        upvalues,
        realm_upvalue_slots: upvalues
            .function()
            .map_or(0, |function| function.realm_upvalue_slots),
    };
    Some(run(
        bytecode,
        env,
        entry,
        bytecode.parameter_slots(),
        arguments,
        None,
        root,
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
    // The interpreter's native fast paths (`charCodeAt`, `String.fromCharCode`,
    // the Math functions, ...) need no frame, and the realm frame is built
    // only by the arms that ask for one.
    if matches!(&callee, Value::Function(function) if function.native_kind().is_some())
        && let Some(result) = crate::bytecode::vm_call::try_fast_global_native_call(
            &callee,
            &this_value,
            arguments,
            &|| activation.env.empty_frame(),
        )
    {
        return result;
    }
    let mut env = activation.env.empty_frame();
    // A plain native takes its arguments where they are; only a bytecode
    // callee, a bound function or a proxy needs an owned vector.
    if let Value::Function(function) = &callee
        && let Some(native) = function.native
        && function.bound.is_none()
        && !function.is_class_constructor
    {
        crate::diagnostics::count!(native_calls);
        return crate::native::call_native_function(
            function, native, this_value, arguments, false, &mut env,
        );
    }
    crate::function::call_function(callee, this_value, arguments.to_vec(), &mut env, false)
}
