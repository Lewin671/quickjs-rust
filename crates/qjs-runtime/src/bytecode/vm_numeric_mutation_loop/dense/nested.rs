//! One-lease execution for a scalar counted loop surrounding one dense loop.
//!
//! The plan is attached to the inner backedge, after the interpreter has
//! committed one seed iteration. It finishes that inner loop, executes the
//! scalar epilogue, and then owns later outer iterations while retaining one
//! distinct-array lease. Every dense source iteration is still transactional:
//! all reads and writes are validated before its stores and local writes are
//! published.

use std::{collections::BTreeSet, rc::Rc};

use super::*;

#[derive(Clone, Debug)]
struct ScalarProgram {
    operations: Vec<NumberInstruction>,
    writes: Vec<LocalWrite>,
    invalidations: Vec<usize>,
}

#[derive(Clone, Debug)]
pub(in super::super) struct NestedDensePlan {
    outer_header: usize,
    outer_backedge: usize,
    outer_exit: usize,
    inner_header: usize,
    outer_counter: usize,
    outer_limit: usize,
    inner_counter: usize,
    inner_limit: usize,
    local_slots: Vec<usize>,
    receiver_slots: Vec<usize>,
    prelude: ScalarProgram,
    inner_operations: Vec<NumberInstruction>,
    inner_compact_program: Option<Box<CompactProgram>>,
    inner_compact_counter_write: Option<usize>,
    inner_writes: Vec<LocalWrite>,
    inner_counter_write: Register,
    inner_store_count: usize,
    epilogue: ScalarProgram,
    max_operations: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum NestedDensePlanRun {
    Handled,
    HandledAndSuppress,
    Suppress,
}

pub(in super::super) enum NestedDenseProbe {
    Nested {
        plan: NestedDensePlan,
        fallback: Rc<DenseNumericMutationLoopPlan>,
    },
    DenseOnly(DenseNumericMutationLoopPlan),
    NoDynamic,
}

#[derive(Clone, Copy, Debug)]
pub(in super::super) struct EnclosingOuter {
    header: usize,
    backedge: usize,
    exit: usize,
    body_start: usize,
    counter_slot: usize,
    limit_slot: usize,
}

#[derive(Clone, Copy)]
struct DiscoveryInterval {
    source_index: usize,
    header: usize,
    backedge: usize,
    counted: Option<EnclosingOuter>,
}

struct BackedgeFenwick {
    tree: Vec<Option<EnclosingOuter>>,
}

impl BackedgeFenwick {
    fn new(coordinate_count: usize) -> Self {
        Self {
            tree: vec![None; coordinate_count + 1],
        }
    }

    fn update(&mut self, mut coordinate: usize, candidate: EnclosingOuter) {
        coordinate += 1;
        while coordinate < self.tree.len() {
            record_nested_dense_discovery_work(1);
            self.tree[coordinate] = more_inner_outer(self.tree[coordinate], Some(candidate));
            coordinate += coordinate & coordinate.wrapping_neg();
        }
    }

    fn query_prefix(&self, mut length: usize) -> Option<EnclosingOuter> {
        let mut nearest = None;
        while length != 0 {
            record_nested_dense_discovery_work(1);
            nearest = more_inner_outer(nearest, self.tree[length]);
            length &= length - 1;
        }
        nearest
    }
}

#[derive(Clone, Copy)]
struct LocalBank {
    values: [f64; MAX_DENSE_LOCALS],
    valid: [bool; MAX_DENSE_LOCALS],
}

impl LocalBank {
    fn number(&self, local: usize) -> Option<f64> {
        self.valid[local].then_some(self.values[local])
    }

    fn write_number(&mut self, local: usize, value: f64) {
        self.values[local] = value;
        self.valid[local] = true;
    }

