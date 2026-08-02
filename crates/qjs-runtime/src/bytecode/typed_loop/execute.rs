//! Running a compiled loop region, and leaving it again.
//!
//! Entry seeds the register files from the frame's slots and the region's
//! globals, declining when any of them holds something the files cannot
//! represent. Leaving happens in one of three ways: the loop's own exit test,
//! the residency bound, or a guard failure — and all three rebuild the operand
//! stack the interpreter expects from the [`DeoptSite`] of the operation that
//! stopped, so a region with side effects is safe to abandon halfway.

use std::rc::Rc;

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use super::super::vm::Vm;
use super::{
    Class, DeoptSite, MAX_NATIVE_ITERATIONS, Typed, TypedLoopProgram, TypedLoopScratch, TypedOp,
};
use crate::Value;

/// Runs the program covering the backedge at `ip`, if one exists and this
/// frame admits it. Returns whether the loop was executed natively.
pub(crate) fn try_run_typed_loop(
    vm: &mut Vm<'_>,
    plans: crate::bytecode::vm_loop_dispatch::LoopPlanView<'_>,
    header: usize,
    backedge: usize,
) -> bool {
    if vm.direct_eval_with_stack {
        return false;
    }
    let programs = plans.typed;
    if programs.is_empty() {
        return false;
    }
    let Some(index) = programs
        .iter()
        .position(|program| program.header() == header && program.backedge() == backedge)
    else {
        return false;
    };
    // A frame that has already declined this region does not re-examine it.
    let declined_bit = (index < u128::BITS as usize).then(|| 1_u128 << index);
    if declined_bit.is_some_and(|bit| vm.declined_typed_loop_programs & bit != 0) {
        return false;
    }
    let decline = |vm: &mut Vm<'_>| {
        if let Some(bit) = declined_bit {
            vm.declined_typed_loop_programs |= bit;
        }
        false
    };
    // A loop another tier already recognizes stays with that tier: those plans
    // own their own deoptimization and replay protocol, and running the region
    // twice through two accelerators is not equivalent to running it once.
    if plans
        .numeric
        .iter()
        .any(|plan| plan.contains_instruction(backedge))
        || plans
            .shared_numeric_mutation
            .iter()
            .any(|plan| plan.contains_instruction(backedge))
        || plans
            .control
            .iter()
            .any(|plan| plan.contains_instruction(backedge))
    {
        return decline(vm);
    }
    // The programs live in the bytecode, whose borrow outlives the frame, so
    // copying the slice handle keeps the borrow checker happy without cloning
    // the op list.
    let program = &programs[index];
    match run(vm, program) {
        Outcome::Ran => true,
        // A region that deoptimizes once will almost certainly do so again — its
        // guards describe the data, not the moment — so the frame stops using
        // the program rather than paying the entry cost every iteration.
        Outcome::Deoptimized => {
            if let Some(bit) = declined_bit {
                vm.declined_typed_loop_programs |= bit;
            }
            true
        }
        Outcome::Declined => decline(vm),
    }
}

/// What one attempt to run a program did.
#[derive(Debug)]
enum Outcome {
    /// The loop finished natively and the frame is positioned after it.
    Ran,
    /// A guard failed; the frame is positioned at the loop header.
    Deoptimized,
    /// The frame never entered the program.
    Declined,
}

fn run(vm: &mut Vm<'_>, program: &TypedLoopProgram) -> Outcome {
    let mut scratch = program.take_scratch();
    let outcome = seed_registers(vm, program, &mut scratch)
        .map(|()| execute(vm, program, &mut scratch))
        .unwrap_or(Outcome::Declined);
    program.recycle_scratch(scratch);
    outcome
}

