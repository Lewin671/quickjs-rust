//! Fail-closed discovery and compilation for reducible three-level traces.

#[cfg(test)]
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use qjs_ast::BinaryOp;

use super::super::super::ir::{Bytecode, Op, decode_index_receiver};
#[cfg(test)]
use super::super::dense::LocalWrite;
use super::super::dense::{
    ArraySource, EnclosingOuter, NumberInstruction, TraceProgramSource, compile_trace_inner,
    compile_trace_program,
};
#[cfg(test)]
use super::{NumericTraceAliasDependency, NumericTraceMetadata, NumericTraceSourceRegion};
use super::{
    NumericTracePlan, NumericTraceProbe,
    definedness::{DefinednessInputs, required_entry_mask},
    ir::{CountedLoop, NumericProgram, Radix2NestProof, ReceiverRole},
    kernel::compile_numeric_dag_kernel,
    record_compiled_plan,
};

pub(super) fn compile(
    bytecode: &Bytecode,
    inner_header: usize,
    inner_backedge: usize,
) -> Option<NumericTraceProbe> {
    if bytecode.contains_direct_eval()
        || bytecode.contains_with()
        || bytecode.code.iter().any(|op| {
            matches!(
                op,
                Op::EnterWith
                    | Op::ExitWith
                    | Op::CallDirectEval { .. }
                    | Op::CallDirectEvalSpread { .. }
                    | Op::EnterDisposableScope
                    | Op::RegisterDisposable
                    | Op::RegisterAsyncDisposable
                    | Op::DisposeScope { .. }
                    | Op::EnterTry { .. }
                    | Op::ExitTry
                    | Op::EndFinally
                    | Op::DiscardPendingAbrupt
                    | Op::AbruptJump(_)
                    | Op::FreshIterationScope(_)
                    | Op::Yield
                    | Op::Await
                    | Op::YieldDelegate { .. }
            )
        })
    {
        return None;
    }

    let inner_interval = (inner_header, inner_backedge);
    let mut ancestors: Vec<_> = bytecode
        .code
        .iter()
        .enumerate()
        .filter_map(|(backedge, op)| match op {
            Op::Jump(header)
                if *header < inner_header
                    && inner_backedge < backedge
                    && (*header, backedge) != inner_interval =>
            {
                compile_counted_loop(bytecode, *header, backedge)
            }
            _ => None,
        })
        .collect();
    ancestors.sort_unstable_by_key(|candidate| candidate.backedge - candidate.header);
    let [middle, outer] = ancestors.as_slice() else {
        return None;
    };
    let middle = *middle;
    let outer = *outer;
    if outer.header >= middle.header
        || middle.header >= inner_header
        || inner_backedge >= middle.backedge
        || middle.backedge >= outer.backedge
        || !region_has_only_three_loop_branches(bytecode, inner_interval, middle, outer)
    {
        return None;
    }

    let expected_inner_exit = inner_backedge.checked_add(1)?;
    if middle.exit != middle.backedge.checked_add(1)?
        || outer.exit != outer.backedge.checked_add(1)?
        || outer.body_start > middle.header
        || middle.body_start > inner_header
        || expected_inner_exit >= middle.backedge
        || middle.exit >= outer.backedge
    {
        return None;
    }

    let outer_prelude_source = compile_trace_program(bytecode, outer.body_start, middle.header)?;
    let middle_prelude_source = compile_trace_program(bytecode, middle.body_start, inner_header)?;
    let middle_epilogue_source =
        compile_trace_program(bytecode, expected_inner_exit + 1, middle.backedge)?;
    let outer_epilogue_source = compile_trace_program(bytecode, middle.exit + 1, outer.backedge)?;
    let mut successor_reads = BTreeSet::new();
    for source in [
        &outer_prelude_source,
        &middle_prelude_source,
        &middle_epilogue_source,
        &outer_epilogue_source,
    ] {
        extend_source_reads(&mut successor_reads, source)?;
    }
    successor_reads.extend([
        outer.counter_slot,
        outer.limit_slot,
        middle.counter_slot,
        middle.limit_slot,
    ]);
    #[cfg(test)]
    let post_handoff_reads = {
        let mut reads = BTreeSet::new();
        extend_post_handoff_reads(bytecode, outer.exit + 1, &mut reads)?;
        successor_reads.extend(reads.iter().copied());
        reads.into_iter().collect::<Vec<_>>()
    };
    #[cfg(not(test))]
    extend_post_handoff_reads(bytecode, outer.exit + 1, &mut successor_reads)?;

    let middle_fallback = EnclosingOuter::new(
        middle.header,
        middle.backedge,
        middle.exit,
        middle.body_start,
        middle.counter_slot,
        middle.limit_slot,
    );
    let (inner_exit, inner_source, fallback) = compile_trace_inner(
        bytecode,
        inner_header,
        inner_backedge,
        middle_fallback,
        &successor_reads,
    )?;
    if inner_exit != expected_inner_exit {
        return None;
    }
    let sources = [
        &outer_prelude_source,
        &middle_prelude_source,
        &middle_epilogue_source,
        &outer_epilogue_source,
        &inner_source.program,
    ];
    let inner_counter_slot = *inner_source
        .program
        .local_slots
        .get(inner_source.counter_local)?;
    let inner_limit_slot = *inner_source
        .program
        .local_slots
        .get(inner_source.limit_local)?;
    let counters = [outer.counter_slot, middle.counter_slot, inner_counter_slot];
    if counters
        .iter()
        .enumerate()
        .any(|(index, counter)| counters[..index].contains(counter))
        || middle.limit_slot != outer.counter_slot
        || inner_limit_slot != outer.limit_slot
        || outer.counter_slot == outer.limit_slot
        || middle.counter_slot == middle.limit_slot
        || inner_counter_slot == inner_limit_slot
        || count_slot_writes(
            &bytecode.code[outer.body_start..outer.backedge],
            outer.counter_slot,
        ) != 1
        || count_slot_writes(
            &bytecode.code[middle.body_start..middle.backedge],
            middle.counter_slot,
        ) != 1
        || count_slot_writes(
            &bytecode.code[inner_header..inner_backedge],
            inner_counter_slot,
        ) != 1
        || count_slot_writes(
            &bytecode.code[outer.body_start..outer.backedge],
            outer.limit_slot,
        ) != 0
    {
        return None;
    }

    let mut local_slots = BTreeSet::new();
    local_slots.extend(counters);
    local_slots.extend([outer.limit_slot, middle.limit_slot, inner_limit_slot]);
    for source in sources {
        local_slots.extend(source.local_slots.iter().copied());
    }
    let local_slots: Vec<_> = local_slots.into_iter().collect();
    if local_slots.len() > super::super::dense::MAX_DENSE_LOCALS {
        return None;
    }

    let mut receiver_sources = Vec::new();
    for source in sources {
        for receiver in &source.receiver_sources {
            if !matches!(receiver, ArraySource::Local(_)) {
                return None;
            }
            if !receiver_sources.contains(receiver) {
                receiver_sources.push(receiver.clone());
            }
        }
    }
    if receiver_sources.iter().any(|source| {
        source.local_slot().is_none_or(|slot| {
            local_slots.contains(&slot)
                || count_slot_writes(&bytecode.code[outer.body_start..outer.backedge], slot) != 0
        })
    }) {
        return None;
    }

    let outer_prelude = remap_program(outer_prelude_source, &local_slots, &receiver_sources)?;
    let middle_prelude = remap_program(middle_prelude_source, &local_slots, &receiver_sources)?;
    let middle_epilogue = remap_program(middle_epilogue_source, &local_slots, &receiver_sources)?;
    let outer_epilogue = remap_program(outer_epilogue_source, &local_slots, &receiver_sources)?;
    #[cfg(test)]
    let materialized_live_out_alias_slots = inner_source.materialized_live_out_alias_slots;
    let inner_counter_write = inner_source.counter_write;
    let inner_store_count = inner_source.store_count;
    let inner_program = remap_program(inner_source.program, &local_slots, &receiver_sources)?;
    let inner_counter = local_slots.binary_search(&inner_counter_slot).ok()?;
    let inner_limit = local_slots.binary_search(&inner_limit_slot).ok()?;
    let middle = remap_loop(middle, &local_slots)?;
    let outer = remap_loop(outer, &local_slots)?;
    let radix2 = matches_counted_schedule(
        &outer_prelude,
        &middle_prelude,
        &middle_epilogue,
        &outer_epilogue,
        outer,
        middle,
        inner_counter,
    )?;
    let kernel_read_locals: Vec<_> = inner_program
        .operations
        .iter()
        .filter_map(|operation| match operation {
            NumberInstruction::LoadLocal(local) => Some(*local),
            _ => None,
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let kernel_written_locals: Vec<_> = inner_program
        .writes
        .iter()
        .map(|write| write.local)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let kernel_killed_locals: Vec<_> = inner_program
        .invalidations
        .iter()
        .copied()
        .filter(|local| kernel_written_locals.binary_search(local).is_err())
        .collect();

    let mut writable = BTreeSet::new();
    for operation in &inner_program.operations {
        if let NumberInstruction::DenseStore { receiver, .. } = operation {
            writable.insert(*receiver);
        }
    }
    if writable.is_empty() {
        return None;
    }
    let writable_receivers: Vec<_> = writable.iter().copied().collect();
    let readable_receivers: Vec<_> = (0..receiver_sources.len())
        .filter(|receiver| !writable.contains(receiver))
        .collect();
    let receiver_roles = (0..receiver_sources.len())
        .map(|receiver| {
            writable_receivers
                .binary_search(&receiver)
                .map(ReceiverRole::Writable)
                .or_else(|_| {
                    readable_receivers
                        .binary_search(&receiver)
                        .map(ReceiverRole::Readable)
                })
                .ok()
        })
        .collect::<Option<Vec<_>>>()?;
    if [
        &outer_prelude,
        &middle_prelude,
        &middle_epilogue,
        &outer_epilogue,
    ]
    .into_iter()
    .flat_map(|program| &program.operations)
    .any(|operation| {
        matches!(operation, NumberInstruction::DenseLoad { receiver, .. } if writable.contains(receiver))
    })
    {
        return None;
    }

    let kernel = compile_numeric_dag_kernel(
        &inner_program.operations,
        &inner_program.writes,
        inner_counter_write,
        inner_store_count,
    )?;
    #[cfg(test)]
    let final_reaching_alias_dependencies = {
        let mut definitions = BTreeMap::new();
        // `writes` is the finalized one-write-per-local publication set. Only
        // direct LoadLocal values are alias edges. Process the true final-
        // iteration order so a later region replaces (or kills) an earlier
        // definition of the same target.
        for (region, program) in [
            (NumericTraceSourceRegion::OuterPrelude, &outer_prelude),
            (NumericTraceSourceRegion::MiddlePrelude, &middle_prelude),
            (NumericTraceSourceRegion::Inner, &inner_program),
            (NumericTraceSourceRegion::MiddleEpilogue, &middle_epilogue),
            (NumericTraceSourceRegion::OuterEpilogue, &outer_epilogue),
        ] {
            update_final_reaching_alias_dependencies(
                &mut definitions,
                region,
                program,
                &local_slots,
            )?;
        }
        definitions.into_values().collect()
    };
    #[cfg(test)]
    let kernel_local_write_slots = kernel
        .metadata()
        .local_write_slots
        .iter()
        .map(|local| local_slots.get(*local).copied())
        .collect::<Option<Vec<_>>>()?;
    if !kernel.matches_counted_index_shape(inner_counter, outer.counter_slot) {
        return None;
    }
    let mut schedule_reads = BTreeSet::new();
    schedule_reads.extend([
        outer.counter_slot,
        outer.limit_slot,
        middle.counter_slot,
        middle.limit_slot,
        inner_counter,
        inner_limit,
    ]);
    for program in [
        &outer_prelude,
        &middle_prelude,
        &middle_epilogue,
        &outer_epilogue,
    ] {
        schedule_reads.extend(
            program
                .operations
                .iter()
                .filter_map(|operation| match operation {
                    NumberInstruction::LoadLocal(local) => Some(*local),
                    _ => None,
                }),
        );
    }
    if !kernel.preflight_dependencies_are_pure(&schedule_reads) {
        return None;
    }
    let required_number_mask = required_entry_mask(DefinednessInputs {
        radix2,
        kernel_reads: &kernel_read_locals,
        kernel_writes: &kernel_written_locals,
        kernel_kills: &kernel_killed_locals,
        outer_prelude: &outer_prelude,
        middle_prelude: &middle_prelude,
        middle_epilogue: &middle_epilogue,
        outer_epilogue: &outer_epilogue,
    })?;
    #[cfg(test)]
    let metadata = NumericTraceMetadata {
        depth: 3,
        inner_header,
        inner_backedge,
        middle_header: middle.header,
        middle_backedge: middle.backedge,
        outer_header: outer.header,
        outer_backedge: outer.backedge,
        outer_exit: outer.exit,
        writable_receivers: writable_receivers.len(),
        readable_receivers: readable_receivers.len(),
        materialized_live_out_alias_slots,
        kernel_local_write_slots,
        final_reaching_alias_dependencies,
        post_handoff_read_slots: post_handoff_reads,
        kernel: kernel.metadata().clone(),
    };
    let plan = NumericTracePlan {
        inner_backedge,
        outer,
        local_slots,
        required_number_mask,
        kernel_killed_locals,
        receiver_sources,
        receiver_roles,
        writable_receivers,
        readable_receivers,
        outer_prelude,
        middle_prelude,
        middle_epilogue,
        outer_epilogue,
        radix2,
        kernel,
        #[cfg(test)]
        metadata,
    };
    record_compiled_plan();
    Some(NumericTraceProbe { plan, fallback })
}

#[cfg(test)]
fn update_final_reaching_alias_dependencies(
    definitions: &mut BTreeMap<usize, NumericTraceAliasDependency>,
    region: NumericTraceSourceRegion,
    program: &NumericProgram,
    local_slots: &[usize],
) -> Option<()> {
    for local in &program.invalidations {
        let target = *local_slots.get(*local)?;
        definitions.remove(&target);
    }
    for write in &program.writes {
        let target = *local_slots.get(write.local)?;
        match program.operations.get(write.value) {
            Some(NumberInstruction::LoadLocal(source)) => {
                definitions.insert(
                    target,
                    NumericTraceAliasDependency {
                        region,
                        target,
                        source: *local_slots.get(*source)?,
                    },
                );
            }
            Some(_) => {
                definitions.remove(&target);
            }
            None => return None,
        }
    }
    Some(())
}

fn compile_counted_loop(
    bytecode: &Bytecode,
    header: usize,
    backedge: usize,
) -> Option<CountedLoop> {
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
    if counter_slot == limit_slot
        || *exit != backedge.checked_add(1)?
        || !matches!(code.get(*exit), Some(Op::Pop))
        || !matches!(code.get(backedge), Some(Op::Jump(target)) if *target == header)
    {
        return None;
    }
    Some(CountedLoop {
        header,
        backedge,
        exit: *exit,
        body_start: header + 5,
        counter_slot: *counter_slot,
        limit_slot: *limit_slot,
    })
}

fn region_has_only_three_loop_branches(
    bytecode: &Bytecode,
    inner: (usize, usize),
    middle: CountedLoop,
    outer: CountedLoop,
) -> bool {
    let allowed_backedges = [
        inner,
        (middle.header, middle.backedge),
        (outer.header, outer.backedge),
    ];
    let allowed_conditions = [inner.0 + 3, middle.header + 3, outer.header + 3];
    bytecode.code[outer.header..=outer.backedge]
        .iter()
        .enumerate()
        .all(|(offset, op)| {
            let ip = outer.header + offset;
            match op {
                Op::Jump(target) => allowed_backedges.contains(&(*target, ip)),
                Op::JumpIfFalse(_) => allowed_conditions.contains(&ip),
                Op::JumpIfTrue(_) | Op::JumpIfNotNullish(_) | Op::AbruptJump(_) => false,
                _ => true,
            }
        })
}

fn count_slot_writes(code: &[Op], slot: usize) -> usize {
    code.iter()
        .filter(|op| {
            matches!(
                op,
                Op::StoreLocal(target) | Op::AssignLocal(target) | Op::ClearLocal(target)
                    if *target == slot
            )
        })
        .count()
}

fn extend_source_reads(reads: &mut BTreeSet<usize>, source: &TraceProgramSource) -> Option<()> {
    for operation in &source.operations {
        if let NumberInstruction::LoadLocal(local) = operation {
            reads.insert(*source.local_slots.get(*local)?);
        }
    }
    Some(())
}

fn extend_post_handoff_reads(
    bytecode: &Bytecode,
    start: usize,
    reads: &mut BTreeSet<usize>,
) -> Option<()> {
    for operation in bytecode.code.get(start..)? {
        match operation {
            Op::LoadLocal(slot)
            | Op::LoadLocalOrUndefined(slot)
            | Op::AppendStringLiteralLocal { slot, .. } => {
                reads.insert(*slot);
            }
            Op::CopyLocal { from, .. } => {
                reads.insert(*from);
            }
            Op::CompareLocalsJumpFalse { left, right, .. } => {
                reads.extend([*left, *right]);
            }
            Op::IncrementLocal { slot, .. } => {
                reads.insert(*slot);
            }
            Op::GetPropNamed { cache, .. } => {
                if let Some(slot) = cache.local_slot() {
                    reads.insert(slot);
                }
            }
            Op::GetPropIndex(encoded) => {
                if let Some(slot) = decode_index_receiver(*encoded).1 {
                    reads.insert(slot);
                }
            }
            _ => {}
        }
    }
    Some(())
}

fn remap_loop(mut counted: CountedLoop, local_slots: &[usize]) -> Option<CountedLoop> {
    counted.counter_slot = local_slots.binary_search(&counted.counter_slot).ok()?;
    counted.limit_slot = local_slots.binary_search(&counted.limit_slot).ok()?;
    Some(counted)
}

fn remap_program(
    source: TraceProgramSource,
    local_slots: &[usize],
    receiver_sources: &[ArraySource],
) -> Option<NumericProgram> {
    let TraceProgramSource {
        local_slots: source_locals,
        receiver_sources: source_receivers,
        mut operations,
        mut writes,
        invalidations,
        ..
    } = source;
    for operation in &mut operations {
        match operation {
            NumberInstruction::LoadLocal(local) => {
                let slot = *source_locals.get(*local)?;
                *local = local_slots.binary_search(&slot).ok()?;
            }
            NumberInstruction::DenseLoad { receiver, .. }
            | NumberInstruction::DenseStore { receiver, .. } => {
                let source = source_receivers.get(*receiver)?;
                *receiver = receiver_sources
                    .iter()
                    .position(|candidate| candidate == source)?;
            }
            NumberInstruction::Constant(_)
            | NumberInstruction::LoadInvariant(_)
            | NumberInstruction::Binary { .. }
            | NumberInstruction::Unary { .. }
            | NumberInstruction::Update { .. }
            | NumberInstruction::MathRound { .. } => {}
        }
    }
    for write in &mut writes {
        let slot = *source_locals.get(write.local)?;
        write.local = local_slots.binary_search(&slot).ok()?;
    }
    let invalidations = invalidations
        .into_iter()
        .map(|local| {
            let slot = *source_locals.get(local)?;
            local_slots.binary_search(&slot).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    Some(NumericProgram {
        operations,
        writes,
        invalidations,
    })
}

fn matches_counted_schedule(
    outer_prelude: &NumericProgram,
    middle_prelude: &NumericProgram,
    middle_epilogue: &NumericProgram,
    outer_epilogue: &NumericProgram,
    outer: CountedLoop,
    middle: CountedLoop,
    inner_counter: usize,
) -> Option<Radix2NestProof> {
    let matches = writes_constant(outer_prelude, middle.counter_slot, 0.0)
        && writes_local(middle_prelude, inner_counter, middle.counter_slot)
        && writes_add_one(middle_epilogue, middle.counter_slot)
        && writes_shift_left_one(outer_epilogue, outer.counter_slot)
        && outer_prelude
            .operations
            .iter()
            .all(|operation| match operation {
                NumberInstruction::DenseLoad { index, .. } => {
                    register_is_local(outer_prelude, *index, outer.counter_slot)
                }
                NumberInstruction::DenseStore { .. } => false,
                _ => true,
            })
        && [middle_prelude, middle_epilogue, outer_epilogue]
            .into_iter()
            .flat_map(|program| &program.operations)
            .all(|operation| {
                !matches!(
                    operation,
                    NumberInstruction::DenseLoad { .. } | NumberInstruction::DenseStore { .. }
                )
            });
    matches.then_some(Radix2NestProof {
        span: outer.counter_slot,
        bound: outer.limit_slot,
        lane: middle.counter_slot,
        index: inner_counter,
    })
}

fn sole_write(program: &NumericProgram, local: usize) -> Option<usize> {
    let mut writes = program
        .writes
        .iter()
        .filter(|write| write.local == local)
        .map(|write| write.value);
    let value = writes.next()?;
    writes.next().is_none().then_some(value)
}

fn writes_constant(program: &NumericProgram, local: usize, expected: f64) -> bool {
    sole_write(program, local).is_some_and(|value| {
        matches!(program.operations.get(value), Some(NumberInstruction::Constant(actual)) if actual.to_bits() == expected.to_bits())
    })
}

fn writes_local(program: &NumericProgram, target: usize, source: usize) -> bool {
    sole_write(program, target).is_some_and(|value| register_is_local(program, value, source))
}

fn writes_add_one(program: &NumericProgram, local: usize) -> bool {
    sole_write(program, local).is_some_and(|value| {
        let Some(NumberInstruction::Update { operation, value }) = program.operations.get(value)
        else {
            return false;
        };
        *operation == qjs_ast::UpdateOp::Increment && register_is_local(program, *value, local)
    })
}

fn writes_shift_left_one(program: &NumericProgram, local: usize) -> bool {
    sole_write(program, local).is_some_and(|value| {
        let Some(NumberInstruction::Binary {
            operation: BinaryOp::Shl,
            left,
            right,
        }) = program.operations.get(value)
        else {
            return false;
        };
        register_is_local(program, *left, local) && register_is_constant(program, *right, 1.0)
    })
}

fn register_is_local(program: &NumericProgram, register: usize, local: usize) -> bool {
    matches!(program.operations.get(register), Some(NumberInstruction::LoadLocal(actual)) if *actual == local)
}

fn register_is_constant(program: &NumericProgram, register: usize, expected: f64) -> bool {
    matches!(program.operations.get(register), Some(NumberInstruction::Constant(actual)) if actual.to_bits() == expected.to_bits())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn later_invalidation_kills_an_earlier_reaching_alias() {
        let local_slots = [10, 20];
        let earlier_alias = NumericProgram {
            operations: vec![NumberInstruction::LoadLocal(1)],
            writes: vec![LocalWrite { local: 0, value: 0 }],
            invalidations: Vec::new(),
        };
        let later_invalidation = NumericProgram {
            operations: Vec::new(),
            writes: Vec::new(),
            invalidations: vec![0],
        };
        let mut definitions = BTreeMap::new();

        assert!(
            update_final_reaching_alias_dependencies(
                &mut definitions,
                NumericTraceSourceRegion::MiddlePrelude,
                &earlier_alias,
                &local_slots,
            )
            .is_some()
        );
        assert_eq!(definitions.get(&10).map(|alias| alias.source), Some(20));

        assert!(
            update_final_reaching_alias_dependencies(
                &mut definitions,
                NumericTraceSourceRegion::MiddleEpilogue,
                &later_invalidation,
                &local_slots,
            )
            .is_some()
        );
        assert!(!definitions.contains_key(&10));
    }

    #[test]
    fn same_region_alias_write_redefines_an_invalidated_local() {
        let local_slots = [10, 20];
        let invalidation_then_alias = NumericProgram {
            operations: vec![NumberInstruction::LoadLocal(1)],
            writes: vec![LocalWrite { local: 0, value: 0 }],
            invalidations: vec![0],
        };
        let stale_alias = NumericTraceAliasDependency {
            region: NumericTraceSourceRegion::OuterPrelude,
            target: 10,
            source: 30,
        };
        let mut definitions = BTreeMap::from([(10, stale_alias)]);

        assert!(
            update_final_reaching_alias_dependencies(
                &mut definitions,
                NumericTraceSourceRegion::Inner,
                &invalidation_then_alias,
                &local_slots,
            )
            .is_some()
        );
        assert_eq!(
            definitions.get(&10),
            Some(&NumericTraceAliasDependency {
                region: NumericTraceSourceRegion::Inner,
                target: 10,
                source: 20,
            })
        );
    }
}
