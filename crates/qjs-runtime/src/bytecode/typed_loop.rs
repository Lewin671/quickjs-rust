//! A shape-independent executor for numeric loop regions.
//!
//! The specialized loop tiers each match one exact opcode sequence, so a loop
//! that computes the same thing in a different shape — an `if`/`else` in the
//! body, an extra temporary, a different operator order — runs on the general
//! interpreter at roughly 4.5ns per opcode. This module accepts *any* loop
//! region built from a whitelist of numeric opcodes, compiles it once into a
//! register program over unboxed values, and runs that program with no operand
//! stack, no `Value` boxing, and no per-opcode dispatch through the main match.
//!
//! Admission is conservative and checked twice: the region's opcodes must all
//! be in the whitelist and its stack behaviour must be statically consistent
//! (compile time), and every slot the program reads must hold a number,
//! boolean, or `undefined` while every slot it writes must be an authoritative
//! frame slot (entry time). Anything else declines and the interpreter runs the
//! loop unchanged. A guard that fails mid-run writes the registers back and
//! resumes interpretation at the loop header, so the program is never
//! observable.

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use super::ir::{Bytecode, Op};
use super::vm::Vm;
use crate::Value;

/// Registers are addressed with 16 bits, which bounds a compiled region.
const MAX_REGISTERS: usize = 1 << 12;

/// Longest region accepted, so compilation stays a bounded one-time cost.
const MAX_REGION_OPS: usize = 512;

/// Iterations run before handing control back, so a program cannot make the
/// engine unresponsive any longer than the interpreter would.
const MAX_NATIVE_ITERATIONS: u64 = 1 << 28;

/// An unboxed loop value. Only these three types take part; anything else
/// declines admission or deoptimizes.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Typed {
    Number(f64),
    Boolean(bool),
    Undefined,
}

