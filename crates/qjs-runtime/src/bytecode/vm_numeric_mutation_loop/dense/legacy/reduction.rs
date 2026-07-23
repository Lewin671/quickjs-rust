use qjs_ast::BinaryOp;

use super::{
    LocalControl, LocalWrite, NumberInstruction, ReadAccess, Register, SunkDenseStore,
    array_index_from_number, record_iteration,
};
use crate::bytecode::vm_numeric_mutation_loop::dense::{
    DenseAccess, DynamicProgramRun, MAX_DENSE_LOCALS,
};

const MAX_REDUCTION_LANES: usize = 8;
const MAX_ARRAY_INDEX: usize = (u32::MAX - 1) as usize;

#[inline(always)]
fn record_reduction_iteration() {
    #[cfg(test)]
    super::record_read_only_iteration();
    crate::bytecode::vm_numeric_mutation_loop::dense::record_reduction_iteration();
}

#[derive(Clone, Copy, Debug)]
enum IndexLocal {
    Counter,
    Invariant(usize),
}

#[derive(Clone, Copy, Debug)]
enum IndexScalar {
    Constant(f64),
    Local(IndexLocal),
}

impl IndexScalar {
    #[inline(always)]
    fn value(self, counter: f64, locals: &[f64; MAX_DENSE_LOCALS]) -> f64 {
        match self {
            Self::Constant(value) => value,
            Self::Local(IndexLocal::Counter) => counter,
            Self::Local(IndexLocal::Invariant(local)) => locals[local],
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum IndexOperation {
    Add,
    Subtract,
    Multiply,
}

#[derive(Clone, Copy, Debug)]
struct ReductionIndex {
    left: IndexScalar,
    right: Option<IndexScalar>,
    operation: Option<IndexOperation>,
}

impl ReductionIndex {
    #[inline(always)]
    fn value(self, counter: f64, locals: &[f64; MAX_DENSE_LOCALS]) -> f64 {
        let left = self.left.value(counter, locals);
        match self.operation {
            None => left,
            Some(IndexOperation::Add) => {
                left + self
                    .right
                    .expect("binary reduction indices have a right operand")
                    .value(counter, locals)
            }
            Some(IndexOperation::Subtract) => {
                left - self
                    .right
                    .expect("binary reduction indices have a right operand")
                    .value(counter, locals)
            }
            Some(IndexOperation::Multiply) => {
                left * self
                    .right
                    .expect("binary reduction indices have a right operand")
                    .value(counter, locals)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ReductionRead {
    receiver: usize,
    index: ReductionIndex,
}

#[derive(Clone, Copy, Debug)]
struct ReductionLane {
    left: ReductionRead,
    right: ReductionRead,
    accumulator_local: usize,
    result: Register,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReductionOutputSource {
    Lane(usize),
    Counter,
}

#[derive(Clone, Copy, Debug)]
struct ReductionOutput {
    local: usize,
    source: ReductionOutputSource,
}

#[derive(Clone, Copy, Debug)]
struct StridedCounterIndex {
    invariant_local: usize,
    counter_first: bool,
}

#[derive(Clone, Copy, Debug)]
struct TwoLaneStridedCounter {
    first: StridedCounterIndex,
    second: StridedCounterIndex,
}

#[derive(Clone, Copy, Debug)]
struct ExactStridedIndices {
    counter: usize,
    first_coefficient: usize,
    second_coefficient: usize,
    first_stride: usize,
    second_stride: usize,
}

#[derive(Clone, Debug)]
pub(super) struct LegacyReductionPlan {
    counter_local: usize,
    lanes: Vec<ReductionLane>,
    outputs: Vec<ReductionOutput>,
    two_lane_strided_counter: Option<TwoLaneStridedCounter>,
}

impl LegacyReductionPlan {
    pub(super) fn compile(
        counter_local: usize,
        control: LocalControl,
        operations: &[NumberInstruction],
        writes: &[LocalWrite],
        store_count: usize,
        sunk_store: Option<SunkDenseStore>,
    ) -> Option<Self> {
        if store_count != 0 || sunk_store.is_some() || !matches!(control, LocalControl::LessThan(_))
        {
            return None;
        }

        let mut cursor = 0;
        let mut lanes = Vec::new();
        while operations.len().checked_sub(cursor)? > 2 {
            if lanes.len() == MAX_REDUCTION_LANES {
                return None;
            }
            let (lane, next) = parse_lane(operations, writes, counter_local, cursor)?;
            if lane.accumulator_local == counter_local
                || lanes.iter().any(|existing: &ReductionLane| {
                    existing.accumulator_local == lane.accumulator_local
                })
            {
                return None;
            }
            lanes.push(lane);
            cursor = next;
        }
        if lanes.is_empty() || operations.len() - cursor != 2 {
            return None;
        }
        let counter_register = cursor;
        if !matches!(
            operations.get(counter_register),
            Some(NumberInstruction::LoadLocal(local)) if *local == counter_local
        ) || !matches!(
            operations.get(counter_register + 1),
            Some(NumberInstruction::Update {
                operation: qjs_ast::UpdateOp::Increment,
                value,
            }) if *value == counter_register
        ) {
            return None;
        }
        let counter_result = counter_register + 1;

        let mut accumulator_writes = [false; MAX_REDUCTION_LANES];
        let mut counter_write = false;
        let mut outputs = Vec::with_capacity(writes.len());
        for write in writes {
            let source = if write.value == counter_result {
                ReductionOutputSource::Counter
            } else {
                let lane = lanes.iter().position(|lane| lane.result == write.value)?;
                ReductionOutputSource::Lane(lane)
            };

            if write.local == counter_local {
                if source != ReductionOutputSource::Counter {
                    return None;
                }
                counter_write = true;
            }
            if let Some(lane) = lanes
                .iter()
                .position(|lane| lane.accumulator_local == write.local)
            {
                if source != ReductionOutputSource::Lane(lane) {
                    return None;
                }
                accumulator_writes[lane] = true;
            }
            outputs.push(ReductionOutput {
                local: write.local,
                source,
            });
        }
        if !counter_write || !accumulator_writes[..lanes.len()].iter().all(|seen| *seen) {
            return None;
        }

        let two_lane_strided_counter = compile_two_lane_strided_counter(&lanes);
        Some(Self {
            counter_local,
            lanes,
            outputs,
            two_lane_strided_counter,
        })
    }

    pub(super) fn run(
        &self,
        access: &ReadAccess<'_, '_>,
        locals: &mut [f64; MAX_DENSE_LOCALS],
        limit: f64,
    ) -> DynamicProgramRun {
        if let Some(strided) = self.two_lane_strided_counter {
            return run_two_lane_strided_counter(self, strided, access, locals, limit);
        }
        if self.lanes.len() == 2 {
            return run_legacy_two_lane_reduction_program(self, access, locals, limit);
        }
        run_legacy_reduction_program(self, access, locals, limit)
    }

    #[cfg(test)]
    pub(super) fn is_two_lane_strided_counter(&self) -> bool {
        self.two_lane_strided_counter.is_some()
    }
}

fn compile_two_lane_strided_counter(lanes: &[ReductionLane]) -> Option<TwoLaneStridedCounter> {
    let [first, second] = lanes else {
        return None;
    };
    Some(TwoLaneStridedCounter {
        first: compile_strided_counter_index(first)?,
        second: compile_strided_counter_index(second)?,
    })
}

fn compile_strided_counter_index(lane: &ReductionLane) -> Option<StridedCounterIndex> {
    if !matches!(
        lane.right.index,
        ReductionIndex {
            left: IndexScalar::Local(IndexLocal::Counter),
            right: None,
            operation: None,
        }
    ) {
        return None;
    }
    let ReductionIndex {
        left,
        right: Some(right),
        operation: Some(IndexOperation::Multiply),
    } = lane.left.index
    else {
        return None;
    };
    match (left, right) {
        (
            IndexScalar::Local(IndexLocal::Counter),
            IndexScalar::Local(IndexLocal::Invariant(invariant_local)),
        ) => Some(StridedCounterIndex {
            invariant_local,
            counter_first: true,
        }),
        (
            IndexScalar::Local(IndexLocal::Invariant(invariant_local)),
            IndexScalar::Local(IndexLocal::Counter),
        ) => Some(StridedCounterIndex {
            invariant_local,
            counter_first: false,
        }),
        _ => None,
    }
}

fn parse_lane(
    operations: &[NumberInstruction],
    writes: &[LocalWrite],
    counter_local: usize,
    cursor: usize,
) -> Option<(ReductionLane, usize)> {
    let (left, left_register, cursor) =
        parse_dense_read(operations, writes, counter_local, cursor)?;
    let (right, right_register, cursor) =
        parse_dense_read(operations, writes, counter_local, cursor)?;
    let product = cursor;
    if !matches!(
        operations.get(product),
        Some(NumberInstruction::Binary {
            operation: BinaryOp::Mul,
            left,
            right,
        }) if *left == left_register && *right == right_register
    ) {
        return None;
    }
    let accumulator_register = product + 1;
    let NumberInstruction::LoadLocal(accumulator_local) = operations.get(accumulator_register)?
    else {
        return None;
    };
    let result = accumulator_register + 1;
    if !matches!(
        operations.get(result),
        Some(NumberInstruction::Binary {
            operation: BinaryOp::Add,
            left,
            right,
        }) if *left == accumulator_register && *right == product
    ) {
        return None;
    }
    Some((
        ReductionLane {
            left,
            right,
            accumulator_local: *accumulator_local,
            result,
        },
        result + 1,
    ))
}

fn parse_dense_read(
    operations: &[NumberInstruction],
    writes: &[LocalWrite],
    counter_local: usize,
    cursor: usize,
) -> Option<(ReductionRead, Register, usize)> {
    let first = parse_index_scalar(operations, writes, counter_local, cursor)?;
    if let Some(NumberInstruction::DenseLoad { receiver, index }) = operations.get(cursor + 1)
        && *index == cursor
    {
        return Some((
            ReductionRead {
                receiver: *receiver,
                index: ReductionIndex {
                    left: first,
                    right: None,
                    operation: None,
                },
            },
            cursor + 1,
            cursor + 2,
        ));
    }

    let second = parse_index_scalar(operations, writes, counter_local, cursor + 1)?;
    let binary_register = cursor + 2;
    let NumberInstruction::Binary {
        operation,
        left,
        right,
    } = operations.get(binary_register)?
    else {
        return None;
    };
    let operation = match operation {
        BinaryOp::Add => IndexOperation::Add,
        BinaryOp::Sub => IndexOperation::Subtract,
        BinaryOp::Mul => IndexOperation::Multiply,
        _ => return None,
    };
    let (left, right) = match (*left, *right) {
        (left, right) if left == cursor && right == cursor + 1 => (first, second),
        (left, right) if left == cursor + 1 && right == cursor => (second, first),
        _ => return None,
    };
    let load_register = cursor + 3;
    let NumberInstruction::DenseLoad { receiver, index } = operations.get(load_register)? else {
        return None;
    };
    if *index != binary_register {
        return None;
    }
    Some((
        ReductionRead {
            receiver: *receiver,
            index: ReductionIndex {
                left,
                right: Some(right),
                operation: Some(operation),
            },
        },
        load_register,
        load_register + 1,
    ))
}

fn parse_index_scalar(
    operations: &[NumberInstruction],
    writes: &[LocalWrite],
    counter_local: usize,
    register: Register,
) -> Option<IndexScalar> {
    match *operations.get(register)? {
        NumberInstruction::Constant(value) => Some(IndexScalar::Constant(value)),
        NumberInstruction::LoadLocal(local) if local == counter_local => {
            Some(IndexScalar::Local(IndexLocal::Counter))
        }
        NumberInstruction::LoadLocal(local) if !writes.iter().any(|write| write.local == local) => {
            Some(IndexScalar::Local(IndexLocal::Invariant(local)))
        }
        _ => None,
    }
}

#[inline(always)]
fn load_dense_number(elements: &[crate::Value], index: usize) -> Option<f64> {
    match elements.get(index)? {
        crate::Value::Number(value) => Some(*value),
        _ => None,
    }
}

fn run_two_lane_strided_counter(
    plan: &LegacyReductionPlan,
    strided: TwoLaneStridedCounter,
    access: &ReadAccess<'_, '_>,
    locals: &mut [f64; MAX_DENSE_LOCALS],
    limit: f64,
) -> DynamicProgramRun {
    if let Some(indices) = exact_strided_indices(plan, strided, locals) {
        let shared_sample_stride = plan.lanes[0].right.receiver == plan.lanes[1].right.receiver
            && indices.first_stride == indices.second_stride
            && indices.first_coefficient == indices.second_coefficient;
        let run = if shared_sample_stride {
            run_legacy_two_lane_shared_sample_stride_exact_index_program(
                plan, indices, access, locals, limit,
            )
        } else {
            run_legacy_two_lane_exact_index_strided_counter_program(
                plan, indices, access, locals, limit,
            )
        };
        #[cfg(test)]
        if run.made_progress {
            super::super::record_exact_index_reduction_path_hit();
            if shared_sample_stride {
                super::super::record_shared_sample_stride_reduction_path_hit();
            }
        }
        return run;
    }
    match (strided.first.counter_first, strided.second.counter_first) {
        (false, false) => run_legacy_two_lane_strided_counter_program::<false, false>(
            plan, strided, access, locals, limit,
        ),
        (false, true) => run_legacy_two_lane_strided_counter_program::<false, true>(
            plan, strided, access, locals, limit,
        ),
        (true, false) => run_legacy_two_lane_strided_counter_program::<true, false>(
            plan, strided, access, locals, limit,
        ),
        (true, true) => run_legacy_two_lane_strided_counter_program::<true, true>(
            plan, strided, access, locals, limit,
        ),
    }
}

fn exact_strided_indices(
    plan: &LegacyReductionPlan,
    strided: TwoLaneStridedCounter,
    locals: &[f64; MAX_DENSE_LOCALS],
) -> Option<ExactStridedIndices> {
    let counter = array_index_from_number(locals[plan.counter_local])?;
    let first_stride = array_index_from_number(locals[strided.first.invariant_local])?;
    let second_stride = array_index_from_number(locals[strided.second.invariant_local])?;
    Some(ExactStridedIndices {
        counter,
        first_coefficient: checked_array_index_product(counter, first_stride)?,
        second_coefficient: checked_array_index_product(counter, second_stride)?,
        first_stride,
        second_stride,
    })
}

#[inline(always)]
fn checked_array_index_product(left: usize, right: usize) -> Option<usize> {
    left.checked_mul(right)
        .filter(|product| *product <= MAX_ARRAY_INDEX)
}

#[inline(always)]
fn checked_next_array_index(index: usize, step: usize) -> Option<usize> {
    index
        .checked_add(step)
        .filter(|next| *next <= MAX_ARRAY_INDEX)
}

#[cfg(test)]
pub(super) fn test_checked_array_index_product(left: usize, right: usize) -> Option<usize> {
    checked_array_index_product(left, right)
}

#[cfg(test)]
pub(super) fn test_checked_next_array_index(index: usize, step: usize) -> Option<usize> {
    checked_next_array_index(index, step)
}

#[inline(always)]
#[allow(clippy::if_same_then_else)] // Preserve source operand order, including NaN payload choice.
fn ordered_stride_product<const COUNTER_FIRST: bool>(counter: f64, stride: f64) -> f64 {
    if COUNTER_FIRST {
        counter * stride
    } else {
        stride * counter
    }
}

#[inline(never)]
fn run_legacy_two_lane_exact_index_strided_counter_program(
    plan: &LegacyReductionPlan,
    indices: ExactStridedIndices,
    access: &ReadAccess<'_, '_>,
    locals: &mut [f64; MAX_DENSE_LOCALS],
    limit: f64,
) -> DynamicProgramRun {
    debug_assert_eq!(plan.lanes.len(), 2);
    let first = plan.lanes[0];
    let second = plan.lanes[1];
    let first_coefficient_elements = access.elements[first.left.receiver].as_slice();
    let first_sample_elements = access.elements[first.right.receiver].as_slice();
    let second_coefficient_elements = access.elements[second.left.receiver].as_slice();
    let second_sample_elements = access.elements[second.right.receiver].as_slice();

    let mut counter = locals[plan.counter_local];
    let mut counter_index = Some(indices.counter);
    let mut first_coefficient_index = Some(indices.first_coefficient);
    let mut second_coefficient_index = Some(indices.second_coefficient);
    let mut first_accumulator = locals[first.accumulator_local];
    let mut second_accumulator = locals[second.accumulator_local];
    let mut made_progress = false;
    let deoptimized = 'program: loop {
        if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
            break false;
        }

        let Some(current_first_coefficient_index) = first_coefficient_index else {
            break 'program true;
        };
        let Some(first_coefficient) =
            load_dense_number(first_coefficient_elements, current_first_coefficient_index)
        else {
            break 'program true;
        };
        let Some(current_counter_index) = counter_index else {
            break 'program true;
        };
        let Some(first_sample) = load_dense_number(first_sample_elements, current_counter_index)
        else {
            break 'program true;
        };
        let first_product = first_coefficient * first_sample;
        let next_first = first_accumulator + first_product;

        let Some(current_second_coefficient_index) = second_coefficient_index else {
            break 'program true;
        };
        let Some(second_coefficient) = load_dense_number(
            second_coefficient_elements,
            current_second_coefficient_index,
        ) else {
            break 'program true;
        };
        let Some(second_sample) = load_dense_number(second_sample_elements, current_counter_index)
        else {
            break 'program true;
        };
        let second_product = second_coefficient * second_sample;
        let next_second = second_accumulator + second_product;

        first_accumulator = next_first;
        second_accumulator = next_second;
        counter += 1.0;
        counter_index = checked_next_array_index(current_counter_index, 1);
        first_coefficient_index =
            checked_next_array_index(current_first_coefficient_index, indices.first_stride);
        second_coefficient_index =
            checked_next_array_index(current_second_coefficient_index, indices.second_stride);
        made_progress = true;
        record_reduction_iteration();
        record_iteration();
    };

    if made_progress {
        for output in &plan.outputs {
            locals[output.local] = match output.source {
                ReductionOutputSource::Lane(0) => first_accumulator,
                ReductionOutputSource::Lane(1) => second_accumulator,
                ReductionOutputSource::Lane(_) => {
                    unreachable!("two-lane reduction output references a third lane")
                }
                ReductionOutputSource::Counter => counter,
            };
        }
    }
    DynamicProgramRun {
        deoptimized,
        made_progress,
    }
}

#[inline(never)]
fn run_legacy_two_lane_shared_sample_stride_exact_index_program(
    plan: &LegacyReductionPlan,
    indices: ExactStridedIndices,
    access: &ReadAccess<'_, '_>,
    locals: &mut [f64; MAX_DENSE_LOCALS],
    limit: f64,
) -> DynamicProgramRun {
    debug_assert_eq!(plan.lanes.len(), 2);
    let first = plan.lanes[0];
    let second = plan.lanes[1];
    debug_assert_eq!(first.right.receiver, second.right.receiver);
    debug_assert_eq!(indices.first_stride, indices.second_stride);
    debug_assert_eq!(indices.first_coefficient, indices.second_coefficient);
    let first_coefficient_elements = access.elements[first.left.receiver].as_slice();
    let sample_elements = access.elements[first.right.receiver].as_slice();
    let second_coefficient_elements = access.elements[second.left.receiver].as_slice();

    let mut counter = locals[plan.counter_local];
    let mut counter_index = Some(indices.counter);
    let mut coefficient_index = Some(indices.first_coefficient);
    let mut first_accumulator = locals[first.accumulator_local];
    let mut second_accumulator = locals[second.accumulator_local];
    let mut made_progress = false;
    let deoptimized = 'program: loop {
        if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
            break false;
        }

        let Some(current_coefficient_index) = coefficient_index else {
            break 'program true;
        };
        let Some(first_coefficient) =
            load_dense_number(first_coefficient_elements, current_coefficient_index)
        else {
            break 'program true;
        };
        let Some(current_counter_index) = counter_index else {
            break 'program true;
        };
        let Some(sample) = load_dense_number(sample_elements, current_counter_index) else {
            break 'program true;
        };
        let first_product = first_coefficient * sample;
        let next_first = first_accumulator + first_product;

        let Some(second_coefficient) =
            load_dense_number(second_coefficient_elements, current_coefficient_index)
        else {
            break 'program true;
        };
        let second_product = second_coefficient * sample;
        let next_second = second_accumulator + second_product;

        first_accumulator = next_first;
        second_accumulator = next_second;
        counter += 1.0;
        counter_index = checked_next_array_index(current_counter_index, 1);
        coefficient_index =
            checked_next_array_index(current_coefficient_index, indices.first_stride);
        made_progress = true;
        record_reduction_iteration();
        record_iteration();
    };

    if made_progress {
        for output in &plan.outputs {
            locals[output.local] = match output.source {
                ReductionOutputSource::Lane(0) => first_accumulator,
                ReductionOutputSource::Lane(1) => second_accumulator,
                ReductionOutputSource::Lane(_) => {
                    unreachable!("two-lane reduction output references a third lane")
                }
                ReductionOutputSource::Counter => counter,
            };
        }
    }
    DynamicProgramRun {
        deoptimized,
        made_progress,
    }
}

#[inline(never)]
fn run_legacy_two_lane_strided_counter_program<
    const FIRST_COUNTER_FIRST: bool,
    const SECOND_COUNTER_FIRST: bool,
>(
    plan: &LegacyReductionPlan,
    strided: TwoLaneStridedCounter,
    access: &ReadAccess<'_, '_>,
    locals: &mut [f64; MAX_DENSE_LOCALS],
    limit: f64,
) -> DynamicProgramRun {
    debug_assert_eq!(plan.lanes.len(), 2);
    let first = plan.lanes[0];
    let second = plan.lanes[1];
    let first_coefficient_elements = access.elements[first.left.receiver].as_slice();
    let first_sample_elements = access.elements[first.right.receiver].as_slice();
    let second_coefficient_elements = access.elements[second.left.receiver].as_slice();
    let second_sample_elements = access.elements[second.right.receiver].as_slice();
    let first_stride = locals[strided.first.invariant_local];
    let second_stride = locals[strided.second.invariant_local];

    let mut counter = locals[plan.counter_local];
    let mut counter_index = None;
    let mut first_accumulator = locals[first.accumulator_local];
    let mut second_accumulator = locals[second.accumulator_local];
    let mut made_progress = false;
    let deoptimized = 'program: loop {
        if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
            break false;
        }

        let first_coefficient_key =
            ordered_stride_product::<FIRST_COUNTER_FIRST>(counter, first_stride);
        let Some(first_coefficient_index) = array_index_from_number(first_coefficient_key) else {
            break 'program true;
        };
        let Some(first_coefficient) =
            load_dense_number(first_coefficient_elements, first_coefficient_index)
        else {
            break 'program true;
        };
        let current_counter_index = match counter_index {
            Some(index) => index,
            None => {
                let Some(index) = array_index_from_number(counter) else {
                    break 'program true;
                };
                index
            }
        };
        let Some(first_sample) = load_dense_number(first_sample_elements, current_counter_index)
        else {
            break 'program true;
        };
        let first_product = first_coefficient * first_sample;
        let next_first = first_accumulator + first_product;

        let second_coefficient_key =
            ordered_stride_product::<SECOND_COUNTER_FIRST>(counter, second_stride);
        let Some(second_coefficient_index) = array_index_from_number(second_coefficient_key) else {
            break 'program true;
        };
        let Some(second_coefficient) =
            load_dense_number(second_coefficient_elements, second_coefficient_index)
        else {
            break 'program true;
        };
        let Some(second_sample) = load_dense_number(second_sample_elements, current_counter_index)
        else {
            break 'program true;
        };
        let second_product = second_coefficient * second_sample;
        let next_second = second_accumulator + second_product;

        first_accumulator = next_first;
        second_accumulator = next_second;
        counter += 1.0;
        counter_index = current_counter_index.checked_add(1);
        made_progress = true;
        record_reduction_iteration();
        record_iteration();
    };

    if made_progress {
        for output in &plan.outputs {
            locals[output.local] = match output.source {
                ReductionOutputSource::Lane(0) => first_accumulator,
                ReductionOutputSource::Lane(1) => second_accumulator,
                ReductionOutputSource::Lane(_) => {
                    unreachable!("two-lane reduction output references a third lane")
                }
                ReductionOutputSource::Counter => counter,
            };
        }
    }
    DynamicProgramRun {
        deoptimized,
        made_progress,
    }
}

#[inline(never)]
fn run_legacy_two_lane_reduction_program(
    plan: &LegacyReductionPlan,
    access: &ReadAccess<'_, '_>,
    locals: &mut [f64; MAX_DENSE_LOCALS],
    limit: f64,
) -> DynamicProgramRun {
    debug_assert_eq!(plan.lanes.len(), 2);
    let first = plan.lanes[0];
    let second = plan.lanes[1];
    let first_left_elements = access.elements[first.left.receiver].as_slice();
    let first_right_elements = access.elements[first.right.receiver].as_slice();
    let second_left_elements = access.elements[second.left.receiver].as_slice();
    let second_right_elements = access.elements[second.right.receiver].as_slice();

    let mut counter = locals[plan.counter_local];
    let mut first_accumulator = locals[first.accumulator_local];
    let mut second_accumulator = locals[second.accumulator_local];
    let mut made_progress = false;
    let deoptimized = 'program: loop {
        if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
            break false;
        }

        let Some(first_left_index) =
            array_index_from_number(first.left.index.value(counter, locals))
        else {
            break 'program true;
        };
        let Some(first_left) = load_dense_number(first_left_elements, first_left_index) else {
            break 'program true;
        };
        let Some(first_right_index) =
            array_index_from_number(first.right.index.value(counter, locals))
        else {
            break 'program true;
        };
        let Some(first_right) = load_dense_number(first_right_elements, first_right_index) else {
            break 'program true;
        };
        let first_product = first_left * first_right;
        let next_first = first_accumulator + first_product;

        let Some(second_left_index) =
            array_index_from_number(second.left.index.value(counter, locals))
        else {
            break 'program true;
        };
        let Some(second_left) = load_dense_number(second_left_elements, second_left_index) else {
            break 'program true;
        };
        let Some(second_right_index) =
            array_index_from_number(second.right.index.value(counter, locals))
        else {
            break 'program true;
        };
        let Some(second_right) = load_dense_number(second_right_elements, second_right_index)
        else {
            break 'program true;
        };
        let second_product = second_left * second_right;
        let next_second = second_accumulator + second_product;

        first_accumulator = next_first;
        second_accumulator = next_second;
        counter += 1.0;
        made_progress = true;
        record_reduction_iteration();
        record_iteration();
    };