fn execute(vm: &mut Vm<'_>, program: &TypedLoopProgram, scratch: &mut TypedLoopScratch) -> Outcome {
    let registers = &mut scratch.registers;
    let receivers = &scratch.receivers;
    let boxed = &mut scratch.boxed;
    let sloppy_global_writes = &scratch.sloppy_global_writes;
    let caches = &mut scratch.caches;
    // Borrowed for the whole region: the program owns these across activations.
    let mut shape_caches = program.shape_caches.borrow_mut();
    // One inline cache per property access site, warmed on the first iteration.
    let mut iterations = 0_u64;
    let mut pc = 0_usize;
    macro_rules! deopt_here {
        ($op:expr) => {{
            let site = program.sites[pc - 1];
            return deopt(vm, program, registers, boxed, site);
        }};
    }
    loop {
        let Some(op) = program.ops.get(pc) else {
            // Fell off the end of the program: that is the backedge.
            pc = 0;
            iterations += 1;
            if iterations >= MAX_NATIVE_ITERATIONS {
                let site = program.sites[0];
                return deopt(vm, program, registers, boxed, site);
            }
            continue;
        };
        pc += 1;
        match *op {
            TypedOp::Move { dst, src } => registers[dst as usize] = registers[src as usize],
            TypedOp::ToNumeric { dst, src } => {
                registers[dst as usize] = registers[src as usize].to_numeric();
            }
            TypedOp::Binary {
                dst,
                op,
                left,
                right,
            } => {
                let Some(value) =
                    typed_binary(registers[left as usize], op, registers[right as usize])
                else {
                    deopt_here!(op);
                };
                registers[dst as usize] = value;
            }
            TypedOp::Unary { dst, op, src } => {
                let Some(value) = typed_unary(op, registers[src as usize]) else {
                    deopt_here!(op);
                };
                registers[dst as usize] = value;
            }
            TypedOp::Update { dst, op, src } => {
                // The unfused shape is `ToNumeric; Update`, so the coercion
                // belongs here and this operation cannot fail.
                let Some(number) = registers[src as usize].to_numeric().number() else {
                    deopt_here!(op);
                };
                registers[dst as usize] = Typed::Number(match op {
                    UpdateOp::Increment => number + 1.0,
                    UpdateOp::Decrement => number - 1.0,
                });
            }
            TypedOp::DenseRead {
                dst,
                receiver,
                index: index_register,
            } => {
                let Some(value) = dense_read(
                    &receivers[receiver as usize],
                    registers[index_register as usize],
                ) else {
                    deopt_here!(op);
                };
                registers[dst as usize] = value;
            }
            TypedOp::DenseWrite {
                receiver,
                index,
                value,
            } => {
                if !dense_write(
                    &receivers[receiver as usize],
                    registers[index as usize],
                    registers[value as usize],
                ) {
                    deopt_here!(op);
                }
            }
            TypedOp::StoreSloppyGlobal { target, value } => {
                let Some(target) = sloppy_global_writes.get(target as usize) else {
                    deopt_here!(op);
                };
                if !vm.write_typed_loop_sloppy_global(target, registers[value as usize].to_value())
                {
                    deopt_here!(op);
                }
            }
            TypedOp::MoveBoxed { dst, src } => boxed[dst as usize] = boxed[src as usize].clone(),
            TypedOp::Unbox { dst, src } => {
                let Some(value) = Typed::from_value(&boxed[src as usize]) else {
                    deopt_here!(op);
                };
                registers[dst as usize] = value;
            }
            TypedOp::Box { dst, src } => {
                boxed[dst as usize] = registers[src as usize].to_value();
            }
            TypedOp::GetNamed {
                dst,
                object,
                name,
                cache,
            } => {
                let Some(value) = get_named(
                    &boxed[object as usize],
                    &program.names[name as usize],
                    &mut caches[cache as usize],
                    &mut shape_caches[cache as usize],
                ) else {
                    deopt_here!(op);
                };
                boxed[dst as usize] = value;
            }
            TypedOp::SetNamed {
                object,
                name,
                value,
            } => {
                if !set_named(
                    &boxed[object as usize],
                    &program.names[name as usize],
                    &boxed[value as usize],
                ) {
                    deopt_here!(op);
                }
            }
            TypedOp::ElementRead {
                dst,
                receiver,
                index,
            } => {
                let Some(value) =
                    element_read(&boxed[receiver as usize], registers[index as usize])
                else {
                    deopt_here!(op);
                };
                boxed[dst as usize] = value;
            }
            TypedOp::ComputedRead { dst, receiver, key } => {
                let Some(value) = computed_read(&boxed[receiver as usize], &boxed[key as usize])
                else {
                    deopt_here!(op);
                };
                boxed[dst as usize] = value;
            }
            TypedOp::ComputedWrite {
                receiver,
                key,
                value,
            } => {
                let written = match (&boxed[receiver as usize], &boxed[key as usize]) {
                    (Value::Object(object), Value::String(name)) => {
                        let value = &boxed[value as usize];
                        match object.write_existing_own_data_property(name.as_str(), value) {
                            crate::value::OwnDataPropertyWrite::Written => true,
                            // The property does not exist yet. A dictionary
                            // loop creates each key exactly once, so refusing
                            // here would deoptimize on the first iteration and
                            // never re-enter -- the whole loop would be lost to
                            // its own first write. The creation path re-checks
                            // extensibility, exotics, and the prototype chain
                            // itself, so anything it will not do still declines.
                            _ => vm.try_create_ordinary_own_data_property(
                                object,
                                Rc::from(name.as_str()),
                                value,
                            ),
                        }
                    }
                    _ => false,
                };
                if !written {
                    deopt_here!(op);
                }
            }
            TypedOp::CallNumericNative {
                dst,
                callee,
                first,
                second,
                arity,
            } => {
                let Some(value) = call_numeric_native(
                    &boxed[callee as usize],
                    registers[first as usize],
                    registers[second as usize],
                    arity,
                ) else {
                    deopt_here!(op);
                };
                registers[dst as usize] = value;
            }
            TypedOp::CallClosedFormLeaf {
                dst,
                receiver,
                callee,
                first,
                second,
                arity,
            } => {
                let Some(value) = call_closed_form_leaf(
                    &boxed[callee as usize],
                    &boxed[receiver as usize],
                    registers[first as usize],
                    registers[second as usize],
                    arity,
                ) else {
                    deopt_here!(op);
                };
                boxed[dst as usize] = value;
            }
            TypedOp::JumpIfFalsy { cond, target } => {
                if !registers[cond as usize].is_truthy() {
                    pc = target as usize;
                }
            }
            TypedOp::Jump { target } => {
                // A backward jump is an inner loop's backedge, and it counts
                // toward the same residency bound as the outer one.
                if (target as usize) < pc {
                    iterations += 1;
                    if iterations >= MAX_NATIVE_ITERATIONS {
                        let site = program.sites[target as usize];
                        return deopt(vm, program, registers, boxed, site);
                    }
                }
                pc = target as usize;
            }
            TypedOp::Exit { cond, exit_ip } => {
                if registers[cond as usize].is_truthy() {
                    continue;
                }
                write_back(vm, program, registers, boxed);
                // The exit target is reached with the same operand stack the
                // branch instruction started from, condition included.
                materialize_stack(vm, program, registers, boxed, program.sites[pc - 1]);
                vm.ip = exit_ip as usize;
                return Outcome::Ran;
            }
        }
    }
}

