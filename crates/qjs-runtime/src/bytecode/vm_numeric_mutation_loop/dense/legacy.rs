//! Compact executor for dense loops whose inputs all live in VM local slots.
//!
//! Own-data sources and guarded native calls need the general executor in the
//! parent module. Keeping the long-standing local-only instruction shape here
//! avoids charging those extensions to the common local-array dispatch loop.

use std::rc::Rc;

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use crate::{
    Value,
    value::{ArrayRef, OwnDataPropertyRead},
};

use super::super::super::vm::Vm;
use super::invariants::{ArraySource as ExtendedArraySource, DynamicLimit, OwnDataOwner};
use super::{
    CompactProgram, CompactScratch, DenseAccess, DenseNumericMutationLoopRun, DynamicControl,
    DynamicDensePlan, DynamicProgramRun, HoleTailAppendAccess, HoleTailAppendPlan,
    INLINE_DENSE_OPS, LocalWrite, MAX_DENSE_LOCALS, MAX_SAFE_INTEGER, MultiAccess,
    NumberInstruction as ExtendedInstruction, ReadAccess, SingleAccess, SunkDenseStore,
    apply_binary, apply_unary, array_index_from_number, descending_counter_is_valid, local_number,
    record_countdown_iteration, record_countdown_path_hit, record_hole_tail_append_attempt,
    record_hole_tail_append_path_hit, record_iteration, record_read_only_bailout,
    record_read_only_path_hit, record_single_path_hit, record_sunk_store_hit,
    record_writable_lease_suppression, record_writable_path_hit, set_local_number,
};

mod reduction;
mod typed_array;

#[cfg(test)]
use super::{
    record_compact_dynamic_attempt, record_compact_dynamic_decline, record_compact_dynamic_hit,
    record_compact_dynamic_suppression, record_read_only_iteration,
};

#[cfg(test)]
#[derive(Clone, Copy)]
enum CompactRunOutcome {
    Pending,
    Handled,
    Suppressed,
}

#[cfg(test)]
struct CompactRunGuard {
    outcome: CompactRunOutcome,
}

#[cfg(test)]
impl CompactRunGuard {
    fn new() -> Self {
        record_compact_dynamic_attempt();
        Self {
            outcome: CompactRunOutcome::Pending,
        }
    }

    fn handled(&mut self) {
        self.outcome = CompactRunOutcome::Handled;
    }

    fn suppressed(&mut self) {
        self.outcome = CompactRunOutcome::Suppressed;
    }
}

#[cfg(test)]
impl Drop for CompactRunGuard {
    fn drop(&mut self) {
        match self.outcome {
            CompactRunOutcome::Pending => record_compact_dynamic_decline(),
            CompactRunOutcome::Handled => record_compact_dynamic_hit(),
            CompactRunOutcome::Suppressed => record_compact_dynamic_suppression(),
        }
    }
}

type Register = usize;

#[derive(Clone, Debug, PartialEq, Eq)]
enum ArraySource {
    Local(usize),
    DirectThisOwnData(Rc<str>),
}

impl ArraySource {
    fn local_slot(&self) -> Option<usize> {
        match self {
            Self::Local(slot) => Some(*slot),
            Self::DirectThisOwnData(_) => None,
        }
    }

    fn resolve_read_only(&self, vm: &Vm<'_>) -> Option<ArrayRef> {
        match self {
            Self::Local(slot) => match vm.locals.get(*slot) {
                Some(Some(Value::Array(array))) => Some(array.clone()),
                _ => None,
            },
            Self::DirectThisOwnData(key) => {
                resolve_direct_this_own_data(vm.direct_this.as_ref()?, key)
            }
        }
    }
}

