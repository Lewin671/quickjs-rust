use std::cell::Ref;

use qjs_ast::{BinaryOp, UpdateOp};

use crate::{Value, value::MAX_DENSE_STORAGE_LENGTH};

use super::{
    DenseAccess, DynamicControl, LocalWrite, NumberInstruction, PendingDenseStore, Register,
    SunkDenseStore,
};

/// A single ordinary Array store that can materialize an implicit hole tail.
///
/// Compilation deliberately accepts only the simple induction shape needed to
/// prove that every successful iteration advances the candidate append index
/// by exactly one. Runtime still checks the actual dense length before every
/// staged store, so unusual initial state or a shorter logical length fails
/// closed without publishing the current iteration.
#[derive(Clone, Copy, Debug)]
pub(super) struct HoleTailAppendPlan {
    writer_receiver: usize,
}

impl HoleTailAppendPlan {
    pub(super) fn compile(
        control: &DynamicControl,
        counter_local: usize,
        operations: &[NumberInstruction],
        writes: &[LocalWrite],
        store_count: usize,
        sunk_store: Option<SunkDenseStore>,
    ) -> Option<Self> {
        if !matches!(control, DynamicControl::LessThan(_)) || store_count != 1 {
            return None;
        }

        let store = match sunk_store {
            Some(store) => store,
            None => operations.iter().find_map(|operation| match operation {
                NumberInstruction::DenseStore {
                    receiver,
                    index,
                    value,
                } => Some(SunkDenseStore {
                    receiver: *receiver,
                    index: *index,
                    value: *value,
                }),
                _ => None,
            })?,
        };
        if !is_counter_load(operations, store.index, counter_local)
            || operations.iter().any(|operation| {
                matches!(
                    operation,
                    NumberInstruction::DenseLoad { receiver, .. }
                        if *receiver == store.receiver
                )
            })
        {
            return None;
        }

        let counter_write = writes.iter().find(|write| write.local == counter_local)?;
        if !is_unit_increment(operations, counter_write.value, counter_local) {
            return None;
        }

        Some(Self {
            writer_receiver: store.receiver,
        })
    }

    pub(super) fn writer_receiver(self) -> usize {
        self.writer_receiver
    }
}

fn is_counter_load(
    operations: &[NumberInstruction],
    register: Register,
    counter_local: usize,
) -> bool {
    matches!(
        operations.get(register),
        Some(NumberInstruction::LoadLocal(local)) if *local == counter_local
    )
}

fn is_one(operations: &[NumberInstruction], register: Register) -> bool {
    matches!(
        operations.get(register),
        Some(NumberInstruction::Constant(value)) if *value == 1.0
    )
}

fn is_unit_increment(
    operations: &[NumberInstruction],
    register: Register,
    counter_local: usize,
) -> bool {
    match operations.get(register) {
        Some(NumberInstruction::Update {
            operation: UpdateOp::Increment,
            value,
        }) => is_counter_load(operations, *value, counter_local),
        Some(NumberInstruction::Binary {
            operation: BinaryOp::Add,
            left,
            right,
        }) => {
            (is_counter_load(operations, *left, counter_local) && is_one(operations, *right))
                || (is_one(operations, *left) && is_counter_load(operations, *right, counter_local))
        }
        _ => false,
    }
}

/// Dense access adapter for one append-only receiver plus any number of
/// fully-dense read-only receivers. Read leases retain original receiver order
/// with the writer omitted, so lookup stays allocation-free inside the loop.
pub(super) struct HoleTailAppendAccess<'a, 'elements> {
    writer_receiver: usize,
    writer: &'a mut Vec<Value>,
    readable: &'a [Ref<'elements, Vec<Value>>],
    logical_length: usize,
    pending: Option<PendingDenseStore>,
}

impl<'a, 'elements> HoleTailAppendAccess<'a, 'elements> {
    pub(super) fn new(
        writer_receiver: usize,
        writer: &'a mut Vec<Value>,
        readable: &'a [Ref<'elements, Vec<Value>>],
        logical_length: usize,
        endpoint: f64,
    ) -> Self {
        let start_index = writer.len();
        reserve_hole_tail_with(start_index, endpoint, logical_length, |additional| {
            writer.try_reserve_exact(additional)
        });
        Self {
            writer_receiver,
            writer,
            readable,
            logical_length,
            pending: None,
        }
    }
}

/// Returns the maximum useful one-shot reserve for an integer induction
/// variable governed by `counter < endpoint`.
///
/// Fractional endpoints admit the last integer below their ceiling. Infinite
/// or otherwise oversized endpoints are capped at the engine's dense-storage
/// limit, and the existing logical length is always a second upper bound.
/// Invalid or already-exhausted ranges reserve nothing.
fn hole_tail_reserve_additional(start_index: usize, endpoint: f64, logical_length: usize) -> usize {
    if endpoint.is_nan() || endpoint <= 0.0 {
        return 0;
    }
    let endpoint_exclusive = if endpoint.is_finite() {
        endpoint.ceil().min(MAX_DENSE_STORAGE_LENGTH as f64) as usize
    } else {
        MAX_DENSE_STORAGE_LENGTH
    };
    endpoint_exclusive
        .min(logical_length)
        .min(MAX_DENSE_STORAGE_LENGTH)
        .saturating_sub(start_index)
}

