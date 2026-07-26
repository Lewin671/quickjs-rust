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
use super::{DeoptSite, MAX_NATIVE_ITERATIONS, Typed, TypedLoopProgram, TypedOp};
use crate::Value;

/// Runs the program covering the backedge at `ip`, if one exists and this
/// frame admits it. Returns whether the loop was executed natively.
pub(crate) fn try_run_typed_loop(vm: &mut Vm<'_>, header: usize, backedge: usize) -> bool {
    if vm.direct_eval_with_stack {
        return false;
    }
    let programs = vm.typed_loop_programs;
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
    if vm
        .numeric_loop_plans
        .iter()
        .any(|plan| plan.contains_instruction(backedge))
        || vm
            .shared_numeric_mutation_loop_plans
            .iter()
            .any(|plan| plan.contains_instruction(backedge))
        || vm
            .control_loop_plans
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
    let Some((mut registers, receivers, mut boxed)) = seed_registers(vm, program) else {
        return Outcome::Declined;
    };
    // One inline cache per property access site, warmed on the first iteration.
    let mut caches: Vec<Option<(Rc<str>, usize)>> = vec![None; program.cache_count];
    let mut iterations = 0_u64;
    let mut pc = 0_usize;
    macro_rules! deopt_here {
        ($op:expr) => {{
            let site = program.sites[pc - 1];
            return deopt(vm, program, &registers, &boxed, site);
        }};
    }
    loop {
        let Some(op) = program.ops.get(pc) else {
            // Fell off the end of the program: that is the backedge.
            pc = 0;
            iterations += 1;
            if iterations >= MAX_NATIVE_ITERATIONS {
                let site = program.sites[0];
                return deopt(vm, program, &registers, &boxed, site);
            }
            continue;
        };
        pc += 1;
        match *op {
            TypedOp::Const { dst, value } => registers[dst as usize] = value,
            TypedOp::Move { dst, src } => registers[dst as usize] = registers[src as usize],
            TypedOp::ConstBoxed { dst, constant } => {
                let Some(value) = program.boxed_constants.get(constant as usize) else {
                    deopt_here!(op);
                };
                boxed[dst as usize] = value.clone();
            }
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
                let Some(number) = registers[src as usize].number() else {
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
                        return deopt(vm, program, &registers, &boxed, site);
                    }
                }
                pc = target as usize;
            }
            TypedOp::Exit { cond, exit_ip } => {
                if registers[cond as usize].is_truthy() {
                    continue;
                }
                write_back(vm, program, &registers, &boxed);
                // The exit target is reached with the same operand stack the
                // branch instruction started from, condition included.
                materialize_stack(vm, &registers, &boxed, program.sites[pc - 1]);
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
) -> Option<(Vec<Typed>, Vec<crate::ArrayRef>, Vec<Value>)> {
    // Resolve every receiver once. Re-reading the slot per element access cost
    // more than the interpreter's own path did.
    let mut receivers = Vec::with_capacity(program.receiver_slots.len());
    for &slot in &program.receiver_slots {
        // The receiver must be a dense array for the whole loop, so a region
        // that also writes that slot declines.
        let Some(Value::Array(array)) = vm.local_slot_value(slot as usize) else {
            return None;
        };
        receivers.push(array);
        if program
            .written_locals
            .iter()
            .filter_map(|register| program.slot_for_register(*register))
            .any(|written| written == slot)
        {
            return None;
        }
    }
    let mut registers = vec![Typed::Undefined; program.register_count];
    for &(register, slot) in &program.local_slots {
        // A receiver slot holds the array itself; its register is only ever the
        // popped left operand of an element read, never a numeric operand, so
        // the unboxed register file does not have to represent it.
        if program.receiver_slots.contains(&slot) {
            continue;
        }
        let value = match vm.local_slot_value(slot as usize) {
            Some(value) => Typed::from_value(&value)?,
            None => Typed::Undefined,
        };
        registers[register as usize] = value;
    }
    for &register in &program.written_locals {
        let slot = program.slot_for_register(register)? as usize;
        if !vm.slot_accepts_typed_loop_write(slot) {
            return None;
        }
    }
    for (register, name) in &program.global_reads {
        // The realm binding is the authority for a global; an accessor on the
        // global object would be observable per read, so it declines.
        if vm
            .global_this_own_property(name)
            .is_some_and(|property| property.is_accessor())
        {
            return None;
        }
        // Resolving through the interpreter's own path keeps `this`, global
        // lexicals, and shadowing exactly as the loop would have seen them.
        let value = vm.load_global(name).ok()?;
        registers[*register as usize] = Typed::from_value(&value)?;
    }
    let mut boxed = vec![Value::Undefined; program.boxed_count];
    for &(register, slot) in &program.boxed_locals {
        // A boxed local must already hold an object: the program only ever uses
        // one as a property receiver or an element source.
        let value = vm.local_slot_value(slot as usize)?;
        if !value_is_ordinary_object(&value) {
            return None;
        }
        boxed[register as usize] = value;
    }
    for &register in &program.written_boxed_locals {
        let slot = program.slot_for_boxed_register(register)? as usize;
        if !vm.slot_accepts_typed_loop_write(slot) {
            return None;
        }
    }
    for (register, name) in &program.boxed_global_reads {
        if vm
            .global_this_own_property(name)
            .is_some_and(|property| property.is_accessor())
        {
            return None;
        }
        let value = vm.load_global(name).ok()?;
        if !value_is_ordinary_object(&value) {
            return None;
        }
        boxed[*register as usize] = value;
    }
    Some((registers, receivers, boxed))
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
    for &register in &program.written_locals {
        let Some(slot) = program.slot_for_register(register) else {
            continue;
        };
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
    materialize_stack(vm, registers, boxed, site);
    vm.ip = site.ip as usize;
    Outcome::Deoptimized
}

/// Pushes the operand stack `site` describes, so the interpreter sees exactly
/// what the bytecode instruction at `site.ip` expects.
fn materialize_stack(vm: &mut Vm<'_>, registers: &[Typed], boxed: &[Value], site: DeoptSite) {
    for depth in 0..usize::from(site.depth) {
        let value = if site.boxed & (1_u64 << depth) != 0 {
            boxed[depth].clone()
        } else {
            registers[depth].to_value()
        };
        vm.stack.push(value);
    }
}

/// Evaluates a `Math` intrinsic whose entire effect is a floating-point
/// computation, after proving the callee is that intrinsic. Anything else — a
/// user function, a bound function, a different native — declines.
fn call_numeric_native(callee: &Value, first: Typed, second: Typed, arity: u8) -> Option<Typed> {
    use crate::function::NativeFunction as Native;
    let Value::Function(function) = callee else {
        return None;
    };
    if function.bound.is_some() {
        return None;
    }
    let native = function.native?;
    let first = first.number();
    let second = second.number();
    let value = match (native, arity) {
        (Native::MathSqrt, 1) => first?.sqrt(),
        (Native::MathAbs, 1) => first?.abs(),
        (Native::MathFloor, 1) => first?.floor(),
        (Native::MathCeil, 1) => first?.ceil(),
        (Native::MathTrunc, 1) => first?.trunc(),
        (Native::MathSin, 1) => first?.sin(),
        (Native::MathCos, 1) => first?.cos(),
        (Native::MathExp, 1) => first?.exp(),
        (Native::MathPow, 2) => crate::operations::number_exponentiate(first?, second?),
        _ => return None,
    };
    Some(Typed::Number(value))
}

/// Reads an own data property, revalidating the cached (name, slot) pair by
/// pointer and re-resolving it once when the receiver's layout differs.
fn get_named(
    receiver: &Value,
    name: &Rc<str>,
    cache: &mut Option<(Rc<str>, usize)>,
) -> Option<Value> {
    let Value::Object(object) = receiver else {
        return None;
    };
    if let Some((key, slot)) = cache.as_ref()
        && let Some(value) = object.shared_data_slot_value(key, *slot)
    {
        return Some(value);
    }
    if let Some((key, slot)) = object.shared_data_slot(name)
        && let Some(value) = object.shared_data_slot_value(&key, slot)
    {
        *cache = Some((key, slot));
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
