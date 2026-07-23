//! Typed dense-array mutation plans compiled from immutable bytecode.
//!
//! Fixed-index recurrences scalar-replace a small set of Number elements.
//! Computed-index loops translate one straight-line body into a bounded Number
//! register program spanning several dense-array receivers. Writable regions
//! require distinct receivers and stage every store; pure-read reductions use
//! shared immutable leases and safely accept receiver aliases. A failed guard
//! can publish only completed scalar iterations before replaying the current
//! iteration at the header.

use std::{
    cell::{Ref, RefMut},
    collections::{BTreeMap, BTreeSet},
};

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use crate::{Value, to_int32_number, to_uint32_number, value::ArrayRef};

use super::super::{
    ir::{Bytecode, Op, decode_index_receiver},
    vm::Vm,
    vm_props::array_index_from_number,
};

const INLINE_DENSE_OPS: usize = 64;
const MAX_DENSE_OPS: usize = 256;
const MAX_DENSE_LOCALS: usize = 64;
const MAX_DENSE_WRITES: usize = 64;
const MAX_DENSE_RECEIVERS: usize = 8;
const MAX_DENSE_STORES: usize = 32;
const MAX_FIXED_MUTATIONS: usize = 16;

#[cfg(test)]
thread_local! {
    static DENSE_LOOP_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static SINGLE_DENSE_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static SUNK_DENSE_STORE_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static READ_ONLY_DENSE_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static READ_ONLY_DENSE_BAILOUTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static READ_ONLY_DENSE_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_test_iterations() {
    DENSE_LOOP_ITERATIONS.set(0);
    SINGLE_DENSE_PATH_HITS.set(0);
    SUNK_DENSE_STORE_HITS.set(0);
    READ_ONLY_DENSE_PATH_HITS.set(0);
    READ_ONLY_DENSE_BAILOUTS.set(0);
    READ_ONLY_DENSE_ITERATIONS.set(0);
}

#[cfg(test)]
pub(super) fn test_iterations() -> usize {
    DENSE_LOOP_ITERATIONS.get()
}

#[cfg(test)]
pub(super) fn test_single_path_hits() -> usize {
    SINGLE_DENSE_PATH_HITS.get()
}

#[cfg(test)]
pub(super) fn test_sunk_store_hits() -> usize {
    SUNK_DENSE_STORE_HITS.get()
}

#[cfg(test)]
pub(super) fn test_read_only_path_hits() -> usize {
    READ_ONLY_DENSE_PATH_HITS.get()
}

#[cfg(test)]
pub(super) fn test_read_only_bailouts() -> usize {
    READ_ONLY_DENSE_BAILOUTS.get()
}

#[cfg(test)]
pub(super) fn test_read_only_iterations() -> usize {
    READ_ONLY_DENSE_ITERATIONS.get()
}

#[inline]
fn record_iteration() {
    #[cfg(test)]
    DENSE_LOOP_ITERATIONS.set(DENSE_LOOP_ITERATIONS.get() + 1);
}

#[inline]
fn record_single_path_hit() {
    #[cfg(test)]
    SINGLE_DENSE_PATH_HITS.set(SINGLE_DENSE_PATH_HITS.get() + 1);
}

#[inline]
fn record_sunk_store_hit() {
    #[cfg(test)]
    SUNK_DENSE_STORE_HITS.set(SUNK_DENSE_STORE_HITS.get() + 1);
}

#[inline]
fn record_read_only_path_hit() {
    #[cfg(test)]
    READ_ONLY_DENSE_PATH_HITS.set(READ_ONLY_DENSE_PATH_HITS.get() + 1);
}

#[inline]
fn record_read_only_bailout() {
    #[cfg(test)]
    READ_ONLY_DENSE_BAILOUTS.set(READ_ONLY_DENSE_BAILOUTS.get() + 1);
}

#[cfg(test)]
#[inline]
fn record_read_only_iteration() {
    READ_ONLY_DENSE_ITERATIONS.set(READ_ONLY_DENSE_ITERATIONS.get() + 1);
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
    receiver_slots: Vec<usize>,
    local_slots: Vec<usize>,
    operations: Vec<NumberInstruction>,
    writes: Vec<LocalWrite>,
    store_count: usize,
    sunk_store: Option<SunkDenseStore>,
    header: usize,
}

type Register = usize;

#[derive(Clone, Debug)]
enum NumberInstruction {
    Constant(f64),
    LoadLocal(usize),
    DenseLoad {
        receiver: usize,
        index: Register,
    },
    DenseStore {
        receiver: usize,
        index: Register,
        value: Register,
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
    Update {
        operation: UpdateOp,
        value: Register,
    },
}

#[derive(Clone, Copy, Debug)]
struct LocalWrite {
    local: usize,
    value: Register,
}

#[derive(Clone, Copy, Debug)]
struct PendingDenseStore {
    receiver: usize,
    index: usize,
    value: f64,
}

#[derive(Clone, Copy, Debug)]
struct SunkDenseStore {
    receiver: usize,
    index: Register,
    value: Register,
}

trait DenseAccess {
    fn reset_iteration(&mut self);
    fn load_number(&self, receiver: usize, index: usize) -> Option<f64>;
    fn stage_store(&mut self, receiver: usize, index: usize, value: f64) -> bool;
    fn staged_store_count(&self) -> usize;
    fn commit_stores(&mut self);
}

struct SingleAccess<'a> {
    elements: &'a mut [Value],
    pending: Option<PendingDenseStore>,
}

impl DenseAccess for SingleAccess<'_> {
    fn reset_iteration(&mut self) {
        self.pending = None;
    }

    fn load_number(&self, receiver: usize, index: usize) -> Option<f64> {
        if receiver != 0 {
            return None;
        }
        if let Some(store) = self
            .pending
            .filter(|store| store.receiver == receiver && store.index == index)
        {
            return Some(store.value);
        }
        match self.elements.get(index)? {
            Value::Number(value) => Some(*value),
            _ => None,
        }
    }

    fn stage_store(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        if receiver != 0 || self.pending.is_some() || index >= self.elements.len() {
            return false;
        }
        self.pending = Some(PendingDenseStore {
            receiver,
            index,
            value,
        });
        true
    }

    fn staged_store_count(&self) -> usize {
        usize::from(self.pending.is_some())
    }

    fn commit_stores(&mut self) {
        let store = self
            .pending
            .take()
            .expect("single-store plan stages exactly one validated write");
        self.elements[store.index] = Value::Number(store.value);
    }
}

