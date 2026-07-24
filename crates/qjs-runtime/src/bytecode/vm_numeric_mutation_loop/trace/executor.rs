//! Entry preflight, total trace execution, and exact outer-exit handoff.

use qjs_ast::UpdateOp;

use crate::{Value, value::ArrayRef};

use super::super::super::vm::Vm;
use super::super::dense::{
    MAX_DENSE_OPS, NumberInstruction, apply_binary, apply_unary, trace_array_index,
};
use super::{
    InvocationCounts, NumericTracePlan, NumericTraceRun,
    ir::{LocalBank, NumericProgram, Radix2NestProof, ReceiverRole, SlotState},
    kernel::{KernelArrays, KernelScratch},
    record_attempt, record_completed, record_decline, record_entry, record_lease_entry,
};

pub(super) fn try_run(plan: &NumericTracePlan, vm: &mut Vm<'_>) -> NumericTraceRun {
    record_attempt();
    let Some(bank) = prepare_entry(plan, vm) else {
        record_decline();
        return NumericTraceRun::DeclinedNoProgress;
    };
    let Some(arrays) = plan
        .receiver_sources
        .iter()
        .map(|source| source.resolve(vm))
        .collect::<Option<Vec<_>>>()
    else {
        record_decline();
        return NumericTraceRun::DeclinedNoProgress;
    };
    let Some(writers) = plan
        .writable_receivers
        .iter()
        .map(|receiver| arrays.get(*receiver).cloned())
        .collect::<Option<Vec<_>>>()
    else {
        record_decline();
        return NumericTraceRun::DeclinedNoProgress;
    };
    let Some(readers) = plan
        .readable_receivers
        .iter()
        .map(|receiver| arrays.get(*receiver).cloned())
        .collect::<Option<Vec<_>>>()
    else {
        record_decline();
        return NumericTraceRun::DeclinedNoProgress;
    };
    let entry_stack_depth = vm.stack.len();

    let outcome = ArrayRef::with_distinct_dense_writable_and_readable_elements(
        &writers,
        &readers,
        |writable, readable| {
            record_lease_entry();
            if !preflight_dense_prefix(plan, &bank, writable, readable) {
                return None;
            }
            record_entry(entry_stack_depth);
            Some(execute_preflighted(plan, bank, writable, readable))
        },
    )
    .flatten();
    let Some((bank, counts)) = outcome else {
        record_decline();
        return NumericTraceRun::DeclinedNoProgress;
    };

    publish_bank(plan, vm, bank);
    vm.ip = plan.outer.exit + 1;
    debug_assert_eq!(vm.ip, plan.outer.exit + 1);
    debug_assert_eq!(vm.stack.len(), entry_stack_depth);
    record_completed(counts, vm.stack.len());
    NumericTraceRun::CompletedOuter
}

fn prepare_entry(plan: &NumericTracePlan, vm: &Vm<'_>) -> Option<LocalBank> {
    if vm.ip != plan.inner_backedge.checked_add(1)?
        || !vm.try_stack.is_empty()
        || vm.pending_throw.is_some()
        || vm.pending_return.is_some()
        || vm.pending_jump.is_some()
        || vm.resume_mode.is_some()
        || !vm.with_stack.is_empty()
        || vm.direct_eval_with_stack
        || !vm.disposable_scopes.is_empty()
        || vm.stop_at_prologue
        || vm.bytecode.contains_direct_eval()
        || vm.bytecode.contains_with()
        || vm.env.deopt_bindings().is_some()
        || vm.env.has_module_imports()
    {
        return None;
    }
    for slot in plan.local_slots.iter().copied().chain(
        plan.receiver_sources
            .iter()
            .filter_map(|source| source.local_slot()),
    ) {
        if !vm.slot_is_authoritative(slot)
            || vm
                .local_upvalues
                .get(slot)
                .and_then(Option::as_ref)
                .is_some()
        {
            return None;
        }
    }

    let mut bank = LocalBank::empty();
    for (local, slot) in plan.local_slots.iter().copied().enumerate() {
        if !load_entry_slot(&mut bank, local, vm.locals.get(slot)?) {
            return None;
        }
    }
    if !valid_radix2_entry(plan.radix2, &bank) {
        return None;
    }
    if !valid_entry_flow(plan, &bank) {
        return None;
    }
    Some(bank)
}

fn valid_entry_flow(plan: &NumericTracePlan, bank: &LocalBank) -> bool {
    bank.valid_mask() & plan.required_number_mask == plan.required_number_mask
}