/// Loads the frame slots the program uses, declining when any slot holds a type
/// the register file cannot represent or a receiver is not a dense array.
fn seed_registers(
    vm: &mut Vm<'_>,
    program: &TypedLoopProgram,
    scratch: &mut TypedLoopScratch,
) -> Option<()> {
    scratch.clear();
    let TypedLoopScratch {
        registers,
        receivers,
        boxed,
        sloppy_global_writes,
        caches,
    } = scratch;
    // Resolve every receiver once. Re-reading the slot per element access cost
    // more than the interpreter's own path did.
    for &slot in &program.receiver_slots {
        // The receiver must be a dense array for the whole loop, so a region
        // that also writes that slot declines.
        let Some(Value::Array(array)) = vm.local_slot_value(slot as usize) else {
            {
                if std::env::var_os("QJS_WALK_TRACE").is_some() {
                    eprintln!("SEED reject #1");
                }
                return None;
            }
        };
        receivers.push(array);
        if program
            .written_locals
            .iter()
            .any(|(_, written)| *written == slot)
        {
            {
                if std::env::var_os("QJS_WALK_TRACE").is_some() {
                    eprintln!("SEED reject #2");
                }
                return None;
            }
        }
    }
    registers.resize(program.register_count, Typed::Undefined);
    for &(register, value) in &program.constant_registers {
        registers[register as usize] = value;
    }
    for &(register, slot) in &program.local_slots {
        let value = match vm.local_slot_value(slot as usize) {
            Some(value) => Typed::from_value(&value)?,
            None => Typed::Undefined,
        };
        registers[register as usize] = value;
    }
    for &(_, slot) in &program.written_locals {
        if !vm.slot_accepts_typed_loop_write(slot as usize) {
            {
                if std::env::var_os("QJS_WALK_TRACE").is_some() {
                    eprintln!("SEED reject #3");
                }
                return None;
            }
        }
    }
    for (register, name) in &program.global_reads {
        // The realm binding is the authority for a global; an accessor on the
        // global object would be observable per read, so it declines.
        if vm
            .global_this_own_property(name)
            .is_some_and(|property| property.is_accessor())
        {
            {
                if std::env::var_os("QJS_WALK_TRACE").is_some() {
                    eprintln!("SEED reject #4");
                }
                return None;
            }
        }
        // Resolving through the interpreter's own path keeps `this`, global
        // lexicals, and shadowing exactly as the loop would have seen them.
        let value = vm.load_global(name).ok()?;
        registers[*register as usize] = Typed::from_value(&value)?;
    }
    boxed.resize(program.boxed_count, Value::Undefined);
    for (register, value) in &program.boxed_constant_registers {
        boxed[*register as usize] = value.clone();
    }
    for &(register, slot) in &program.boxed_locals {
        // A boxed register holds any JavaScript value, and every operation
        // that consumes one checks what it actually is before acting:
        // `get_named` and `set_named` require an object, `element_read` an
        // array, `computed_read`/`computed_write` discriminate, `Unbox` and
        // `call_numeric_native` deoptimize on anything unexpected. Seeding
        // therefore does not need to pre-judge the value.
        //
        // It used to require an ordinary object here, which excluded a string
        // -- and a string key is the whole point of a computed access, so a
        // dictionary loop could compile and then always decline at entry.
        boxed[register as usize] = vm.local_slot_value(slot as usize)?;
    }
    for &register in &program.written_boxed_locals {
        let slot = program.slot_for_boxed_register(register)? as usize;
        if !vm.slot_accepts_typed_loop_write(slot) {
            {
                if std::env::var_os("QJS_WALK_TRACE").is_some() {
                    eprintln!("SEED reject #6");
                }
                return None;
            }
        }
    }
    for (register, name) in &program.boxed_global_reads {
        if vm
            .global_this_own_property(name)
            .is_some_and(|property| property.is_accessor())
        {
            {
                if std::env::var_os("QJS_WALK_TRACE").is_some() {
                    eprintln!("SEED reject #7");
                }
                return None;
            }
        }
        let value = vm.load_global(name).ok()?;
        if !value_is_ordinary_object(&value) {
            {
                if std::env::var_os("QJS_WALK_TRACE").is_some() {
                    eprintln!("SEED reject #8");
                }
                return None;
            }
        }
        boxed[*register as usize] = value;
    }
    for (slot, name) in &program.sloppy_global_writes {
        sloppy_global_writes.push(vm.prepare_typed_loop_sloppy_global_write(*slot as usize, name)?);
    }
    // A later generic native call may refresh a fallback slot from the realm
    // after user code runs. Register every sink once on entry so that slow-path
    // continuation observes the same live binding identity as ordinary stores.
    for (_, name) in &program.sloppy_global_writes {
        vm.record_sloppy_global_name(name);
    }
    caches.resize(program.cache_count, None);
    program
        .shape_caches
        .borrow_mut()
        .resize_with(program.cache_count, super::ShapeWays::default);
    Some(())
}

