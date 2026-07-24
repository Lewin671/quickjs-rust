//! Allocation-free execution for a preflighted Numeric Trace kernel.

use std::cell::{Ref, RefMut};

use crate::Value;

use super::super::super::dense::{MAX_DENSE_OPS, apply_binary, trace_array_index};
use super::super::{
    InvocationCounts,
    ir::{LocalBank, ReceiverRole},
};
use super::{CanonicalOperation, NumericDagKernel};

pub(in super::super) struct KernelScratch {
    indices: [usize; MAX_DENSE_OPS],
}

pub(in super::super) struct KernelArrays<'borrow, 'writers, 'readers> {
    pub(in super::super) roles: &'borrow [ReceiverRole],
    pub(in super::super) writers: &'borrow mut [RefMut<'writers, Vec<Value>>],
    pub(in super::super) readers: &'borrow [Ref<'readers, Vec<Value>>],
}

impl KernelScratch {
    pub(in super::super) fn new() -> Self {
        Self {
            indices: [0; MAX_DENSE_OPS],
        }
    }
}

impl NumericDagKernel {
    /// Executes one iteration after the whole region has passed preflight.
    /// Every operation below is total under the retained dense lease.
    #[inline(always)]
    pub(in super::super) fn run_preflighted_iteration(
        &self,
        arrays: &mut KernelArrays<'_, '_, '_>,
        bank: &mut LocalBank,
        values: &mut [f64; MAX_DENSE_OPS],
        scratch: &mut KernelScratch,
        counts: &mut InvocationCounts,
    ) {
        for value in &self.pure_values {
            values[value.index()] =
                evaluate_preflighted(&self.operations[value.index()], bank, values);
        }
        for (resolved, value) in scratch.indices.iter_mut().zip(&self.unique_index_values) {
            *resolved = trace_array_index(values[value.index()])
                .expect("preflighted trace index remains an array index");
        }
        for cell in &self.cells {
            scratch.indices[cell.resolved_slot.index()] = scratch.indices[cell.index_slot.index()];
            let Some(value_number) = cell.initial_load else {
                continue;
            };
            let index = scratch.indices[cell.resolved_slot.index()];
            let elements = receiver_elements(
                arrays.roles,
                arrays.writers,
                arrays.readers,
                cell.address.receiver,
            )
            .expect("preflighted receiver role remains valid");
            let Value::Number(value) = &elements[index] else {
                unreachable!("preflighted dense cell remains a Number")
            };
            values[value_number.index()] = *value;
        }
        for value in &self.dependent_values {
            values[value.index()] =
                evaluate_preflighted(&self.operations[value.index()], bank, values);
        }

        for commit in &self.commits {
            let cell = &self.cells[commit.cell.index()];
            let ReceiverRole::Writable(writer) = arrays.roles[cell.address.receiver] else {
                unreachable!("compiled trace commits target only writer receivers")
            };
            let index = scratch.indices[cell.resolved_slot.index()];
            let Value::Number(payload) = &mut arrays.writers[writer][index] else {
                unreachable!("preflighted writable dense cell remains a Number")
            };
            *payload = values[commit.value.index()];
        }
        for write in &self.local_writes {
            bank.write_number(write.local, values[write.value.index()]);
        }
        counts.inner_iteration(
            self.unique_index_values.len(),
            self.cells
                .iter()
                .filter(|cell| cell.initial_load.is_some())
                .count(),
            self.commits.len(),
        );
    }
}

#[inline(always)]
fn evaluate_preflighted(
    operation: &CanonicalOperation,
    bank: &LocalBank,
    values: &[f64; MAX_DENSE_OPS],
) -> f64 {
    match operation {
        CanonicalOperation::Constant(bits) => f64::from_bits(*bits),
        CanonicalOperation::LoadLocal(local) => bank
            .number(*local)
            .expect("preflighted numeric local remains a Number"),
        CanonicalOperation::Binary {
            operation,
            left,
            right,
        } => apply_binary(*operation, values[left.index()], values[right.index()])
            .expect("compiled trace binary operation remains supported"),
        CanonicalOperation::InitialLoad(_) => {
            unreachable!("initial loads are materialized directly from their dense cell")
        }
    }
}

#[inline(always)]
fn receiver_elements<'a>(
    roles: &[ReceiverRole],
    writers: &'a [RefMut<'_, Vec<Value>>],
    readers: &'a [Ref<'_, Vec<Value>>],
    receiver: usize,
) -> Option<&'a [Value]> {
    match *roles.get(receiver)? {
        ReceiverRole::Writable(writer) => writers.get(writer).map(|elements| elements.as_slice()),
        ReceiverRole::Readable(reader) => readers.get(reader).map(|elements| elements.as_slice()),
    }
}