fn valid_radix2_entry(proof: Radix2NestProof, bank: &LocalBank) -> bool {
    let Some(span) = bank.number(proof.span) else {
        return false;
    };
    let Some(bound) = bank.number(proof.bound) else {
        return false;
    };
    let Some(lane) = bank.number(proof.lane) else {
        return false;
    };
    let Some(index) = bank.number(proof.index) else {
        return false;
    };
    if !integer(span)
        || !integer(bound)
        || !integer(lane)
        || !integer(index)
        || span < 1.0
        || span >= bound
        || bound > f64::from(i32::MAX)
        || lane < 0.0
        || lane >= span
        || index != lane + 2.0 * span
    {
        return false;
    }
    let span = span as u64;
    let bound = bound as u64;
    if bound % span != 0 {
        return false;
    }
    let quotient = bound / span;
    quotient >= 2 && quotient.is_power_of_two()
}

fn preflight_dense_prefix(
    plan: &NumericTracePlan,
    bank: &LocalBank,
    writers: &[std::cell::RefMut<'_, Vec<Value>>],
    readers: &[std::cell::Ref<'_, Vec<Value>>],
) -> bool {
    let bound = bank
        .number(plan.radix2.bound)
        .expect("entry algebra requires a numeric bound") as usize;
    writers
        .iter()
        .map(|elements| elements.as_slice())
        .chain(readers.iter().map(|elements| elements.as_slice()))
        .all(|elements| {
            elements.len() >= bound
                && elements[..bound]
                    .iter()
                    .all(|value| matches!(value, Value::Number(_)))
        })
}

fn execute_preflighted(
    plan: &NumericTracePlan,
    mut bank: LocalBank,
    writers: &mut [std::cell::RefMut<'_, Vec<Value>>],
    readers: &[std::cell::Ref<'_, Vec<Value>>],
) -> (LocalBank, InvocationCounts) {
    let mut values = [0.0; MAX_DENSE_OPS];
    let mut scratch = KernelScratch::new();
    let mut counts = InvocationCounts::default();

    loop {
        while bank
            .number(plan.radix2.index)
            .expect("preflighted inner index remains numeric")
            < bank
                .number(plan.radix2.bound)
                .expect("preflighted bound remains numeric")
        {
            plan.kernel.run_preflighted_iteration(
                &mut KernelArrays {
                    roles: &plan.receiver_roles,
                    writers,
                    readers,
                },
                &mut bank,
                &mut values,
                &mut scratch,
                &mut counts,
            );
            for local in &plan.kernel_killed_locals {
                bank.write_undefined(*local);
            }
        }

        run_program_preflighted(
            &plan.middle_epilogue,
            &plan.receiver_roles,
            readers,
            &mut bank,
            &mut values,
            &mut counts,
        );
        counts.middle_completion();
        if bank
            .number(plan.radix2.lane)
            .expect("preflighted lane remains numeric")
            < bank
                .number(plan.radix2.span)
                .expect("preflighted span remains numeric")
        {
            run_program_preflighted(
                &plan.middle_prelude,
                &plan.receiver_roles,
                readers,
                &mut bank,
                &mut values,
                &mut counts,
            );
            continue;
        }

        run_program_preflighted(
            &plan.outer_epilogue,
            &plan.receiver_roles,
            readers,
            &mut bank,
            &mut values,
            &mut counts,
        );
        counts.outer_completion();
        if bank
            .number(plan.radix2.span)
            .expect("preflighted span remains numeric")
            >= bank
                .number(plan.radix2.bound)
                .expect("preflighted bound remains numeric")
        {
            break;
        }
        run_program_preflighted(
            &plan.outer_prelude,
            &plan.receiver_roles,
            readers,
            &mut bank,
            &mut values,
            &mut counts,
        );
        run_program_preflighted(
            &plan.middle_prelude,
            &plan.receiver_roles,
            readers,
            &mut bank,
            &mut values,
            &mut counts,
        );
    }
    (bank, counts)
}

#[inline(always)]
fn run_program_preflighted(
    program: &NumericProgram,
    roles: &[ReceiverRole],
    readers: &[std::cell::Ref<'_, Vec<Value>>],
    bank: &mut LocalBank,
    values: &mut [f64; MAX_DENSE_OPS],
    counts: &mut InvocationCounts,
) {
    for (register, operation) in program.operations.iter().enumerate() {
        values[register] = match *operation {
            NumberInstruction::Constant(value) => value,
            NumberInstruction::LoadLocal(local) => bank
                .number(local)
                .expect("preflighted schedule local remains numeric"),
            NumberInstruction::DenseLoad { receiver, index } => {
                let index = trace_array_index(values[index])
                    .expect("radix-2 schedule index remains in bounds");
                let ReceiverRole::Readable(reader) = roles[receiver] else {
                    unreachable!("compiled schedules read only reader receivers")
                };
                let Value::Number(value) = readers[reader][index] else {
                    unreachable!("preflighted schedule payload remains numeric")
                };
                counts.readonly_number_load();
                value
            }
            NumberInstruction::Binary {
                operation,
                left,
                right,
            } => apply_binary(operation, values[left], values[right])
                .expect("compiled schedule binary remains supported"),
            NumberInstruction::Unary { operation, value } => apply_unary(operation, values[value])
                .expect("compiled schedule unary remains supported"),
            NumberInstruction::Update { operation, value } => match operation {
                UpdateOp::Increment => values[value] + 1.0,
                UpdateOp::Decrement => values[value] - 1.0,
            },
            NumberInstruction::LoadInvariant(_)
            | NumberInstruction::DenseStore { .. }
            | NumberInstruction::MathRound { .. } => {
                unreachable!("compiled numeric trace schedule remains total")
            }
        };
    }
    for local in &program.invalidations {
        bank.write_undefined(*local);
    }
    for write in &program.writes {
        bank.write_number(write.local, values[write.value]);
    }
}

fn publish_bank(plan: &NumericTracePlan, vm: &mut Vm<'_>, bank: LocalBank) {
    for (local, slot) in plan.local_slots.iter().copied().enumerate() {
        vm.locals[slot] = published_slot(&bank, local);
    }
}

fn load_entry_slot(bank: &mut LocalBank, local: usize, slot: &Option<Value>) -> bool {
    match slot {
        Some(Value::Number(value)) => bank.write_number(local, *value),
        Some(Value::Undefined) => bank.write_undefined(local),
        None => {}
        Some(_) => return false,
    }
    true
}

fn published_slot(bank: &LocalBank, local: usize) -> Option<Value> {
    match bank.state(local) {
        Some(SlotState::Cleared) => None,
        Some(SlotState::Undefined) => Some(Value::Undefined),
        Some(SlotState::Number) => Some(Value::Number(
            bank.number(local)
                .expect("Number slot state retains its numeric payload"),
        )),
        None => unreachable!("compiled trace local remains in the fixed bank"),
    }
}

fn integer(value: f64) -> bool {
    value.is_finite() && value.fract() == 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROOF: Radix2NestProof = Radix2NestProof {
        span: 0,
        bound: 1,
        lane: 2,
        index: 3,
    };

    fn bank(span: f64, bound: f64, lane: f64, index: f64) -> LocalBank {
        let mut bank = LocalBank::empty();
        for (local, value) in [(0, span), (1, bound), (2, lane), (3, index)] {
            bank.write_number(local, value);
        }
        bank
    }

    #[test]
    fn radix2_entry_algebra_rejects_invalid_counters_without_array_work() {
        assert!(valid_radix2_entry(PROOF, &bank(1.0, 8.0, 0.0, 2.0)));
        assert!(valid_radix2_entry(PROOF, &bank(1.0, 8.0, -0.0, 2.0)));

        for (span, bound, lane, index) in [
            (0.0, 8.0, 0.0, 0.0),
            (-1.0, 8.0, 0.0, -2.0),
            (1.5, 8.0, 0.0, 3.0),
            (f64::NAN, 8.0, 0.0, 2.0),
            (f64::INFINITY, 8.0, 0.0, 2.0),
            (1.0, 8.5, 0.0, 2.0),
            (1.0, f64::NAN, 0.0, 2.0),
            (1.0, f64::INFINITY, 0.0, 2.0),
            (1.0, f64::from(i32::MAX) + 1.0, 0.0, 2.0),
            (1.0, 8.0, -1.0, 1.0),
            (1.0, 8.0, 1.0, 3.0),
            (1.0, 8.0, f64::NAN, 2.0),
            (1.0, 8.0, f64::INFINITY, 2.0),
            (1.0, 8.0, 0.0, 3.0),
            (1.0, 8.0, 0.0, f64::NAN),
            (1.0, 8.0, 0.0, f64::INFINITY),
            (1.0, 6.0, 0.0, 2.0),
            (2.0, 12.0, 0.0, 4.0),
            (3.0, 8.0, 0.0, 6.0),
            (1_073_741_824.0, f64::from(i32::MAX), 0.0, 2_147_483_648.0),
        ] {
            assert!(
                !valid_radix2_entry(PROOF, &bank(span, bound, lane, index)),
                "unexpectedly accepted H={span}, N={bound}, F={lane}, I={index}"
            );
        }
    }

    #[test]
    fn entry_and_publication_preserve_cleared_undefined_and_number_slots() {
        for (local, slot) in [
            (0, None),
            (1, Some(Value::Undefined)),
            (2, Some(Value::Number(-0.0))),
        ] {
            let mut bank = LocalBank::empty();
            assert!(load_entry_slot(&mut bank, local, &slot));
            let published = published_slot(&bank, local);
            match (slot, published) {
                (None, None) | (Some(Value::Undefined), Some(Value::Undefined)) => {}
                (Some(Value::Number(expected)), Some(Value::Number(actual))) => {
                    assert_eq!(actual.to_bits(), expected.to_bits());
                }
                state => panic!("slot state changed across the bank: {state:?}"),
            }
        }
    }
}