    if made_progress {
        for output in &plan.outputs {
            locals[output.local] = match output.source {
                ReductionOutputSource::Lane(0) => first_accumulator,
                ReductionOutputSource::Lane(1) => second_accumulator,
                ReductionOutputSource::Lane(_) => {
                    unreachable!("two-lane reduction output references a third lane")
                }
                ReductionOutputSource::Counter => counter,
            };
        }
    }
    DynamicProgramRun {
        deoptimized,
        made_progress,
    }
}

#[inline(never)]
fn run_legacy_reduction_program(
    plan: &LegacyReductionPlan,
    access: &ReadAccess<'_, '_>,
    locals: &mut [f64; MAX_DENSE_LOCALS],
    limit: f64,
) -> DynamicProgramRun {
    let mut counter = locals[plan.counter_local];
    let mut accumulator_storage = [0.0; MAX_REDUCTION_LANES];
    let mut next_storage = [0.0; MAX_REDUCTION_LANES];
    for (lane_index, lane) in plan.lanes.iter().enumerate() {
        accumulator_storage[lane_index] = locals[lane.accumulator_local];
    }
    let mut accumulators = &mut accumulator_storage;
    let mut next = &mut next_storage;
    let mut made_progress = false;
    let deoptimized = 'program: loop {
        if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
            break false;
        }
        for (lane_index, lane) in plan.lanes.iter().enumerate() {
            let Some(left_index) = array_index_from_number(lane.left.index.value(counter, locals))
            else {
                break 'program true;
            };
            let Some(left) = access.load_number(lane.left.receiver, left_index) else {
                break 'program true;
            };
            let Some(right_index) =
                array_index_from_number(lane.right.index.value(counter, locals))
            else {
                break 'program true;
            };
            let Some(right) = access.load_number(lane.right.receiver, right_index) else {
                break 'program true;
            };
            let product = left * right;
            next[lane_index] = accumulators[lane_index] + product;
        }
        std::mem::swap(&mut accumulators, &mut next);
        counter += 1.0;
        made_progress = true;
        record_reduction_iteration();
        record_iteration();
    };

    if made_progress {
        for output in &plan.outputs {
            locals[output.local] = match output.source {
                ReductionOutputSource::Lane(lane) => accumulators[lane],
                ReductionOutputSource::Counter => counter,
            };
        }
    }
    DynamicProgramRun {
        deoptimized,
        made_progress,
    }
}
