//! Batched false-prefix scans for pure dense numeric predicates.
//!
//! The ordinary VM still runs every true body. This plan only collapses a
//! consecutive run of iterations whose leading predicate is false and whose
//! false edge contains no observable work beyond completion bookkeeping and
//! the counted-loop update. Runtime guards require authoritative Number locals
//! and a present-own dense Number load before the read-only array lease begins.

use std::collections::BTreeSet;

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use crate::{Value, to_int32_number, to_uint32_number};

use super::super::{
    ir::{Bytecode, Op},
    vm::Vm,
    vm_props::array_index_from_number,
};

const MAX_SCAN_LOCALS: usize = 32;
const MAX_SCAN_OPS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PredicateScanRun {
    Handled,
    Suppress,
}

#[cfg(test)]
thread_local! {
    static PLAN_ATTEMPTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static FALSE_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static BODY_HANDOFFS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static MID_SCAN_DEOPTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn reset_test_counters() {
    PLAN_ATTEMPTS.set(0);
    FALSE_ITERATIONS.set(0);
    BODY_HANDOFFS.set(0);
    MID_SCAN_DEOPTS.set(0);
}

#[inline]
fn record_false_iteration() {
    #[cfg(test)]
    FALSE_ITERATIONS.set(FALSE_ITERATIONS.get() + 1);
}

#[inline]
fn record_false_iterations(iterations: usize) {
    #[cfg(test)]
    FALSE_ITERATIONS.set(FALSE_ITERATIONS.get() + iterations);
    #[cfg(not(test))]
    let _ = iterations;
}

#[inline]
fn record_body_handoff() {
    #[cfg(test)]
    BODY_HANDOFFS.set(BODY_HANDOFFS.get() + 1);
}

#[inline]
fn record_mid_scan_deopt() {
    #[cfg(test)]
    MID_SCAN_DEOPTS.set(MID_SCAN_DEOPTS.get() + 1);
}

type Register = usize;

#[derive(Clone, Copy, Debug)]
enum ScanInstruction {
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
enum AbstractValue {
    Local(usize),
    Number(Register),
    Boolean(Register),
    Key(Register),
}

#[derive(Clone, Copy, Debug)]
enum ScanValue {
    Number(f64),
    Boolean(bool),
}

impl ScanValue {
    fn is_truthy(self) -> bool {
        match self {
            Self::Boolean(value) => value,
            Self::Number(value) => value != 0.0 && !value.is_nan(),
        }
    }
}

struct PredicateTranslator<'a> {
    bytecode: &'a Bytecode,
    stack: Vec<AbstractValue>,
    operations: Vec<ScanInstruction>,
    number_slots: BTreeSet<usize>,
    receiver_slot: Option<usize>,
    dense_loads: usize,
}

impl<'a> PredicateTranslator<'a> {
    fn new(bytecode: &'a Bytecode) -> Self {
        Self {
            bytecode,
            stack: Vec::new(),
            operations: Vec::new(),
            number_slots: BTreeSet::new(),
            receiver_slot: None,
            dense_loads: 0,
        }
    }

    fn emit(&mut self, instruction: ScanInstruction) -> Option<Register> {
        if self.operations.len() >= MAX_SCAN_OPS {
            return None;
        }
        let register = self.operations.len();
        self.operations.push(instruction);
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
                self.emit(ScanInstruction::LoadLocal(slot))
            }
            AbstractValue::Boolean(_) => None,
        }
    }

    fn value(&mut self, value: AbstractValue) -> Option<Register> {
        match value {
            AbstractValue::Number(register)
            | AbstractValue::Boolean(register)
            | AbstractValue::Key(register) => Some(register),
            AbstractValue::Local(slot) => self.number(AbstractValue::Local(slot)),
        }
    }

    fn translate(&mut self, op: &Op) -> Option<()> {
        match op {
            Op::LoadLocal(slot) => self.stack.push(AbstractValue::Local(*slot)),
            Op::LoadConst(index) => {
                let Value::Number(value) = self.bytecode.constants.get(*index)? else {
                    return None;
                };
                let register = self.emit(ScanInstruction::Constant(*value))?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::Dup => self.stack.push(*self.stack.last()?),
            Op::Pop => {
                self.pop()?;
            }
            Op::RequireObjectCoercible => {
                self.stack.last()?;
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
                let register = self.emit(ScanInstruction::Binary {
                    operation: *operation,
                    left,
                    right,
                })?;
                if binary_returns_boolean(*operation) {
                    self.stack.push(AbstractValue::Boolean(register));
                } else {
                    self.stack.push(AbstractValue::Number(register));
                }
            }
            Op::Unary(operation) if supported_unary(*operation) => {
                let value = self.pop()?;
                let value = match operation {
                    UnaryOp::Not => self.value(value)?,
                    UnaryOp::Plus | UnaryOp::Minus | UnaryOp::BitwiseNot => self.number(value)?,
                    _ => return None,
                };
                let register = self.emit(ScanInstruction::Unary {
                    operation: *operation,
                    value,
                })?;
                if operation == &UnaryOp::Not {
                    self.stack.push(AbstractValue::Boolean(register));
                } else {
                    self.stack.push(AbstractValue::Number(register));
                }
            }
            Op::GetProp => {
                let key = self.pop()?;
                let object = self.pop()?;
                let index = self.number(key)?;
                let AbstractValue::Local(receiver_slot) = object else {
                    return None;
                };
                if self
                    .receiver_slot
                    .is_some_and(|current| current != receiver_slot)
                    || self.dense_loads != 0
                {
                    return None;
                }
                self.receiver_slot = Some(receiver_slot);
                self.dense_loads += 1;
                let register = self.emit(ScanInstruction::DenseLoad { index })?;
                self.stack.push(AbstractValue::Number(register));
            }
            _ => return None,
        }
        Some(())
    }

    fn finish(mut self) -> Option<TranslatedPredicate> {
        if self.stack.len() != 1 || self.dense_loads != 1 {
            return None;
        }
        let value = self.pop()?;
        let predicate = self.value(value)?;
        let receiver_slot = self.receiver_slot?;
        if self.number_slots.contains(&receiver_slot) {
            return None;
        }
        Some(TranslatedPredicate {
            operations: self.operations,
            number_slots: self.number_slots,
            receiver_slot,
            predicate,
        })
    }
}