/// Whether a value is an ordinary object or array the program may hold in a
/// boxed register: a proxy, a symbol, or a callable would need the observable
/// property protocol.
fn value_is_ordinary_object(value: &Value) -> bool {
    match value {
        Value::Object(object) => !crate::symbol::is_symbol_primitive(object),
        Value::Array(_) => true,
        _ => false,
    }
}

fn write_back(vm: &mut Vm<'_>, program: &TypedLoopProgram, registers: &[Typed], boxed: &[Value]) {
    for &(register, slot) in &program.written_locals {
        vm.write_typed_loop_slot(slot as usize, registers[register as usize].to_value());
    }
    for &register in &program.written_boxed_locals {
        let Some(slot) = program.slot_for_boxed_register(register) else {
            continue;
        };
        vm.write_typed_loop_slot(slot as usize, boxed[register as usize].clone());
    }
}

/// Restores the frame and resumes interpretation at the loop header.
/// Abandons the program at `site`: the frame's slots take the register values,
/// the operand stack is rebuilt from the registers the site names, and the
/// interpreter resumes at the bytecode instruction that was about to run.
fn deopt(
    vm: &mut Vm<'_>,
    program: &TypedLoopProgram,
    registers: &[Typed],
    boxed: &[Value],
    site: DeoptSite,
) -> Outcome {
    write_back(vm, program, registers, boxed);
    materialize_stack(vm, program, registers, boxed, site);
    vm.ip = site.ip as usize;
    Outcome::Deoptimized
}

