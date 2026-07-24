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
    DenseAccess, DenseNumericMutationLoopRun, DynamicControl, DynamicDensePlan, DynamicProgramRun,
    HoleTailAppendAccess, HoleTailAppendPlan, INLINE_DENSE_OPS, LocalWrite, MAX_DENSE_LOCALS,
    MAX_DENSE_OPS, MAX_SAFE_INTEGER, MultiAccess, NumberInstruction as ExtendedInstruction,
    ReadAccess, SingleAccess, SunkDenseStore, apply_binary, apply_unary, array_index_from_number,
    descending_counter_is_valid, local_number, record_countdown_iteration,
    record_countdown_path_hit, record_hole_tail_append_attempt, record_hole_tail_append_path_hit,
    record_iteration, record_read_only_bailout, record_read_only_path_hit, record_single_path_hit,
    record_sunk_store_hit, record_writable_lease_suppression, record_writable_path_hit,
    set_local_number,
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
#[cfg_attr(test, derive(PartialEq))]
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
struct NumberInputPrefix {
    constant_count: usize,
    local_count: usize,
}

impl NumberInputPrefix {
    fn validated_dynamic_start(self, operation_count: usize) -> Option<usize> {
        self.constant_count
            .checked_add(self.local_count)
            .filter(|dynamic_start| *dynamic_start <= operation_count)
    }
}

fn operation_registers_are_valid(operation: &NumberInstruction, destination: usize) -> bool {
    let valid = |register: Register| register < destination;
    match operation {
        NumberInstruction::Constant(_) | NumberInstruction::LoadLocal(_) => true,
        NumberInstruction::DenseLoad { index, .. } => valid(*index),
        NumberInstruction::DenseStore { index, value, .. }
        | NumberInstruction::Binary {
            left: index,
            right: value,
            ..
        } => valid(*index) && valid(*value),
        NumberInstruction::Unary { value, .. } | NumberInstruction::Update { value, .. } => {
            valid(*value)
        }
    }
}

fn remap_operation_registers(operation: &mut NumberInstruction, remap: &[usize]) {
    let remap_register = |register: &mut Register| *register = remap[*register];
    match operation {
        NumberInstruction::Constant(_) | NumberInstruction::LoadLocal(_) => {}
        NumberInstruction::DenseLoad { index, .. } => remap_register(index),
        NumberInstruction::DenseStore { index, value, .. }
        | NumberInstruction::Binary {
            left: index,
            right: value,
            ..
        } => {
            remap_register(index);
            remap_register(value);
        }
        NumberInstruction::Unary { value, .. } | NumberInstruction::Update { value, .. } => {
            remap_register(value);
        }
    }
}

/// Canonicalizes the pure iteration inputs ahead of the effectful instruction
/// stream. Constants are keyed by their exact IEEE-754 bits, while local loads
/// are keyed by local index. The translator's SSA map guarantees that an
/// emitted `LoadLocal` always reads the iteration-entry snapshot; reads after a
/// same-body write already refer to that write's result register.
fn compact_number_inputs(
    operations: &mut Vec<NumberInstruction>,
    writes: &mut [LocalWrite],
    sunk_store: &mut Option<SunkDenseStore>,
) -> Option<NumberInputPrefix> {
    let operation_count = operations.len();
    if operation_count > MAX_DENSE_OPS
        || operations
            .iter()
            .enumerate()
            .any(|(destination, operation)| !operation_registers_are_valid(operation, destination))
        || writes.iter().any(|write| write.value >= operation_count)
        || sunk_store
            .is_some_and(|store| store.index >= operation_count || store.value >= operation_count)
    {
        return None;
    }

    let mut constant_bits = [0_u64; MAX_DENSE_OPS];
    let mut constant_count = 0;
    let mut local_indices = [0_usize; MAX_DENSE_OPS];
    let mut local_count = 0;
    for operation in operations.iter() {
        match *operation {
            NumberInstruction::Constant(value) => {
                let bits = value.to_bits();
                if !constant_bits[..constant_count].contains(&bits) {
                    constant_bits[constant_count] = bits;
                    constant_count += 1;
                }
            }
            NumberInstruction::LoadLocal(local) => {
                if !local_indices[..local_count].contains(&local) {
                    local_indices[local_count] = local;
                    local_count += 1;
                }
            }
            NumberInstruction::DenseLoad { .. }
            | NumberInstruction::DenseStore { .. }
            | NumberInstruction::Binary { .. }
            | NumberInstruction::Unary { .. }
            | NumberInstruction::Update { .. } => {}
        }
    }

    let prefix = NumberInputPrefix {
        constant_count,
        local_count,
    };
    let mut remap = [usize::MAX; MAX_DENSE_OPS];
    let mut next_dynamic = prefix.validated_dynamic_start(operation_count)?;
    for (old_register, operation) in operations.iter().enumerate() {
        remap[old_register] = match *operation {
            NumberInstruction::Constant(value) => constant_bits[..constant_count]
                .iter()
                .position(|bits| *bits == value.to_bits())?,
            NumberInstruction::LoadLocal(local) => {
                constant_count
                    + local_indices[..local_count]
                        .iter()
                        .position(|candidate| *candidate == local)?
            }
            NumberInstruction::DenseLoad { .. }
            | NumberInstruction::DenseStore { .. }
            | NumberInstruction::Binary { .. }
            | NumberInstruction::Unary { .. }
            | NumberInstruction::Update { .. } => {
                let register = next_dynamic;
                next_dynamic += 1;
                register
            }
        };
    }

    let old_operations = std::mem::take(operations);
    operations.reserve(next_dynamic);
    operations.extend(
        constant_bits[..constant_count]
            .iter()
            .copied()
            .map(f64::from_bits)
            .map(NumberInstruction::Constant),
    );
    operations.extend(
        local_indices[..local_count]
            .iter()
            .copied()
            .map(NumberInstruction::LoadLocal),
    );
    for (old_register, mut operation) in old_operations.into_iter().enumerate() {
        if matches!(
            operation,
            NumberInstruction::Constant(_) | NumberInstruction::LoadLocal(_)
        ) {
            continue;
        }
        debug_assert_eq!(operations.len(), remap[old_register]);
        remap_operation_registers(&mut operation, &remap[..operation_count]);
        operations.push(operation);
    }
    for write in writes {
        write.value = remap[write.value];
    }
    if let Some(store) = sunk_store {
        store.index = remap[store.index];
        store.value = remap[store.value];
    }
    debug_assert_eq!(operations.len(), next_dynamic);
    Some(prefix)
}

