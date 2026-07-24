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
    collections::BTreeSet,
};

use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use crate::{Value, to_int32_number, to_uint32_number, value::ArrayRef};

use super::super::{
    ir::{Bytecode, Op, decode_index_receiver},
    vm::Vm,
    vm_props::array_index_from_number,
};

mod compiler;
mod hole_tail_append;
mod invariants;
mod legacy;
mod nested;

use hole_tail_append::{HoleTailAppendAccess, HoleTailAppendPlan};
use invariants::{
    ArraySource, DynamicLimit, OwnDataOwner, OwnDataSource, native_math_round_is_current,
};
pub(super) use nested::{
    EnclosingOuter, NestedDensePlan, NestedDensePlanRun, NestedDenseProbe,
    discover_enclosing_outers,
};
#[cfg(test)]
pub(super) use nested::{test_discover_enclosing_bytecode, test_discover_enclosing_intervals};

const INLINE_DENSE_OPS: usize = 64;
const MAX_DENSE_OPS: usize = 256;
const MAX_DENSE_LOCALS: usize = 64;
const MAX_DENSE_WRITES: usize = 64;
const MAX_DENSE_RECEIVERS: usize = 8;
const MAX_DENSE_STORES: usize = 32;
const MAX_FIXED_MUTATIONS: usize = 16;
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

#[inline]
fn descending_counter_is_valid(counter: f64) -> bool {
    counter.is_finite() && counter.fract() == 0.0 && (-1.0..=MAX_SAFE_INTEGER).contains(&counter)
}

#[cfg(test)]
pub(super) fn test_descending_counter_is_valid(counter: f64) -> bool {
    descending_counter_is_valid(counter)
}

