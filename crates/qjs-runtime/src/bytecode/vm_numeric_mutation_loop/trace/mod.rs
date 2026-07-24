//! A prevalidated numeric trace for one reducible three-level dense loop nest.

use super::super::{ir::Bytecode, vm::Vm};
use super::dense::{ArraySource, TraceFallback};

mod compiler;
mod definedness;
mod executor;
mod ir;
mod kernel;
#[cfg(test)]
mod tests;

use ir::{CountedLoop, NumericProgram, Radix2NestProof, ReceiverRole};
use kernel::NumericDagKernel;

#[derive(Clone, Debug)]
pub(super) struct NumericTracePlan {
    inner_backedge: usize,
    outer: CountedLoop,
    local_slots: Vec<usize>,
    required_number_mask: u64,
    kernel_killed_locals: Vec<usize>,
    receiver_sources: Vec<ArraySource>,
    receiver_roles: Vec<ReceiverRole>,
    writable_receivers: Vec<usize>,
    readable_receivers: Vec<usize>,
    outer_prelude: NumericProgram,
    middle_prelude: NumericProgram,
    middle_epilogue: NumericProgram,
    outer_epilogue: NumericProgram,
    radix2: Radix2NestProof,
    kernel: NumericDagKernel,
    #[cfg(test)]
    metadata: NumericTraceMetadata,
}

pub(super) struct NumericTraceProbe {
    pub(super) plan: NumericTracePlan,
    pub(super) fallback: TraceFallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NumericTraceRun {
    DeclinedNoProgress,
    CompletedOuter,
}

impl NumericTracePlan {
    pub(super) fn compile(
        bytecode: &Bytecode,
        inner_header: usize,
        inner_backedge: usize,
    ) -> Option<NumericTraceProbe> {
        compiler::compile(bytecode, inner_header, inner_backedge)
    }

    pub(super) fn contains_instruction(&self, ip: usize) -> bool {
        (self.outer.header..=self.outer.backedge).contains(&ip)
    }

    pub(super) fn exit(&self) -> usize {
        self.outer.exit
    }