    fn invalidate(&mut self, local: usize) {
        self.valid[local] = false;
    }
}

enum RegionOutcome {
    Complete(LocalBank),
    ReplayInner(LocalBank),
    ReplayOuter(LocalBank),
}

impl NestedDensePlan {
    #[cold]
    #[inline(never)]
    pub(in super::super) fn probe(
        bytecode: &Bytecode,
        inner_header: usize,
        inner_backedge: usize,
        enclosing_outer: EnclosingOuter,
    ) -> NestedDenseProbe {
        let Some((inner_exit, inner)) =
            compiler::compile_dynamic(bytecode, inner_header, inner_backedge)
        else {
            return NestedDenseProbe::NoDynamic;
        };
        let plan = Self::compile_pretranslated(
            bytecode,
            inner_header,
            inner_backedge,
            enclosing_outer,
            inner_exit,
            &inner,
        );
        let fallback = DenseNumericMutationLoopPlan::from_dynamic(inner_exit, inner);
        match plan {
            Some(plan) => NestedDenseProbe::Nested {
                plan,
                fallback: Rc::new(fallback),
            },
            None => NestedDenseProbe::DenseOnly(fallback),
        }
    }

    fn compile_pretranslated(
        bytecode: &Bytecode,
        inner_header: usize,
        inner_backedge: usize,
        enclosing_outer: EnclosingOuter,
        inner_exit: usize,
        inner: &DynamicDensePlan,
    ) -> Option<Self> {
        let expected_inner_exit = inner_backedge.checked_add(1)?;
        if inner_exit != expected_inner_exit
            || inner.store_count < 2
            || inner.sunk_store.is_some()
            || inner.hole_tail_append.is_some()
            || inner.uses_math_round
            || !inner.number_sources.is_empty()
            || !inner
                .receiver_sources
                .iter()
                .all(|source| matches!(source, ArraySource::Local(_)))
        {
            return None;
        }
        let inner_limit = match &inner.control {
            DynamicControl::LessThan(DynamicLimit::LocalNumber(inner_limit)) => *inner_limit,
            _ => return None,
        };

        let EnclosingOuter {
            header: outer_header,
            backedge: outer_backedge,
            exit: outer_exit,
            body_start,
            counter_slot: outer_counter_slot,
            limit_slot: outer_limit_slot,
        } = enclosing_outer;
        if body_start > inner_header || inner_exit >= outer_backedge {
            return None;
        }
        let prelude = compiler::compile_scalar(bytecode, body_start, inner_header)?;
        let epilogue = compiler::compile_scalar(bytecode, inner_exit + 1, outer_backedge)?;

        let inner_counter_slot = *inner.local_slots.get(inner.counter_local)?;
        let inner_limit_slot = *inner.local_slots.get(inner_limit)?;
        let inner_counter_write = inner
            .writes
            .iter()
            .find(|write| write.local == inner.counter_local)?
            .value;
        if inner_counter_write >= inner.operations.len() {
            return None;
        }
        if !program_writes_slot(&prelude, inner_counter_slot)
            || program_writes_slot(&prelude, outer_counter_slot)
            || program_writes_slot(&prelude, outer_limit_slot)
            || inner_writes_slot(inner, outer_counter_slot)
            || inner_writes_slot(inner, outer_limit_slot)
            || inner_writes_slot(inner, inner_limit_slot)
            || program_writes_slot(&epilogue, outer_limit_slot)
            || count_slot_writes(
                &bytecode.code[body_start..outer_backedge],
                outer_counter_slot,
            ) != 1
            || count_slot_writes(&bytecode.code[body_start..outer_backedge], outer_limit_slot) != 0
        {
            return None;
        }

        let receiver_slots = inner
            .receiver_sources
            .iter()
            .map(|source| match source {
                ArraySource::Local(slot) => Some(*slot),
                ArraySource::OwnData(_) => None,
            })
            .collect::<Option<Vec<_>>>()?;
        if receiver_slots.contains(&outer_counter_slot)
            || receiver_slots.contains(&outer_limit_slot)
        {
            return None;
        }

        let mut local_slots = BTreeSet::new();
        local_slots.extend(prelude.local_slots.iter().copied());
        local_slots.extend(inner.local_slots.iter().copied());
        local_slots.extend(epilogue.local_slots.iter().copied());
        local_slots.insert(outer_counter_slot);
        local_slots.insert(outer_limit_slot);
        let local_slots: Vec<_> = local_slots.into_iter().collect();
        if local_slots.len() > MAX_DENSE_LOCALS {
            return None;
        }

        let outer_counter = local_slots.binary_search(&outer_counter_slot).ok()?;
        let outer_limit = local_slots.binary_search(&outer_limit_slot).ok()?;
        let inner_counter = local_slots.binary_search(&inner_counter_slot).ok()?;
        let inner_limit = local_slots.binary_search(&inner_limit_slot).ok()?;
        let prelude = remap_scalar(prelude, &local_slots)?;
        let epilogue = remap_scalar(epilogue, &local_slots)?;
        let mut inner_operations = inner.operations.clone();
        let mut inner_writes = inner.writes.clone();
        remap_operations(&mut inner_operations, &inner.local_slots, &local_slots)?;
        remap_writes(&mut inner_writes, &inner.local_slots, &local_slots)?;
        // The ordinary plan's compact program was lowered against its own
        // local bank. Re-lower only after nested local remapping so LoadLocal
        // operands and transactional writes address the merged bank.
        let inner_compact_program = CompactProgram::lower(
            &inner_operations,
            &inner_writes,
            None,
            &[inner_counter_write],
        )
        .map(Box::new);
        let inner_compact_counter_write = inner_compact_program
            .as_ref()
            .and_then(|program| program.number_output(inner_counter_write));
        let max_operations = prelude
            .operations
            .len()
            .max(inner_operations.len())
            .max(epilogue.operations.len());

        let plan = Self {
            outer_header,
            outer_backedge,
            outer_exit,
            inner_header,
            outer_counter,
            outer_limit,
            inner_counter,
            inner_limit,
            local_slots,
            receiver_slots,
            prelude,
            inner_operations,
            inner_compact_program,
            inner_compact_counter_write,
            inner_writes,
            inner_counter_write,
            inner_store_count: inner.store_count,
            epilogue,
            max_operations,
        };
        Some(plan)
    }