struct MultiAccess<'a, 'elements> {
    elements: &'a mut [RefMut<'elements, Vec<Value>>],
    pending: Vec<PendingDenseStore>,
}

struct ReadAccess<'a, 'elements> {
    elements: &'a [Ref<'elements, Vec<Value>>],
}

impl DenseAccess for ReadAccess<'_, '_> {
    #[inline]
    fn reset_iteration(&mut self) {}

    #[inline]
    fn load_number(&self, receiver: usize, index: usize) -> Option<f64> {
        match self.elements.get(receiver)?.get(index)? {
            Value::Number(value) => Some(*value),
            _ => None,
        }
    }

    #[inline]
    fn stage_store(&mut self, _receiver: usize, _index: usize, _value: f64) -> bool {
        false
    }

    #[inline]
    fn staged_store_count(&self) -> usize {
        0
    }

    #[inline]
    fn commit_stores(&mut self) {}
}

impl DenseAccess for MultiAccess<'_, '_> {
    fn reset_iteration(&mut self) {
        self.pending.clear();
    }

    fn load_number(&self, receiver: usize, index: usize) -> Option<f64> {
        if let Some(store) = self
            .pending
            .iter()
            .rev()
            .find(|store| store.receiver == receiver && store.index == index)
        {
            return Some(store.value);
        }
        match self.elements.get(receiver)?.get(index)? {
            Value::Number(value) => Some(*value),
            _ => None,
        }
    }

    fn stage_store(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        if self
            .elements
            .get(receiver)
            .is_none_or(|elements| index >= elements.len())
        {
            return false;
        }
        self.pending.push(PendingDenseStore {
            receiver,
            index,
            value,
        });
        true
    }

    fn staged_store_count(&self) -> usize {
        self.pending.len()
    }

    fn commit_stores(&mut self) {
        for store in &self.pending {
            self.elements[store.receiver][store.index] = Value::Number(store.value);
        }
    }
}

#[derive(Clone, Copy)]
struct DynamicProgramRun {
    deoptimized: bool,
    made_progress: bool,
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
    receiver_slots: Vec<usize>,
    store_count: usize,
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
            receiver_slots: Vec::new(),
            store_count: 0,
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