struct TranslatedPredicate {
    operations: Vec<ScanInstruction>,
    number_slots: BTreeSet<usize>,
    receiver_slot: usize,
    predicate: Register,
}

#[derive(Clone, Debug)]
enum PredicateKernel {
    Interpreted {
        operations: Vec<ScanInstruction>,
        predicate: Register,
    },
    /// A packed-bitset membership test of the general form
    /// `words[index >> shift] & (bit << (index & mask))`.
    PackedBitset(PackedBitset),
}

#[derive(Clone, Copy, Debug)]
struct PackedBitset {
    word_shift: f64,
    word_operation: BinaryOp,
    bit_mask: f64,
    bit: f64,
}

impl PredicateKernel {
    fn compile(
        operations: Vec<ScanInstruction>,
        predicate: Register,
        counter_local: usize,
    ) -> Self {
        if let Some(kernel) = compile_packed_bitset(&operations, predicate, counter_local) {
            return kernel;
        }
        Self::Interpreted {
            operations,
            predicate,
        }
    }
}

fn compile_packed_bitset(
    operations: &[ScanInstruction],
    predicate: Register,
    counter_local: usize,
) -> Option<PredicateKernel> {
    if operations.len() != 10 {
        return None;
    }
    let ScanInstruction::Binary {
        operation: BinaryOp::BitwiseAnd,
        left,
        right,
    } = *operations.get(predicate)?
    else {
        return None;
    };
    let ((word_local, word_shift, word_operation), (bit_local, bit_mask, bit)) =
        parse_dense_word(operations, left)
            .zip(parse_shifted_bit(operations, right))
            .or_else(|| {
                parse_dense_word(operations, right).zip(parse_shifted_bit(operations, left))
            })?;
    if word_local != counter_local || bit_local != counter_local {
        return None;
    }
    Some(PredicateKernel::PackedBitset(PackedBitset {
        word_shift,
        word_operation,
        bit_mask,
        bit,
    }))
}

fn parse_dense_word(
    operations: &[ScanInstruction],
    register: Register,
) -> Option<(usize, f64, BinaryOp)> {
    let ScanInstruction::DenseLoad { index } = *operations.get(register)? else {
        return None;
    };
    let ScanInstruction::Binary {
        operation: operation @ (BinaryOp::Shr | BinaryOp::UShr),
        left,
        right,
    } = *operations.get(index)?
    else {
        return None;
    };
    let (ScanInstruction::LoadLocal(local), ScanInstruction::Constant(shift)) =
        (*operations.get(left)?, *operations.get(right)?)
    else {
        return None;
    };
    Some((local, shift, operation))
}

fn parse_shifted_bit(
    operations: &[ScanInstruction],
    register: Register,
) -> Option<(usize, f64, f64)> {
    let ScanInstruction::Binary {
        operation: BinaryOp::Shl,
        left,
        right,
    } = *operations.get(register)?
    else {
        return None;
    };
    let ScanInstruction::Constant(bit) = *operations.get(left)? else {
        return None;
    };
    let ScanInstruction::Binary {
        operation: BinaryOp::BitwiseAnd,
        left,
        right,
    } = *operations.get(right)?
    else {
        return None;
    };
    let (ScanInstruction::LoadLocal(local), ScanInstruction::Constant(mask)) =
        (*operations.get(left)?, *operations.get(right)?)
    else {
        return None;
    };
    Some((local, mask, bit))
}

/// A counted loop whose leading dense numeric predicate can skip consecutive
/// false iterations without suppressing any true-body behavior.
#[derive(Clone, Debug)]
pub(super) struct DenseNumericPredicateScanPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    body_entry: usize,
    false_path_start: usize,
    counter_slot: usize,
    limit_slot: usize,
    counter_local: usize,
    limit_local: usize,
    counter_condition: BinaryOp,
    counter_update: UpdateOp,
    receiver_slot: usize,
    false_completion_slot: Option<usize>,
    local_slots: Vec<usize>,
    predicate: PredicateKernel,
}

