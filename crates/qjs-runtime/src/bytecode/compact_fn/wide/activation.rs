//! The wide tier's VM-free activation and frame driver.
//!
//! This is the numeric tier's driver (`compact_fn::activation`) with three
//! additions: every frame owns its call inputs (Context is lazy), a resolved call takes
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
use crate::function::{BindingCell, Function};
use crate::{RuntimeError, Value};

use crate::bytecode::frame_bank::{CONSTRUCT, FrameStorage, PREFIX, RootActivation, clear_written};
mod exits;

#[cfg(test)]
#[path = "activation/owner_tests.rs"]
mod owner_tests;

use exits::{EXIT_ARGC, ExitOutcome, RootExit, exit_to_interpreter};

/// Everything an admitted body needs to run.
struct WideActivation<'a, 's> {
    bytecode: &'a Bytecode,
    /// Services and this callee's actual typed activation, borrowed per segment.
    env: &'a mut crate::context::Context<'s>,
    upvalue_owner: Option<&'a Function>,
    upvalue_slots: u128,
    strict: bool,
}

impl WideActivation<'_, '_> {
    fn upvalue_cell(&self, slot: usize) -> Option<&BindingCell> {
        let owner = self.upvalue_owner?;
        let bit = (slot < u128::BITS as usize).then(|| 1_u128 << slot)?;
        (self.upvalue_slots & bit != 0).then_some(())?;
        let index = self.bytecode.cell_received_upvalue_index(slot)?;
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
    fn compare(&mut self, left: &Value, op: BinaryOp, right: &Value) -> Result<bool, RuntimeError> {
        if matches!(op, BinaryOp::StrictEq | BinaryOp::StrictNe)
            && let Some(equal) = crate::bytecode::vm_ops::fast_strict_eq(left, right)
        {
            return Ok(equal == (op == BinaryOp::StrictEq));
        }
        // Loose equality between two strings is string equality -- the
        // `typeof a != typeof b` of every hand-written `equals` -- with no
        // operand clone or general operator dispatch.
        if matches!(op, BinaryOp::Eq | BinaryOp::Ne)
            && let (Value::String(left), Value::String(right)) = (left, right)
        {
            return Ok(crate::string::js_string_eq(left, right) == (op == BinaryOp::Eq));
        }
        // The rest that runs no user code -- `node.left == null` -- without
        // cloning the operands into the general path.
        if let Some(Value::Boolean(result)) =
            crate::operations::eval_binary_without_env(left, op, right)
        {
            return Ok(result);
        }
        if matches!(op, BinaryOp::Eq | BinaryOp::Ne)
            && let Some(equal) = loose_equality_without_conversion(left, right)
        {
            return Ok(equal == (op == BinaryOp::Eq));
        }
        let value = self.eval_binary(left.clone(), op, right.clone())?;
        Ok(crate::is_truthy(&value))
    }

    #[inline(never)]
    fn eval_binary(
        &mut self,
        left: Value,
        op: BinaryOp,
        right: Value,
    ) -> Result<Value, RuntimeError> {
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
        let env = &mut *self.env;
        crate::operations::eval_binary(left, op, right, env)
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
fn to_property_key(
    register: &mut Value,
    env: &mut crate::context::Context<'_>,
) -> Result<(), RuntimeError> {
    let value = std::mem::replace(register, Value::Undefined);
    let env = &mut *env;
    let key = match crate::property::try_to_property_key_without_coercion(value) {
        Ok(key) => key,
        Err(value) => crate::to_property_key_value(value, env)?,
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
    let upvalue_slots = bytecode.cell_received_upvalue_slots().unwrap_or(0);
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

/// `Function::wide_inline_facts`: computed, and eligible to run on this
/// driver as far as the function's fixed facts decide; then the facts it is
/// entered with, the register count in the high half.
const FACTS_KNOWN: u64 = 1;
const FACTS_ELIGIBLE: u64 = 1 << 1;
const FACTS_REQUIRES_THIS: u64 = 1 << 2;
const FACTS_STRICT: u64 = 1 << 3;
const FACTS_LEXICAL_SLOTS: u64 = 1 << 4;
/// A base class constructor: entered only by `new`, after its fields.
const FACTS_CLASS: u64 = 1 << 5;
/// A body the compact tier lowered to an `f64` plan (`numeric_plan`).
const FACTS_NUMERIC_PLAN: u64 = 1 << 6;
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
    // A base class constructor with plain public fields is entered like
    // an ordinary constructor once its fields are installed; a direct call
    // of it must still throw, which `FACTS_CLASS` keeps off this driver.
    let class = function.is_class_constructor;
    let shape_admitted = if class {
        crate::function::is_direct_construct_class(function)
    } else {
        crate::function::is_direct_leaf_shape(callee)
    };
    if !shape_admitted
        || inherits_lexical_this
        || function.has_dynamic_function_realm
        || function.has_module_imports()
    {
        return FACTS_KNOWN;
    }
    // `admit`'s checks that do not change once the function exists; its
    // activation count is the caller's to check.
    let Some(program) = super::program_for(bytecode) else {
        return FACTS_KNOWN;
    };
    let upvalue_slots = bytecode.cell_received_upvalue_slots().unwrap_or(0);
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
    if class {
        facts |= FACTS_CLASS;
    }
    if !class
        && !program.requires_this
        && super::super::numeric_plan::plan_for(bytecode).is_some_and(|plan| {
            // A plan that calls out resolves its callees through cells the
            // compact tier's admission vouches for; one that does not reads
            // nothing but its arguments.
            !plan.calls_out() || super::super::activation::admits_numeric_callee(callee, function)
        })
    {
        facts |= FACTS_NUMERIC_PLAN;
    }
    facts
}

const MAX_FRAMES: usize = 200_000;

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

/// An operand of an operation that consumes it: a stack register's value is
/// moved out, but a local's -- a forwarded operand (`compile`'s `aliases`)
/// -- is copied, unless the operation writes its result back there.
#[inline(always)]
fn take_operand(window: &mut [Value], register: u16, dst: u16, locals: u16) -> Value {
    if register == dst || register >= locals {
        std::mem::replace(&mut window[register as usize], Value::Undefined)
    } else {
        crate::bytecode::vm_bindings::clone_local_value(&window[register as usize])
    }
}

/// Runs `callee`'s numeric plan on `arguments` when every one is a number;
/// `None` leaves the call to this driver.
#[inline(never)]
fn try_numeric_call(callee: &Value, arguments: &[Value]) -> Option<Value> {
    let Value::Function(function) = callee else {
        return None;
    };
    let bytecode = function.bytecode.as_ref()?;
    let plan = super::super::numeric_plan::plan_for(bytecode)?;
    let mut numbers = [0.0; super::super::compile::MAX_CALL_ARITY];
    if arguments.len() > numbers.len() {
        return None;
    }
    for (number, argument) in numbers.iter_mut().zip(arguments) {
        let Value::Number(value) = argument else {
            return None;
        };
        *number = *value;
    }
    super::super::numeric_plan::run(
        plan,
        bytecode,
        &function.upvalues,
        &numbers[..arguments.len()],
    )
}

/// What the driver needs to enter an admitted callee in its own window. The
/// callee's activation is built only once a numeric plan has not answered.
struct InlineCallee {
    register_count: usize,
    requires_this: bool,
    /// Whether the callee's window starts with dead-zone markers to seed.
    has_lexical_slots: bool,
    /// Whether the callee has a numeric plan to try first.
    numeric: bool,
}

/// Proves a callee may run on this driver, in a window of its register stack.
fn inline_callee(callee: &Value) -> Option<InlineCallee> {
    let (inline, class) = inline_entry(callee)?;
    (!class).then_some(inline)
}

/// `inline_callee` for any admitted body, with whether it is a class
/// constructor, which only `new` may enter. Inlined into the driver's call
/// and construct sites, which it runs on every call.
#[inline(always)]
fn inline_entry(callee: &Value) -> Option<(InlineCallee, bool)> {
    let Value::Function(function) = callee else {
        return None;
    };
    let bytecode = function.bytecode.as_ref()?;
    let facts = fixed_inline_facts(callee, function, bytecode);
    if facts & FACTS_ELIGIBLE == 0
        || function.has_dynamic_function_realm_override.get()
        || !crate::function::lexical_scope_admits_entry(function)
    {
        return None;
    }
    if !super::program_for(bytecode)?.admit_activation() {
        return None;
    }
    Some((
        InlineCallee {
            register_count: (facts >> FACTS_REGISTER_SHIFT) as usize,
            requires_this: facts & FACTS_REQUIRES_THIS != 0,
            has_lexical_slots: facts & FACTS_LEXICAL_SLOTS != 0,
            numeric: facts & FACTS_NUMERIC_PLAN != 0,
        },
        facts & FACTS_CLASS != 0,
    ))
}

/// Proves `new callee(...)` may run on this driver: an ordinary (not class,
/// bound or native) constructor the driver would inline as a call, whose
/// `prototype` is an ordinary object. Returns the fresh receiver.
#[inline(never)]
fn inline_constructor(
    callee: &Value,
    _env: &mut crate::context::Context<'_>,
) -> Option<(InlineCallee, crate::ObjectRef, bool)> {
    let Value::Function(function) = callee else {
        return None;
    };
    if function.native.is_some() || function.bound.is_some() || !function.constructable {
        return None;
    }
    let (inline, class) = inline_entry(callee)?;
    let receiver = function.wide_instance_receiver(function.creation_context().realm.atoms())?;
    Some((inline, receiver, class))
}

/// Admits `new callee(...)` as a frame of this driver (`inline_constructor`),
/// running a base class's instance field initializers in the caller's
/// context; `Ok(None)` hands the callee back for the general construct path.
/// Out of line, so the driver loop's own code is the same whether or not a
/// body constructs.
#[inline(never)]
fn admit_constructor(
    callee: &Value,
    env: &mut crate::context::Context<'_>,
    depth: usize,
) -> Result<Option<(InlineCallee, crate::ObjectRef)>, RuntimeError> {
    let Some((inline, receiver, class)) = inline_constructor(callee, env) else {
        return Ok(None);
    };
    if depth >= MAX_FRAMES {
        return Err(call_stack_exhausted());
    }
    // A base class installs its instance fields before its body runs.
    if class && let Value::Function(function) = callee {
        crate::function::initialize_direct_instance_fields(
            &function.instance_elements(),
            &receiver,
            env,
        )?;
    }
    Ok(Some((inline, receiver)))
}

/// A constructor's result: the returned value when it is an object, else
/// the receiver, as the general construct path decides.
fn constructed(value: Value, env: &crate::context::Context<'_>) -> Value {
    match value {
        Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Object(_)
        | Value::Proxy(_) => value,
        other => {
            release(other);
            env.activation.receiver_seed().unwrap_or(Value::Undefined)
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

#[allow(clippy::too_many_arguments)]
fn run(
    bytecode: &Bytecode,
    activation: &mut crate::context::ActivationContext,
    runtime: &mut crate::context::RuntimeServices,
    entry: WideEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
    root: RootExit<'_>,
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
        root,
    )
}
#[allow(clippy::too_many_arguments)]
fn run_root(
    bytecode: &Bytecode,
    mut activation: RootActivation<'_>,
    receiver: Value,
    target: Option<Value>,
    runtime: &mut crate::context::RuntimeServices,
    entry: WideEntry<'_>,
    parameter_slots: &[usize],
    arguments: &[Value],
    root: RootExit<'_>,
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
    seed_lexical_markers(
        entry.program,
        &mut storage.registers[PREFIX..PREFIX + entry.program.register_count],
    );
    let result = run_frames(
        bytecode,
        runtime,
        &mut activation,
        entry.upvalue_owner.as_ref(),
        entry.upvalue_slots,
        root,
        &mut storage,
    );
    runtime.frame_stacks.put(storage);
    result
}
enum Boundary {
    Return(Value),
    Call {
        callee: Value,
        receiver: Value,
        target: Option<Value>,
        inline: InlineCallee,
        arguments: usize,
        argc: usize,
        dst: u16,
        resume_pc: usize,
        flags: u32,
    },
}
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn run_frames(
    root_bytecode: &Bytecode,
    runtime: &mut crate::context::RuntimeServices,
    root_activation: &mut RootActivation<'_>,
    root_owner: Option<&Function>,
    root_slots: u128,
    root: RootExit<'_>,
    storage: &mut FrameStorage,
) -> Result<Value, RuntimeError> {
    loop {
        let depth = storage.frames.len();
        let boundary = {
            let header = storage.frames.last_mut().expect("current indexed header");
            let current_base = header.service.base as usize;
            let current_len = header.width as usize;
            let constructs = header.flags & CONSTRUCT != 0;
            let mut pc = header.pc as usize;
            let (previous, registers) = storage.registers.split_at_mut(current_base);
            let prefix = &previous[current_base - PREFIX..current_base];
            let current_callee = &prefix[0];
            let bytecode = running_bytecode(current_callee, root_bytecode);
            let (upvalue_owner, upvalue_slots) = match current_callee {
                Value::Function(function) => (
                    Some(function),
                    bytecode.cell_received_upvalue_slots().unwrap_or(0),
                ),
                _ => (root_owner, root_slots),
            };
            let Some(program) = super::program_for(bytecode) else {
                return Err(missing_program());
            };
            let mut env = root_activation.view(runtime, header, prefix, depth == 1);
            let strict = env.caller_strict();
            let mut activation = WideActivation {
                bytecode,
                env: &mut env,
                upvalue_owner,
                upvalue_slots,
                strict,
            };
            let window = &mut registers[..current_len];
            let ops = &program.ops[..];
            loop {
                let outcome = loop {
                    let Some(op) = ops.get(pc) else {
                        break Ok(Action::Return(Value::Undefined));
                    };
                    pc += 1;
                    #[cfg(feature = "perf-counters")]
                    crate::diagnostics::update(|c| c.compact_function_ops += 1);
                    match *op {
                        WideOp::LoadConst { dst, index } => {
                            let Some(value) = activation.bytecode.constants.get(index as usize)
                            else {
                                break Err(execute::constant_out_of_bounds());
                            };
                            execute::store(
                                &mut window[dst as usize],
                                crate::bytecode::vm_bindings::clone_local_value(value),
                            );
                        }
                        WideOp::Move { dst, src } | WideOp::Dup { src, dst } => {
                            let value = crate::bytecode::vm_bindings::clone_local_value(
                                &window[src as usize],
                            );
                            execute::store(&mut window[dst as usize], value);
                        }
                        WideOp::LoadUpvalueLocal { dst, slot } => {
                            let Some(cell) = activation.upvalue_cell(slot as usize) else {
                                break Err(execute::uninitialized_local());
                            };
                            // One state borrow; a dead-zone cell takes the
                            // interpreter's diagnostic.
                            let value = match cell.value_if_initialized() {
                                Some(value) => value,
                                None => match activation.uninitialized_upvalue(slot as usize) {
                                    Ok(value) => value,
                                    Err(error) => break Err(error),
                                },
                            };
                            execute::store(&mut window[dst as usize], value);
                        }
                        WideOp::StoreUpvalueLocal { slot, src } => {
                            let Some(cell) = activation.upvalue_cell(slot as usize) else {
                                break Err(execute::uninitialized_local());
                            };
                            let value =
                                std::mem::replace(&mut window[src as usize], Value::Undefined);
                            if let Err(error) = cell.assign(value, activation.strict) {
                                break Err(error);
                            }
                        }
                        WideOp::LoadThis { dst } => {
                            // Admission required a seeded receiver, so this never
                            // fabricates one.
                            let Some(value) = activation.env.activation.initialized_receiver()
                            else {
                                break Err(execute::uninitialized_local());
                            };
                            execute::store(&mut window[dst as usize], value.clone());
                        }
                        WideOp::GetPropThis { dst, index } => {
                            let (Some(site), Some(receiver)) = (
                                program.named_reads.get(index as usize),
                                activation.env.activation.initialized_receiver(),
                            ) else {
                                break Err(execute::uninitialized_local());
                            };
                            if let Some(value) =
                                property::get_prop_named_hot(receiver, &site.key, &site.cache)
                            {
                                execute::store(&mut window[dst as usize], value);
                                continue;
                            }
                            let receiver = receiver.clone();
                            match property::get_prop_named(
                                &receiver,
                                &site.key,
                                &site.cache,
                                activation.env,
                            ) {
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
                            if let Some(value) = property::get_prop_named_hot(
                                &window[obj as usize],
                                &site.key,
                                &site.cache,
                            ) {
                                execute::store(&mut window[dst as usize], value);
                                continue;
                            }
                            match property::get_prop_named(
                                &window[obj as usize],
                                &site.key,
                                &site.cache,
                                activation.env,
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
                            if property::set_prop_named_hot(
                                &window[obj as usize],
                                site.cache.as_ref(),
                                &assigned,
                                activation.env,
                            ) {
                                execute::store(&mut window[obj as usize], assigned);
                                continue;
                            }
                            match property::set_prop_named(
                                &window[obj as usize],
                                &site.key,
                                site.cache.as_ref(),
                                site.is_strict,
                                assigned,
                                activation.env,
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
                                activation.env.activation.initialized_receiver(),
                            ) else {
                                break Err(execute::uninitialized_local());
                            };
                            let assigned =
                                std::mem::replace(&mut window[value as usize], Value::Undefined);
                            if property::set_prop_named_hot(
                                receiver,
                                site.cache.as_ref(),
                                &assigned,
                                activation.env,
                            ) {
                                execute::store(&mut window[dst as usize], assigned);
                                continue;
                            }
                            let receiver = receiver.clone();
                            match property::set_prop_named(
                                &receiver,
                                &site.key,
                                site.cache.as_ref(),
                                site.is_strict,
                                assigned,
                                activation.env,
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
                            let locals = program.local_registers;
                            let left = take_operand(window, left, dst, locals);
                            let right = take_operand(window, right, dst, locals);
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
                                && let Err(error) =
                                    to_property_key(&mut window[dst as usize], activation.env)
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
                            let value =
                                std::mem::replace(&mut window[src as usize], Value::Undefined);
                            match eval_unary(op, value, activation.env) {
                                Ok(value) => execute::store(&mut window[dst as usize], value),
                                Err(error) => break Err(error),
                            }
                        }
                        WideOp::Typeof { dst, src } => {
                            let value =
                                std::mem::replace(&mut window[src as usize], Value::Undefined);
                            let name = crate::bytecode::util::typeof_value(value);
                            execute::store(&mut window[dst as usize], Value::String(name));
                        }
                        WideOp::ToNumeric { dst } => {
                            if !matches!(window[dst as usize], Value::Number(_)) {
                                let value =
                                    std::mem::replace(&mut window[dst as usize], Value::Undefined);
                                match to_numeric(value, activation.env) {
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
                                match update(op, value, activation.env) {
                                    Ok(value) => execute::store(&mut window[dst as usize], value),
                                    Err(error) => break Err(error),
                                }
                            }
                        }
                        WideOp::GetProp { dst, obj, key } => {
                            // `v[i]` on a dense array: answered in place, the
                            // number key left in its register.
                            if let (Value::Array(elements), Value::Number(number)) =
                                (&window[obj as usize], &window[key as usize])
                                && let Some(index) =
                                    crate::bytecode::vm_props::array_index_from_number(*number)
                                && let Some(value) = elements.plain_dense_index_value(index)
                            {
                                execute::store(&mut window[dst as usize], value);
                                continue;
                            }
                            if let Some(value) = property::get_prop_string_own(
                                &window[obj as usize],
                                &window[key as usize],
                            ) {
                                execute::store(&mut window[dst as usize], value);
                                continue;
                            }
                            let key = take_operand(window, key, dst, program.local_registers);
                            // A forwarded local receiver is read, not consumed.
                            let result = if obj != dst && obj < program.local_registers {
                                property::get_prop_element_of(
                                    &window[obj as usize],
                                    key,
                                    activation.env,
                                )
                            } else {
                                property::get_prop_element(
                                    &mut window[obj as usize],
                                    key,
                                    activation.env,
                                )
                            };
                            match result {
                                Ok(value) => execute::store(&mut window[dst as usize], value),
                                Err(error) => break Err(error),
                            }
                        }
                        WideOp::GetPropIndex { dst, obj, index } => {
                            if let Value::Array(elements) = &window[obj as usize]
                                && let Some(value) =
                                    elements.plain_dense_index_value(usize::from(index))
                            {
                                execute::store(&mut window[dst as usize], value);
                                continue;
                            }
                            match property::get_prop_index(
                                &window[obj as usize],
                                index,
                                activation.env,
                            ) {
                                Ok(value) => execute::store(&mut window[dst as usize], value),
                                Err(error) => break Err(error),
                            }
                        }
                        WideOp::LoadGlobal { dst, index } => {
                            let Some(name) = program.global_names.get(index as usize) else {
                                break Err(execute::constant_out_of_bounds());
                            };
                            let site = &program.global_reads[index as usize];
                            match property::load_global_cached(name, site, activation.env) {
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
                                Value::Array(crate::ArrayRef::new(activation.env.realm(), values)),
                            );
                        }
                        WideOp::ClearLocal { slot } => {
                            execute::store(&mut window[slot as usize], program.tdz_marker.clone());
                        }
                        WideOp::MoveChecked { dst, src } => {
                            if window[src as usize].is_uninitialized_lexical_marker() {
                                break Err(activation.uninitialized_lexical(src as usize));
                            }
                            let value = crate::bytecode::vm_bindings::clone_local_value(
                                &window[src as usize],
                            );
                            execute::store(&mut window[dst as usize], value);
                        }
                        WideOp::AssignChecked { dst, src } => {
                            if window[dst as usize].is_uninitialized_lexical_marker() {
                                break Err(activation.uninitialized_lexical(dst as usize));
                            }
                            let value = crate::bytecode::vm_bindings::clone_local_value(
                                &window[src as usize],
                            );
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
                        WideOp::ReferenceCheck { event } => {
                            let references = &activation.bytecode.binding_plan.references;
                            let statically_elided = references.skips_indexed_entry()
                                && activation.env.activation.scope_cursor().is_none();
                            if !statically_elided
                                && !references.event_starts_elidable(
                                    event as usize,
                                    &activation.env.activation,
                                )
                                && let Err(error) =
                                    references.select_checked_starts(event as usize, activation.env)
                            {
                                break Err(error);
                            }
                        }
                        WideOp::Exit { ip, depth } => {
                            if program.probed_header(pc).is_none()
                                && let Some(index) = program.probed_backedge(ip)
                                && program.backedge_is_native(index)
                            {
                                pc = program.backedge_jump_pc(index);
                                continue;
                            }
                            break Ok(Action::Call {
                                dst: depth,
                                base: 0,
                                argc: EXIT_ARGC,
                                resume_pc: ip as usize,
                            });
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
                            current_callee,
                            root_bytecode,
                            root,
                            ip as u32,
                            depth,
                            pc,
                            window,
                            activation.env,
                        ) {
                            ExitOutcome::Continue { pc: next } => {
                                pc = next;
                                continue;
                            }
                            ExitOutcome::Finished(Ok(value)) => Action::Return(value),
                            ExitOutcome::Finished(Err(error)) => {
                                return Err(crate::error::capture_error_origin(
                                    error,
                                    activation.env,
                                ));
                            }
                        }
                    }
                    Ok(action) => action,
                    Err(error) => {
                        return Err(crate::error::capture_error_origin(error, activation.env));
                    }
                };
                match action {
                    Action::Return(value) => {
                        let value = if constructs {
                            constructed(value, activation.env)
                        } else {
                            value
                        };
                        clear_written(window, &program.written, storage.poison);
                        break Boundary::Return(value);
                    }
                    Action::New {
                        dst,
                        base,
                        argc,
                        resume_pc,
                    } => {
                        let index = base as usize;
                        let callee = std::mem::replace(&mut window[index], Value::Undefined);
                        match admit_constructor(&callee, activation.env, depth) {
                            Ok(Some((inline, receiver))) => {
                                let target = Some(callee.clone());
                                break Boundary::Call {
                                    callee,
                                    receiver: Value::Object(receiver),
                                    target,
                                    inline,
                                    arguments: current_base + index + 1,
                                    argc: argc as usize,
                                    dst,
                                    resume_pc,
                                    flags: CONSTRUCT,
                                };
                            }
                            Ok(None) => {}
                            Err(error) => {
                                return Err(crate::error::capture_error_origin(
                                    error,
                                    activation.env,
                                ));
                            }
                        }
                        let args = &window[index + 1..index + 1 + argc as usize];
                        match construct_from_activation(activation.env, callee, args) {
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
                    action => {
                        let (dst, base, argc, resume_pc, resolved) = match action {
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
                            _ => unreachable!("handled completion"),
                        };
                        let index = base as usize;
                        let callee = std::mem::replace(&mut window[index], Value::Undefined);
                        let receiver = if resolved {
                            Some(std::mem::replace(&mut window[index - 1], Value::Undefined))
                        } else {
                            None
                        };
                        let args = &window[index + 1..index + 1 + argc as usize];
                        if let Some(inline) = inline_callee(&callee) {
                            if inline.numeric
                                && let Some(value) = try_numeric_call(&callee, args)
                            {
                                crate::diagnostics::count!(ordinary_call_attempts);
                                if let Some(value) = receiver {
                                    release(value);
                                }
                                release(callee);
                                execute::store(&mut window[dst as usize], value);
                                pc = resume_pc;
                                continue;
                            }
                            break Boundary::Call {
                                callee,
                                receiver: receiver.unwrap_or(Value::Undefined),
                                target: None,
                                inline,
                                arguments: current_base + index + 1,
                                argc: argc as usize,
                                dst,
                                resume_pc,
                                flags: 0,
                            };
                        }
                        match call_from_activation(
                            &mut activation,
                            callee,
                            receiver.unwrap_or(Value::Undefined),
                            args,
                        ) {
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
                receiver,
                target,
                inline,
                arguments,
                argc,
                dst,
                resume_pc,
                flags,
            } => {
                let receiver = if flags & CONSTRUCT != 0 {
                    receiver
                } else {
                    let Value::Function(function) = &callee else {
                        unreachable!("admitted callee");
                    };
                    crate::bytecode::frame_bank::receiver(
                        function,
                        Some(receiver),
                        inline.requires_this,
                        runtime,
                    )
                };
                storage.frames.last_mut().expect("caller header").pc = resume_pc as u32;
                storage.push(
                    Value::Undefined,
                    receiver,
                    target,
                    inline.register_count,
                    argc,
                    dst,
                    flags,
                )?;
                storage.move_arguments(&callee, arguments, argc);
                if inline.has_lexical_slots {
                    let bytecode = running_bytecode(&callee, root_bytecode);
                    let program = super::program_for(bytecode).expect("admitted lexical body");
                    let base = storage.frames.last().expect("callee header").service.base as usize;
                    seed_lexical_markers(
                        program,
                        &mut storage.registers[base..base + inline.register_count],
                    );
                }
                storage.install_callee(callee);
                crate::diagnostics::count!(ordinary_call_attempts);
                crate::diagnostics::count!(compact_direct_calls);
            }
        }
    }
}

/// `left == right` where no conversion can run: `null` or `undefined`
/// against anything -- nothing else equals them but the IsHTMLDDA host
/// object -- and two numbers. Kept out of the shared operator helpers:
/// growing those changes this tier's dispatch codegen.
#[inline(never)]
fn loose_equality_without_conversion(left: &Value, right: &Value) -> Option<bool> {
    match (left, right) {
        (Value::Null | Value::Undefined, other) | (other, Value::Null | Value::Undefined) => Some(
            matches!(other, Value::Null | Value::Undefined) || crate::html_dda::is_html_dda(other),
        ),
        (Value::Number(left), Value::Number(right)) => Some(left == right),
        _ => None,
    }
}

/// A unary operator, with the interpreter's own fast paths: `!` and `void`
/// coerce nothing, a number answers inline, everything else runs the general
/// operator in an empty realm frame like the tier's binary path.
#[inline(never)]
fn eval_unary(
    op: UnaryOp,
    value: Value,
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    match op {
        UnaryOp::Not => return Ok(Value::Boolean(!crate::is_truthy(&value))),
        UnaryOp::Void => return Ok(Value::Undefined),
        _ => {}
    }
    if let Some(value) = crate::bytecode::vm_props::fast_number_unary(op, &value) {
        return Ok(value);
    }
    let env = &mut *env;
    crate::operations::eval_unary(op, value, env)
}

/// `ToNumeric` on a non-number, mirroring `Vm::eval_to_numeric`.
#[cold]
#[inline(never)]
fn to_numeric(value: Value, env: &mut crate::context::Context<'_>) -> Result<Value, RuntimeError> {
    if matches!(value, Value::BigInt(_)) {
        return Ok(value);
    }
    let env = &mut *env;
    let primitive = crate::to_primitive_with_hint(value, crate::PreferredType::Number, env)?;
    Ok(match primitive {
        Value::BigInt(_) => primitive,
        value => Value::Number(crate::to_number_with_env(value, env)?),
    })
}

/// `++`/`--` on a non-number, mirroring `Vm::eval_update`.
#[cold]
#[inline(never)]
fn update(
    op: UpdateOp,
    value: Value,
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    let one = num_bigint::BigInt::from(1);
    let step = |value: num_bigint::BigInt| match op {
        UpdateOp::Increment => Value::bigint(value + one.clone()),
        UpdateOp::Decrement => Value::bigint(value - one.clone()),
    };
    let value = match value {
        Value::BigInt(value) => return Ok(step(std::rc::Rc::unwrap_or_clone(value))),
        value => value,
    };
    let env = &mut *env;
    let primitive = crate::to_primitive_with_hint(value, crate::PreferredType::Number, env)?;
    Ok(match primitive {
        Value::BigInt(value) => step(std::rc::Rc::unwrap_or_clone(value)),
        value => {
            let number = crate::to_number_with_env(value, env)?;
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
    env: &mut crate::context::Context<'_>,
    callee: Value,
    arguments: &[Value],
) -> Result<Value, RuntimeError> {
    if let Some(result) =
        crate::function::try_construct_direct_leaf_function(&callee, arguments, env)
    {
        return result;
    }
    if let Some(array) = construct_plain_array(env, &callee, arguments) {
        return Ok(array);
    }
    if let Value::Function(function) = &callee
        && function.native.is_some()
        && let Some(result) =
            crate::function::construct_intrinsic_directly(function, arguments, env)
    {
        return result;
    }
    let env = &mut *env;
    crate::function::construct_function_slice(callee.clone(), callee, arguments, env)
}

/// `new Array(...)` on the realm's own Array constructor, whose `prototype`
/// is non-writable and non-configurable: the array the constructor builds,
/// without first allocating the ordinary receiver the general construct
/// path makes for it. A length that is not a valid array length, or any
/// other constructor, takes the general path and its errors.
fn construct_plain_array(
    env: &mut crate::context::Context<'_>,
    callee: &Value,
    arguments: &[Value],
) -> Option<Value> {
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
            crate::ArrayRef::new_with_length(env.realm(), *length as usize)
        }
        values => crate::ArrayRef::new(env.realm(), values.to_vec()),
    };
    Some(Value::Array(array))
}

/// Terminal error cleanup only: every caller immediately returns Err from
/// run_frames. The fresh call activation is then discarded, even if it now
/// describes an inline child. Never resume or expose it as the parked caller;
/// restoring caller frames is the normal-return protocol, not this cleanup.
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
/// `None` having consumed nothing. Guards precede receiver consumption and
/// observable work; the actual prepared callee is borrowed in place.
#[inline(always)]
pub(in crate::bytecode) fn try_run_standalone(
    bytecode: &Bytecode,
    env: &mut crate::context::ActivationContext,
    runtime: &mut crate::context::RuntimeServices,
    slots: &mut DirectCallSlots<'_>,
) -> Option<Result<Value, RuntimeError>> {
    if !env.indexed_bindings_only() {
        crate::diagnostics::count!(indexed_binding_admission_rejections);
        super::super::activation::trace_reference_decline(
            bytecode,
            "dynamic scope or selected global",
        );
        return None;
    }
    // Checked global starts run as compiled checkpoints (`ReferenceCheck`);
    // only sites that need the dynamic resolver keep the interpreter.
    if bytecode
        .binding_plan
        .references
        .needs_dynamic_resolution(env)
    {
        super::super::activation::trace_reference_decline(bytecode, "dynamic reference sites");
        return None;
    }
    let entry = admit(bytecode, slots.upvalues)?;
    // An admitted receiver read borrows the same owner the VM will resume.
    if entry.program.requires_this && env.initialized_receiver().is_none() {
        return None;
    }
    let call_env = env;
    let call_slots = slots;
    let root = RootExit {
        upvalues: call_slots.upvalues,
        realm_upvalue_slots: call_slots.realm_upvalue_slots,
    };
    Some(run(
        bytecode,
        call_env,
        runtime,
        entry,
        call_slots.parameter_slots,
        call_slots.arguments,
        root,
    ))
}

/// Runs one call the driver could not inline, through the same entries the
/// interpreter's general call path reaches.
#[inline(never)]
fn call_from_activation(
    activation: &mut WideActivation<'_, '_>,
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
            &mut *activation.env,
        )
    {
        return result;
    }
    let env = &mut *activation.env;
    // A plain native takes its arguments where they are; only a bytecode
    // callee can also read a slice; only semantic forwarding builds a list.
    if let Value::Function(function) = &callee
        && let Some(entry) = function.native_entry
        && function.bound.is_none()
        && !function.is_class_constructor
    {
        crate::diagnostics::count!(native_calls);
        return crate::native::call_native_function(
            entry,
            crate::native::CallInfo {
                callee: function,
                this_value: &this_value,
                is_construct: false,
            },
            arguments,
            None,
            None,
            env,
        );
    }
    crate::function::call_function_slice(&callee, this_value, arguments, env)
}

/// What running a loop's typed program from an exit did.
pub(super) enum LoopHere {
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
pub(super) fn run_typed_loop_here(
    bytecode: &Bytecode,
    program: &WideProgram,
    env: &mut crate::context::Context<'_>,
    window: &mut [Value],
    upvalues: &[BindingCell],
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
    let consulted_first = plans
        .shared_numeric_mutation
        .iter()
        .any(|plan| overlaps(plan.region()));
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
        bytecode.cell_received_upvalue_slots().unwrap_or(0),
        stack.get_mut(base_depth..).unwrap_or_default(),
    );
    if !crate::bytecode::typed_loop::try_run_typed_loop(&mut frame, plans, header, backedge) {
        return LoopHere::Declined;
    }
    // A declined attempt hands the edge to the interpreter, which counts it.
    crate::diagnostics::count!(loop_backedges);
    crate::diagnostics::count!(loop_plan_entries);
    let (resume, deoptimized) = (frame.resume_ip, frame.deoptimized);
    if let Some(iterations) = frame.iterations
        && let Some(index) = u32::try_from(backedge)
            .ok()
            .and_then(|backedge| program.probed_backedge(backedge))
    {
        program.record_typed_entry(index, iterations);
    }
    let declined_typed_loop_programs = frame.declined_typed_loop_programs();
    let (pushed, overflowed) = (frame.pushed, frame.stack_overflowed);
    let Some(resume) = resume else {
        return LoopHere::Declined;
    };
    // The program rebuilt the stack above the loop's own base, in place;
    // what lay below it when the loop began is still in its registers.
    let depth = base_depth + pushed;
    if overflowed {
        return LoopHere::Declined;
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

#[inline(always)]
pub(in crate::bytecode) fn try_run_indexed(
    function: &Function,
    receiver: &mut Option<Value>,
    target: &mut Option<Value>,
    arguments: &[Value],
    runtime: &mut crate::context::RuntimeServices,
) -> Option<Result<Value, RuntimeError>> {
    let bytecode = function.bytecode.as_ref()?;
    if bytecode
        .binding_plan
        .references
        .needs_dynamic_indexed_entry()
    {
        return None;
    }
    let upvalues = crate::bytecode::DirectCallUpvalues::Function(function);
    let entry = admit(bytecode, upvalues)?;
    if entry.program.requires_this && function.lexical_this {
        let owner = function.lexical_call()?;
        if owner.this.read().is_err() {
            return None;
        }
    }
    let receiver = crate::bytecode::frame_bank::receiver(
        function,
        receiver.take(),
        entry.program.requires_this,
        runtime,
    );
    Some(run_root(
        bytecode,
        RootActivation::Indexed { function },
        receiver,
        target.take(),
        runtime,
        entry,
        bytecode.parameter_slots(),
        arguments,
        RootExit {
            upvalues,
            realm_upvalue_slots: function.realm_upvalue_slots,
        },
    ))
}