    fn receiver(&mut self, value: &AbstractValue) -> Option<usize> {
        let slot = Self::original_local(value)?;
        if let Some(receiver) = self
            .receiver_slots
            .iter()
            .position(|existing| *existing == slot)
        {
            return Some(receiver);
        }
        if self.receiver_slots.len() >= MAX_DENSE_RECEIVERS {
            return None;
        }
        let receiver = self.receiver_slots.len();
        self.receiver_slots.push(slot);
        Some(receiver)
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
            Op::ToNumeric => {
                let value = self.pop()?;
                let register = self.number(value)?;
                self.stack.push(AbstractValue::Number(register));
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
            Op::Update(operation) => {
                let value = self.pop()?;
                let value = self.number(value)?;
                let register = self.emit(NumberInstruction::Update {
                    operation: *operation,
                    value,
                })?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::GetProp => {
                let key = self.pop()?;
                let object = self.pop()?;
                let index = self.number(key)?;
                let receiver = self.receiver(&object)?;
                let register = self.emit(NumberInstruction::DenseLoad { receiver, index })?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::SetProp { .. } => {
                if self.store_count >= MAX_DENSE_STORES {
                    return None;
                }
                let value = self.pop()?;
                let key = self.pop()?;
                let object = self.pop()?;
                let value = self.number(value)?;
                let index = self.number(key)?;
                let receiver = self.receiver(&object)?;
                self.emit(NumberInstruction::DenseStore {
                    receiver,
                    index,
                    value,
                })?;
                self.store_count += 1;
                self.stack.push(AbstractValue::Number(value));
            }
            _ => return None,
        }
        Some(())
    }
}

fn instruction_uses_register(operation: &NumberInstruction, target: Register) -> bool {
    match operation {
        NumberInstruction::Constant(_) | NumberInstruction::LoadLocal(_) => false,
        NumberInstruction::DenseLoad { index, .. } => *index == target,
        NumberInstruction::DenseStore { index, value, .. }
        | NumberInstruction::Binary {
            left: index,
            right: value,
            ..
        } => *index == target || *value == target,
        NumberInstruction::Unary { value, .. } | NumberInstruction::Update { value, .. } => {
            *value == target
        }
    }
}

fn remap_removed_register(register: &mut Register, removed: Register) -> bool {
    if *register == removed {
        return false;
    }
    if *register > removed {
        *register -= 1;
    }
    true
}

fn remap_instruction_registers(operation: &mut NumberInstruction, removed: Register) -> bool {
    match operation {
        NumberInstruction::Constant(_) | NumberInstruction::LoadLocal(_) => true,
        NumberInstruction::DenseLoad { index, .. } => remap_removed_register(index, removed),
        NumberInstruction::DenseStore { index, value, .. }
        | NumberInstruction::Binary {
            left: index,
            right: value,
            ..
        } => remap_removed_register(index, removed) && remap_removed_register(value, removed),
        NumberInstruction::Unary { value, .. } | NumberInstruction::Update { value, .. } => {
            remap_removed_register(value, removed)
        }
    }
}

fn sink_unique_store(
    operations: &mut Vec<NumberInstruction>,
    writes: &mut [LocalWrite],
    store_count: usize,
) -> Option<Option<SunkDenseStore>> {
    if store_count != 1 {
        return Some(None);
    }
    let (position, mut store) =
        operations
            .iter()
            .enumerate()
            .find_map(|(position, operation)| match operation {
                NumberInstruction::DenseStore {
                    receiver,
                    index,
                    value,
                } => Some((
                    position,
                    SunkDenseStore {
                        receiver: *receiver,
                        index: *index,
                        value: *value,
                    },
                )),
                _ => None,
            })?;
    if operations[position + 1..].iter().any(|operation| {
        matches!(
            operation,
            NumberInstruction::DenseLoad { .. } | NumberInstruction::DenseStore { .. }
        )
    }) {
        return Some(None);
    }

    let store_result_is_used = operations.iter().enumerate().any(|(index, operation)| {
        index != position && instruction_uses_register(operation, position)
    }) || writes.iter().any(|write| write.value == position);
    debug_assert!(
        !store_result_is_used,
        "SetProp keeps its input value as the expression result"
    );
    if store_result_is_used {
        return None;
    }

    operations.remove(position);
    if !operations
        .iter_mut()
        .all(|operation| remap_instruction_registers(operation, position))
        || !writes
            .iter_mut()
            .all(|write| remap_removed_register(&mut write.value, position))
        || !remap_removed_register(&mut store.index, position)
        || !remap_removed_register(&mut store.value, position)
    {
        return None;
    }
    Some(Some(store))
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
    if !translator.stack.is_empty()
        || !translator
            .operations
            .iter()
            .any(|operation| matches!(operation, NumberInstruction::DenseLoad { .. }))
    {
        return None;
    }
    if counter_slot == limit_slot
        || translator.receiver_slots.iter().any(|receiver| {
            counter_slot == receiver
                || limit_slot == receiver
                || translator.number_slots.contains(receiver)
                || translator.written_slots.contains(receiver)
        })
        || translator.written_slots.contains(limit_slot)
        || !translator.written_slots.contains(counter_slot)
    {
        return None;
    }
    let sunk_store = sink_unique_store(
        &mut translator.operations,
        &mut translator.writes,
        translator.store_count,
    )?;
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
            receiver_slots: translator.receiver_slots,
            local_slots,
            operations: translator.operations,
            writes: translator.writes,
            store_count: translator.store_count,
            sunk_store,
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
    fn run_program<A: DenseAccess>(
        &self,
        access: &mut A,
        locals: &mut [f64; MAX_DENSE_LOCALS],
        registers: &mut [f64],
    ) -> DynamicProgramRun {
        let mut made_progress = false;
        loop {
            let counter = locals[self.counter_local];
            let limit = locals[self.limit_local];
            if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
                return DynamicProgramRun {
                    deoptimized: false,
                    made_progress,
                };
            }
            access.reset_iteration();
            for (register, operation) in self.operations.iter().enumerate() {
                let value = match *operation {
                    NumberInstruction::Constant(value) => value,
                    NumberInstruction::LoadLocal(local) => locals[local],
                    NumberInstruction::DenseLoad { receiver, index } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        let Some(value) = access.load_number(receiver, index) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        value
                    }
                    NumberInstruction::DenseStore {
                        receiver,
                        index,
                        value,
                    } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        };
                        let value = registers[value];
                        if !access.stage_store(receiver, index, value) {
                            return DynamicProgramRun {
                                deoptimized: true,
                                made_progress,
                            };
                        }
                        value
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
                    NumberInstruction::Update { operation, value } => match operation {
                        UpdateOp::Increment => registers[value] + 1.0,
                        UpdateOp::Decrement => registers[value] - 1.0,
                    },
                };
                registers[register] = value;
            }
            if let Some(store) = self.sunk_store {
                let Some(index) = array_index_from_number(registers[store.index]) else {
                    return DynamicProgramRun {
                        deoptimized: true,
                        made_progress,
                    };
                };
                if !access.stage_store(store.receiver, index, registers[store.value]) {
                    return DynamicProgramRun {
                        deoptimized: true,
                        made_progress,
                    };
                }
            }
            debug_assert_eq!(access.staged_store_count(), self.store_count);
            access.commit_stores();
            for write in &self.writes {
                locals[write.local] = registers[write.value];
            }
            #[cfg(test)]
            if self.store_count == 0 {
                record_read_only_iteration();
            }
            made_progress = true;
            record_iteration();
        }
    }

    fn try_run(&self, vm: &mut Vm<'_>, exit: usize) -> bool {
        if vm.direct_eval_with_stack {
            return false;
        }
        for slot in self
            .local_slots
            .iter()
            .copied()
            .chain(self.receiver_slots.iter().copied())
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
        let mut inline_registers = [0.0; INLINE_DENSE_OPS];
        let mut large_registers =
            (self.operations.len() > INLINE_DENSE_OPS).then(|| vec![0.0; self.operations.len()]);
        let registers = match large_registers.as_mut() {
            Some(registers) => registers.as_mut_slice(),
            None => &mut inline_registers[..self.operations.len()],
        };

        let ran = if self.store_count == 0 {
            let Some(arrays) = self
                .receiver_slots
                .iter()
                .map(|slot| match vm.locals.get(*slot) {
                    Some(Some(Value::Array(array))) => Some(array.clone()),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
            else {
                record_read_only_bailout();
                return false;
            };
            let ran = ArrayRef::with_dense_readable_element_sets(&arrays, |elements| {
                let mut access = ReadAccess { elements };
                self.run_program(&mut access, &mut locals, registers)
            });
            match ran {
                Some(run) => {
                    if run.made_progress {
                        record_read_only_path_hit();
                    }
                    if run.deoptimized {
                        record_read_only_bailout();
                    }
                }
                None => record_read_only_bailout(),
            }
            ran
        } else if self.receiver_slots.len() == 1 && self.store_count == 1 {
            let Some(Some(Value::Array(array))) = vm.locals.get(self.receiver_slots[0]) else {
                return false;
            };
            let array = array.clone();
            let ran = array.with_dense_writable_elements(|elements| {
                let mut access = SingleAccess {
                    elements,
                    pending: None,
                };
                self.run_program(&mut access, &mut locals, registers)
            });
            if ran.is_some_and(|run| run.made_progress) {
                record_single_path_hit();
            }
            ran
        } else {
            let Some(arrays) = self
                .receiver_slots
                .iter()
                .map(|slot| match vm.locals.get(*slot) {
                    Some(Some(Value::Array(array))) => Some(array.clone()),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
            else {
                return false;
            };
            ArrayRef::with_distinct_dense_writable_elements(&arrays, |elements| {
                let mut access = MultiAccess {
                    elements,
                    pending: Vec::with_capacity(self.store_count),
                };
                self.run_program(&mut access, &mut locals, registers)
            })
        };
        let Some(run) = ran else {
            return false;
        };
        if self.sunk_store.is_some() && run.made_progress {
            record_sunk_store_hit();
        }
        if run.deoptimized && !run.made_progress {
            return false;
        }
        for (slot, value) in self.local_slots.iter().copied().zip(locals) {
            set_local_number(vm, slot, value);
        }
        vm.ip = if run.deoptimized {
            self.header
        } else {
            exit + 1
        };
        true
    }
}

#[inline(always)]
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