/// Capacity preparation is opportunistic: allocation failure must not change
/// the observable loop result because subsequent `Vec::push` calls retain
/// their normal behavior.
fn reserve_hole_tail_with<E>(
    start_index: usize,
    endpoint: f64,
    logical_length: usize,
    reserve: impl FnOnce(usize) -> Result<(), E>,
) {
    let additional = hole_tail_reserve_additional(start_index, endpoint, logical_length);
    if additional != 0 {
        let _ = reserve(additional);
    }
}

impl DenseAccess for HoleTailAppendAccess<'_, '_> {
    fn reset_iteration(&mut self) {
        self.pending = None;
    }

    fn load_number(&self, receiver: usize, index: usize) -> Option<f64> {
        if receiver == self.writer_receiver {
            return None;
        }
        let readable = if receiver < self.writer_receiver {
            receiver
        } else {
            receiver.checked_sub(1)?
        };
        match self.readable.get(readable)?.get(index)? {
            Value::Number(value) => Some(*value),
            _ => None,
        }
    }

    fn stage_store(&mut self, receiver: usize, index: usize, value: f64) -> bool {
        if receiver != self.writer_receiver
            || self.pending.is_some()
            || index != self.writer.len()
            || index >= self.logical_length
            || index >= MAX_DENSE_STORAGE_LENGTH
        {
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
            .expect("hole-tail append plan stages one validated write");
        debug_assert_eq!(store.receiver, self.writer_receiver);
        debug_assert_eq!(store.index, self.writer.len());
        debug_assert!(store.index < self.logical_length);
        debug_assert!(store.index < MAX_DENSE_STORAGE_LENGTH);
        self.writer.push(Value::Number(store.value));
        super::record_hole_tail_append_iteration();
    }
}

#[cfg(test)]
impl Drop for HoleTailAppendAccess<'_, '_> {
    fn drop(&mut self) {
        if self.pending.is_some() {
            super::record_hole_tail_append_staged_discard();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_bound_tracks_less_than_endpoint_and_dense_limits() {
        assert_eq!(hole_tail_reserve_additional(1, 4.0, 10), 3);
        assert_eq!(hole_tail_reserve_additional(1, 4.25, 10), 4);
        assert_eq!(hole_tail_reserve_additional(1, 100.0, 4), 3);
        assert_eq!(hole_tail_reserve_additional(4, 4.25, 10), 1);
        assert_eq!(hole_tail_reserve_additional(5, 4.25, 10), 0);

        for endpoint in [f64::NAN, -3.5, -0.0, f64::NEG_INFINITY] {
            assert_eq!(hole_tail_reserve_additional(1, endpoint, 10), 0);
        }
        assert_eq!(hole_tail_reserve_additional(1, f64::INFINITY, 12), 11);
        assert_eq!(
            hole_tail_reserve_additional(1, f64::MAX, MAX_DENSE_STORAGE_LENGTH + 10),
            MAX_DENSE_STORAGE_LENGTH - 1
        );
    }

    #[test]
    fn reserve_is_one_shot_and_allocation_failure_is_fail_open() {
        let mut requested = None;
        reserve_hole_tail_with(1, 4.25, 10, |additional| {
            requested = Some(additional);
            Err(())
        });
        assert_eq!(requested, Some(4));

        let mut writer = vec![Value::Number(0.0)];
        let readable = Vec::new();
        let access = HoleTailAppendAccess::new(0, &mut writer, &readable, 8, 4.25);
        assert!(access.writer.capacity() >= 5);
    }

    #[test]
    fn append_shape_requires_exact_counter_index_and_unit_increment() {
        let operations = vec![
            NumberInstruction::LoadLocal(0),
            NumberInstruction::Constant(1.0),
            NumberInstruction::Update {
                operation: UpdateOp::Increment,
                value: 0,
            },
        ];
        let writes = [LocalWrite { local: 0, value: 2 }];
        let store = SunkDenseStore {
            receiver: 1,
            index: 0,
            value: 1,
        };

        assert!(
            HoleTailAppendPlan::compile(
                &DynamicControl::LessThan(super::super::invariants::DynamicLimit::LocalNumber(1)),
                0,
                &operations,
                &writes,
                1,
                Some(store),
            )
            .is_some()
        );
        assert!(
            HoleTailAppendPlan::compile(
                &DynamicControl::AtLeastZero,
                0,
                &operations,
                &writes,
                1,
                Some(store),
            )
            .is_none()
        );

        let wrong_index = SunkDenseStore { index: 1, ..store };
        assert!(
            HoleTailAppendPlan::compile(
                &DynamicControl::LessThan(super::super::invariants::DynamicLimit::LocalNumber(1)),
                0,
                &operations,
                &writes,
                1,
                Some(wrong_index),
            )
            .is_none()
        );
    }
}