    pub(super) fn try_run(&self, vm: &mut Vm<'_>) -> NumericTraceRun {
        executor::try_run(self, vm)
    }
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(super) struct NumericTraceMetadata {
    pub(super) depth: usize,
    pub(super) inner_header: usize,
    pub(super) inner_backedge: usize,
    pub(super) middle_header: usize,
    pub(super) middle_backedge: usize,
    pub(super) outer_header: usize,
    pub(super) outer_backedge: usize,
    pub(super) outer_exit: usize,
    pub(super) writable_receivers: usize,
    pub(super) readable_receivers: usize,
    pub(super) materialized_live_out_alias_slots: Vec<usize>,
    pub(super) kernel_local_write_slots: Vec<usize>,
    pub(super) final_reaching_alias_dependencies: Vec<NumericTraceAliasDependency>,
    pub(super) post_handoff_read_slots: Vec<usize>,
    pub(super) kernel: kernel::NumericDagKernelMetadata,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NumericTraceSourceRegion {
    OuterPrelude,
    MiddlePrelude,
    Inner,
    MiddleEpilogue,
    OuterEpilogue,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NumericTraceAliasDependency {
    pub(super) region: NumericTraceSourceRegion,
    pub(super) target: usize,
    pub(super) source: usize,
}

#[cfg(test)]
impl NumericTracePlan {
    pub(super) fn test_metadata(&self) -> &NumericTraceMetadata {
        &self.metadata
    }
}

#[cfg(test)]
thread_local! {
    static TRACE_PLANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_ATTEMPTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_ENTRIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_NORMAL_EXITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_LEASE_BATCHES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_SEEDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_INNER_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_MIDDLE_COMPLETIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_OUTER_COMPLETIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_DECLINES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_INDEX_CONVERSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_NUMBER_LOADS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_READONLY_NUMBER_LOADS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_DIRECT_WRITES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_ENTRY_STACK_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TRACE_EXIT_STACK_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[derive(Default)]
pub(super) struct InvocationCounts {
    #[cfg(test)]
    inner_iterations: usize,
    #[cfg(test)]
    middle_completions: usize,
    #[cfg(test)]
    outer_completions: usize,
    #[cfg(test)]
    index_conversions: usize,
    #[cfg(test)]
    number_loads: usize,
    #[cfg(test)]
    readonly_number_loads: usize,
    #[cfg(test)]
    direct_writes: usize,
}

impl InvocationCounts {
    #[inline(always)]
    pub(super) fn inner_iteration(&mut self, conversions: usize, loads: usize, writes: usize) {
        #[cfg(test)]
        {
            self.inner_iterations += 1;
            self.index_conversions += conversions;
            self.number_loads += loads;
            self.direct_writes += writes;
        }
        #[cfg(not(test))]
        let _ = (conversions, loads, writes);
    }

    #[inline(always)]
    pub(super) fn readonly_number_load(&mut self) {
        #[cfg(test)]
        {
            self.readonly_number_loads += 1;
        }
    }

    #[inline(always)]
    pub(super) fn middle_completion(&mut self) {
        #[cfg(test)]
        {
            self.middle_completions += 1;
        }
    }

    #[inline(always)]
    pub(super) fn outer_completion(&mut self) {
        #[cfg(test)]
        {
            self.outer_completions += 1;
        }
    }
}

#[cfg(test)]
fn add(cell: &'static std::thread::LocalKey<std::cell::Cell<usize>>, amount: usize) {
    cell.set(cell.get() + amount);
}

pub(super) fn record_compiled_plan() {
    #[cfg(test)]
    add(&TRACE_PLANS, 1);
}

#[inline(always)]
fn record_attempt() {
    #[cfg(test)]
    add(&TRACE_ATTEMPTS, 1);
}

#[inline(always)]
fn record_entry(_stack_depth: usize) {
    #[cfg(test)]
    {
        add(&TRACE_ENTRIES, 1);
        add(&TRACE_SEEDS, 1);
        TRACE_ENTRY_STACK_DEPTH.set(_stack_depth);
    }
}

#[inline(always)]
fn record_lease_entry() {
    #[cfg(test)]
    add(&TRACE_LEASE_BATCHES, 1);
}

fn record_decline() {
    #[cfg(test)]
    add(&TRACE_DECLINES, 1);
}

fn record_completed(_counts: InvocationCounts, _stack_depth: usize) {
    #[cfg(test)]
    {
        add(&TRACE_NORMAL_EXITS, 1);
        add(&TRACE_INNER_ITERATIONS, _counts.inner_iterations);
        add(&TRACE_MIDDLE_COMPLETIONS, _counts.middle_completions);
        add(&TRACE_OUTER_COMPLETIONS, _counts.outer_completions);
        add(&TRACE_INDEX_CONVERSIONS, _counts.index_conversions);
        add(&TRACE_NUMBER_LOADS, _counts.number_loads);
        add(&TRACE_READONLY_NUMBER_LOADS, _counts.readonly_number_loads);
        add(&TRACE_DIRECT_WRITES, _counts.direct_writes);
        TRACE_EXIT_STACK_DEPTH.set(_stack_depth);
    }
}

#[cfg(test)]
pub(super) fn reset_test_counters() {
    for counter in [
        &TRACE_PLANS,
        &TRACE_ATTEMPTS,
        &TRACE_ENTRIES,
        &TRACE_NORMAL_EXITS,
        &TRACE_LEASE_BATCHES,
        &TRACE_SEEDS,
        &TRACE_INNER_ITERATIONS,
        &TRACE_MIDDLE_COMPLETIONS,
        &TRACE_OUTER_COMPLETIONS,
        &TRACE_DECLINES,
        &TRACE_INDEX_CONVERSIONS,
        &TRACE_NUMBER_LOADS,
        &TRACE_READONLY_NUMBER_LOADS,
        &TRACE_DIRECT_WRITES,
        &TRACE_ENTRY_STACK_DEPTH,
        &TRACE_EXIT_STACK_DEPTH,
    ] {
        counter.set(0);
    }
}

#[cfg(test)]
macro_rules! counter_getters {
    ($(($name:ident, $counter:ident)),+ $(,)?) => {$(
        pub(super) fn $name() -> usize {
            $counter.get()
        }
    )+};
}

#[cfg(test)]
counter_getters!(
    (test_plans, TRACE_PLANS),
    (test_attempts, TRACE_ATTEMPTS),
    (test_entries, TRACE_ENTRIES),
    (test_normal_exits, TRACE_NORMAL_EXITS),
    (test_lease_batches, TRACE_LEASE_BATCHES),
    (test_seed_count, TRACE_SEEDS),
    (test_native_inner_iterations, TRACE_INNER_ITERATIONS),
    (test_middle_completions, TRACE_MIDDLE_COMPLETIONS),
    (test_outer_completions, TRACE_OUTER_COMPLETIONS),
    (test_declines, TRACE_DECLINES),
    (test_index_conversions, TRACE_INDEX_CONVERSIONS),
    (test_number_loads, TRACE_NUMBER_LOADS),
    (test_readonly_number_loads, TRACE_READONLY_NUMBER_LOADS),
    (test_direct_writes, TRACE_DIRECT_WRITES),
    (test_entry_stack_depth, TRACE_ENTRY_STACK_DEPTH),
    (test_exit_stack_depth, TRACE_EXIT_STACK_DEPTH),
);