/// Pushes the operand stack `site` describes, so the interpreter sees exactly
/// what the bytecode instruction at `site.ip` expects.
fn materialize_stack(
    vm: &mut Vm<'_>,
    program: &TypedLoopProgram,
    registers: &[Typed],
    boxed: &[Value],
    site: DeoptSite,
) {
    let start = site.start as usize;
    for &(class, register) in &program.site_entries[start..start + usize::from(site.len)] {
        let value = match class {
            Class::Scalar => registers[register as usize].to_value(),
            Class::Boxed => boxed[register as usize].clone(),
        };
        vm.stack.push(value);
    }
}

/// Evaluates a `Math` intrinsic whose entire effect is a floating-point
/// computation, after proving the callee is that intrinsic. Anything else — a
/// user function, a bound function, a different native — declines.
fn call_numeric_native(callee: &Value, first: Typed, second: Typed, arity: u8) -> Option<Typed> {
    let Value::Function(function) = callee else {
        return None;
    };
    if function.bound.is_some() {
        return None;
    }
    let native = function.native?;
    // The same admitted set the counted-loop tier uses, so a region that hoists
    // its own receiver reaches every intrinsic that one does.
    let value = match arity {
        1 => super::super::vm_numeric_leaf::math_unary(native, first.number()?)?,
        2 => super::super::vm_numeric_leaf::math_binary(native, first.number()?, second.number()?)?,
        _ => return None,
    };
    Some(Typed::Number(value))
}