fn resolve_direct_this_own_data(value: &Value, key: &Rc<str>) -> Option<ArrayRef> {
    let Value::Object(object) = value else {
        return None;
    };
    if crate::symbol::is_symbol_primitive(object)
        || crate::typed_array::is_typed_array_object(object)
        || object.is_module_namespace_exotic()
    {
        return None;
    }
    match object.own_data_property_read(key) {
        OwnDataPropertyRead::Data(Value::Array(array)) => Some(array),
        OwnDataPropertyRead::Data(_)
        | OwnDataPropertyRead::Missing
        | OwnDataPropertyRead::NeedsSlowPath => None,
    }
}

#[cfg(test)]
pub(super) fn test_direct_this_own_data_array_resolves(value: &Value, key: &str) -> bool {
    resolve_direct_this_own_data(value, &Rc::from(key)).is_some()
}

#[cfg(test)]
pub(super) fn test_checked_array_index_product(left: usize, right: usize) -> Option<usize> {
    reduction::test_checked_array_index_product(left, right)
}

#[cfg(test)]
pub(super) fn test_checked_next_array_index(index: usize, step: usize) -> Option<usize> {
    reduction::test_checked_next_array_index(index, step)
}

#[derive(Clone, Copy, Debug)]
enum LocalLimit {
    Number(usize),
    ArrayLength(usize),
}

#[derive(Clone, Copy, Debug)]
enum LocalControl {
    LessThan(LocalLimit),
    AtLeastZero,
    Countdown,
}

impl LocalControl {
    fn array_length_slot(self) -> Option<usize> {
        match self {
            Self::LessThan(LocalLimit::ArrayLength(slot)) => Some(slot),
            Self::LessThan(LocalLimit::Number(_)) | Self::AtLeastZero | Self::Countdown => None,
        }
    }

    fn is_countdown(self) -> bool {
        matches!(self, Self::Countdown)
    }
}

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