impl Typed {
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Number(number) => Some(Self::Number(*number)),
            Value::Boolean(value) => Some(Self::Boolean(*value)),
            Value::Undefined => Some(Self::Undefined),
            _ => None,
        }
    }

    fn to_value(self) -> Value {
        match self {
            Self::Number(number) => Value::Number(number),
            Self::Boolean(value) => Value::Boolean(value),
            Self::Undefined => Value::Undefined,
        }
    }

    fn number(self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(number),
            _ => None,
        }
    }

    fn is_truthy(self) -> bool {
        match self {
            Self::Number(number) => number != 0.0 && !number.is_nan(),
            Self::Boolean(value) => value,
            Self::Undefined => false,
        }
    }

    /// `ToNumeric` for the admitted types, which never observes user code.
    fn to_numeric(self) -> Self {
        match self {
            Self::Number(_) => self,
            Self::Boolean(value) => Self::Number(f64::from(u8::from(value))),
            Self::Undefined => Self::Number(f64::NAN),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum TypedOp {
    Const {
        dst: u16,
        value: Typed,
    },
    Move {
        dst: u16,
        src: u16,
    },
    ToNumeric {
        dst: u16,
        src: u16,
    },
    Binary {
        dst: u16,
        op: BinaryOp,
        left: u16,
        right: u16,
    },
    Unary {
        dst: u16,
        op: UnaryOp,
        src: u16,
    },
    Update {
        dst: u16,
        op: UpdateOp,
        src: u16,
    },
    /// Reads one element of a dense array held in a frame slot. The receiver
    /// stays a slot rather than a register because registers hold only unboxed
    /// values.
    DenseRead {
        dst: u16,
        receiver_slot: u32,
        index: u16,
    },
    JumpIfFalsy {
        cond: u16,
        target: u32,
    },
    Jump {
        target: u32,
    },
    /// Leaves the loop: the condition value goes back on the operand stack,
    /// because the instruction at the loop's exit pops it.
    Exit {
        cond: u16,
        exit_ip: u32,
    },
}

/// A compiled loop region.
#[derive(Clone, Debug)]
pub(super) struct TypedLoopProgram {
    header: usize,
    backedge: usize,
    ops: Vec<TypedOp>,
    register_count: usize,
    /// Register holding each referenced frame slot, as (register, slot).
    local_slots: Vec<(u16, u32)>,
    /// Registers that must be written back to their slots when the loop ends.
    written_locals: Vec<u16>,
    /// Slots that must hold a dense-readable array on entry.
    receiver_slots: Vec<u32>,
}

impl TypedLoopProgram {
    pub(super) fn header(&self) -> usize {
        self.header
    }

    pub(super) fn backedge(&self) -> usize {
        self.backedge
    }

    fn slot_for_register(&self, register: u16) -> Option<u32> {
        self.local_slots
            .iter()
            .find(|(candidate, _)| *candidate == register)
            .map(|(_, slot)| *slot)
    }
}

/// Compiles every loop region of `bytecode` that the whitelist admits.
pub(super) fn compile_all(bytecode: &Bytecode) -> Vec<TypedLoopProgram> {
    bytecode
        .code
        .iter()
        .enumerate()
        .filter_map(|(backedge, op)| match op {
            Op::Jump(target) if *target < backedge => Some((*target, backedge)),
            _ => None,
        })
        .filter_map(|(header, backedge)| compile(bytecode, header, backedge))
        .collect()
}

fn compile(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<TypedLoopProgram> {
    let code = &bytecode.code;
    if backedge.checked_sub(header)? > MAX_REGION_OPS {
        return None;
    }
    let mut builder = Builder::new(bytecode, header, backedge);
    builder.compile()?;
    let Builder {
        ops,
        next_register,
        local_slots,
        written_locals,
        receiver_slots,
        ..
    } = builder;
    // A region that never leaves through its header test cannot be entered
    // safely, and one with no operations is not worth a program.
    if ops.is_empty() || !ops.iter().any(|op| matches!(op, TypedOp::Exit { .. })) {
        return None;
    }
    debug_assert!(matches!(code.get(backedge), Some(Op::Jump(_))));
    Some(TypedLoopProgram {
        header,
        backedge,
        ops,
        register_count: next_register,
        local_slots,
        written_locals,
        receiver_slots,
    })
}

/// Where a register's value came from, so a property read can recognize a
/// receiver that is really a frame slot.
#[derive(Clone, Copy, PartialEq)]
enum Origin {
    Computed,
    Local(u32),
}

struct Builder<'a> {
    bytecode: &'a Bytecode,
    header: usize,
    backedge: usize,
    ops: Vec<TypedOp>,
    next_register: usize,
    /// Register holding each referenced frame slot, keyed by slot.
    local_slots: Vec<(u16, u32)>,
    written_locals: Vec<u16>,
    receiver_slots: Vec<u32>,
    /// Abstract operand stack of (register, origin).
    stack: Vec<(u16, Origin)>,
    /// Stack depth recorded for each bytecode index reached by a jump, so a
    /// join with a different depth declines instead of miscompiling.
    depths: Vec<Option<usize>>,
    /// Bytecode index -> program index, for patching jumps.
    program_index: Vec<Option<u32>>,
    pending_jumps: Vec<(usize, usize)>,
}

impl<'a> Builder<'a> {
    fn new(bytecode: &'a Bytecode, header: usize, backedge: usize) -> Self {
        let span = backedge - header + 1;
        Self {
            bytecode,
            header,
            backedge,
            ops: Vec::new(),
            next_register: 0,
            local_slots: Vec::new(),
            written_locals: Vec::new(),
            receiver_slots: Vec::new(),
            stack: Vec::new(),
            depths: vec![None; span],
            program_index: vec![None; span],
            pending_jumps: Vec::new(),
        }
    }

    fn fresh(&mut self) -> Option<u16> {
        if self.next_register >= MAX_REGISTERS {
            return None;
        }
        let register = u16::try_from(self.next_register).ok()?;
        self.next_register += 1;
        Some(register)
    }

    fn local_register(&mut self, slot: usize) -> Option<u16> {
        let slot = u32::try_from(slot).ok()?;
        if let Some((register, _)) = self
            .local_slots
            .iter()
            .find(|(_, candidate)| *candidate == slot)
        {
            return Some(*register);
        }
        let register = self.fresh()?;
        self.local_slots.push((register, slot));
        Some(register)
    }

    fn note_write(&mut self, register: u16) {
        if !self.written_locals.contains(&register) {
            self.written_locals.push(register);
        }
    }

    fn push(&mut self, register: u16, origin: Origin) {
        self.stack.push((register, origin));
    }

    fn pop(&mut self) -> Option<(u16, Origin)> {
        self.stack.pop()
    }

    fn emit(&mut self, op: TypedOp) {
        self.ops.push(op);
    }

    fn compile(&mut self) -> Option<()> {
        let code = &self.bytecode.code;
        let mut ip = self.header;
        while ip <= self.backedge {
            let offset = ip - self.header;
            // A join must agree on stack depth with every path that reaches it.
            match self.depths[offset] {
                Some(depth) if depth != self.stack.len() => return None,
                Some(_) => {}
                None => self.depths[offset] = Some(self.stack.len()),
            }
            self.program_index[offset] = u32::try_from(self.ops.len()).ok();
            let op = code.get(ip)?;
            self.compile_op(op, ip)?;
            ip += 1;
        }
        // Patch the forward jumps now that every program index is known.
        for (program_slot, target_ip) in std::mem::take(&mut self.pending_jumps) {
            let offset = target_ip.checked_sub(self.header)?;
            let target = (*self.program_index.get(offset)?)?;
            match &mut self.ops[program_slot] {
                TypedOp::JumpIfFalsy { target: slot, .. } | TypedOp::Jump { target: slot } => {
                    *slot = target;
                }
                _ => return None,
            }
        }
        Some(())
    }

    fn compile_op(&mut self, op: &Op, ip: usize) -> Option<()> {
        match op {
            Op::LoadConst(index) => {
                let value = Typed::from_value(self.bytecode.constants.get(*index)?)?;
                let dst = self.fresh()?;
                self.emit(TypedOp::Const { dst, value });
                self.push(dst, Origin::Computed);
            }
            Op::LoadLocal(slot) => {
                let src = self.local_register(*slot)?;
                let dst = self.fresh()?;
                self.emit(TypedOp::Move { dst, src });
                self.push(dst, Origin::Local(u32::try_from(*slot).ok()?));
            }
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                let (src, _) = self.pop()?;
                let dst = self.local_register(*slot)?;
                self.emit(TypedOp::Move { dst, src });
                self.note_write(dst);
            }
            Op::Dup => {
                let top = *self.stack.last()?;
                self.stack.push(top);
            }
            Op::Pop => {
                self.pop()?;
            }
            Op::ToNumeric => {
                let (src, _) = self.pop()?;
                let dst = self.fresh()?;
                self.emit(TypedOp::ToNumeric { dst, src });
                self.push(dst, Origin::Computed);
            }
            Op::Binary(binary) if admitted_binary(*binary) => {
                let (right, _) = self.pop()?;
                let (left, _) = self.pop()?;
                let dst = self.fresh()?;
                self.emit(TypedOp::Binary {
                    dst,
                    op: *binary,
                    left,
                    right,
                });
                self.push(dst, Origin::Computed);
            }
            Op::Unary(unary) if admitted_unary(*unary) => {
                let (src, _) = self.pop()?;
                let dst = self.fresh()?;
                self.emit(TypedOp::Unary {
                    dst,
                    op: *unary,
                    src,
                });
                self.push(dst, Origin::Computed);
            }
            Op::Update(update) => {
                let (src, _) = self.pop()?;
                let dst = self.fresh()?;
                self.emit(TypedOp::Update {
                    dst,
                    op: *update,
                    src,
                });
                self.push(dst, Origin::Computed);
            }
            Op::GetProp => {
                let (index, _) = self.pop()?;
                let (_, receiver) = self.pop()?;
                let Origin::Local(receiver_slot) = receiver else {
                    return None;
                };
                if !self.receiver_slots.contains(&receiver_slot) {
                    self.receiver_slots.push(receiver_slot);
                }
                let dst = self.fresh()?;
                self.emit(TypedOp::DenseRead {
                    dst,
                    receiver_slot,
                    index,
                });
                self.push(dst, Origin::Computed);
            }
            Op::JumpIfFalse(target) => {
                // The condition stays on the operand stack: an in-region branch
                // has a `Pop` at its target, and the loop's exit pops it too.
                let (cond, _) = *self.stack.last()?;
                if *target > self.backedge {
                    self.emit(TypedOp::Exit {
                        cond,
                        exit_ip: u32::try_from(*target).ok()?,
                    });
                } else {
                    if *target <= ip {
                        return None;
                    }
                    self.record_target(*target, self.stack.len())?;
                    self.pending_jumps.push((self.ops.len(), *target));
                    self.emit(TypedOp::JumpIfFalsy { cond, target: 0 });
                }
            }
            Op::Jump(target) if *target == self.header && ip == self.backedge => {
                // The backedge itself: the executor loops on its own.
            }
            Op::Jump(target) => {
                if *target <= ip || *target > self.backedge {
                    return None;
                }
                self.record_target(*target, self.stack.len())?;
                self.pending_jumps.push((self.ops.len(), *target));
                self.emit(TypedOp::Jump { target: 0 });
                // Everything up to the target is unreachable from here; the
                // linear walk continues, and the recorded depth keeps the join
                // consistent.
            }
            _ => return None,
        }
        Some(())
    }

    fn record_target(&mut self, target: usize, depth: usize) -> Option<()> {
        let offset = target.checked_sub(self.header)?;
        match self.depths.get_mut(offset)? {
            Some(existing) if *existing != depth => None,
            slot => {
                *slot = Some(depth);
                Some(())
            }
        }
    }
}

fn admitted_binary(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::Pow
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseOr
            | BinaryOp::BitwiseXor
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
            | BinaryOp::Eq
            | BinaryOp::Ne
            | BinaryOp::StrictEq
            | BinaryOp::StrictNe
    )
}