/// Answers a resolved call whose whole body a closed-form leaf evaluator can
/// compute, or declines so the loop stops before the call becomes observable.
///
/// `is_direct_leaf_function` is asked first because it is exactly the
/// precondition under which the interpreter reaches these evaluators, and this
/// operation's contract is to answer what the interpreter would answer. Probing
/// found no callee the evaluators alone admit wrongly -- bound functions,
/// `arguments` users, generators, async functions, and mutable captures are all
/// declined by the plans themselves -- so this is alignment with the
/// interpreter's own entry condition rather than a demonstrated last line of
/// defence. It is memoized on the function object, so after the first iteration
/// it is one load, which is not a price worth trading for an unverifiable
/// assumption that the plans are independently total.
fn call_closed_form_leaf(
    callee: &Value,
    receiver: &Value,
    first: Typed,
    second: Typed,
    arity: u8,
) -> Option<Value> {
    // A hoisted `Math` receiver is compiled to the unboxed operation, but a
    // callee that only turns out to be an intrinsic at run time still reaches
    // here; answering it costs one predicate and keeps that shape working.
    if let Some(value) = call_numeric_native(callee, first, second, arity) {
        return Some(value.to_value());
    }
    if !crate::function::is_direct_leaf_function(callee) {
        return None;
    }
    let Value::Function(function) = callee else {
        return None;
    };
    let bytecode = function.bytecode.as_ref()?;
    let arguments: [Value; 2] = [first.to_value(), second.to_value()];
    let arguments = arguments.get(..usize::from(arity))?;
    let value = super::super::vm_numeric_leaf::try_eval_numeric_leaf(
        bytecode,
        &function.params,
        arguments,
        &function.upvalues,
    )
    .or_else(|| {
        super::super::vm_this_property_leaf::try_eval_this_property_leaf(
            bytecode, receiver, arguments,
        )
    })?;
    // This operation answers an ordinary call, so it owes the same two counts
    // the interpreter's call path reports. They are taken here rather than
    // before the evaluators so a declined callee, which the interpreter then
    // runs and counts itself, is not counted twice.
    crate::diagnostics::count!(ordinary_call_attempts);
    crate::diagnostics::count!(closed_form_leaf_evaluations);
    Some(value)
}

/// Reads an own data property, revalidating the cached (name, slot) pair by
/// pointer and re-resolving it once when the receiver's layout differs.
fn get_named(
    receiver: &Value,
    name: &Rc<str>,
    cache: &mut Option<(Rc<str>, usize)>,
    shapes: &mut super::ShapeWays,
) -> Option<Value> {
    // An array's `length` is an own data property whose value is the element
    // count, and `try_direct_get_string` answers it exactly this way, without
    // consulting any descriptor. `for (i = 0; i < a.length; i++)` puts that read
    // in the header test of most loops written in JavaScript, so without it the
    // tier deoptimized on the region's first instruction and the whole loop fell
    // back to the generic interpreter.
    if let Value::Array(elements) = receiver {
        return (name.as_ref() == "length").then(|| Value::Number(elements.len() as f64));
    }
    let Value::Object(object) = receiver else {
        return None;
    };
    if let Some((key, slot)) = cache.as_ref()
        && let Some(value) = object.shared_data_slot_value(key, *slot)
    {
        return Some(value);
    }
    // Shape identity is an `Rc` pointer comparison, so scanning the remembered
    // shapes is cheaper than resolving the name even when the site is
    // polymorphic. `literal_data_slot_value` re-checks the revision, so a
    // mutated receiver misses rather than reading a stale slot.
    for (shape, slot) in shapes.entries() {
        if let Some(value) = object.literal_data_slot_value(shape, *slot) {
            return Some(value);
        }
    }
    if let Some((key, slot)) = object.shared_data_slot(name)
        && let Some(value) = object.shared_data_slot_value(&key, slot)
    {
        *cache = Some((key, slot));
        return Some(value);
    }
    if let Some((shape, slot)) = object.literal_data_slot(name)
        && let Some(value) = object.literal_data_slot_value(&shape, slot)
    {
        shapes.record(shape, slot);
        return Some(value);
    }
    // The name is not an own property. A method call site reads its callee
    // from the prototype every iteration, so remember where it resolved:
    // revisiting is then a pointer comparison and a slot read instead of a
    // hash lookup per level of the chain.
    //
    // The own lookups above are what keep this sound -- they run first on
    // every read, so a property that later appears on the receiver shadows the
    // remembered one rather than being masked by it.
    if let Some(inherited) = shapes.inherited()
        && let Some(value) = inherited.read(object)
    {
        return Some(value);
    }
    if let crate::value::Prototype::Object(prototype) = object.prototype_slot()?
        && let Some(slot) = prototype.own_data_slot(name)
        && let Some(value) = prototype.own_data_slot_value(slot)
    {
        shapes.record_inherited(prototype.clone(), prototype.property_revision(), slot);
        return Some(value);
    }
    // Storage the slot cache cannot address — a builtin's dynamic map, say — and
    // inherited data properties still read without observable behaviour, they
    // just resolve the name every time.
    ordinary_data_property(object, name)
}