impl DenseNumericPredicateScanPlan {
    pub(super) fn compile(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<Self> {
        let code = &bytecode.code;
        let (
            Op::LoadLocal(counter_slot),
            Op::LoadLocal(limit_slot),
            Op::Binary(counter_condition),
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
        if !matches!(
            counter_condition,
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge
        ) || counter_slot == limit_slot
            || *exit <= backedge
            || !matches!(code.get(*exit), Some(Op::Pop))
        {
            return None;
        }

        let tail = backedge.checked_sub(6)?;
        let (
            Op::LoadLocal(tail_counter_slot),
            Op::ToNumeric,
            Op::Dup,
            Op::Update(counter_update),
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
        )
        else {
            return None;
        };
        if tail + 6 != backedge
            || tail_counter_slot != counter_slot
            || assigned_counter_slot != counter_slot
            || tail_header != &header
        {
            return None;
        }

        for predicate_jump in header + 5..tail {
            let Some(Op::JumpIfFalse(false_target)) = code.get(predicate_jump) else {
                continue;
            };
            let Some(false_completion_slot) = compile_false_path(bytecode, *false_target, tail)
            else {
                continue;
            };
            if !matches!(code.get(predicate_jump + 1), Some(Op::Pop)) {
                continue;
            }
            let mut translator = PredicateTranslator::new(bytecode);
            if code[header + 5..predicate_jump]
                .iter()
                .any(|op| translator.translate(op).is_none())
            {
                continue;
            }
            let Some(mut translated) = translator.finish() else {
                continue;
            };
            translated.number_slots.insert(*counter_slot);
            translated.number_slots.insert(*limit_slot);
            if false_completion_slot.is_some_and(|slot| {
                translated.number_slots.contains(&slot)
                    || slot == translated.receiver_slot
                    || !bytecode.local_is_compiler_temporary(slot)
            }) {
                continue;
            }
            let local_slots: Vec<_> = translated.number_slots.into_iter().collect();
            if local_slots.len() > MAX_SCAN_LOCALS {
                continue;
            }
            let local_index = |slot| local_slots.binary_search(&slot).ok();
            for operation in &mut translated.operations {
                if let ScanInstruction::LoadLocal(slot) = operation {
                    *slot = local_index(*slot)?;
                }
            }
            let predicate = PredicateKernel::compile(
                translated.operations,
                translated.predicate,
                local_index(*counter_slot)?,
            );
            return Some(Self {
                header,
                backedge,
                exit: *exit,
                body_entry: predicate_jump + 2,
                false_path_start: *false_target,
                counter_slot: *counter_slot,
                limit_slot: *limit_slot,
                counter_local: local_index(*counter_slot)?,
                limit_local: local_index(*limit_slot)?,
                counter_condition: *counter_condition,
                counter_update: *counter_update,
                receiver_slot: translated.receiver_slot,
                false_completion_slot,
                local_slots,
                predicate,
            });
        }
        None
    }

    pub(super) fn exit(&self) -> usize {
        self.exit
    }

    /// Instructions interpreted by the scalar VM on a true predicate remain
    /// eligible for virtual-object lowering. Only the header/predicate prefix
    /// and the false epilogue that this plan may bypass need to stay in their
    /// original bytecode form.
    pub(super) fn contains_instruction(&self, ip: usize) -> bool {
        (self.header..self.body_entry).contains(&ip)
            || (self.false_path_start..=self.backedge).contains(&ip)
    }

    pub(super) fn try_run(&self, vm: &mut Vm<'_>) -> PredicateScanRun {
        #[cfg(test)]
        PLAN_ATTEMPTS.set(PLAN_ATTEMPTS.get() + 1);
        if vm.direct_eval_with_stack
            || self
                .local_slots
                .iter()
                .copied()
                .chain(std::iter::once(self.receiver_slot))
                .chain(self.false_completion_slot)
                .any(|slot| !vm.slot_is_authoritative(slot))
        {
            return PredicateScanRun::Suppress;
        }

        let Some(Some(Value::Array(array))) = vm.locals.get(self.receiver_slot) else {
            return PredicateScanRun::Suppress;
        };
        let array = array.clone();
        let (outcome, skipped, completed_counter) = match &self.predicate {
            PredicateKernel::Interpreted {
                operations,
                predicate,
            } => {
                let mut locals = [0.0; MAX_SCAN_LOCALS];
                for (local, slot) in self.local_slots.iter().enumerate() {
                    let Some(value) = local_number(vm, *slot) else {
                        return PredicateScanRun::Suppress;
                    };
                    locals[local] = value;
                }
                let Some((outcome, skipped)) = array.with_dense_readable_elements(|elements| {
                    self.scan_interpreted(&mut locals, elements, operations, *predicate)
                }) else {
                    return PredicateScanRun::Suppress;
                };
                (outcome, skipped, locals[self.counter_local])
            }
            PredicateKernel::PackedBitset(packed) => {
                let (Some(mut counter), Some(limit)) = (
                    local_number(vm, self.counter_slot),
                    local_number(vm, self.limit_slot),
                ) else {
                    return PredicateScanRun::Suppress;
                };
                let Some((outcome, skipped)) = array.with_dense_readable_elements(|elements| {
                    self.scan_packed_bitset(&mut counter, limit, elements, packed)
                }) else {
                    return PredicateScanRun::Suppress;
                };
                (outcome, skipped, counter)
            }
        };

        if skipped != 0 {
            set_local_number(vm, self.counter_slot, completed_counter);
            if let Some(slot) = self.false_completion_slot {
                vm.locals[slot] = Some(Value::Undefined);
            }
        }
        match outcome {
            ScanOutcome::Body => {
                record_body_handoff();
                vm.ip = self.body_entry;
                PredicateScanRun::Handled
            }
            ScanOutcome::Exit => {
                vm.ip = self.exit + 1;
                PredicateScanRun::Handled
            }
            ScanOutcome::Deopt if skipped != 0 => {
                record_mid_scan_deopt();
                vm.ip = self.header;
                PredicateScanRun::Handled
            }
            ScanOutcome::Deopt => PredicateScanRun::Suppress,
        }
    }

    fn scan_interpreted(
        &self,
        locals: &mut [f64; MAX_SCAN_LOCALS],
        elements: &[Value],
        operations: &[ScanInstruction],
        predicate: Register,
    ) -> (ScanOutcome, usize) {
        let mut registers = [ScanValue::Number(0.0); MAX_SCAN_OPS];
        let mut skipped = 0;
        loop {
            let counter = locals[self.counter_local];
            let limit = locals[self.limit_local];
            if !apply_counter_condition(self.counter_condition, counter, limit) {
                return (ScanOutcome::Exit, skipped);
            }
            for (register, instruction) in operations.iter().enumerate() {
                let Some(value) = apply_instruction(*instruction, locals, &registers, elements)
                else {
                    return (ScanOutcome::Deopt, skipped);
                };
                registers[register] = value;
            }
            if registers[predicate].is_truthy() {
                return (ScanOutcome::Body, skipped);
            }
            self.advance_counter(locals, counter);
            skipped += 1;
            record_false_iteration();
        }
    }

    fn scan_packed_bitset(
        &self,
        counter: &mut f64,
        limit: f64,
        elements: &[Value],
        packed: &PackedBitset,
    ) -> (ScanOutcome, usize) {
        if let Some(outcome) = self.scan_packed_words(counter, limit, elements, packed) {
            return outcome;
        }

        let mut skipped = 0;
        loop {
            if !apply_counter_condition(self.counter_condition, *counter, limit) {
                return (ScanOutcome::Exit, skipped);
            }
            let shifted = match packed.word_operation {
                BinaryOp::Shr => f64::from(
                    to_int32_number(*counter) >> (to_uint32_number(packed.word_shift) & 0x1f),
                ),
                BinaryOp::UShr => f64::from(
                    to_uint32_number(*counter) >> (to_uint32_number(packed.word_shift) & 0x1f),
                ),
                _ => unreachable!("packed bitset compiler only admits shifts"),
            };
            let Some(index) = array_index_from_number(shifted) else {
                return (ScanOutcome::Deopt, skipped);
            };
            let Some(Value::Number(word)) = elements.get(index) else {
                return (ScanOutcome::Deopt, skipped);
            };
            let offset = to_uint32_number(f64::from(
                to_int32_number(*counter) & to_int32_number(packed.bit_mask),
            )) & 0x1f;
            let selected = to_int32_number(*word) & (to_int32_number(packed.bit) << offset);
            if selected != 0 {
                return (ScanOutcome::Body, skipped);
            }
            *counter = match self.counter_update {
                UpdateOp::Increment => *counter + 1.0,
                UpdateOp::Decrement => *counter - 1.0,
            };
            skipped += 1;
            record_false_iteration();
        }
    }

    fn scan_packed_words(
        &self,
        counter: &mut f64,
        limit: f64,
        elements: &[Value],
        packed: &PackedBitset,
    ) -> Option<(ScanOutcome, usize)> {
        if self.counter_condition != BinaryOp::Lt
            || self.counter_update != UpdateOp::Increment
            || !matches!(packed.word_operation, BinaryOp::Shr | BinaryOp::UShr)
        {
            return None;
        }
        let shift = to_uint32_number(packed.word_shift) & 0x1f;
        if shift > 5 {
            return None;
        }
        let width = 1_u32 << shift;
        if to_int32_number(packed.bit_mask) as u32 != width - 1 {
            return None;
        }
        let bit = to_int32_number(packed.bit) as u32;
        if bit.count_ones() != 1 {
            return None;
        }
        let base_bit = bit.trailing_zeros();
        if base_bit + width > u32::BITS {
            return None;
        }

        if !apply_counter_condition(self.counter_condition, *counter, limit) {
            return Some((ScanOutcome::Exit, 0));
        }
        if !counter.is_finite() || *counter < 0.0 || counter.fract() != 0.0 || !limit.is_finite() {
            return None;
        }
        let end = limit.ceil();
        if end > f64::from(i32::MAX) + 1.0 || end <= *counter {
            return None;
        }

        let mut current = *counter as usize;
        let end = end as usize;
        let width = width as usize;
        let shift = shift as usize;
        let base_bit = base_bit as usize;
        let mut skipped = 0;
        while current < end {
            let group_base = (current >> shift) << shift;
            let group_end = (group_base + width).min(end);
            let index = current >> shift;
            let Some(Value::Number(word)) = elements.get(index) else {
                return Some((ScanOutcome::Deopt, skipped));
            };
            let first_bit = base_bit + current - group_base;
            let end_bit = base_bit + group_end - group_base;
            let range = low_bits(end_bit) & !low_bits(first_bit);
            let matching = to_int32_number(*word) as u32 & range;
            if matching != 0 {
                let found = group_base + matching.trailing_zeros() as usize - base_bit;
                debug_assert!((current..group_end).contains(&found));
                let false_count = found - current;
                skipped += false_count;
                record_false_iterations(false_count);
                *counter = found as f64;
                return Some((ScanOutcome::Body, skipped));
            }
            let false_count = group_end - current;
            skipped += false_count;
            record_false_iterations(false_count);
            current = group_end;
            *counter = current as f64;
        }
        Some((ScanOutcome::Exit, skipped))
    }

    #[inline(always)]
    fn advance_counter(&self, locals: &mut [f64; MAX_SCAN_LOCALS], counter: f64) {
        locals[self.counter_local] = match self.counter_update {
            UpdateOp::Increment => counter + 1.0,
            UpdateOp::Decrement => counter - 1.0,
        };
    }
}

#[inline(always)]
fn low_bits(bits: usize) -> u32 {
    if bits == u32::BITS as usize {
        u32::MAX
    } else {
        (1_u32 << bits) - 1
    }
}

#[derive(Clone, Copy, Debug)]
enum ScanOutcome {
    Body,
    Exit,
    Deopt,
}

fn compile_false_path(bytecode: &Bytecode, target: usize, tail: usize) -> Option<Option<usize>> {
    let code = &bytecode.code;
    if target >= tail || !matches!(code.get(target), Some(Op::Pop)) {
        return None;
    }
    if target + 1 == tail {
        return Some(None);
    }
    let (Some(Op::LoadConst(value)), Some(Op::StoreLocal(slot))) =
        (code.get(target + 1), code.get(target + 2))
    else {
        return None;
    };
    if target + 3 != tail || !matches!(bytecode.constants.get(*value), Some(Value::Undefined)) {
        return None;
    }
    Some(Some(*slot))
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
            | BinaryOp::Eq
            | BinaryOp::StrictEq
            | BinaryOp::Ne
            | BinaryOp::StrictNe
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
    )
}

fn binary_returns_boolean(operation: BinaryOp) -> bool {
    matches!(
        operation,
        BinaryOp::Eq
            | BinaryOp::StrictEq
            | BinaryOp::Ne
            | BinaryOp::StrictNe
            | BinaryOp::Lt
            | BinaryOp::Le
            | BinaryOp::Gt
            | BinaryOp::Ge
    )
}

fn supported_unary(operation: UnaryOp) -> bool {
    matches!(
        operation,
        UnaryOp::Plus | UnaryOp::Minus | UnaryOp::Not | UnaryOp::BitwiseNot
    )
}

fn apply_counter_condition(operation: BinaryOp, left: f64, right: f64) -> bool {
    match operation {
        BinaryOp::Lt => left < right,
        BinaryOp::Le => left <= right,
        BinaryOp::Gt => left > right,
        BinaryOp::Ge => left >= right,
        _ => unreachable!("compiler only admits relational counter conditions"),
    }
}

fn apply_instruction(
    instruction: ScanInstruction,
    locals: &[f64; MAX_SCAN_LOCALS],
    registers: &[ScanValue; MAX_SCAN_OPS],
    elements: &[Value],
) -> Option<ScanValue> {
    Some(match instruction {
        ScanInstruction::Constant(value) => ScanValue::Number(value),
        ScanInstruction::LoadLocal(local) => ScanValue::Number(locals[local]),
        ScanInstruction::DenseLoad { index } => {
            let ScanValue::Number(index) = registers[index] else {
                return None;
            };
            let index = array_index_from_number(index)?;
            let Value::Number(value) = elements.get(index)? else {
                return None;
            };
            ScanValue::Number(*value)
        }
        ScanInstruction::Binary {
            operation,
            left,
            right,
        } => {
            let (ScanValue::Number(left), ScanValue::Number(right)) =
                (registers[left], registers[right])
            else {
                return None;
            };
            apply_binary(operation, left, right)?
        }
        ScanInstruction::Unary { operation, value } => match operation {
            UnaryOp::Not => ScanValue::Boolean(!registers[value].is_truthy()),
            UnaryOp::Plus => {
                let ScanValue::Number(value) = registers[value] else {
                    return None;
                };
                ScanValue::Number(value)
            }
            UnaryOp::Minus => {
                let ScanValue::Number(value) = registers[value] else {
                    return None;
                };
                ScanValue::Number(-value)
            }
            UnaryOp::BitwiseNot => {
                let ScanValue::Number(value) = registers[value] else {
                    return None;
                };
                ScanValue::Number(f64::from(!to_int32_number(value)))
            }
            _ => return None,
        },
    })
}

fn apply_binary(operation: BinaryOp, left: f64, right: f64) -> Option<ScanValue> {
    Some(match operation {
        BinaryOp::Add => ScanValue::Number(left + right),
        BinaryOp::Sub => ScanValue::Number(left - right),
        BinaryOp::Mul => ScanValue::Number(left * right),
        BinaryOp::Div => ScanValue::Number(left / right),
        BinaryOp::Rem => ScanValue::Number(left % right),
        BinaryOp::Shl => ScanValue::Number(f64::from(
            to_int32_number(left) << (to_uint32_number(right) & 0x1f),
        )),
        BinaryOp::Shr => ScanValue::Number(f64::from(
            to_int32_number(left) >> (to_uint32_number(right) & 0x1f),
        )),
        BinaryOp::UShr => ScanValue::Number(f64::from(
            to_uint32_number(left) >> (to_uint32_number(right) & 0x1f),
        )),
        BinaryOp::BitwiseAnd => {
            ScanValue::Number(f64::from(to_int32_number(left) & to_int32_number(right)))
        }
        BinaryOp::BitwiseXor => {
            ScanValue::Number(f64::from(to_int32_number(left) ^ to_int32_number(right)))
        }
        BinaryOp::BitwiseOr => {
            ScanValue::Number(f64::from(to_int32_number(left) | to_int32_number(right)))
        }
        BinaryOp::Eq | BinaryOp::StrictEq => ScanValue::Boolean(left == right),
        BinaryOp::Ne | BinaryOp::StrictNe => ScanValue::Boolean(left != right),
        BinaryOp::Lt => ScanValue::Boolean(left < right),
        BinaryOp::Le => ScanValue::Boolean(left <= right),
        BinaryOp::Gt => ScanValue::Boolean(left > right),
        BinaryOp::Ge => ScanValue::Boolean(left >= right),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;
    use crate::{Value, eval};

    fn nested_function(source: &str) -> Bytecode {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        let bytecode = compiler::compile_script(&script).expect("source should compile");
        bytecode
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
                _ => None,
            })
            .expect("function bytecode should be nested in the script")
    }

    fn predicate_scan_count(bytecode: &Bytecode) -> usize {
        bytecode
            .code
            .iter()
            .enumerate()
            .filter(|(backedge, op)| {
                matches!(op, Op::Jump(header) if *header < *backedge)
                    && matches!(op, Op::Jump(header) if DenseNumericPredicateScanPlan::compile(bytecode, *header, *backedge).is_some())
            })
            .count()
    }

    #[test]
    fn recognizes_nsieve_prefix_but_rejects_observable_predicates_and_false_edges() {
        let nsieve = nested_function(
            "function primes(isPrime, n) { var i, count = 0, m = 10000 << n; for (i = 2; i < m; i++) if (isPrime[i >> 5] & (1 << (i & 31))) { for (var j = i + i; j < m; j += i) isPrime[j >> 5] &= ~(1 << (j & 31)); count++; } return count; }",
        );
        assert_eq!(predicate_scan_count(&nsieve), 1, "{:#?}", nsieve.code);
        assert!(nsieve.code.iter().enumerate().any(|(backedge, op)| {
            let Op::Jump(header) = op else {
                return false;
            };
            *header < backedge
                && DenseNumericPredicateScanPlan::compile(&nsieve, *header, backedge)
                    .is_some_and(|plan| matches!(plan.predicate, PredicateKernel::PackedBitset(_)))
        }));

        let call = nested_function(
            "function run(a, n, test) { for (var i = 0; i < n; i++) if (test(a[i])) return i; return -1; }",
        );
        assert_eq!(predicate_scan_count(&call), 0);

        let false_effect = nested_function(
            "function run(a, n) { var hits = 0; for (var i = 0; i < n; i++) if (a[i] & 1) hits++; else hits += 2; return hits; }",
        );
        assert_eq!(predicate_scan_count(&false_effect), 0);
    }

    #[test]
    fn true_body_remains_eligible_for_virtual_object_lowering() {
        let bytecode = nested_function(
            "function run(words, n) { var sum = 0; for (var i = 0; i < n; i++) if (words[i >> 5] & (1 << (i & 31))) { var point = { x: i, y: 1 }; sum += point.x + point.y; } return sum; }",
        );
        assert_eq!(predicate_scan_count(&bytecode), 1, "{:#?}", bytecode.code);
        let lowered = super::super::super::virtual_object::lower(&bytecode);
        let execution = lowered.code(&bytecode.code);
        assert!(execution.iter().any(|op| matches!(
            op,
            Op::InitVirtualObject { .. } | Op::InitVirtualConstants { .. }
        )));
    }

    #[test]
    fn batches_false_prefixes_then_releases_the_array_before_the_true_body() {
        reset_test_counters();
        assert_eq!(
            eval(
                "var log = 0; function touch(a, i) { log = log * 10 + i; a[i] = 0; } function run(a, n, callback) { var i; for (i = 0; i < n; i++) if (a[i] & 1) callback(a, i); return i + ':' + log + ':' + a.join(','); } run([0, 0, 1, 0], 4, touch);"
            ),
            Ok(Value::String("4:2:0,0,0,0".to_owned().into()))
        );
        assert!(FALSE_ITERATIONS.get() >= 2);
        assert!(BODY_HANDOFFS.get() >= 1);
    }

    #[test]
    fn all_false_scan_commits_the_counter_and_exits_normally() {
        reset_test_counters();
        assert_eq!(
            eval(
                "function run(a, n) { var i, hits = 0; for (i = 0; i < n; i++) if (a[i] & 1) hits++; return i + ':' + hits; } run([0, 2, 4, 6, 8], 5);"
            ),
            Ok(Value::String("5:0".to_owned().into()))
        );
        assert!(FALSE_ITERATIONS.get() >= 3);
        assert_eq!(BODY_HANDOFFS.get(), 0);
    }

    #[test]
    fn packed_bitset_scan_stops_at_each_first_true_bit_and_at_the_limit() {
        reset_test_counters();
        assert_eq!(
            eval(
                "function run(words, n) { var out = ''; var i; for (i = 0; i < n; i++) if (words[i >> 5] & (1 << (i & 31))) out += i + ','; return i + ':' + out; } run([(1 << 3) | (1 << 9), 1 << 8], 50);"
            ),
            Ok(Value::String("50:3,9,40,".to_owned().into()))
        );
        assert!(FALSE_ITERATIONS.get() >= 40);
        assert_eq!(BODY_HANDOFFS.get(), 3);

        reset_test_counters();
        assert_eq!(
            eval(
                "function beforeLimit(words, n) { var hits = 0; var i; for (i = 0; i < n; i++) if (words[i >> 5] & (1 << (i & 31))) hits++; return i + ':' + hits; } beforeLimit([1 << 10], 10);"
            ),
            Ok(Value::String("10:0".to_owned().into()))
        );
        assert_eq!(BODY_HANDOFFS.get(), 0);
    }

    #[test]
    fn packed_bitset_fields_are_not_tied_to_32_bit_groups_or_bit_zero() {
        reset_test_counters();
        assert_eq!(
            eval(
                "function run(words, n) { var out = ''; for (var i = 0; i < n; i++) if (words[i >> 3] & (2 << (i & 7))) out += i + ','; return out; } run([(1 << 4) | (1 << 7), 1 << 1], 12);"
            ),
            Ok(Value::String("3,6,8,".to_owned().into()))
        );
        assert!(FALSE_ITERATIONS.get() >= 5);
        assert_eq!(BODY_HANDOFFS.get(), 3);
    }

    #[test]
    fn holes_and_accessors_preserve_generic_get_semantics() {
        reset_test_counters();
        assert_eq!(
            eval(
                "function run() { var inherited = 0, own = 0; var proto = Object.create(Array.prototype); Object.defineProperty(proto, '1', { get: function () { inherited++; return 0; } }); var sparse = [0, , 1]; Object.setPrototypeOf(sparse, proto); var described = [0, 1]; Object.defineProperty(described, '0', { get: function () { own++; return 0; } }); var hits = 0; for (var i = 0; i < 3; i++) if (sparse[i] & 1) hits++; for (var j = 0; j < 2; j++) if (described[j] & 1) hits++; return inherited + ':' + own + ':' + hits; } run();"
            ),
            Ok(Value::String("1:1:2".to_owned().into()))
        );
        assert_eq!(FALSE_ITERATIONS.get(), 0);
    }

    #[test]
    fn proxy_receiver_never_enters_the_dense_lease() {
        reset_test_counters();
        assert_eq!(
            eval(
                "function run(n) { var gets = 0; var values = new Proxy([0, 1, 0], { get: function (target, key) { gets++; return target[key]; } }); var hits = 0; for (var i = 0; i < n; i++) if (values[i] & 1) hits++; return gets + ':' + hits; } run(3);"
            ),
            Ok(Value::String("3:1".to_owned().into()))
        );
        assert_eq!(FALSE_ITERATIONS.get(), 0);
    }

    #[test]
    fn custom_prototype_cannot_intercept_present_own_dense_numbers() {
        reset_test_counters();
        assert_eq!(
            eval(
                "var gets = 0; var proto = Object.create(Array.prototype); Object.defineProperty(proto, '0', { get: function () { gets++; return 1; } }); var values = [0, 0, 1]; Object.setPrototypeOf(values, proto); function run(values, n) { var hits = 0; for (var i = 0; i < n; i++) if (values[i] & 1) hits++; return gets + ':' + hits; } run(values, 3);"
            ),
            Ok(Value::String("0:1".to_owned().into()))
        );
        assert!(FALSE_ITERATIONS.get() >= 1);
    }

    #[test]
    fn non_number_deopt_replays_only_the_current_iteration() {
        reset_test_counters();
        assert_eq!(
            eval(
                "var coerces = 0; var marker = { valueOf: function () { coerces++; return 0; } }; function run(values, n) { var hits = 0; for (var i = 0; i < n; i++) if (values[i] & 1) hits++; return i + ':' + coerces + ':' + hits; } run([0, 0, marker, 1], 4);"
            ),
            Ok(Value::String("4:1:1".to_owned().into()))
        );
        assert!(FALSE_ITERATIONS.get() >= 1);
        assert!(MID_SCAN_DEOPTS.get() >= 1);
    }

    #[test]
    fn zero_progress_guard_failure_suppresses_later_frame_attempts() {
        reset_test_counters();
        assert_eq!(
            eval(
                "var coerces = 0; var marker = { valueOf: function () { coerces++; return 0; } }; function run(values, n) { var hits = 0; for (var i = 0; i < n; i++) if (values[i] & 1) hits++; return i + ':' + coerces + ':' + hits; } run([marker, marker, marker, marker], 4);"
            ),
            Ok(Value::String("4:4:0".to_owned().into()))
        );
        assert_eq!(PLAN_ATTEMPTS.get(), 1);
        assert_eq!(FALSE_ITERATIONS.get(), 0);
    }

    #[test]
    fn removing_one_suppressed_plan_keeps_later_backedge_indices_valid() {
        reset_test_counters();
        assert_eq!(
            eval(
                "var coerces = 0; var marker = { valueOf: function () { coerces++; return 0; } }; function run(bad, words, n) { var badHits = 0, goodHits = 0; for (var i = 0; i < n; i++) if (bad[i] & 1) badHits++; for (var j = 0; j < n; j++) if (words[j >> 5] & (1 << (j & 31))) goodHits++; return coerces + ':' + badHits + ':' + goodHits; } run([marker, marker, marker, marker], [1 << 3], 4);"
            ),
            Ok(Value::String("4:0:1".to_owned().into()))
        );
        assert!(PLAN_ATTEMPTS.get() >= 2);
        assert!(FALSE_ITERATIONS.get() >= 1);
        assert!(BODY_HANDOFFS.get() >= 1);
    }

    #[test]
    fn direct_eval_and_captured_counter_or_limit_fail_closed() {
        reset_test_counters();
        assert_eq!(
            eval(
                "function direct(a, n) { var hits = 0; eval(''); for (var i = 0; i < n; i++) if (a[i] & 1) hits++; return i + ':' + hits; } direct([0, 0, 1], 3);"
            ),
            Ok(Value::String("3:1".to_owned().into()))
        );
        assert_eq!(FALSE_ITERATIONS.get(), 0);

        reset_test_counters();
        assert_eq!(
            eval(
                "function captured(a, n) { var hits = 0; function read() { return i + n; } for (var i = 0; i < n; i++) if (a[i] & 1) hits++; return read() + ':' + hits; } captured([0, 0, 1], 3);"
            ),
            Ok(Value::String("6:1".to_owned().into()))
        );
        assert_eq!(FALSE_ITERATIONS.get(), 0);
    }

    #[test]
    fn aliased_counter_and_limit_are_not_snapshotted() {
        let source = "function run(words) { var i = 0; for (; i <= i; i++) if (words[i >> 5] & (1 << (i & 31))) return i; return -1; }";
        let bytecode = nested_function(source);
        assert_eq!(predicate_scan_count(&bytecode), 0, "{:#?}", bytecode.code);
        reset_test_counters();
        assert_eq!(
            eval(&format!("{source} run([1 << 3]);")),
            Ok(Value::Number(3.0))
        );
        assert_eq!(PLAN_ATTEMPTS.get(), 0);
    }

    #[test]
    fn number_truthiness_and_bitwise_coercion_match_the_vm_edges() {
        reset_test_counters();
        assert_eq!(
            eval(
                "function bitwise(values, n) { var out = ''; for (var i = -0; i < n; i++) if (values[i] & 1) out += i; return i + ':' + out; } bitwise([0 / 0, 1 / 0, -0, 3], 4);"
            ),
            Ok(Value::String("4:3".to_owned().into()))
        );
        assert!(FALSE_ITERATIONS.get() >= 1);

        reset_test_counters();
        assert_eq!(
            eval(
                "function truthy(values, n) { var out = ''; for (var i = 0; i < n; i++) if (values[i]) out += i; return out; } truthy([0 / 0, 1 / 0, -0, 3], 4);"
            ),
            Ok(Value::String("13".to_owned().into()))
        );
        assert!(FALSE_ITERATIONS.get() >= 1);
        assert!(BODY_HANDOFFS.get() >= 1);
    }
}