fn admitted_unary(op: UnaryOp) -> bool {
    matches!(
        op,
        UnaryOp::Minus | UnaryOp::Plus | UnaryOp::BitwiseNot | UnaryOp::Not
    )
}

/// Runs the program covering the backedge at `ip`, if one exists and this
/// frame admits it. Returns whether the loop was executed natively.
pub(super) fn try_run_typed_loop(vm: &mut Vm<'_>, header: usize, backedge: usize) -> bool {
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
    if !run(vm, program) {
        return decline(vm);
    }
    true
}

fn run(vm: &mut Vm<'_>, program: &TypedLoopProgram) -> bool {
    let Some(mut registers) = seed_registers(vm, program) else {
        return false;
    };
    let mut iterations = 0_u64;
    let mut index = 0_usize;
    loop {
        let Some(op) = program.ops.get(index) else {
            // Fell off the end of the program: that is the backedge.
            index = 0;
            iterations += 1;
            if iterations >= MAX_NATIVE_ITERATIONS {
                write_back(vm, program, &registers);
                vm.ip = program.header;
                return true;
            }
            continue;
        };
        index += 1;
        match *op {
            TypedOp::Const { dst, value } => registers[dst as usize] = value,
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
                    return deopt(vm, program, &registers);
                };
                registers[dst as usize] = value;
            }
            TypedOp::Unary { dst, op, src } => {
                let Some(value) = typed_unary(op, registers[src as usize]) else {
                    return deopt(vm, program, &registers);
                };
                registers[dst as usize] = value;
            }
            TypedOp::Update { dst, op, src } => {
                let Some(number) = registers[src as usize].number() else {
                    return deopt(vm, program, &registers);
                };
                registers[dst as usize] = Typed::Number(match op {
                    UpdateOp::Increment => number + 1.0,
                    UpdateOp::Decrement => number - 1.0,
                });
            }
            TypedOp::DenseRead {
                dst,
                receiver_slot,
                index: index_register,
            } => {
                let Some(value) = dense_read(vm, receiver_slot, registers[index_register as usize])
                else {
                    return deopt(vm, program, &registers);
                };
                registers[dst as usize] = value;
            }
            TypedOp::JumpIfFalsy { cond, target } => {
                if !registers[cond as usize].is_truthy() {
                    index = target as usize;
                }
            }
            TypedOp::Jump { target } => index = target as usize,
            TypedOp::Exit { cond, exit_ip } => {
                if registers[cond as usize].is_truthy() {
                    continue;
                }
                write_back(vm, program, &registers);
                vm.stack.push(registers[cond as usize].to_value());
                vm.ip = exit_ip as usize;
                return true;
            }
        }
    }
}