/// Reads `name` as a plain data property of `object` or of its prototype chain,
/// refusing anything the observable property protocol would have to run: an
/// accessor, a proxy, an exotic object, a symbol wrapper.
fn ordinary_data_property(object: &crate::ObjectRef, name: &str) -> Option<Value> {
    use crate::value::{OwnDataPropertyRead, Prototype};

    let mut current = object.clone();
    // A chain longer than this is not worth a lookup per iteration.
    for _ in 0..8 {
        if crate::symbol::is_symbol_primitive(&current) {
            return None;
        }
        match current.own_data_property_read(name) {
            OwnDataPropertyRead::Data(value) => return Some(value),
            OwnDataPropertyRead::NeedsSlowPath => return None,
            OwnDataPropertyRead::Missing => {}
        }
        match current.prototype_slot() {
            None => return Some(Value::Undefined),
            Some(Prototype::Object(prototype)) => current = prototype,
            Some(_) => return None,
        }
    }
    None
}

/// Overwrites an existing own data property, declining a read-only property, an
/// accessor, and anything the observable path would have to handle.
fn set_named(receiver: &Value, name: &Rc<str>, value: &Value) -> bool {
    let Value::Object(object) = receiver else {
        return false;
    };
    matches!(
        object.write_existing_own_data_property(name, value),
        crate::value::OwnDataPropertyWrite::Written
    )
}

/// Reads `receiver[key]` for a boxed key, discriminating on what the receiver
/// and key actually are rather than assuming either.
fn computed_read(receiver: &Value, key: &Value) -> Option<Value> {
    match (receiver, key) {
        (Value::Object(object), Value::String(name)) => {
            ordinary_data_property(object, name.as_str())
        }
        // A boxed key is not automatically a string: the same site carries an
        // array index once a discovery forces its producer boxed. Answering
        // both keeps that discovery from deoptimizing ordinary indexing.
        (Value::Array(array), Value::Number(index)) => {
            if *index < 0.0 || index.fract() != 0.0 || *index > u32::MAX as f64 {
                return None;
            }
            array.direct_dense_index_value(*index as usize)
        }
        _ => None,
    }
}

/// Reads one element of a dense array held in a boxed register.
fn element_read(receiver: &Value, index: Typed) -> Option<Value> {
    let Value::Array(array) = receiver else {
        return None;
    };
    let number = index.number()?;
    if number < 0.0 || number.fract() != 0.0 || number > u32::MAX as f64 {
        return None;
    }
    array.direct_dense_index_value(number as usize)
}

fn dense_read(array: &crate::ArrayRef, index: Typed) -> Option<Typed> {
    let number = index.number()?;
    if number < 0.0 || number.fract() != 0.0 || number > u32::MAX as f64 {
        return None;
    }
    Typed::from_value(&array.direct_dense_index_value(number as usize)?)
}

/// Overwrites an in-bounds element of a dense array, declining anything that a
/// plain element store would not cover: a non-integer index, growth past the
/// current length, a hole, an own indexed descriptor, or a frozen array.
fn dense_write(array: &crate::ArrayRef, index: Typed, value: Typed) -> bool {
    let Some(number) = index.number() else {
        return false;
    };
    if number < 0.0 || number.fract() != 0.0 || number > u32::MAX as f64 {
        return false;
    }
    let index = number as usize;
    // `with_dense_writable_elements` already rejects a frozen array, holes, and
    // own indexed descriptors, so an in-bounds write there is the whole
    // observable effect. Growth stays on the observable path.
    array
        .with_dense_writable_elements(|elements| match elements.get_mut(index) {
            Some(element) => {
                *element = value.to_value();
                true
            }
            None => false,
        })
        .unwrap_or(false)
}