impl NumberInstruction {
    fn from_extended(operation: ExtendedInstruction) -> Self {
        match operation {
            ExtendedInstruction::Constant(value) => Self::Constant(value),
            ExtendedInstruction::LoadLocal(local) => Self::LoadLocal(local),
            ExtendedInstruction::DenseLoad { receiver, index } => {
                Self::DenseLoad { receiver, index }
            }
            ExtendedInstruction::DenseStore {
                receiver,
                index,
                value,
            } => Self::DenseStore {
                receiver,
                index,
                value,
            },
            ExtendedInstruction::Binary {
                operation,
                left,
                right,
            } => Self::Binary {
                operation,
                left,
                right,
            },
            ExtendedInstruction::Unary { operation, value } => Self::Unary { operation, value },
            ExtendedInstruction::Update { operation, value } => Self::Update { operation, value },
            ExtendedInstruction::LoadInvariant(_) | ExtendedInstruction::MathRound { .. } => {
                unreachable!("extended-only instructions are rejected before conversion")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct LegacyDynamicDensePlan {
    counter_local: usize,
    control: LocalControl,
    receiver_sources: Vec<ArraySource>,
    local_slots: Vec<usize>,
    operations: Vec<NumberInstruction>,
    compact_program: Option<Box<CompactProgram>>,
    writes: Vec<LocalWrite>,
    store_count: usize,
    sunk_store: Option<SunkDenseStore>,
    hole_tail_append: Option<HoleTailAppendPlan>,
    reduction: Option<reduction::LegacyReductionPlan>,
    header: usize,
}

impl LegacyDynamicDensePlan {
    pub(super) fn supports(plan: &DynamicDensePlan) -> bool {
        plan.number_sources.is_empty()
            && !plan.uses_math_round
            && plan.receiver_sources.iter().all(|source| match source {
                ExtendedArraySource::Local(_) => true,
                ExtendedArraySource::OwnData(source) => {
                    plan.store_count == 0 && source.owner == OwnDataOwner::DirectThis
                }
            })
            && !matches!(
                plan.control,
                DynamicControl::LessThan(DynamicLimit::OwnDataNumber(_))
            )
            && plan.operations.iter().all(|operation| {
                !matches!(
                    operation,
                    ExtendedInstruction::LoadInvariant(_) | ExtendedInstruction::MathRound { .. }
                )
            })
    }

    pub(super) fn from_extended(plan: DynamicDensePlan) -> Self {
        debug_assert!(Self::supports(&plan));

        let DynamicDensePlan {
            counter_local,
            control,
            receiver_sources,
            number_sources: _,
            local_slots,
            operations,
            compact_program,
            writes,
            store_count,
            sunk_store,
            hole_tail_append,
            uses_math_round: _,
            header,
        } = plan;
        let control = match control {
            DynamicControl::LessThan(DynamicLimit::LocalNumber(local)) => {
                LocalControl::LessThan(LocalLimit::Number(local))
            }
            DynamicControl::LessThan(DynamicLimit::LocalArrayLength(slot)) => {
                LocalControl::LessThan(LocalLimit::ArrayLength(slot))
            }
            DynamicControl::AtLeastZero => LocalControl::AtLeastZero,
            DynamicControl::Countdown => LocalControl::Countdown,
            DynamicControl::LessThan(DynamicLimit::OwnDataNumber(_)) => {
                unreachable!("own-data limits are rejected before conversion")
            }
        };
        let receiver_sources = receiver_sources
            .into_iter()
            .map(|source| match source {
                ExtendedArraySource::Local(slot) => ArraySource::Local(slot),
                ExtendedArraySource::OwnData(source) => match source.owner {
                    OwnDataOwner::DirectThis => ArraySource::DirectThisOwnData(source.key),
                    OwnDataOwner::Local(_) => {
                        unreachable!("local-owner sources are rejected before conversion")
                    }
                },
            })
            .collect();
        let operations = operations
            .into_iter()
            .map(NumberInstruction::from_extended)
            .collect::<Vec<_>>();
        let reduction = reduction::LegacyReductionPlan::compile(
            counter_local,
            control,
            &operations,
            &writes,
            store_count,
            sunk_store,
        );

        Self {
            counter_local,
            control,
            receiver_sources,
            local_slots,
            operations,
            compact_program,
            writes,
            store_count,
            sunk_store,
            hole_tail_append,
            reduction,
            header,
        }
    }

    #[cfg(test)]
    pub(super) fn is_reduction(&self) -> bool {
        self.reduction.is_some()
    }

    #[cfg(test)]
    pub(super) fn is_two_lane_strided_reduction(&self) -> bool {
        self.reduction
            .as_ref()
            .is_some_and(reduction::LegacyReductionPlan::is_two_lane_strided_counter)
    }

    fn deoptimized_run(
        &self,
        locals: &mut [f64; MAX_DENSE_LOCALS],
        countdown_old: Option<f64>,
        made_progress: bool,
    ) -> DynamicProgramRun {
        if let Some(old) = countdown_old {
            locals[self.counter_local] = old;
        }
        DynamicProgramRun {
            deoptimized: true,
            made_progress,
        }
    }

    fn run_program<A: DenseAccess>(
        &self,
        access: &mut A,
        locals: &mut [f64; MAX_DENSE_LOCALS],
        registers: &mut [f64],
        limit: Option<f64>,
    ) -> DynamicProgramRun {
        let mut made_progress = false;
        loop {
            let counter = locals[self.counter_local];
            let countdown_old = match self.control {
                LocalControl::LessThan(_) => {
                    let limit = limit.expect("less-than controls always resolve a limit");
                    if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
                        return DynamicProgramRun {
                            deoptimized: false,
                            made_progress,
                        };
                    }
                    None
                }
                LocalControl::AtLeastZero => {
                    if counter < 0.0 {
                        return DynamicProgramRun {
                            deoptimized: false,
                            made_progress,
                        };
                    }
                    None
                }
                LocalControl::Countdown => {
                    if counter == 0.0 {
                        // The failed postfix test still stores the decremented
                        // value before branching out of the loop.
                        locals[self.counter_local] = -1.0;
                        return DynamicProgramRun {
                            deoptimized: false,
                            made_progress,
                        };
                    }
                    locals[self.counter_local] = counter - 1.0;
                    Some(counter)
                }
            };
            access.reset_iteration();
            for (register, operation) in self.operations.iter().enumerate() {
                let value = match *operation {
                    NumberInstruction::Constant(value) => value,
                    NumberInstruction::LoadLocal(local) => locals[local],
                    NumberInstruction::DenseLoad { receiver, index } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return self.deoptimized_run(locals, countdown_old, made_progress);
                        };
                        let Some(value) = access.load_number(receiver, index) else {
                            return self.deoptimized_run(locals, countdown_old, made_progress);
                        };
                        value
                    }
                    NumberInstruction::DenseStore {
                        receiver,
                        index,
                        value,
                    } => {
                        let Some(index) = array_index_from_number(registers[index]) else {
                            return self.deoptimized_run(locals, countdown_old, made_progress);
                        };
                        let value = registers[value];
                        if !access.stage_store(receiver, index, value) {
                            return self.deoptimized_run(locals, countdown_old, made_progress);
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
                    return self.deoptimized_run(locals, countdown_old, made_progress);
                };
                if !access.stage_store(store.receiver, index, registers[store.value]) {
                    return self.deoptimized_run(locals, countdown_old, made_progress);
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
            if self.control.is_countdown() {
                record_countdown_iteration();
            }
            made_progress = true;
            record_iteration();
        }
    }

    fn run_compact_program<A: DenseAccess>(
        &self,
        program: &CompactProgram,
        scratch: &mut CompactScratch,
        access: &mut A,
        locals: &mut [f64; MAX_DENSE_LOCALS],
        limit: Option<f64>,
    ) -> DynamicProgramRun {
        let (numbers, words) = scratch.banks(program);
        let mut made_progress = false;
        loop {
            let counter = locals[self.counter_local];
            let countdown_old = match self.control {
                LocalControl::LessThan(_) => {
                    let limit = limit.expect("less-than controls always resolve a limit");
                    if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
                        return DynamicProgramRun {
                            deoptimized: false,
                            made_progress,
                        };
                    }
                    None
                }
                LocalControl::AtLeastZero => {
                    if counter < 0.0 {
                        return DynamicProgramRun {
                            deoptimized: false,
                            made_progress,
                        };
                    }
                    None
                }
                LocalControl::Countdown => {
                    if counter == 0.0 {
                        locals[self.counter_local] = -1.0;
                        return DynamicProgramRun {
                            deoptimized: false,
                            made_progress,
                        };
                    }
                    locals[self.counter_local] = counter - 1.0;
                    Some(counter)
                }
            };
            access.reset_iteration();
            if !program.run_iteration(access, |local| Some(locals[local]), &[], numbers, words) {
                return self.deoptimized_run(locals, countdown_old, made_progress);
            }
            if let Some(store) = program.sunk_store() {
                let Some(index) = array_index_from_number(numbers[store.index]) else {
                    return self.deoptimized_run(locals, countdown_old, made_progress);
                };
                if !access.stage_store(store.receiver, index, numbers[store.value]) {
                    return self.deoptimized_run(locals, countdown_old, made_progress);
                }
            }
            debug_assert_eq!(access.staged_store_count(), self.store_count);
            access.commit_stores();
            for write in program.writes() {
                locals[write.local] = numbers[write.value];
            }
            #[cfg(test)]
            if self.store_count == 0 {
                record_read_only_iteration();
            }
            if self.control.is_countdown() {
                record_countdown_iteration();
            }
            made_progress = true;
            super::record_compact_word_iteration();
            record_iteration();
        }
    }

    #[inline]
    fn run_selected_program<A: DenseAccess>(
        &self,
        access: &mut A,
        locals: &mut [f64; MAX_DENSE_LOCALS],
        registers: &mut [f64],
        compact_scratch: &mut Option<CompactScratch>,
        limit: Option<f64>,
    ) -> DynamicProgramRun {
        match (&self.compact_program, compact_scratch) {
            (Some(program), Some(scratch)) => {
                self.run_compact_program(program, scratch, access, locals, limit)
            }
            _ => self.run_program(access, locals, registers, limit),
        }
    }

    fn try_run_hole_tail_append(
        &self,
        vm: &mut Vm<'_>,
        arrays: &[ArrayRef],
        locals: &mut [f64; MAX_DENSE_LOCALS],
        registers: &mut [f64],
        compact_scratch: &mut Option<CompactScratch>,
        limit: Option<f64>,
    ) -> Option<DynamicProgramRun> {
        let append = self.hole_tail_append?;
        record_hole_tail_append_attempt();
        let writer_receiver = append.writer_receiver();
        let writer = arrays.get(writer_receiver)?;
        let start_index = array_index_from_number(locals[self.counter_local])?;
        if !vm.array_uses_realm_prototype(writer)
            || vm.array_prototype_chain_has_index_hazard().unwrap_or(true)
        {
            return None;
        }

        let ran = ArrayRef::with_dense_hole_tail_append_and_readable_elements(
            arrays,
            writer_receiver,
            start_index,
            |writer, readable, logical_length| {
                let mut access = HoleTailAppendAccess::new(
                    writer_receiver,
                    writer,
                    readable,
                    logical_length,
                    limit.expect("append plans use less-than control"),
                );
                self.run_selected_program(&mut access, locals, registers, compact_scratch, limit)
            },
        );
        if ran.is_some_and(|run| run.made_progress) {
            record_hole_tail_append_path_hit();
        }
        ran
    }

    #[inline(never)]
    pub(super) fn try_run(&self, vm: &mut Vm<'_>, exit: usize) -> DenseNumericMutationLoopRun {
        debug_assert!(self.store_count <= 1);
        #[cfg(test)]
        let mut run_guard = CompactRunGuard::new();
        if vm.direct_eval_with_stack {
            return DenseNumericMutationLoopRun::Declined;
        }
        for slot in self
            .local_slots
            .iter()
            .copied()
            .chain(
                self.receiver_sources
                    .iter()
                    .filter_map(ArraySource::local_slot),
            )
            .chain(self.control.array_length_slot())
        {
            if !vm.slot_is_authoritative(slot) {
                return DenseNumericMutationLoopRun::Declined;
            }
        }
        let mut locals = [0.0; MAX_DENSE_LOCALS];
        for (local, slot) in self.local_slots.iter().enumerate() {
            let Some(value) = local_number(vm, *slot) else {
                return DenseNumericMutationLoopRun::Declined;
            };
            locals[local] = value;
        }
        if self.control.is_countdown() {
            let counter = locals[self.counter_local];
            if !counter.is_finite()
                || counter <= 0.0
                || counter.fract() != 0.0
                || counter > MAX_SAFE_INTEGER
            {
                return DenseNumericMutationLoopRun::Declined;
            }
        }
        if matches!(self.control, LocalControl::AtLeastZero) {
            let counter = locals[self.counter_local];
            if !descending_counter_is_valid(counter) {
                return DenseNumericMutationLoopRun::Declined;
            }
        }
        let limit = match self.control {
            LocalControl::LessThan(LocalLimit::Number(local)) => Some(locals[local]),
            LocalControl::LessThan(LocalLimit::ArrayLength(slot)) => match vm.locals.get(slot) {
                Some(Some(Value::Array(array))) => Some(array.len() as f64),
                _ => {
                    if let Some(run) = typed_array::try_run(self, vm, exit) {
                        #[cfg(test)]
                        match run {
                            DenseNumericMutationLoopRun::Handled => run_guard.handled(),
                            DenseNumericMutationLoopRun::Suppress => run_guard.suppressed(),
                            DenseNumericMutationLoopRun::Declined => {}
                        }
                        return run;
                    }
                    return DenseNumericMutationLoopRun::Declined;
                }
            },
            LocalControl::AtLeastZero => None,
            LocalControl::Countdown => None,
        };
        if let Some(reduction) = &self.reduction {
            let Some(arrays) = self
                .receiver_sources
                .iter()
                .map(|source| source.resolve_read_only(vm))
                .collect::<Option<Vec<_>>>()
            else {
                record_read_only_bailout();
                if let Some(run) = typed_array::try_run(self, vm, exit) {
                    #[cfg(test)]
                    match run {
                        DenseNumericMutationLoopRun::Handled => run_guard.handled(),
                        DenseNumericMutationLoopRun::Suppress => run_guard.suppressed(),
                        DenseNumericMutationLoopRun::Declined => {}
                    }
                    return run;
                }
                return DenseNumericMutationLoopRun::Declined;
            };
            let ran = ArrayRef::with_dense_readable_element_sets(&arrays, |elements| {
                let access = ReadAccess { elements };
                reduction.run(
                    &access,
                    &mut locals,
                    limit.expect("reduction plans use less-than control"),
                )
            });
            match ran {
                Some(run) => {
                    if run.made_progress {
                        record_read_only_path_hit();
                        super::record_reduction_path_hit();
                    }
                    if run.deoptimized {
                        record_read_only_bailout();
                    }
                }
                None => record_read_only_bailout(),
            }
            let Some(run) = ran else {
                return DenseNumericMutationLoopRun::Declined;
            };
            if run.deoptimized && !run.made_progress {
                return DenseNumericMutationLoopRun::Declined;
            }
            if run.made_progress {
                for (slot, value) in self.local_slots.iter().copied().zip(locals) {
                    set_local_number(vm, slot, value);
                }
            }
            vm.ip = if run.deoptimized {
                self.header
            } else {
                exit + 1
            };
            #[cfg(test)]
            run_guard.handled();
            return DenseNumericMutationLoopRun::Handled;
        }
        let mut inline_registers = [0.0; INLINE_DENSE_OPS];
        let mut large_registers = (self.compact_program.is_none()
            && self.operations.len() > INLINE_DENSE_OPS)
            .then(|| vec![0.0; self.operations.len()]);
        let registers = match large_registers.as_mut() {
            Some(registers) => registers.as_mut_slice(),
            None if self.compact_program.is_some() => &mut inline_registers[..0],
            None => &mut inline_registers[..self.operations.len()],
        };
        let mut compact_scratch = self.compact_program.as_deref().map(CompactScratch::new);

        let ran = if self.store_count == 0 {
            let Some(arrays) = self
                .receiver_sources
                .iter()
                .map(|source| source.resolve_read_only(vm))
                .collect::<Option<Vec<_>>>()
            else {
                record_read_only_bailout();
                if let Some(run) = typed_array::try_run(self, vm, exit) {
                    #[cfg(test)]
                    match run {
                        DenseNumericMutationLoopRun::Handled => run_guard.handled(),
                        DenseNumericMutationLoopRun::Suppress => run_guard.suppressed(),
                        DenseNumericMutationLoopRun::Declined => {}
                    }
                    return run;
                }
                return DenseNumericMutationLoopRun::Declined;
            };
            let ran = ArrayRef::with_dense_readable_element_sets(&arrays, |elements| {
                let mut access = ReadAccess { elements };
                self.run_selected_program(
                    &mut access,
                    &mut locals,
                    registers,
                    &mut compact_scratch,
                    limit,
                )
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
        } else if self.receiver_sources.len() == 1 && self.store_count == 1 {
            let Some(slot) = self.receiver_sources[0].local_slot() else {
                return DenseNumericMutationLoopRun::Declined;
            };
            let Some(Some(Value::Array(array))) = vm.locals.get(slot) else {
                if let Some(run) = typed_array::try_run(self, vm, exit) {
                    #[cfg(test)]
                    match run {
                        DenseNumericMutationLoopRun::Handled => run_guard.handled(),
                        DenseNumericMutationLoopRun::Suppress => run_guard.suppressed(),
                        DenseNumericMutationLoopRun::Declined => {}
                    }
                    return run;
                }
                return DenseNumericMutationLoopRun::Declined;
            };
            let array = array.clone();
            let mut ran = array.with_dense_writable_elements(|elements| {
                let mut access = SingleAccess {
                    elements,
                    pending: None,
                };
                self.run_selected_program(
                    &mut access,
                    &mut locals,
                    registers,
                    &mut compact_scratch,
                    limit,
                )
            });
            if ran.is_none() {
                ran = self.try_run_hole_tail_append(
                    vm,
                    std::slice::from_ref(&array),
                    &mut locals,
                    registers,
                    &mut compact_scratch,
                    limit,
                );
            }
            if ran.is_some_and(|run| run.made_progress) {
                record_single_path_hit();
            }
            ran
        } else {
            let Some(arrays) = self
                .receiver_sources
                .iter()
                .map(|source| match source.local_slot() {
                    Some(slot) => match vm.locals.get(slot) {
                        Some(Some(Value::Array(array))) => Some(array.clone()),
                        _ => None,
                    },
                    None => None,
                })
                .collect::<Option<Vec<_>>>()
            else {
                if let Some(run) = typed_array::try_run(self, vm, exit) {
                    #[cfg(test)]
                    match run {
                        DenseNumericMutationLoopRun::Handled => run_guard.handled(),
                        DenseNumericMutationLoopRun::Suppress => run_guard.suppressed(),
                        DenseNumericMutationLoopRun::Declined => {}
                    }
                    return run;
                }
                return DenseNumericMutationLoopRun::Declined;
            };
            let mut ran = ArrayRef::with_distinct_dense_writable_elements(&arrays, |elements| {
                let mut access = MultiAccess {
                    elements,
                    pending: Vec::with_capacity(self.store_count),
                };
                self.run_selected_program(
                    &mut access,
                    &mut locals,
                    registers,
                    &mut compact_scratch,
                    limit,
                )
            });
            if ran.is_none() {
                ran = self.try_run_hole_tail_append(
                    vm,
                    &arrays,
                    &mut locals,
                    registers,
                    &mut compact_scratch,
                    limit,
                );
            }
            ran
        };
        let Some(run) = ran else {
            if self.hole_tail_append.is_some() {
                record_writable_lease_suppression();
                #[cfg(test)]
                run_guard.suppressed();
                return DenseNumericMutationLoopRun::Suppress;
            }
            return DenseNumericMutationLoopRun::Declined;
        };
        if self.control.is_countdown() && run.made_progress {
            record_countdown_path_hit();
        }
        if self.sunk_store.is_some() && run.made_progress {
            record_sunk_store_hit();
        }
        if run.deoptimized && !run.made_progress {
            return DenseNumericMutationLoopRun::Declined;
        }
        for (slot, value) in self.local_slots.iter().copied().zip(locals) {
            set_local_number(vm, slot, value);
        }
        vm.ip = if run.deoptimized {
            self.header
        } else {
            exit + 1
        };
        #[cfg(test)]
        run_guard.handled();
        DenseNumericMutationLoopRun::Handled
    }

    #[inline(never)]
    pub(super) fn try_run_suppressing(
        &self,
        vm: &mut Vm<'_>,
        exit: usize,
    ) -> DenseNumericMutationLoopRun {
        debug_assert!(self.store_count > 1);
        #[cfg(test)]
        let mut run_guard = CompactRunGuard::new();
        debug_assert!(
            self.receiver_sources
                .iter()
                .all(|source| matches!(source, ArraySource::Local(_)))
        );
        if vm.direct_eval_with_stack {
            return DenseNumericMutationLoopRun::Declined;
        }
        for slot in self
            .local_slots
            .iter()
            .copied()
            .chain(
                self.receiver_sources
                    .iter()
                    .filter_map(ArraySource::local_slot),
            )
            .chain(self.control.array_length_slot())
        {
            if !vm.slot_is_authoritative(slot) {
                return DenseNumericMutationLoopRun::Declined;
            }
        }
        let Some(arrays) = self
            .receiver_sources
            .iter()
            .map(|source| match source {
                ArraySource::Local(slot) => match vm.locals.get(*slot) {
                    Some(Some(Value::Array(array))) => Some(array.clone()),
                    _ => None,
                },
                ArraySource::DirectThisOwnData(_) => None,
            })
            .collect::<Option<Vec<_>>>()
        else {
            if let Some(run) = typed_array::try_run(self, vm, exit) {
                #[cfg(test)]
                match run {
                    DenseNumericMutationLoopRun::Handled => run_guard.handled(),
                    DenseNumericMutationLoopRun::Suppress => run_guard.suppressed(),
                    DenseNumericMutationLoopRun::Declined => {}
                }
                return run;
            }
            #[cfg(test)]
            run_guard.suppressed();
            return DenseNumericMutationLoopRun::Suppress;
        };
        let mut locals = [0.0; MAX_DENSE_LOCALS];
        for (local, slot) in self.local_slots.iter().enumerate() {
            let Some(value) = local_number(vm, *slot) else {
                return DenseNumericMutationLoopRun::Declined;
            };
            locals[local] = value;
        }
        if self.control.is_countdown() {
            let counter = locals[self.counter_local];
            if !counter.is_finite()
                || counter <= 0.0
                || counter.fract() != 0.0
                || counter > MAX_SAFE_INTEGER
            {
                return DenseNumericMutationLoopRun::Declined;
            }
        }
        if matches!(self.control, LocalControl::AtLeastZero) {
            let counter = locals[self.counter_local];
            if !descending_counter_is_valid(counter) {
                return DenseNumericMutationLoopRun::Declined;
            }
        }
        let limit = match self.control {
            LocalControl::LessThan(LocalLimit::Number(local)) => Some(locals[local]),
            LocalControl::LessThan(LocalLimit::ArrayLength(slot)) => match vm.locals.get(slot) {
                Some(Some(Value::Array(array))) => Some(array.len() as f64),
                _ => return DenseNumericMutationLoopRun::Declined,
            },
            LocalControl::AtLeastZero => None,
            LocalControl::Countdown => None,
        };
        let mut inline_registers = [0.0; INLINE_DENSE_OPS];
        let mut large_registers = (self.compact_program.is_none()
            && self.operations.len() > INLINE_DENSE_OPS)
            .then(|| vec![0.0; self.operations.len()]);
        let registers = match large_registers.as_mut() {
            Some(registers) => registers.as_mut_slice(),
            None if self.compact_program.is_some() => &mut inline_registers[..0],
            None => &mut inline_registers[..self.operations.len()],
        };
        let mut compact_scratch = self.compact_program.as_deref().map(CompactScratch::new);
        let ran = ArrayRef::with_distinct_dense_writable_elements(&arrays, |elements| {
            let mut access = MultiAccess {
                elements,
                pending: Vec::with_capacity(self.store_count),
            };
            self.run_selected_program(
                &mut access,
                &mut locals,
                registers,
                &mut compact_scratch,
                limit,
            )
        });
        let Some(run) = ran else {
            record_writable_lease_suppression();
            #[cfg(test)]
            run_guard.suppressed();
            return DenseNumericMutationLoopRun::Suppress;
        };
        if self.control.is_countdown() && run.made_progress {
            record_countdown_path_hit();
        }
        if self.sunk_store.is_some() && run.made_progress {
            record_sunk_store_hit();
        }
        if run.made_progress {
            record_writable_path_hit();
        }
        if run.deoptimized && !run.made_progress {
            return DenseNumericMutationLoopRun::Declined;
        }
        for (slot, value) in self.local_slots.iter().copied().zip(locals) {
            set_local_number(vm, slot, value);
        }
        vm.ip = if run.deoptimized {
            self.header
        } else {
            exit + 1
        };
        #[cfg(test)]
        run_guard.handled();
        DenseNumericMutationLoopRun::Handled
    }
}