/// Loads the frame slots the program uses, declining when any slot holds a type
/// the register file cannot represent or a receiver is not a dense array.
fn seed_registers(vm: &Vm<'_>, program: &TypedLoopProgram) -> Option<Vec<Typed>> {
    for &slot in &program.receiver_slots {
        // The receiver must be a dense array for the whole loop, so a region
        // that also writes that slot declines.
        if !matches!(vm.local_slot_value(slot as usize), Some(Value::Array(_))) {
            return None;
        }
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
    Some(registers)
}

fn write_back(vm: &mut Vm<'_>, program: &TypedLoopProgram, registers: &[Typed]) {
    for &register in &program.written_locals {
        let Some(slot) = program.slot_for_register(register) else {
            continue;
        };
        vm.write_typed_loop_slot(slot as usize, registers[register as usize].to_value());
    }
}

/// Restores the frame and resumes interpretation at the loop header.
fn deopt(vm: &mut Vm<'_>, program: &TypedLoopProgram, registers: &[Typed]) -> bool {
    write_back(vm, program, registers);
    vm.ip = program.header;
    true
}

fn dense_read(vm: &Vm<'_>, receiver_slot: u32, index: Typed) -> Option<Typed> {
    let number = index.number()?;
    if number < 0.0 || number.fract() != 0.0 || number > u32::MAX as f64 {
        return None;
    }
    let Some(Value::Array(array)) = vm.local_slot_value(receiver_slot as usize) else {
        return None;
    };
    Typed::from_value(&array.direct_dense_index_value(number as usize)?)
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
    use crate::{Value, eval};

    /// Every case here is a loop the typed tier accepts. The expected values are
    /// what the interpreter produces for the same source, so a divergence in the
    /// register program shows up as a failing assertion rather than a silent
    /// wrong answer.
    #[test]
    fn typed_loops_match_interpreted_results() {
        // Arithmetic, an if/else body, and a shift — the shape the specialized
        // tiers decline because of the branch.
        assert_eq!(
            eval(
                "function run(n) { var a = 0, b = 1, c = 0;\
                   for (var i = 0; i < n; i++) {\
                     if (a > b) { c = a - (b >> i); b = (a >> i) + b; a = c; }\
                     else { c = a + (b >> i); b = -(a >> i) + b; a = c; }\
                   }\
                   return a + ':' + b + ':' + c; }\
                 run(40);"
            ),
            Ok(Value::String("2:1:2".to_owned().into()))
        );
        // An element read from a dense array in a local slot.
        assert_eq!(
            eval(
                "function run(n) { var table = [1, 2, 4, 8, 16], total = 0;\
                   for (var i = 0; i < n; i++) { total = total + table[i % 5]; }\
                   return total; }\
                 run(20);"
            ),
            Ok(Value::Number(124.0))
        );
        // Every admitted operator, so the register program's semantics are
        // pinned against the interpreter's.
        assert_eq!(
            eval(
                "function run() { var s = 0;\
                   for (var i = 1; i < 12; i++) {\
                     s += i * 3 - 1;\
                     s += i / 2;\
                     s += i % 4;\
                     s += i ** 2;\
                     s += (i << 2) | (i >> 1);\
                     s += (i & 6) ^ (i >>> 1);\
                     s += -i;\
                     s += ~i;\
                     if (i < 5) s += 1;\
                     if (i <= 5) s += 1;\
                     if (i > 5) s += 1;\
                     if (i >= 5) s += 1;\
                     if (i === 5) s += 1;\
                     if (i !== 5) s += 1;\
                     if (!(i === 5)) s += 1;\
                   }\
                   return s; }\
                 run();"
            ),
            Ok(Value::Number(980.0))
        );
        // A zero-iteration loop leaves everything untouched.
        assert_eq!(
            eval(
                "function run() { var s = 7; for (var i = 0; i < 0; i++) { s = s + 1; } return s + ':' + i; } run();"
            ),
            Ok(Value::String("7:0".to_owned().into()))
        );
    }

    #[test]
    fn typed_loops_deoptimize_instead_of_guessing() {
        // A non-numeric operand mid-loop must fall back to the interpreter and
        // produce the interpreter's answer, including string concatenation.
        assert_eq!(
            eval(
                "function run() { var s = 0, flip = 3;\
                   for (var i = 0; i < 6; i++) { if (i === 3) flip = 'x'; s = s + flip; }\
                   return String(s); }\
                 run();"
            ),
            Ok(Value::String("9xxx".to_owned().into()))
        );
        // A hole and an out-of-range index both leave the fast path.
        assert_eq!(
            eval(
                "function run() { var table = [1, , 3], total = 0;\
                   for (var i = 0; i < 4; i++) { total = total + table[i]; }\
                   return String(total); }\
                 run();"
            ),
            Ok(Value::String("NaN".to_owned().into()))
        );
        // A receiver that stops being an array declines on the next entry.
        assert_eq!(
            eval(
                "function run() { var table = [1, 2, 3], total = 0;\
                   for (var i = 0; i < 3; i++) { total = total + table[i]; }\
                   table = { 0: 10, 1: 20, 2: 30 };\
                   for (var j = 0; j < 3; j++) { total = total + table[j]; }\
                   return total; }\
                 run();"
            ),
            Ok(Value::Number(66.0))
        );
        // A captured counter keeps the observable path, so the closure sees
        // every value the interpreter would produce.
        assert_eq!(
            eval(
                "function run() { var seen = [], s = 0;\
                   for (var i = 0; i < 3; i++) { s = s + i; seen.push(function () { return i; }); }\
                   return s + ':' + seen[0]() + ':' + seen.length; }\
                 run();"
            ),
            Ok(Value::String("3:3:3".to_owned().into()))
        );
    }
}