#[cfg(test)]
mod input_prefix_tests {
    use super::*;

    fn valid_inputs() -> (
        Vec<NumberInstruction>,
        Vec<LocalWrite>,
        Option<SunkDenseStore>,
    ) {
        (
            vec![
                NumberInstruction::Constant(1.0),
                NumberInstruction::LoadLocal(2),
                NumberInstruction::Binary {
                    operation: BinaryOp::Add,
                    left: 0,
                    right: 1,
                },
            ],
            vec![LocalWrite { local: 3, value: 2 }],
            Some(SunkDenseStore {
                receiver: 0,
                index: 1,
                value: 2,
            }),
        )
    }

    fn assert_rejected_without_mutation(
        mut operations: Vec<NumberInstruction>,
        mut writes: Vec<LocalWrite>,
        mut sunk_store: Option<SunkDenseStore>,
    ) {
        let operations_before = operations.clone();
        let writes_before = writes.clone();
        let sunk_store_before = sunk_store;

        assert!(compact_number_inputs(&mut operations, &mut writes, &mut sunk_store).is_none());
        assert_eq!(operations, operations_before);
        assert_eq!(writes, writes_before);
        assert_eq!(sunk_store, sunk_store_before);
    }

    #[test]
    fn input_prefix_deduplicates_exact_bits_and_remaps_every_output() {
        let first_nan = f64::from_bits(0x7ff8_0000_0000_0001);
        let second_nan = f64::from_bits(0x7ff8_0000_0000_0002);
        let mut operations = vec![
            NumberInstruction::Constant(0.0),
            NumberInstruction::Constant(-0.0),
            NumberInstruction::Constant(first_nan),
            NumberInstruction::Constant(first_nan),
            NumberInstruction::Constant(second_nan),
            NumberInstruction::LoadLocal(3),
            NumberInstruction::LoadLocal(3),
            NumberInstruction::LoadLocal(4),
            NumberInstruction::Binary {
                operation: BinaryOp::Add,
                left: 0,
                right: 1,
            },
            NumberInstruction::Binary {
                operation: BinaryOp::Mul,
                left: 3,
                right: 6,
            },
        ];
        let mut writes = [LocalWrite { local: 7, value: 9 }];
        let mut sunk_store = Some(SunkDenseStore {
            receiver: 0,
            index: 5,
            value: 8,
        });

        let prefix = compact_number_inputs(&mut operations, &mut writes, &mut sunk_store)
            .expect("well-formed input stream should compact");

        assert_eq!(prefix.constant_count, 4);
        assert_eq!(prefix.local_count, 2);
        assert_eq!(operations.len(), 8);
        let constant_bits = operations[..4]
            .iter()
            .map(|operation| match operation {
                NumberInstruction::Constant(value) => value.to_bits(),
                _ => panic!("expected constant prefix"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            constant_bits,
            vec![
                0.0_f64.to_bits(),
                (-0.0_f64).to_bits(),
                first_nan.to_bits(),
                second_nan.to_bits(),
            ]
        );
        assert!(matches!(operations[4], NumberInstruction::LoadLocal(3)));
        assert!(matches!(operations[5], NumberInstruction::LoadLocal(4)));
        assert!(matches!(
            operations[6],
            NumberInstruction::Binary {
                operation: BinaryOp::Add,
                left: 0,
                right: 1,
            }
        ));
        assert!(matches!(
            operations[7],
            NumberInstruction::Binary {
                operation: BinaryOp::Mul,
                left: 2,
                right: 4,
            }
        ));
        assert_eq!(writes[0].value, 7);
        let sunk_store = sunk_store.expect("sunk store should remain present");
        assert_eq!(sunk_store.index, 4);
        assert_eq!(sunk_store.value, 6);
    }

    #[test]
    fn input_prefix_rejects_non_topological_registers_without_mutation() {
        let (mut operations, writes, sunk_store) = valid_inputs();
        operations[1] = NumberInstruction::Binary {
            operation: BinaryOp::Add,
            left: 2,
            right: 0,
        };
        assert_rejected_without_mutation(operations, writes, sunk_store);
    }

    #[test]
    fn input_prefix_rejects_invalid_write_register_without_mutation() {
        let (operations, mut writes, sunk_store) = valid_inputs();
        writes[0].value = operations.len();
        assert_rejected_without_mutation(operations, writes, sunk_store);
    }

    #[test]
    fn input_prefix_rejects_invalid_sunk_store_registers_without_mutation() {
        let (operations, writes, mut sunk_store) = valid_inputs();
        sunk_store.as_mut().expect("fixture has a sunk store").index = operations.len();
        assert_rejected_without_mutation(operations, writes, sunk_store);

        let (operations, writes, mut sunk_store) = valid_inputs();
        sunk_store.as_mut().expect("fixture has a sunk store").value = operations.len();
        assert_rejected_without_mutation(operations, writes, sunk_store);
    }

    #[test]
    fn input_prefix_bounds_validation_is_constant_time_and_fail_closed() {
        assert_eq!(
            NumberInputPrefix {
                constant_count: 2,
                local_count: 3,
            }
            .validated_dynamic_start(8),
            Some(5)
        );
        assert_eq!(
            NumberInputPrefix {
                constant_count: 2,
                local_count: 3,
            }
            .validated_dynamic_start(4),
            None
        );
        assert_eq!(
            NumberInputPrefix {
                constant_count: usize::MAX,
                local_count: 1,
            }
            .validated_dynamic_start(usize::MAX),
            None
        );
    }
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
    input_prefix: Option<NumberInputPrefix>,
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
        let mut operations = operations
            .into_iter()
            .map(NumberInstruction::from_extended)
            .collect::<Vec<_>>();
        let mut writes = writes;
        let mut sunk_store = sunk_store;
        let reduction = reduction::LegacyReductionPlan::compile(
            counter_local,
            control,
            &operations,
            &writes,
            store_count,
            sunk_store,
        );
        let input_prefix = compact_number_inputs(&mut operations, &mut writes, &mut sunk_store);

        Self {
            counter_local,
            control,
            receiver_sources,
            local_slots,
            operations,
            input_prefix,
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

    #[cfg(test)]
    pub(super) fn input_layout(&self) -> Option<(usize, usize, usize)> {
        let prefix = self.input_prefix?;
        let dynamic_start = prefix.validated_dynamic_start(self.operations.len())?;
        Some((
            prefix.constant_count,
            prefix.local_count,
            self.operations.len() - dynamic_start,
        ))
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

    fn try_run_hole_tail_append(
        &self,
        vm: &mut Vm<'_>,
        arrays: &[ArrayRef],
        locals: &mut [f64; MAX_DENSE_LOCALS],
        registers: &mut [f64],
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
                self.run_program(&mut access, locals, registers, limit)
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
        let mut large_registers =
            (self.operations.len() > INLINE_DENSE_OPS).then(|| vec![0.0; self.operations.len()]);
        let registers = match large_registers.as_mut() {
            Some(registers) => registers.as_mut_slice(),
            None => &mut inline_registers[..self.operations.len()],
        };

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
                self.run_program(&mut access, &mut locals, registers, limit)
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
                self.run_program(&mut access, &mut locals, registers, limit)
            });
            if ran.is_none() {
                ran = self.try_run_hole_tail_append(
                    vm,
                    std::slice::from_ref(&array),
                    &mut locals,
                    registers,
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
                self.run_program(&mut access, &mut locals, registers, limit)
            });
            if ran.is_none() {
                ran = self.try_run_hole_tail_append(vm, &arrays, &mut locals, registers, limit);
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
        let mut large_registers =
            (self.operations.len() > INLINE_DENSE_OPS).then(|| vec![0.0; self.operations.len()]);
        let registers = match large_registers.as_mut() {
            Some(registers) => registers.as_mut_slice(),
            None => &mut inline_registers[..self.operations.len()],
        };
        let ran = ArrayRef::with_distinct_dense_writable_elements(&arrays, |elements| {
            let mut access = MultiAccess {
                elements,
                pending: Vec::with_capacity(self.store_count),
            };
            self.run_program(&mut access, &mut locals, registers, limit)
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
