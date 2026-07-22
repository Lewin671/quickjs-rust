//! Typed dense-array mutation plans compiled from immutable bytecode.
//!
//! Fixed-index recurrences scalar-replace a small set of Number elements.
//! Computed-index loops translate one straight-line body into a bounded Number
//! register program with one dense read/modify/write. The runtime checks every
//! potentially observable condition before the current iteration's store, so a
//! failed guard can publish completed iterations and restart at the header.

use std::collections::{BTreeMap, BTreeSet};

use qjs_ast::{BinaryOp, UnaryOp};

use crate::{Value, to_int32_number, to_uint32_number};

use super::super::{
    ir::{Bytecode, Op, decode_index_receiver},
    vm::Vm,
    vm_props::array_index_from_number,
};

const MAX_DENSE_OPS: usize = 64;
const MAX_DENSE_LOCALS: usize = 64;
const MAX_DENSE_WRITES: usize = 64;
const MAX_FIXED_MUTATIONS: usize = 16;

#[cfg(test)]
thread_local! {
    static DENSE_LOOP_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_test_iterations() {
    DENSE_LOOP_ITERATIONS.set(0);
}

#[cfg(test)]
pub(super) fn test_iterations() -> usize {
    DENSE_LOOP_ITERATIONS.get()
}

#[inline]
fn record_iteration() {
    #[cfg(test)]
    DENSE_LOOP_ITERATIONS.set(DENSE_LOOP_ITERATIONS.get() + 1);
}

#[derive(Clone, Debug)]
pub(super) struct DenseNumericMutationLoopPlan {
    exit: usize,
    kind: DensePlanKind,
}

#[derive(Clone, Debug)]
enum DensePlanKind {
    Fixed(FixedDensePlan),
    Dynamic(DynamicDensePlan),
}

#[derive(Clone, Copy, Debug)]
enum FixedMutationOp {
    Copy,
    Add(f64),
    Subtract(f64),
}

#[derive(Clone, Copy, Debug)]
struct FixedMutation {
    source: usize,
    target: usize,
    operation: FixedMutationOp,
}

#[derive(Clone, Debug)]
struct FixedDensePlan {
    counter_slot: usize,
    limit_slot: usize,
    accumulator_slot: usize,
    block_result_slot: usize,
    loop_result_slot: usize,
    receiver_slot: usize,
    indices: Vec<usize>,
    mutations: Vec<FixedMutation>,
    checksum_index: usize,
}

#[derive(Clone, Debug)]
struct DynamicDensePlan {
    counter_local: usize,
    limit_local: usize,
    receiver_slot: usize,
    local_slots: Vec<usize>,
    operations: Vec<NumberInstruction>,
    writes: Vec<LocalWrite>,
    store: DenseStore,
    header: usize,
}

type Register = usize;

#[derive(Clone, Debug)]
enum NumberInstruction {
    Constant(f64),
    LoadLocal(usize),
    DenseLoad {
        index: Register,
    },
    Binary {
        operation: BinaryOp,
        left: Register,
        right: Register,
    },
    Unary {
        operation: UnaryOp,
        value: Register,
    },
}

#[derive(Clone, Copy, Debug)]
struct LocalWrite {
    local: usize,
    value: Register,
}

#[derive(Clone, Copy, Debug)]
struct DenseStore {
    index: Register,
    value: Register,
}

impl DenseNumericMutationLoopPlan {
    pub(super) fn compile(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<Self> {
        compile_fixed(bytecode, header, backedge).or_else(|| {
            compile_dynamic(bytecode, header, backedge).map(|(exit, plan)| Self {
                exit,
                kind: DensePlanKind::Dynamic(plan),
            })
        })
    }

    pub(super) fn exit(&self) -> usize {
        self.exit
    }

    pub(super) fn try_run(&self, vm: &mut Vm<'_>) -> bool {
        match &self.kind {
            DensePlanKind::Fixed(plan) => plan.try_run(vm, self.exit),
            DensePlanKind::Dynamic(plan) => plan.try_run(vm, self.exit),
        }
    }
}

fn compile_fixed(
    bytecode: &Bytecode,
    header: usize,
    backedge: usize,
) -> Option<DenseNumericMutationLoopPlan> {
    let code = &bytecode.code;
    let (
        Op::LoadLocal(counter_slot),
        Op::LoadLocal(limit_slot),
        Op::Binary(BinaryOp::Lt),
        Op::JumpIfFalse(exit),
        Op::Pop,
    ) = (
        code.get(header)?,
        code.get(header + 1)?,
        code.get(header + 2)?,
        code.get(header + 3)?,
        code.get(header + 4)?,
    )
    else {
        return None;
    };
    if *exit <= backedge || !matches!(code.get(*exit), Some(Op::Pop)) {
        return None;
    }

    let tail = backedge.checked_sub(8)?;
    let (
        Op::LoadLocal(tail_block_result_slot),
        Op::StoreLocal(loop_result_slot),
        Op::LoadLocal(tail_counter_slot),
        Op::ToNumeric,
        Op::Dup,
        Op::Update(qjs_ast::UpdateOp::Increment),
        Op::AssignLocal(assigned_counter_slot),
        Op::Pop,
        Op::Jump(tail_header),
    ) = (
        code.get(tail)?,
        code.get(tail + 1)?,
        code.get(tail + 2)?,
        code.get(tail + 3)?,
        code.get(tail + 4)?,
        code.get(tail + 5)?,
        code.get(tail + 6)?,
        code.get(tail + 7)?,
        code.get(tail + 8)?,
    )
    else {
        return None;
    };
    if tail_header != &header
        || tail_counter_slot != counter_slot
        || assigned_counter_slot != counter_slot
    {
        return None;
    }

    let (Op::LoadConst(_), Op::StoreLocal(block_result_slot)) =
        (code.get(header + 5)?, code.get(header + 6)?)
    else {
        return None;
    };
    if tail_block_result_slot != block_result_slot {
        return None;
    }

    let mut cursor = header + 7;
    let mut receiver_slot = None;
    let mut raw_mutations = Vec::new();
    while let Some((mutation, slot, next)) =
        compile_fixed_mutation(bytecode, cursor, *block_result_slot, *loop_result_slot)
    {
        if receiver_slot.is_some_and(|current| current != slot) {
            return None;
        }
        receiver_slot = Some(slot);
        raw_mutations.push(mutation);
        cursor = next;
    }
    if raw_mutations.is_empty() || raw_mutations.len() > MAX_FIXED_MUTATIONS {
        return None;
    }

    let (
        Op::LoadLocal(accumulator_slot),
        Op::GetPropIndex(encoded_checksum),
        Op::Binary(BinaryOp::Add),
        Op::Dup,
        Op::AssignLocal(assigned_accumulator_slot),
        Op::Dup,
        Op::StoreLocal(accumulator_block_result_slot),
        Op::StoreLocal(accumulator_loop_result_slot),
    ) = (
        code.get(cursor)?,
        code.get(cursor + 1)?,
        code.get(cursor + 2)?,
        code.get(cursor + 3)?,
        code.get(cursor + 4)?,
        code.get(cursor + 5)?,
        code.get(cursor + 6)?,
        code.get(cursor + 7)?,
    )
    else {
        return None;
    };
    let (checksum_index, checksum_receiver) = decode_index_receiver(*encoded_checksum);
    let receiver_slot = receiver_slot?;
    let required_distinct_slots = [
        *counter_slot,
        *limit_slot,
        *accumulator_slot,
        *block_result_slot,
        *loop_result_slot,
        receiver_slot,
    ];
    if cursor + 8 != tail
        || checksum_receiver != Some(receiver_slot)
        || assigned_accumulator_slot != accumulator_slot
        || accumulator_block_result_slot != block_result_slot
        || accumulator_loop_result_slot != loop_result_slot
        || required_distinct_slots
            .iter()
            .enumerate()
            .any(|(index, slot)| required_distinct_slots[..index].contains(slot))
    {
        return None;
    }

    let mut indices = BTreeSet::new();
    indices.insert(checksum_index);
    for (source, target, _) in &raw_mutations {
        indices.insert(*source);
        indices.insert(*target);
    }
    let indices: Vec<_> = indices.into_iter().collect();
    let position = |index| indices.binary_search(&index).ok();
    let mutations = raw_mutations
        .into_iter()
        .map(|(source, target, operation)| {
            Some(FixedMutation {
                source: position(source)?,
                target: position(target)?,
                operation,
            })
        })
        .collect::<Option<Vec<_>>>()?;

    Some(DenseNumericMutationLoopPlan {
        exit: *exit,
        kind: DensePlanKind::Fixed(FixedDensePlan {
            counter_slot: *counter_slot,
            limit_slot: *limit_slot,
            accumulator_slot: *accumulator_slot,
            block_result_slot: *block_result_slot,
            loop_result_slot: *loop_result_slot,
            receiver_slot,
            checksum_index: position(checksum_index)?,
            indices,
            mutations,
        }),
    })
}

fn compile_fixed_mutation(
    bytecode: &Bytecode,
    cursor: usize,
    block_result_slot: usize,
    loop_result_slot: usize,
) -> Option<((usize, usize, FixedMutationOp), usize, usize)> {
    let code = &bytecode.code;
    let Op::LoadLocal(receiver_slot) = code.get(cursor)? else {
        return None;
    };
    let Op::GetPropIndex(encoded_source) = code.get(cursor + 1)? else {
        return None;
    };
    let (source, cached_receiver) = decode_index_receiver(*encoded_source);
    if cached_receiver != Some(*receiver_slot) {
        return None;
    }

    let (operation, set_offset) = match (code.get(cursor + 2), code.get(cursor + 3)) {
        (Some(Op::LoadConst(constant)), Some(Op::Binary(operation))) => {
            let Value::Number(constant) = bytecode.constants.get(*constant)? else {
                return None;
            };
            let operation = match operation {
                BinaryOp::Add => FixedMutationOp::Add(*constant),
                BinaryOp::Sub => FixedMutationOp::Subtract(*constant),
                _ => return None,
            };
            (operation, 4)
        }
        (Some(Op::SetPropIndex { .. }), _) => (FixedMutationOp::Copy, 2),
        _ => return None,
    };
    let Op::SetPropIndex { index: target, .. } = code.get(cursor + set_offset)? else {
        return None;
    };
    let next = cursor + set_offset + 4;
    if !matches!(code.get(cursor + set_offset + 1), Some(Op::Dup))
        || !matches!(code.get(cursor + set_offset + 2), Some(Op::StoreLocal(slot)) if *slot == block_result_slot)
        || !matches!(code.get(cursor + set_offset + 3), Some(Op::StoreLocal(slot)) if *slot == loop_result_slot)
    {
        return None;
    }
    Some(((source, *target, operation), *receiver_slot, next))
}

impl FixedDensePlan {
    fn try_run(&self, vm: &mut Vm<'_>, exit: usize) -> bool {
        if vm.direct_eval_with_stack {
            return false;
        }
        for slot in [
            self.counter_slot,
            self.limit_slot,
            self.accumulator_slot,
            self.block_result_slot,
            self.loop_result_slot,
            self.receiver_slot,
        ] {
            if !vm.slot_is_authoritative(slot) {
                return false;
            }
        }
        let (Some(mut counter), Some(limit), Some(mut accumulator)) = (
            local_number(vm, self.counter_slot),
            local_number(vm, self.limit_slot),
            local_number(vm, self.accumulator_slot),
        ) else {
            return false;
        };
        let Some(Some(Value::Array(array))) = vm.locals.get(self.receiver_slot) else {
            return false;
        };
        let array = array.clone();
        let completed = array.with_dense_writable_elements(|elements| {
            let mut values = self
                .indices
                .iter()
                .map(|index| match elements.get(*index) {
                    Some(Value::Number(value)) => Some(*value),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?;
            while counter < limit {
                for mutation in &self.mutations {
                    values[mutation.target] = match mutation.operation {
                        FixedMutationOp::Copy => values[mutation.source],
                        FixedMutationOp::Add(constant) => values[mutation.source] + constant,
                        FixedMutationOp::Subtract(constant) => values[mutation.source] - constant,
                    };
                }
                accumulator += values[self.checksum_index];
                counter += 1.0;
                record_iteration();
            }
            for mutation in &self.mutations {
                elements[self.indices[mutation.target]] = Value::Number(values[mutation.target]);
            }
            Some(())
        });
        if !matches!(completed, Some(Some(()))) {
            return false;
        }
        set_local_number(vm, self.counter_slot, counter);
        set_local_number(vm, self.accumulator_slot, accumulator);
        set_local_number(vm, self.block_result_slot, accumulator);
        set_local_number(vm, self.loop_result_slot, accumulator);
        vm.ip = exit + 1;
        true
    }
}

#[derive(Clone, Debug)]
enum AbstractValue {
    Local(usize),
    Number(Register),
    Key(Register),
    Other,
}

struct Translator<'a> {
    bytecode: &'a Bytecode,
    stack: Vec<AbstractValue>,
    locals: BTreeMap<usize, AbstractValue>,
    operations: Vec<NumberInstruction>,
    writes: Vec<LocalWrite>,
    written_slots: BTreeSet<usize>,
    number_slots: BTreeSet<usize>,
    receiver_slot: Option<usize>,
    store: Option<DenseStore>,
}

impl<'a> Translator<'a> {
    fn new(bytecode: &'a Bytecode) -> Self {
        Self {
            bytecode,
            stack: Vec::new(),
            locals: BTreeMap::new(),
            operations: Vec::new(),
            writes: Vec::new(),
            written_slots: BTreeSet::new(),
            number_slots: BTreeSet::new(),
            receiver_slot: None,
            store: None,
        }
    }

    fn emit(&mut self, operation: NumberInstruction) -> Option<Register> {
        if self.operations.len() >= MAX_DENSE_OPS {
            return None;
        }
        let register = self.operations.len();
        self.operations.push(operation);
        Some(register)
    }

    fn pop(&mut self) -> Option<AbstractValue> {
        self.stack.pop()
    }

    fn number(&mut self, value: AbstractValue) -> Option<Register> {
        match value {
            AbstractValue::Number(register) | AbstractValue::Key(register) => Some(register),
            AbstractValue::Local(slot) => {
                self.number_slots.insert(slot);
                self.emit(NumberInstruction::LoadLocal(slot))
            }
            AbstractValue::Other => None,
        }
    }

    fn original_local(value: &AbstractValue) -> Option<usize> {
        match value {
            AbstractValue::Local(slot) => Some(*slot),
            _ => None,
        }
    }

    fn translate(&mut self, op: &Op) -> Option<()> {
        match op {
            Op::LoadLocal(slot) => self.stack.push(
                self.locals
                    .get(slot)
                    .cloned()
                    .unwrap_or(AbstractValue::Local(*slot)),
            ),
            Op::LoadConst(index) => match self.bytecode.constants.get(*index)? {
                Value::Number(value) => {
                    let register = self.emit(NumberInstruction::Constant(*value))?;
                    self.stack.push(AbstractValue::Number(register));
                }
                _ => self.stack.push(AbstractValue::Other),
            },
            Op::Dup => self.stack.push(self.stack.last()?.clone()),
            Op::Pop => {
                self.pop()?;
            }
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                let mut value = self.pop()?;
                self.written_slots.insert(*slot);
                let register = match &value {
                    AbstractValue::Number(register) | AbstractValue::Key(register) => {
                        Some(*register)
                    }
                    AbstractValue::Local(_)
                        if !self.bytecode.local_is_compiler_temporary(*slot) =>
                    {
                        let register = self.number(value.clone())?;
                        value = AbstractValue::Number(register);
                        Some(register)
                    }
                    AbstractValue::Other if !self.bytecode.local_is_compiler_temporary(*slot) => {
                        return None;
                    }
                    AbstractValue::Local(_) | AbstractValue::Other => None,
                };
                if let Some(register) = register {
                    if self.writes.len() >= MAX_DENSE_WRITES {
                        return None;
                    }
                    self.writes.push(LocalWrite {
                        local: *slot,
                        value: register,
                    });
                }
                self.locals.insert(*slot, value);
            }
            Op::RequireObjectCoercible => {
                if matches!(self.stack.last(), Some(AbstractValue::Other) | None) {
                    return None;
                }
            }
            Op::ToPropertyKeyForAccess => {
                let value = self.pop()?;
                let register = self.number(value)?;
                self.stack.push(AbstractValue::Key(register));
            }
            Op::Binary(operation) if supported_binary(*operation) => {
                let right = self.pop()?;
                let left = self.pop()?;
                let right = self.number(right)?;
                let left = self.number(left)?;
                let register = self.emit(NumberInstruction::Binary {
                    operation: *operation,
                    left,
                    right,
                })?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::Unary(operation)
                if matches!(
                    operation,
                    UnaryOp::Plus | UnaryOp::Minus | UnaryOp::BitwiseNot
                ) =>
            {
                let value = self.pop()?;
                let value = self.number(value)?;
                let register = self.emit(NumberInstruction::Unary {
                    operation: *operation,
                    value,
                })?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::GetProp => {
                let key = self.pop()?;
                let object = self.pop()?;
                let AbstractValue::Key(index) = key else {
                    return None;
                };
                let receiver = Self::original_local(&object)?;
                if self.receiver_slot.is_some_and(|slot| slot != receiver) {
                    return None;
                }
                self.receiver_slot = Some(receiver);
                let register = self.emit(NumberInstruction::DenseLoad { index })?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::SetProp { .. } => {
                if self.store.is_some() {
                    return None;
                }
                let value = self.pop()?;
                let key = self.pop()?;
                let object = self.pop()?;
                let value = self.number(value)?;
                let AbstractValue::Key(index) = key else {
                    return None;
                };
                let receiver = Self::original_local(&object)?;
                if self.receiver_slot != Some(receiver) {
                    return None;
                }
                self.store = Some(DenseStore { index, value });
                self.stack.push(AbstractValue::Number(value));
            }
            _ => return None,
        }
        Some(())
    }
}

fn compile_dynamic(
    bytecode: &Bytecode,
    header: usize,
    backedge: usize,
) -> Option<(usize, DynamicDensePlan)> {
    let code = &bytecode.code;
    let (
        Op::LoadLocal(counter_slot),
        Op::LoadLocal(limit_slot),
        Op::Binary(BinaryOp::Lt),
        Op::JumpIfFalse(exit),
        Op::Pop,
    ) = (
        code.get(header)?,
        code.get(header + 1)?,
        code.get(header + 2)?,
        code.get(header + 3)?,
        code.get(header + 4)?,
    )
    else {
        return None;
    };
    if *exit <= backedge
        || !matches!(code.get(*exit), Some(Op::Pop))
        || !matches!(code.get(backedge), Some(Op::Jump(target)) if *target == header)
    {
        return None;
    }

    let mut translator = Translator::new(bytecode);
    for op in &code[header + 5..backedge] {
        translator.translate(op)?;
    }
    if !translator.stack.is_empty() || translator.store.is_none() {
        return None;
    }
    let receiver_slot = translator.receiver_slot?;
    if counter_slot == limit_slot
        || counter_slot == &receiver_slot
        || limit_slot == &receiver_slot
        || translator.number_slots.contains(&receiver_slot)
        || translator.written_slots.contains(&receiver_slot)
        || translator.written_slots.contains(limit_slot)
        || !translator.written_slots.contains(counter_slot)
        || translator
            .operations
            .iter()
            .filter(|operation| matches!(operation, NumberInstruction::DenseLoad { .. }))
            .count()
            != 1
    {
        return None;
    }
    translator.number_slots.insert(*counter_slot);
    translator.number_slots.insert(*limit_slot);
    let mut local_slots = translator.number_slots;
    local_slots.extend(translator.writes.iter().map(|write| write.local));
    let local_slots: Vec<_> = local_slots.into_iter().collect();
    if local_slots.len() > MAX_DENSE_LOCALS {
        return None;
    }
    let local_index = |slot| local_slots.binary_search(&slot).ok();
    for operation in &mut translator.operations {
        if let NumberInstruction::LoadLocal(slot) = operation {
            *slot = local_index(*slot)?;
        }
    }
    for write in &mut translator.writes {
        write.local = local_index(write.local)?;
    }

    Some((
        *exit,
        DynamicDensePlan {
            counter_local: local_index(*counter_slot)?,
            limit_local: local_index(*limit_slot)?,
            receiver_slot,
            local_slots,
            operations: translator.operations,
            writes: translator.writes,
            store: translator.store?,
            header,
        },
    ))
}

fn supported_binary(operation: BinaryOp) -> bool {
    matches!(
        operation,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseXor
            | BinaryOp::BitwiseOr
    )
}

impl DynamicDensePlan {
    fn try_run(&self, vm: &mut Vm<'_>, exit: usize) -> bool {
        if vm.direct_eval_with_stack {
            return false;
        }
        for slot in self
            .local_slots
            .iter()
            .copied()
            .chain(std::iter::once(self.receiver_slot))
        {
            if !vm.slot_is_authoritative(slot) {
                return false;
            }
        }
        let mut locals = [0.0; MAX_DENSE_LOCALS];
        for (local, slot) in self.local_slots.iter().enumerate() {
            let Some(value) = local_number(vm, *slot) else {
                return false;
            };
            locals[local] = value;
        }
        let Some(Some(Value::Array(array))) = vm.locals.get(self.receiver_slot) else {
            return false;
        };
        let array = array.clone();
        let mut deoptimized = false;
        let mut registers = [0.0; MAX_DENSE_OPS];
        let ran = array.with_dense_writable_elements(|elements| {
            loop {
                let counter = locals[self.counter_local];
                let limit = locals[self.limit_local];
                if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
                    return;
                }
                let mut loaded_index = None;
                for (register, operation) in self.operations.iter().enumerate() {
                    let value = match *operation {
                        NumberInstruction::Constant(value) => value,
                        NumberInstruction::LoadLocal(local) => locals[local],
                        NumberInstruction::DenseLoad { index } => {
                            let Some(index) = array_index_from_number(registers[index]) else {
                                deoptimized = true;
                                return;
                            };
                            let Some(Value::Number(value)) = elements.get(index) else {
                                deoptimized = true;
                                return;
                            };
                            loaded_index = Some(index);
                            *value
                        }
                        NumberInstruction::Binary {
                            operation,
                            left,
                            right,
                        } => apply_binary(operation, registers[left], registers[right])
                            .expect("translator only admits Number binary operations"),
                        NumberInstruction::Unary { operation, value } => {
                            apply_unary(operation, registers[value])
                                .expect("translator only admits Number unary operations")
                        }
                    };
                    registers[register] = value;
                }
                let Some(index) = array_index_from_number(registers[self.store.index]) else {
                    deoptimized = true;
                    return;
                };
                if loaded_index != Some(index) || index >= elements.len() {
                    deoptimized = true;
                    return;
                }
                elements[index] = Value::Number(registers[self.store.value]);
                record_iteration();
                for write in &self.writes {
                    locals[write.local] = registers[write.value];
                }
            }
        });
        if ran.is_none() {
            return false;
        }
        for (slot, value) in self.local_slots.iter().copied().zip(locals) {
            set_local_number(vm, slot, value);
        }
        vm.ip = if deoptimized { self.header } else { exit + 1 };
        true
    }
}

fn apply_binary(operation: BinaryOp, left: f64, right: f64) -> Option<f64> {
    Some(match operation {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Div => left / right,
        BinaryOp::Rem => left % right,
        BinaryOp::Shl => f64::from(to_int32_number(left) << (to_uint32_number(right) & 0x1f)),
        BinaryOp::Shr => f64::from(to_int32_number(left) >> (to_uint32_number(right) & 0x1f)),
        BinaryOp::UShr => f64::from(to_uint32_number(left) >> (to_uint32_number(right) & 0x1f)),
        BinaryOp::BitwiseAnd => f64::from(to_int32_number(left) & to_int32_number(right)),
        BinaryOp::BitwiseXor => f64::from(to_int32_number(left) ^ to_int32_number(right)),
        BinaryOp::BitwiseOr => f64::from(to_int32_number(left) | to_int32_number(right)),
        _ => return None,
    })
}

fn apply_unary(operation: UnaryOp, value: f64) -> Option<f64> {
    Some(match operation {
        UnaryOp::Plus => value,
        UnaryOp::Minus => -value,
        UnaryOp::BitwiseNot => f64::from(!to_int32_number(value)),
        _ => return None,
    })
}

fn local_number(vm: &Vm<'_>, slot: usize) -> Option<f64> {
    match vm.local_slot_value(slot)? {
        Value::Number(value) => Some(value),
        _ => None,
    }
}

fn set_local_number(vm: &mut Vm<'_>, slot: usize, value: f64) {
    vm.locals[slot] = Some(Value::Number(value));
}