#[cfg(test)]
thread_local! {
    static DENSE_LOOP_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static SINGLE_DENSE_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static SUNK_DENSE_STORE_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static READ_ONLY_DENSE_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static READ_ONLY_DENSE_BAILOUTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static READ_ONLY_DENSE_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static COUNTDOWN_DENSE_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static COUNTDOWN_DENSE_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static WRITABLE_DENSE_LEASE_SUPPRESSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static WRITABLE_DENSE_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static MATH_ROUND_OPERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static COMPACT_DYNAMIC_ATTEMPTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static COMPACT_DYNAMIC_DECLINES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static COMPACT_DYNAMIC_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static COMPACT_DYNAMIC_SUPPRESSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static REDUCTION_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static REDUCTION_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static EXACT_INDEX_REDUCTION_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static SHARED_SAMPLE_STRIDE_REDUCTION_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TYPED_ARRAY_DENSE_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TYPED_ARRAY_DENSE_SUPPRESSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TYPED_ARRAY_DENSE_ATTEMPTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static HOLE_TAIL_APPEND_ATTEMPTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static HOLE_TAIL_APPEND_PATH_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static HOLE_TAIL_APPEND_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static HOLE_TAIL_APPEND_STAGED_DISCARDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static NESTED_DENSE_ENTRIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static NESTED_DENSE_OUTER_COMPLETIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static NESTED_DENSE_INNER_COMMITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static NESTED_DENSE_SEEDED_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static NESTED_DENSE_BAILOUTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static NESTED_DENSE_DISCOVERY_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static DYNAMIC_DENSE_COMPILATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_test_iterations() {
    DENSE_LOOP_ITERATIONS.set(0);
    SINGLE_DENSE_PATH_HITS.set(0);
    SUNK_DENSE_STORE_HITS.set(0);
    READ_ONLY_DENSE_PATH_HITS.set(0);
    READ_ONLY_DENSE_BAILOUTS.set(0);
    READ_ONLY_DENSE_ITERATIONS.set(0);
    COUNTDOWN_DENSE_PATH_HITS.set(0);
    COUNTDOWN_DENSE_ITERATIONS.set(0);
    WRITABLE_DENSE_LEASE_SUPPRESSIONS.set(0);
    WRITABLE_DENSE_PATH_HITS.set(0);
    MATH_ROUND_OPERATIONS.set(0);
    COMPACT_DYNAMIC_ATTEMPTS.set(0);
    COMPACT_DYNAMIC_DECLINES.set(0);
    COMPACT_DYNAMIC_HITS.set(0);
    COMPACT_DYNAMIC_SUPPRESSIONS.set(0);
    REDUCTION_PATH_HITS.set(0);
    REDUCTION_ITERATIONS.set(0);
    EXACT_INDEX_REDUCTION_PATH_HITS.set(0);
    SHARED_SAMPLE_STRIDE_REDUCTION_PATH_HITS.set(0);
    TYPED_ARRAY_DENSE_PATH_HITS.set(0);
    TYPED_ARRAY_DENSE_SUPPRESSIONS.set(0);
    TYPED_ARRAY_DENSE_ATTEMPTS.set(0);
    HOLE_TAIL_APPEND_ATTEMPTS.set(0);
    HOLE_TAIL_APPEND_PATH_HITS.set(0);
    HOLE_TAIL_APPEND_ITERATIONS.set(0);
    HOLE_TAIL_APPEND_STAGED_DISCARDS.set(0);
    NESTED_DENSE_ENTRIES.set(0);
    NESTED_DENSE_OUTER_COMPLETIONS.set(0);
    NESTED_DENSE_INNER_COMMITS.set(0);
    NESTED_DENSE_SEEDED_ITERATIONS.set(0);
    NESTED_DENSE_BAILOUTS.set(0);
    NESTED_DENSE_DISCOVERY_WORK.set(0);
    DYNAMIC_DENSE_COMPILATIONS.set(0);
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

#[cfg(test)]
pub(super) fn test_countdown_path_hits() -> usize {
    COUNTDOWN_DENSE_PATH_HITS.get()
}

#[cfg(test)]
pub(super) fn test_countdown_iterations() -> usize {
    COUNTDOWN_DENSE_ITERATIONS.get()
}

#[cfg(test)]
pub(super) fn test_writable_lease_suppressions() -> usize {
    WRITABLE_DENSE_LEASE_SUPPRESSIONS.get()
}

#[cfg(test)]
pub(super) fn test_writable_path_hits() -> usize {
    WRITABLE_DENSE_PATH_HITS.get()
}

#[cfg(test)]
pub(super) fn test_math_round_operations() -> usize {
    MATH_ROUND_OPERATIONS.get()
}

#[cfg(test)]
pub(super) fn test_compact_dynamic_attempts() -> usize {
    COMPACT_DYNAMIC_ATTEMPTS.get()
}

#[cfg(test)]
pub(super) fn test_compact_dynamic_declines() -> usize {
    COMPACT_DYNAMIC_DECLINES.get()
}

#[cfg(test)]
pub(super) fn test_compact_dynamic_hits() -> usize {
    COMPACT_DYNAMIC_HITS.get()
}

#[cfg(test)]
pub(super) fn test_compact_dynamic_suppressions() -> usize {
    COMPACT_DYNAMIC_SUPPRESSIONS.get()
}

#[cfg(test)]
pub(super) fn test_reduction_path_hits() -> usize {
    REDUCTION_PATH_HITS.get()
}

#[cfg(test)]
pub(super) fn test_reduction_iterations() -> usize {
    REDUCTION_ITERATIONS.get()
}

#[cfg(test)]
pub(super) fn test_exact_index_reduction_path_hits() -> usize {
    EXACT_INDEX_REDUCTION_PATH_HITS.get()
}

#[cfg(test)]
pub(super) fn test_shared_sample_stride_reduction_path_hits() -> usize {
    SHARED_SAMPLE_STRIDE_REDUCTION_PATH_HITS.get()
}

#[cfg(test)]
pub(super) fn test_typed_array_dense_path_hits() -> usize {
    TYPED_ARRAY_DENSE_PATH_HITS.get()
}

#[cfg(test)]
pub(super) fn test_typed_array_dense_suppressions() -> usize {
    TYPED_ARRAY_DENSE_SUPPRESSIONS.get()
}

#[cfg(test)]
pub(super) fn test_typed_array_dense_attempts() -> usize {
    TYPED_ARRAY_DENSE_ATTEMPTS.get()
}

#[cfg(test)]
pub(super) fn test_hole_tail_append_attempts() -> usize {
    HOLE_TAIL_APPEND_ATTEMPTS.get()
}

#[cfg(test)]
pub(super) fn test_hole_tail_append_path_hits() -> usize {
    HOLE_TAIL_APPEND_PATH_HITS.get()
}

#[cfg(test)]
pub(super) fn test_hole_tail_append_iterations() -> usize {
    HOLE_TAIL_APPEND_ITERATIONS.get()
}

#[cfg(test)]
pub(super) fn test_hole_tail_append_staged_discards() -> usize {
    HOLE_TAIL_APPEND_STAGED_DISCARDS.get()
}

#[cfg(test)]
pub(super) fn test_nested_dense_entries() -> usize {
    NESTED_DENSE_ENTRIES.get()
}

#[cfg(test)]
pub(super) fn test_nested_dense_outer_completions() -> usize {
    NESTED_DENSE_OUTER_COMPLETIONS.get()
}

#[cfg(test)]
pub(super) fn test_nested_dense_inner_commits() -> usize {
    NESTED_DENSE_INNER_COMMITS.get()
}

#[cfg(test)]
pub(super) fn test_nested_dense_seeded_iterations() -> usize {
    NESTED_DENSE_SEEDED_ITERATIONS.get()
}

#[cfg(test)]
pub(super) fn test_nested_dense_bailouts() -> usize {
    NESTED_DENSE_BAILOUTS.get()
}

#[cfg(test)]
pub(super) fn test_nested_dense_discovery_work() -> usize {
    NESTED_DENSE_DISCOVERY_WORK.get()
}

#[cfg(test)]
pub(super) fn test_dynamic_dense_compilations() -> usize {
    DYNAMIC_DENSE_COMPILATIONS.get()
}

#[cfg(test)]
pub(super) fn test_checked_array_index_product(left: usize, right: usize) -> Option<usize> {
    legacy::test_checked_array_index_product(left, right)
}

#[cfg(test)]
pub(super) fn test_checked_next_array_index(index: usize, step: usize) -> Option<usize> {
    legacy::test_checked_next_array_index(index, step)
}

#[cfg(test)]
pub(super) fn test_legacy_direct_this_array_source_resolves(value: &Value, key: &str) -> bool {
    legacy::test_direct_this_own_data_array_resolves(value, key)
}

#[cfg(test)]
fn record_compact_dynamic_attempt() {
    COMPACT_DYNAMIC_ATTEMPTS.set(COMPACT_DYNAMIC_ATTEMPTS.get() + 1);
}

#[cfg(test)]
fn record_compact_dynamic_decline() {
    COMPACT_DYNAMIC_DECLINES.set(COMPACT_DYNAMIC_DECLINES.get() + 1);
}

#[cfg(test)]
fn record_compact_dynamic_hit() {
    COMPACT_DYNAMIC_HITS.set(COMPACT_DYNAMIC_HITS.get() + 1);
}

#[cfg(test)]
fn record_compact_dynamic_suppression() {
    COMPACT_DYNAMIC_SUPPRESSIONS.set(COMPACT_DYNAMIC_SUPPRESSIONS.get() + 1);
}

fn record_reduction_path_hit() {
    #[cfg(test)]
    REDUCTION_PATH_HITS.set(REDUCTION_PATH_HITS.get() + 1);
}

fn record_reduction_iteration() {
    #[cfg(test)]
    REDUCTION_ITERATIONS.set(REDUCTION_ITERATIONS.get() + 1);
}

#[cfg(test)]
fn record_exact_index_reduction_path_hit() {
    EXACT_INDEX_REDUCTION_PATH_HITS.set(EXACT_INDEX_REDUCTION_PATH_HITS.get() + 1);
}

#[cfg(test)]
fn record_shared_sample_stride_reduction_path_hit() {
    SHARED_SAMPLE_STRIDE_REDUCTION_PATH_HITS
        .set(SHARED_SAMPLE_STRIDE_REDUCTION_PATH_HITS.get() + 1);
}

#[inline]
fn record_typed_array_dense_path_hit() {
    #[cfg(test)]
    TYPED_ARRAY_DENSE_PATH_HITS.set(TYPED_ARRAY_DENSE_PATH_HITS.get() + 1);
}

#[inline]
fn record_typed_array_dense_suppression() {
    #[cfg(test)]
    TYPED_ARRAY_DENSE_SUPPRESSIONS.set(TYPED_ARRAY_DENSE_SUPPRESSIONS.get() + 1);
}

#[inline]
fn record_typed_array_dense_attempt() {
    #[cfg(test)]
    TYPED_ARRAY_DENSE_ATTEMPTS.set(TYPED_ARRAY_DENSE_ATTEMPTS.get() + 1);
}

#[inline]
fn record_hole_tail_append_attempt() {
    #[cfg(test)]
    HOLE_TAIL_APPEND_ATTEMPTS.set(HOLE_TAIL_APPEND_ATTEMPTS.get() + 1);
}

#[inline]
fn record_hole_tail_append_path_hit() {
    #[cfg(test)]
    HOLE_TAIL_APPEND_PATH_HITS.set(HOLE_TAIL_APPEND_PATH_HITS.get() + 1);
}

#[inline]
fn record_hole_tail_append_iteration() {
    #[cfg(test)]
    HOLE_TAIL_APPEND_ITERATIONS.set(HOLE_TAIL_APPEND_ITERATIONS.get() + 1);
}

#[cfg(test)]
#[inline]
fn record_hole_tail_append_staged_discard() {
    HOLE_TAIL_APPEND_STAGED_DISCARDS.set(HOLE_TAIL_APPEND_STAGED_DISCARDS.get() + 1);
}

#[inline]
fn record_nested_dense_entry() {
    #[cfg(test)]
    NESTED_DENSE_ENTRIES.set(NESTED_DENSE_ENTRIES.get() + 1);
}

#[inline]
fn record_nested_dense_outer_completion() {
    #[cfg(test)]
    NESTED_DENSE_OUTER_COMPLETIONS.set(NESTED_DENSE_OUTER_COMPLETIONS.get() + 1);
}

#[inline]
fn record_nested_dense_inner_commit() {
    #[cfg(test)]
    NESTED_DENSE_INNER_COMMITS.set(NESTED_DENSE_INNER_COMMITS.get() + 1);
}

#[inline]
fn record_nested_dense_seeded_iteration() {
    #[cfg(test)]
    NESTED_DENSE_SEEDED_ITERATIONS.set(NESTED_DENSE_SEEDED_ITERATIONS.get() + 1);
}

#[inline]
fn record_nested_dense_bailout() {
    #[cfg(test)]
    NESTED_DENSE_BAILOUTS.set(NESTED_DENSE_BAILOUTS.get() + 1);
}

#[inline]
pub(super) fn record_nested_dense_discovery_work(_amount: usize) {
    #[cfg(test)]
    NESTED_DENSE_DISCOVERY_WORK.set(NESTED_DENSE_DISCOVERY_WORK.get() + _amount);
}

#[inline]
fn record_dynamic_dense_compilation() {
    #[cfg(test)]
    DYNAMIC_DENSE_COMPILATIONS.set(DYNAMIC_DENSE_COMPILATIONS.get() + 1);
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

#[inline]
fn record_countdown_path_hit() {
    #[cfg(test)]
    COUNTDOWN_DENSE_PATH_HITS.set(COUNTDOWN_DENSE_PATH_HITS.get() + 1);
}

#[inline]
fn record_countdown_iteration() {
    #[cfg(test)]
    COUNTDOWN_DENSE_ITERATIONS.set(COUNTDOWN_DENSE_ITERATIONS.get() + 1);
}

#[inline]
fn record_writable_lease_suppression() {
    #[cfg(test)]
    WRITABLE_DENSE_LEASE_SUPPRESSIONS.set(WRITABLE_DENSE_LEASE_SUPPRESSIONS.get() + 1);
}

#[inline]
fn record_writable_path_hit() {
    #[cfg(test)]
    WRITABLE_DENSE_PATH_HITS.set(WRITABLE_DENSE_PATH_HITS.get() + 1);
}

#[inline]
fn record_math_round_operation() {
    #[cfg(test)]
    MATH_ROUND_OPERATIONS.set(MATH_ROUND_OPERATIONS.get() + 1);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DenseNumericMutationLoopRun {
    Handled,
    Declined,
    Suppress,
}

impl DenseNumericMutationLoopRun {
    fn from_handled(handled: bool) -> Self {
        if handled {
            Self::Handled
        } else {
            Self::Declined
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct DenseNumericMutationLoopPlan {
    exit: usize,
    kind: DensePlanKind,
}

#[derive(Clone, Debug)]
enum DensePlanKind {
    Fixed(FixedDensePlan),
    LegacyDynamic(legacy::LegacyDynamicDensePlan),
    LegacySuppressingDynamic(legacy::LegacyDynamicDensePlan),
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
    control: DynamicControl,
    receiver_sources: Vec<ArraySource>,
    number_sources: Vec<OwnDataSource>,
    local_slots: Vec<usize>,
    operations: Vec<NumberInstruction>,
    writes: Vec<LocalWrite>,
    store_count: usize,
    sunk_store: Option<SunkDenseStore>,
    hole_tail_append: Option<HoleTailAppendPlan>,
    uses_math_round: bool,
    header: usize,
}

#[derive(Clone, Debug)]
struct ScalarDenseProgram {
    local_slots: Vec<usize>,
    operations: Vec<NumberInstruction>,
    writes: Vec<LocalWrite>,
    invalidations: Vec<usize>,
}

#[derive(Clone, Debug)]
enum DynamicControl {
    LessThan(DynamicLimit),
    AtLeastZero,
    Countdown,
}

impl DynamicControl {
    fn limit(&self) -> Option<&DynamicLimit> {
        match self {
            Self::LessThan(limit) => Some(limit),
            Self::AtLeastZero | Self::Countdown => None,
        }
    }

    fn is_countdown(&self) -> bool {
        matches!(self, Self::Countdown)
    }
}

type Register = usize;

#[derive(Clone, Debug)]
enum NumberInstruction {
    Constant(f64),
    LoadLocal(usize),
    LoadInvariant(usize),
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
    MathRound {
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
    pub(super) fn compile_fixed_only(
        bytecode: &Bytecode,
        header: usize,
        backedge: usize,
    ) -> Option<Self> {
        compile_fixed(bytecode, header, backedge)
    }

    pub(super) fn compile_dynamic_only(
        bytecode: &Bytecode,
        header: usize,
        backedge: usize,
    ) -> Option<Self> {
        compiler::compile_dynamic(bytecode, header, backedge)
            .map(|(exit, plan)| Self::from_dynamic(exit, plan))
    }

    fn from_dynamic(exit: usize, plan: DynamicDensePlan) -> Self {
        let kind = if legacy::LegacyDynamicDensePlan::supports(&plan) {
            let requires_suppression = plan.store_count > 1;
            let plan = legacy::LegacyDynamicDensePlan::from_extended(plan);
            if requires_suppression {
                DensePlanKind::LegacySuppressingDynamic(plan)
            } else {
                DensePlanKind::LegacyDynamic(plan)
            }
        } else {
            DensePlanKind::Dynamic(plan)
        };
        Self { exit, kind }
    }

    pub(super) fn exit(&self) -> usize {
        self.exit
    }

    #[cfg(test)]
    pub(super) fn is_legacy_dynamic(&self) -> bool {
        matches!(self.kind, DensePlanKind::LegacyDynamic(_))
    }

    #[cfg(test)]
    pub(super) fn is_suppressing_legacy_dynamic(&self) -> bool {
        matches!(self.kind, DensePlanKind::LegacySuppressingDynamic(_))
    }

    #[cfg(test)]
    pub(super) fn is_legacy_reduction(&self) -> bool {
        matches!(
            &self.kind,
            DensePlanKind::LegacyDynamic(plan) if plan.is_reduction()
        )
    }

    #[cfg(test)]
    pub(super) fn is_two_lane_strided_reduction(&self) -> bool {
        matches!(
            &self.kind,
            DensePlanKind::LegacyDynamic(plan) if plan.is_two_lane_strided_reduction()
        )
    }

    pub(super) fn try_run(&self, vm: &mut Vm<'_>) -> DenseNumericMutationLoopRun {
        match &self.kind {
            DensePlanKind::Fixed(plan) => {
                DenseNumericMutationLoopRun::from_handled(plan.try_run(vm, self.exit))
            }
            DensePlanKind::LegacyDynamic(plan) => plan.try_run(vm, self.exit),
            DensePlanKind::LegacySuppressingDynamic(plan) => {
                plan.try_run_suppressing(vm, self.exit)
            }
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
    #[inline(never)]
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

impl DynamicDensePlan {
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
        invariant_numbers: &[f64],
        registers: &mut [f64],
        limit: Option<f64>,
    ) -> DynamicProgramRun {
        let mut made_progress = false;
        loop {
            let counter = locals[self.counter_local];
            let countdown_old = match self.control {
                DynamicControl::LessThan(_) => {
                    let limit = limit.expect("less-than controls always resolve a limit");
                    if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
                        return DynamicProgramRun {
                            deoptimized: false,
                            made_progress,
                        };
                    }
                    None
                }
                DynamicControl::AtLeastZero => {
                    if counter < 0.0 {
                        return DynamicProgramRun {
                            deoptimized: false,
                            made_progress,
                        };
                    }
                    None
                }
                DynamicControl::Countdown => {
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
                    NumberInstruction::LoadInvariant(source) => invariant_numbers[source],
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
                    NumberInstruction::MathRound { value } => {
                        record_math_round_operation();
                        crate::math::round_number(registers[value])
                    }
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
        invariant_numbers: &[f64],
        registers: &mut [f64],
        limit: Option<f64>,
    ) -> Option<DynamicProgramRun> {
        let append = self.hole_tail_append?;
        record_hole_tail_append_attempt();
        let writer_receiver = append.writer_receiver();
        let writer = arrays.get(writer_receiver)?;
        let start_index = array_index_from_number(locals[self.counter_local])?;

        // ArrayRef owns the storage/integrity checks, but only the VM can
        // validate the effective realm prototype chain. Custom prototypes are
        // conservatively rejected; the standard chain is walked in full when
        // either intrinsic link has changed.
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
                self.run_program(&mut access, locals, invariant_numbers, registers, limit)
            },
        );
        if ran.is_some_and(|run| run.made_progress) {
            record_hole_tail_append_path_hit();
        }
        ran
    }

    #[inline(never)]
    fn try_run(&self, vm: &mut Vm<'_>, exit: usize) -> DenseNumericMutationLoopRun {
        if vm.direct_eval_with_stack {
            return DenseNumericMutationLoopRun::Declined;
        }
        if self.uses_math_round && !native_math_round_is_current(vm) {
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
            .chain(
                self.number_sources
                    .iter()
                    .filter_map(|source| source.owner.local_slot()),
            )
            .chain(
                self.control
                    .limit()
                    .and_then(DynamicLimit::additional_authority_slot),
            )
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
        if matches!(&self.control, DynamicControl::AtLeastZero) {
            let counter = locals[self.counter_local];
            if !descending_counter_is_valid(counter) {
                return DenseNumericMutationLoopRun::Declined;
            }
        }
        let invariant_numbers = self
            .number_sources
            .iter()
            .map(|source| source.resolve_number(vm))
            .collect::<Option<Vec<_>>>();
        let Some(invariant_numbers) = invariant_numbers else {
            return DenseNumericMutationLoopRun::Declined;
        };
        let limit = match &self.control {
            DynamicControl::LessThan(DynamicLimit::LocalNumber(local)) => Some(locals[*local]),
            DynamicControl::LessThan(DynamicLimit::LocalArrayLength(slot)) => {
                match vm.locals.get(*slot) {
                    Some(Some(Value::Array(array))) => Some(array.len() as f64),
                    _ => return DenseNumericMutationLoopRun::Declined,
                }
            }
            DynamicControl::LessThan(DynamicLimit::OwnDataNumber(source)) => {
                let Some(limit) = source.resolve_number(vm) else {
                    return DenseNumericMutationLoopRun::Declined;
                };
                Some(limit)
            }
            DynamicControl::AtLeastZero => None,
            DynamicControl::Countdown => None,
        };
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
                .map(|source| source.resolve(vm))
                .collect::<Option<Vec<_>>>()
            else {
                record_read_only_bailout();
                return DenseNumericMutationLoopRun::Declined;
            };
            let ran = ArrayRef::with_dense_readable_element_sets(&arrays, |elements| {
                let mut access = ReadAccess { elements };
                self.run_program(
                    &mut access,
                    &mut locals,
                    &invariant_numbers,
                    registers,
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
            let Some(array) = self.receiver_sources[0].resolve(vm) else {
                return DenseNumericMutationLoopRun::Declined;
            };
            let mut ran = array.with_dense_writable_elements(|elements| {
                let mut access = SingleAccess {
                    elements,
                    pending: None,
                };
                self.run_program(
                    &mut access,
                    &mut locals,
                    &invariant_numbers,
                    registers,
                    limit,
                )
            });
            if ran.is_none() {
                ran = self.try_run_hole_tail_append(
                    vm,
                    std::slice::from_ref(&array),
                    &mut locals,
                    &invariant_numbers,
                    registers,
                    limit,
                );
            }
            if ran.is_some_and(|run| run.made_progress) {
                record_single_path_hit();
            }
            if ran.is_none() {
                record_writable_lease_suppression();
                return DenseNumericMutationLoopRun::Suppress;
            }
            ran
        } else {
            let Some(arrays) = self
                .receiver_sources
                .iter()
                .map(|source| source.resolve(vm))
                .collect::<Option<Vec<_>>>()
            else {
                return DenseNumericMutationLoopRun::Declined;
            };
            let mut ran = ArrayRef::with_distinct_dense_writable_elements(&arrays, |elements| {
                let mut access = MultiAccess {
                    elements,
                    pending: Vec::with_capacity(self.store_count),
                };
                self.run_program(
                    &mut access,
                    &mut locals,
                    &invariant_numbers,
                    registers,
                    limit,
                )
            });
            if ran.is_none() {
                ran = self.try_run_hole_tail_append(
                    vm,
                    &arrays,
                    &mut locals,
                    &invariant_numbers,
                    registers,
                    limit,
                );
            }
            if ran.is_none() {
                record_writable_lease_suppression();
                return DenseNumericMutationLoopRun::Suppress;
            }
            ran
        };
        let Some(run) = ran else {
            return DenseNumericMutationLoopRun::Declined;
        };
        if self.control.is_countdown() && run.made_progress {
            record_countdown_path_hit();
        }
        if self.sunk_store.is_some() && run.made_progress {
            record_sunk_store_hit();
        }
        if self.store_count != 0 && run.made_progress {
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
        DenseNumericMutationLoopRun::Handled
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