fn typed_binary(left: Typed, op: BinaryOp, right: Typed) -> Option<Typed> {
    let (Typed::Number(left), Typed::Number(right)) = (left, right) else {
        return None;
    };
    let value = match op {
        BinaryOp::Add => Typed::Number(left + right),
        BinaryOp::Sub => Typed::Number(left - right),
        BinaryOp::Mul => Typed::Number(left * right),
        BinaryOp::Div => Typed::Number(left / right),
        BinaryOp::Rem => Typed::Number(crate::operations::number_remainder(left, right)),
        BinaryOp::Pow => Typed::Number(crate::operations::number_exponentiate(left, right)),
        BinaryOp::Shl => Typed::Number(f64::from(to_int32(left) << (to_uint32(right) & 0x1f))),
        BinaryOp::Shr => Typed::Number(f64::from(to_int32(left) >> (to_uint32(right) & 0x1f))),
        BinaryOp::UShr => Typed::Number(f64::from(to_uint32(left) >> (to_uint32(right) & 0x1f))),
        BinaryOp::BitwiseAnd => Typed::Number(f64::from(to_int32(left) & to_int32(right))),
        BinaryOp::BitwiseOr => Typed::Number(f64::from(to_int32(left) | to_int32(right))),
        BinaryOp::BitwiseXor => Typed::Number(f64::from(to_int32(left) ^ to_int32(right))),
        BinaryOp::Lt => Typed::Boolean(left < right),
        BinaryOp::Le => Typed::Boolean(left <= right),
        BinaryOp::Gt => Typed::Boolean(left > right),
        BinaryOp::Ge => Typed::Boolean(left >= right),
        BinaryOp::Eq | BinaryOp::StrictEq => Typed::Boolean(left == right),
        BinaryOp::Ne | BinaryOp::StrictNe => Typed::Boolean(left != right),
        _ => return None,
    };
    Some(value)
}

fn typed_unary(op: UnaryOp, argument: Typed) -> Option<Typed> {
    if let UnaryOp::Not = op {
        return Some(Typed::Boolean(!argument.is_truthy()));
    }
    let number = argument.number()?;
    let value = match op {
        UnaryOp::Minus => Typed::Number(-number),
        UnaryOp::Plus => Typed::Number(number),
        UnaryOp::BitwiseNot => Typed::Number(f64::from(!to_int32(number))),
        UnaryOp::Not => unreachable!("handled above"),
        _ => return None,
    };
    Some(value)
}

fn to_int32(number: f64) -> i32 {
    crate::conversion::to_int32_number(number)
}

fn to_uint32(number: f64) -> u32 {
    crate::conversion::to_uint32_number(number)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::super::ShapeWays;
    use super::get_named;
    use crate::value::ArrayRef;
    use crate::{Value, eval};

    fn read(receiver: &Value, name: &str) -> Option<Value> {
        get_named(
            receiver,
            &Rc::from(name),
            &mut None,
            &mut ShapeWays::default(),
        )
    }

    #[test]
    fn an_array_answers_length_from_its_element_count() {
        let array = Value::Array(ArrayRef::new(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]));
        assert_eq!(read(&array, "length"), Some(Value::Number(3.0)));
    }

    #[test]
    fn an_array_declines_every_name_except_length() {
        let array = Value::Array(ArrayRef::new(vec![Value::Number(1.0)]));
        assert_eq!(read(&array, "push"), None);
        assert_eq!(read(&array, "0"), None);
    }

    #[test]
    fn a_length_guarded_loop_observes_growth_from_its_own_body() {
        // The read is answered from the live element count, so an array grown
        // inside the loop must extend it exactly as the interpreter does.
        let source = "function run(a) { var seen = 0; \
             for (var i = 0; i < a.length; i++) { seen++; \
             if (a.length < 8) { a.push(a[i] + 1); } } \
             return seen * 100 + a.length; } run([1]);";
        assert_eq!(eval(source), Ok(Value::Number(808.0)));
    }

    #[test]
    fn a_length_guarded_loop_observes_truncation_from_its_own_body() {
        let source = "function run(a) { var seen = 0; \
             for (var i = 0; i < a.length; i++) { seen++; a.length = 2; } \
             return seen; } run([1, 2, 3, 4, 5]);";
        assert_eq!(eval(source), Ok(Value::Number(2.0)));
    }
}