    pub(in super::super) fn exit(&self) -> usize {
        self.outer_exit
    }

    pub(in super::super) fn contains_instruction(&self, ip: usize) -> bool {
        (self.outer_header..=self.outer_backedge).contains(&ip)
    }

    #[cold]
    #[inline(never)]
    pub(in super::super) fn try_run(&self, vm: &mut Vm<'_>) -> NestedDensePlanRun {
        if vm.direct_eval_with_stack {
            return NestedDensePlanRun::Suppress;
        }
        for slot in self
            .local_slots
            .iter()
            .copied()
            .chain(self.receiver_slots.iter().copied())
        {
            if !vm.slot_is_authoritative(slot) {
                return NestedDensePlanRun::Suppress;
            }
        }

        let mut bank = LocalBank {
            values: [0.0; MAX_DENSE_LOCALS],
            valid: [false; MAX_DENSE_LOCALS],
        };
        for (local, slot) in self.local_slots.iter().copied().enumerate() {
            match vm.local_slot_value(slot) {
                Some(Value::Number(value)) => bank.write_number(local, value),
                Some(Value::Undefined) => {}
                _ => return NestedDensePlanRun::Suppress,
            }
        }
        if !valid_counted_value(bank.number(self.outer_counter))
            || !valid_counted_value(bank.number(self.outer_limit))
            || !valid_counted_value(bank.number(self.inner_counter))
            || !valid_counted_value(bank.number(self.inner_limit))
        {
            return NestedDensePlanRun::Suppress;
        }
        let Some(arrays) = self
            .receiver_slots
            .iter()
            .map(|slot| match vm.locals.get(*slot) {
                Some(Some(Value::Array(array))) => Some(array.clone()),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
        else {
            return NestedDensePlanRun::Suppress;
        };

        let mut inline_registers = [0.0; INLINE_DENSE_OPS];
        let mut large_registers =
            (self.max_operations > INLINE_DENSE_OPS).then(|| vec![0.0; self.max_operations]);
        let registers = match large_registers.as_mut() {
            Some(registers) => registers.as_mut_slice(),
            None => &mut inline_registers[..self.max_operations],
        };
        let outcome = match (
            self.inner_compact_program.as_ref(),
            self.inner_compact_counter_write,
        ) {
            (Some(program), Some(counter_write)) => {
                let mut scratch = CompactScratch::new(program);
                let (numbers, words) = scratch.banks(program);
                ArrayRef::with_distinct_dense_writable_elements(&arrays, |elements| {
                    record_nested_dense_entry();
                    record_nested_dense_seeded_iteration();
                    let mut access = MultiAccess {
                        elements,
                        pending: Vec::with_capacity(self.inner_store_count),
                    };
                    self.run_region_with(
                        &mut access,
                        bank,
                        registers,
                        numbers,
                        words,
                        |plan, access, bank, _, numbers, words| {
                            plan.run_compact_inner_iteration(
                                program,
                                counter_write,
                                access,
                                bank,
                                numbers,
                                words,
                            )
                        },
                    )
                })
            }
            _ => ArrayRef::with_distinct_dense_writable_elements(&arrays, |elements| {
                record_nested_dense_entry();
                record_nested_dense_seeded_iteration();
                let mut access = MultiAccess {
                    elements,
                    pending: Vec::with_capacity(self.inner_store_count),
                };
                self.run_region_with(
                    &mut access,
                    bank,
                    registers,
                    &mut [],
                    &mut [],
                    |plan, access, bank, registers, _, _| {
                        plan.run_inner_iteration(access, bank, registers)
                    },
                )
            }),
        };
        let Some(outcome) = outcome else {
            return NestedDensePlanRun::Suppress;
        };

        match outcome {
            RegionOutcome::Complete(bank) => {
                self.publish_bank(vm, bank);
                vm.ip = self.outer_exit + 1;
                NestedDensePlanRun::Handled
            }
            RegionOutcome::ReplayInner(bank) => {
                record_nested_dense_bailout();
                self.publish_bank(vm, bank);
                vm.ip = self.inner_header;
                NestedDensePlanRun::HandledAndSuppress
            }
            RegionOutcome::ReplayOuter(bank) => {
                record_nested_dense_bailout();
                self.publish_bank(vm, bank);
                vm.ip = self.outer_header;
                NestedDensePlanRun::HandledAndSuppress
            }
        }
    }

    fn run_region_with<F>(
        &self,
        access: &mut MultiAccess<'_, '_>,
        mut bank: LocalBank,
        registers: &mut [f64],
        numbers: &mut [f64],
        words: &mut [u32],
        mut run_inner: F,
    ) -> RegionOutcome
    where
        F: FnMut(
            &Self,
            &mut MultiAccess<'_, '_>,
            &mut LocalBank,
            &mut [f64],
            &mut [f64],
            &mut [u32],
        ) -> bool,
    {
        // Entry is the first inner backedge: one interpreted iteration has
        // already committed in this outer iteration.
        let mut resumed_inner = true;
        loop {
            let outer_start = bank;
            if !resumed_inner {
                let Some(continues) = less_than(&bank, self.outer_counter, self.outer_limit) else {
                    return RegionOutcome::ReplayOuter(outer_start);
                };
                if !continues {
                    return RegionOutcome::Complete(bank);
                }
                if !run_scalar(&self.prelude, &mut bank, registers)
                    || !valid_counted_value(bank.number(self.inner_counter))
                    || !valid_counted_value(bank.number(self.inner_limit))
                {
                    return RegionOutcome::ReplayOuter(outer_start);
                }
            }

            let mut native_inner_commits = 0usize;
            loop {
                let Some(continues) = less_than(&bank, self.inner_counter, self.inner_limit) else {
                    return if resumed_inner || native_inner_commits != 0 {
                        RegionOutcome::ReplayInner(bank)
                    } else {
                        RegionOutcome::ReplayOuter(outer_start)
                    };
                };
                if !continues {
                    if !resumed_inner && native_inner_commits == 0 {
                        // The skipped source loop must publish an Undefined
                        // completion temporary, which the numeric bank cannot
                        // fabricate speculatively. Replay this untouched outer.
                        return RegionOutcome::ReplayOuter(outer_start);
                    }
                    break;
                }
                if !run_inner(self, access, &mut bank, registers, numbers, words) {
                    return if resumed_inner || native_inner_commits != 0 {
                        RegionOutcome::ReplayInner(bank)
                    } else {
                        RegionOutcome::ReplayOuter(outer_start)
                    };
                }
                native_inner_commits += 1;
                record_nested_dense_inner_commit();
            }

            let old_outer = bank
                .number(self.outer_counter)
                .expect("validated outer counter remains a Number");
            if !run_scalar(&self.epilogue, &mut bank, registers)
                || bank.number(self.outer_counter) != Some(old_outer + 1.0)
                || !valid_counted_value(bank.number(self.outer_counter))
            {
                return RegionOutcome::ReplayInner(bank);
            }
            record_nested_dense_outer_completion();
            resumed_inner = false;
        }
    }

    fn run_inner_iteration(
        &self,
        access: &mut MultiAccess<'_, '_>,
        bank: &mut LocalBank,
        registers: &mut [f64],
    ) -> bool {
        let old_counter = match bank.number(self.inner_counter) {
            Some(counter) => counter,
            None => return false,
        };
        access.reset_iteration();
        for (register, operation) in self.inner_operations.iter().enumerate() {
            let Some(value) = run_operation(operation, register, access, bank, registers) else {
                return false;
            };
            registers[register] = value;
        }
        debug_assert_eq!(access.staged_store_count(), self.inner_store_count);
        let next_counter = registers[self.inner_counter_write];
        if !valid_counted_value(Some(next_counter)) || next_counter <= old_counter {
            return false;
        }

        access.commit_stores();
        for write in &self.inner_writes {
            bank.write_number(write.local, registers[write.value]);
        }
        true
    }

    fn run_compact_inner_iteration(
        &self,
        program: &CompactProgram,
        counter_write: usize,
        access: &mut MultiAccess<'_, '_>,
        bank: &mut LocalBank,
        numbers: &mut [f64],
        words: &mut [u32],
    ) -> bool {
        let old_counter = match bank.number(self.inner_counter) {
            Some(counter) => counter,
            None => return false,
        };
        access.reset_iteration();
        if !program.run_iteration(access, |local| bank.number(local), &[], numbers, words) {
            return false;
        }
        debug_assert_eq!(access.staged_store_count(), self.inner_store_count);
        let next_counter = numbers[counter_write];
        if !valid_counted_value(Some(next_counter)) || next_counter <= old_counter {
            return false;
        }

        access.commit_stores();
        for write in program.writes() {
            bank.write_number(write.local, numbers[write.value]);
        }
        record_compact_word_iteration();
        true
    }

    fn publish_bank(&self, vm: &mut Vm<'_>, bank: LocalBank) {
        for (local, slot) in self.local_slots.iter().copied().enumerate() {
            vm.locals[slot] = Some(if bank.valid[local] {
                Value::Number(bank.values[local])
            } else {
                Value::Undefined
            });
        }
    }
}

pub(in super::super) fn discover_enclosing_outers(
    bytecode: &Bytecode,
    backedges: &[(usize, usize)],
) -> Vec<Option<EnclosingOuter>> {
    let intervals: Vec<_> = backedges
        .iter()
        .copied()
        .enumerate()
        .map(|(source_index, (header, backedge))| {
            record_nested_dense_discovery_work(1);
            DiscoveryInterval {
                source_index,
                header,
                backedge,
                counted: compile_enclosing_outer(bytecode, header, backedge),
            }
        })
        .collect();
    discover_enclosing_intervals(intervals)
}

fn discover_enclosing_intervals(intervals: Vec<DiscoveryInterval>) -> Vec<Option<EnclosingOuter>> {
    let mut coordinates: Vec<_> = intervals
        .iter()
        .filter_map(|interval| interval.counted.map(|outer| outer.backedge))
        .collect();
    coordinates.sort_unstable_by(|left, right| {
        record_nested_dense_discovery_work(1);
        right.cmp(left)
    });
    record_nested_dense_discovery_work(coordinates.len());
    coordinates.dedup();

    let mut order: Vec<_> = (0..intervals.len()).collect();
    order.sort_unstable_by(|left, right| {
        record_nested_dense_discovery_work(1);
        intervals[*left]
            .header
            .cmp(&intervals[*right].header)
            .then_with(|| {
                intervals[*left]
                    .source_index
                    .cmp(&intervals[*right].source_index)
            })
    });

    let mut enclosing = vec![None; intervals.len()];
    let mut fenwick = BackedgeFenwick::new(coordinates.len());
    let mut group_start = 0;
    while group_start < order.len() {
        let header = intervals[order[group_start]].header;
        let mut group_end = group_start + 1;
        while group_end < order.len() && intervals[order[group_end]].header == header {
            record_nested_dense_discovery_work(1);
            group_end += 1;
        }

        // Query the whole header group before publishing any of its counted
        // loops, so equal-header continue edges can never become ancestors.
        for index in &order[group_start..group_end] {
            record_nested_dense_discovery_work(1);
            let interval = intervals[*index];
            let inner_exit = interval.backedge.saturating_add(1);
            let eligible = descending_prefix_greater_than(&coordinates, inner_exit);
            enclosing[interval.source_index] = fenwick.query_prefix(eligible);
        }
        for index in &order[group_start..group_end] {
            let Some(candidate) = intervals[*index].counted else {
                continue;
            };
            let coordinate = descending_coordinate(&coordinates, candidate.backedge)
                .expect("counted backedge was used to build coordinates");
            fenwick.update(coordinate, candidate);
        }
        group_start = group_end;
    }
    enclosing
}

fn more_inner_outer(
    left: Option<EnclosingOuter>,
    right: Option<EnclosingOuter>,
) -> Option<EnclosingOuter> {
    match (left, right) {
        (None, candidate) | (candidate, None) => candidate,
        (Some(left), Some(right)) => Some(
            if (right.header, std::cmp::Reverse(right.backedge))
                > (left.header, std::cmp::Reverse(left.backedge))
            {
                right
            } else {
                left
            },
        ),
    }
}

fn descending_prefix_greater_than(coordinates: &[usize], threshold: usize) -> usize {
    let mut low = 0;
    let mut high = coordinates.len();
    while low < high {
        record_nested_dense_discovery_work(1);
        let middle = low + (high - low) / 2;
        if coordinates[middle] > threshold {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    low
}

fn descending_coordinate(coordinates: &[usize], target: usize) -> Option<usize> {
    let mut low = 0;
    let mut high = coordinates.len();
    while low < high {
        record_nested_dense_discovery_work(1);
        let middle = low + (high - low) / 2;
        match coordinates[middle].cmp(&target) {
            std::cmp::Ordering::Greater => low = middle + 1,
            std::cmp::Ordering::Less => high = middle,
            std::cmp::Ordering::Equal => return Some(middle),
        }
    }
    None
}

#[cfg(test)]
pub(in super::super) fn test_discover_enclosing_intervals(
    backedges: &[(usize, usize)],
    counted: &[bool],
) -> Vec<Option<(usize, usize)>> {
    assert_eq!(backedges.len(), counted.len());
    let intervals = backedges
        .iter()
        .copied()
        .zip(counted.iter().copied())
        .enumerate()
        .map(
            |(source_index, ((header, backedge), counted))| DiscoveryInterval {
                source_index,
                header,
                backedge,
                counted: counted.then_some(EnclosingOuter {
                    header,
                    backedge,
                    exit: backedge + 1,
                    body_start: header + 5,
                    counter_slot: source_index * 2,
                    limit_slot: source_index * 2 + 1,
                }),
            },
        )
        .collect();
    discover_enclosing_intervals(intervals)
        .into_iter()
        .map(|outer| outer.map(|outer| (outer.header, outer.backedge)))
        .collect()
}

#[cfg(test)]
type TestLoopInterval = (usize, usize);

#[cfg(test)]
type TestEnclosingIntervals = Vec<(TestLoopInterval, Option<TestLoopInterval>)>;

#[cfg(test)]
pub(in super::super) fn test_discover_enclosing_bytecode(
    bytecode: &Bytecode,
) -> TestEnclosingIntervals {
    let backedges: Vec<_> = bytecode
        .code
        .iter()
        .enumerate()
        .filter_map(|(backedge, op)| match op {
            Op::Jump(header) if *header < backedge => Some((*header, backedge)),
            _ => None,
        })
        .collect();
    let enclosing = discover_enclosing_outers(bytecode, &backedges);
    backedges
        .into_iter()
        .zip(enclosing)
        .map(|(inner, outer)| (inner, outer.map(|outer| (outer.header, outer.backedge))))
        .collect()
}

fn compile_enclosing_outer(
    bytecode: &Bytecode,
    outer_header: usize,
    outer_backedge: usize,
) -> Option<EnclosingOuter> {
    let code = &bytecode.code;
    let Op::Jump(jump_header) = code.get(outer_backedge)? else {
        return None;
    };
    if *jump_header != outer_header {
        return None;
    }
    let (
        Op::LoadLocal(counter),
        Op::LoadLocal(limit),
        Op::Binary(BinaryOp::Lt),
        Op::JumpIfFalse(exit),
        Op::Pop,
    ) = (
        code.get(outer_header)?,
        code.get(outer_header + 1)?,
        code.get(outer_header + 2)?,
        code.get(outer_header + 3)?,
        code.get(outer_header + 4)?,
    )
    else {
        return None;
    };
    let tail = outer_backedge.checked_sub(6)?;
    if *exit != outer_backedge + 1
        || !matches!(code.get(*exit), Some(Op::Pop))
        || !matches!(code.get(tail), Some(Op::LoadLocal(slot)) if slot == counter)
        || !matches!(code.get(tail + 1), Some(Op::ToNumeric))
        || !matches!(code.get(tail + 2), Some(Op::Dup))
        || !matches!(code.get(tail + 3), Some(Op::Update(UpdateOp::Increment)))
        || !matches!(code.get(tail + 4), Some(Op::AssignLocal(slot)) if slot == counter)
        || !matches!(code.get(tail + 5), Some(Op::Pop))
        || counter == limit
    {
        return None;
    }
    Some(EnclosingOuter {
        header: outer_header,
        backedge: outer_backedge,
        exit: *exit,
        body_start: outer_header + 5,
        counter_slot: *counter,
        limit_slot: *limit,
    })
}

fn count_slot_writes(code: &[Op], slot: usize) -> usize {
    code.iter()
        .filter(
            |op| matches!(op, Op::StoreLocal(target) | Op::AssignLocal(target) if *target == slot),
        )
        .count()
}

fn program_writes_slot(program: &ScalarDenseProgram, slot: usize) -> bool {
    program
        .writes
        .iter()
        .any(|write| program.local_slots.get(write.local).copied() == Some(slot))
        || program
            .invalidations
            .iter()
            .any(|local| program.local_slots.get(*local).copied() == Some(slot))
}

fn inner_writes_slot(plan: &DynamicDensePlan, slot: usize) -> bool {
    plan.writes
        .iter()
        .any(|write| plan.local_slots.get(write.local).copied() == Some(slot))
}

fn remap_scalar(program: ScalarDenseProgram, locals: &[usize]) -> Option<ScalarProgram> {
    let ScalarDenseProgram {
        local_slots,
        mut operations,
        mut writes,
        mut invalidations,
    } = program;
    remap_operations(&mut operations, &local_slots, locals)?;
    remap_writes(&mut writes, &local_slots, locals)?;
    for local in &mut invalidations {
        let slot = *local_slots.get(*local)?;
        *local = locals.binary_search(&slot).ok()?;
    }
    Some(ScalarProgram {
        operations,
        writes,
        invalidations,
    })
}

fn remap_operations(
    operations: &mut [NumberInstruction],
    old_locals: &[usize],
    new_locals: &[usize],
) -> Option<()> {
    for operation in operations {
        if let NumberInstruction::LoadLocal(local) = operation {
            let slot = *old_locals.get(*local)?;
            *local = new_locals.binary_search(&slot).ok()?;
        }
    }
    Some(())
}

fn remap_writes(
    writes: &mut [LocalWrite],
    old_locals: &[usize],
    new_locals: &[usize],
) -> Option<()> {
    for write in writes {
        let slot = *old_locals.get(write.local)?;
        write.local = new_locals.binary_search(&slot).ok()?;
    }
    Some(())
}

fn valid_counted_value(value: Option<f64>) -> bool {
    value.is_some_and(|value| {
        value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= MAX_SAFE_INTEGER
    })
}

fn less_than(bank: &LocalBank, left: usize, right: usize) -> Option<bool> {
    Some(bank.number(left)? < bank.number(right)?)
}

fn run_scalar(program: &ScalarProgram, bank: &mut LocalBank, registers: &mut [f64]) -> bool {
    for (register, operation) in program.operations.iter().enumerate() {
        let value = match *operation {
            NumberInstruction::Constant(value) => value,
            NumberInstruction::LoadLocal(local) => match bank.number(local) {
                Some(value) => value,
                None => return false,
            },
            NumberInstruction::Binary {
                operation,
                left,
                right,
            } => match apply_binary(operation, registers[left], registers[right]) {
                Some(value) => value,
                None => return false,
            },
            NumberInstruction::Unary { operation, value } => {
                match apply_unary(operation, registers[value]) {
                    Some(value) => value,
                    None => return false,
                }
            }
            NumberInstruction::Update { operation, value } => match operation {
                UpdateOp::Increment => registers[value] + 1.0,
                UpdateOp::Decrement => registers[value] - 1.0,
            },
            NumberInstruction::LoadInvariant(_)
            | NumberInstruction::DenseLoad { .. }
            | NumberInstruction::DenseStore { .. }
            | NumberInstruction::MathRound { .. } => return false,
        };
        registers[register] = value;
    }
    for local in &program.invalidations {
        bank.invalidate(*local);
    }
    for write in &program.writes {
        bank.write_number(write.local, registers[write.value]);
    }
    true
}

fn run_operation<A: DenseAccess>(
    operation: &NumberInstruction,
    _register: usize,
    access: &mut A,
    bank: &LocalBank,
    registers: &[f64],
) -> Option<f64> {
    Some(match *operation {
        NumberInstruction::Constant(value) => value,
        NumberInstruction::LoadLocal(local) => bank.number(local)?,
        NumberInstruction::DenseLoad { receiver, index } => {
            let index = array_index_from_number(registers[index])?;
            access.load_number(receiver, index)?
        }
        NumberInstruction::DenseStore {
            receiver,
            index,
            value,
        } => {
            let index = array_index_from_number(registers[index])?;
            let value = registers[value];
            access
                .stage_store(receiver, index, value)
                .then_some(value)?
        }
        NumberInstruction::Binary {
            operation,
            left,
            right,
        } => apply_binary(operation, registers[left], registers[right])?,
        NumberInstruction::Unary { operation, value } => apply_unary(operation, registers[value])?,
        NumberInstruction::Update { operation, value } => match operation {
            UpdateOp::Increment => registers[value] + 1.0,
            UpdateOp::Decrement => registers[value] - 1.0,
        },
        NumberInstruction::LoadInvariant(_) | NumberInstruction::MathRound { .. } => return None,
    })
}
