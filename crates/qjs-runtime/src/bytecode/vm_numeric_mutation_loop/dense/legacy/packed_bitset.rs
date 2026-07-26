//! Native executor for canonical packed-bitset word updates.
//!
//! The dense compiler has already proved that every input is a Number and the
//! array access is an ordinary dense own-element read/modify/write. This plan
//! recognizes the remaining data-flow shape rather than a source workload:
//! `words[counter >> shift] <bitwise-op>= bit << (counter & mask)`. It keeps
//! the generic dense program as a fall-through whenever an entry guard cannot
//! prove the native index arithmetic safe.

use qjs_ast::{BinaryOp, UnaryOp};

use crate::{Value, to_int32_number, to_uint32_number};

use super::{
    DynamicProgramRun, LocalControl, LocalLimit, LocalWrite, MAX_DENSE_LOCALS, NumberInstruction,
    Register, SunkDenseStore,
};

#[derive(Clone, Copy, Debug)]
enum NumberInput {
    Constant(f64),
    Local(usize),
}

impl NumberInput {
    fn resolve(self, locals: &[f64; MAX_DENSE_LOCALS]) -> Option<f64> {
        match self {
            Self::Constant(value) => Some(value),
            Self::Local(local) => locals.get(local).copied(),
        }
    }

    fn local(self) -> Option<usize> {
        match self {
            Self::Constant(_) => None,
            Self::Local(local) => Some(local),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WriteValue {
    Index,
    UpdatedWord,
    NextCounter,
}

#[derive(Clone, Copy, Debug)]
struct PackedWrite {
    local: usize,
    value: WriteValue,
}

/// A normalized packed-word operation with a counted, positive integer
/// counter. `invert_bit` covers the common clear form (`word &= ~bit`) while
/// the other bitwise operations support set, toggle, and mask families.
#[derive(Clone, Debug)]
pub(super) struct PackedBitsetMutationPlan {
    counter_local: usize,
    word_shift: NumberInput,
    bit_mask: NumberInput,
    bit: NumberInput,
    step: NumberInput,
    update: BinaryOp,
    invert_bit: bool,
    writes: Vec<PackedWrite>,
}

impl PackedBitsetMutationPlan {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn compile(
        counter_local: usize,
        control: LocalControl,
        receiver_count: usize,
        operations: &[NumberInstruction],
        writes: &[LocalWrite],
        store_count: usize,
        sunk_store: Option<SunkDenseStore>,
    ) -> Option<Self> {
        let LocalControl::LessThan(LocalLimit::Number(limit_local)) = control else {
            return None;
        };
        if receiver_count != 1 || store_count != 1 || limit_local == counter_local {
            return None;
        }
        let store = sunk_store?;
        if store.receiver != 0 {
            return None;
        }

        let (index_counter, word_shift_register) = binary(operations, store.index, BinaryOp::Shr)?;
        if !is_counter(operations, index_counter, counter_local) {
            return None;
        }
        let word_shift = number_input(operations, word_shift_register)?;

        let (update, update_left, update_right) = bitwise_binary(operations, store.value)?;
        let (word_register, update_bit_register) = match dense_load(operations, update_left) {
            Some((receiver, index)) if receiver == store.receiver && index == store.index => {
                (update_left, update_right)
            }
            _ => match dense_load(operations, update_right) {
                Some((receiver, index)) if receiver == store.receiver && index == store.index => {
                    (update_right, update_left)
                }
                _ => return None,
            },
        };

        let (invert_bit, shifted_bit_register) = match operations.get(update_bit_register)? {
            NumberInstruction::Unary {
                operation: UnaryOp::BitwiseNot,
                value,
            } => (true, *value),
            _ => (false, update_bit_register),
        };
        let (bit_register, offset_register) =
            binary(operations, shifted_bit_register, BinaryOp::Shl)?;
        let bit = number_input(operations, bit_register)?;
        let (offset_left, offset_right) =
            binary(operations, offset_register, BinaryOp::BitwiseAnd)?;
        let (offset_counter, bit_mask_register) =
            if is_counter(operations, offset_left, counter_local) {
                (offset_left, offset_right)
            } else if is_counter(operations, offset_right, counter_local) {
                (offset_right, offset_left)
            } else {
                return None;
            };
        let bit_mask = number_input(operations, bit_mask_register)?;

        let counter_write = writes
            .iter()
            .filter(|write| write.local == counter_local)
            .collect::<Vec<_>>();
        if counter_write.len() != 1 {
            return None;
        }
        let next_counter_register = counter_write[0].value;
        let (next_left, next_right) = binary(operations, next_counter_register, BinaryOp::Add)?;
        let (next_counter, step_register) = if is_counter(operations, next_left, counter_local) {
            (next_left, next_right)
        } else if is_counter(operations, next_right, counter_local) {
            (next_right, next_left)
        } else {
            return None;
        };
        let step = number_input(operations, step_register)?;

        let inputs = [word_shift, bit_mask, bit, step];
        if inputs
            .into_iter()
            .filter_map(NumberInput::local)
            .any(|local| {
                local == counter_local
                    || local == limit_local
                    || writes.iter().any(|write| write.local == local)
            })
        {
            return None;
        }
        if writes.iter().any(|write| write.local == limit_local) {
            return None;
        }

        let mut packed_writes = Vec::with_capacity(writes.len());
        for write in writes {
            let value = if write.value == store.index {
                WriteValue::Index
            } else if write.value == store.value {
                WriteValue::UpdatedWord
            } else if write.value == next_counter_register {
                WriteValue::NextCounter
            } else {
                return None;
            };
            if write.local >= MAX_DENSE_LOCALS
                || (write.local == counter_local && value != WriteValue::NextCounter)
            {
                return None;
            }
            packed_writes.push(PackedWrite {
                local: write.local,
                value,
            });
        }

        let mut used = vec![false; operations.len()];
        for register in [
            store.index,
            store.value,
            word_register,
            update_bit_register,
            bit_register,
            shifted_bit_register,
            offset_register,
            index_counter,
            word_shift_register,
            offset_counter,
            bit_mask_register,
            next_counter_register,
            next_counter,
            step_register,
        ] {
            *used.get_mut(register)? = true;
        }
        if !used.into_iter().all(|used| used) {
            return None;
        }

        Some(Self {
            counter_local,
            word_shift,
            bit_mask,
            bit,
            step,
            update,
            invert_bit,
            writes: packed_writes,
        })
    }

    /// Returns `None` only before this executor publishes any iteration. The
    /// caller then runs the original dense program. Once an earlier write has
    /// completed, a failed guard returns a replay-at-header outcome so the
    /// first unsafe iteration retains ordinary VM semantics.
    pub(super) fn run(
        &self,
        elements: &mut [Value],
        locals: &mut [f64; MAX_DENSE_LOCALS],
        limit: f64,
    ) -> Option<DynamicProgramRun> {
        let word_shift = self.word_shift.resolve(locals)?;
        let bit_mask = self.bit_mask.resolve(locals)?;
        let bit = self.bit.resolve(locals)?;
        let step = self.step.resolve(locals)?;
        let mut counter = *locals.get(self.counter_local)?;
        let mut made_progress = false;

        loop {
            if !matches!(counter.partial_cmp(&limit), Some(std::cmp::Ordering::Less)) {
                return Some(DynamicProgramRun {
                    deoptimized: false,
                    made_progress,
                });
            }
            if !native_counter(counter) || !native_step(step) || !limit.is_finite() {
                return deopt_or_decline(made_progress);
            }

            let shifted = to_int32_number(counter) >> (to_uint32_number(word_shift) & 0x1f);
            debug_assert!(shifted >= 0);
            let index = shifted as usize;
            let Some(Value::Number(word)) = elements.get(index) else {
                return deopt_or_decline(made_progress);
            };

            let offset = to_uint32_number(f64::from(
                to_int32_number(counter) & to_int32_number(bit_mask),
            )) & 0x1f;
            let mut update_bit = to_int32_number(bit) << offset;
            if self.invert_bit {
                update_bit = !update_bit;
            }
            let word = to_int32_number(*word);
            let updated = match self.update {
                BinaryOp::BitwiseAnd => word & update_bit,
                BinaryOp::BitwiseOr => word | update_bit,
                BinaryOp::BitwiseXor => word ^ update_bit,
                _ => unreachable!("packed bitset compiler only admits bitwise word updates"),
            };
            let next_counter = counter + step;
            let index_value = f64::from(shifted);
            let updated_value = f64::from(updated);

            elements[index] = Value::Number(updated_value);
            for write in &self.writes {
                locals[write.local] = match write.value {
                    WriteValue::Index => index_value,
                    WriteValue::UpdatedWord => updated_value,
                    WriteValue::NextCounter => next_counter,
                };
            }
            counter = next_counter;
            made_progress = true;
            super::super::record_iteration();
            record_iteration();
        }
    }
}

fn binary(
    operations: &[NumberInstruction],
    register: Register,
    operation: BinaryOp,
) -> Option<(Register, Register)> {
    match operations.get(register)? {
        NumberInstruction::Binary {
            operation: current,
            left,
            right,
        } if *current == operation => Some((*left, *right)),
        _ => None,
    }
}

fn bitwise_binary(
    operations: &[NumberInstruction],
    register: Register,
) -> Option<(BinaryOp, Register, Register)> {
    match operations.get(register)? {
        NumberInstruction::Binary {
            operation:
                operation @ (BinaryOp::BitwiseAnd | BinaryOp::BitwiseOr | BinaryOp::BitwiseXor),
            left,
            right,
        } => Some((*operation, *left, *right)),
        _ => None,
    }
}

fn dense_load(operations: &[NumberInstruction], register: Register) -> Option<(usize, Register)> {
    match operations.get(register)? {
        NumberInstruction::DenseLoad { receiver, index } => Some((*receiver, *index)),
        _ => None,
    }
}

fn number_input(operations: &[NumberInstruction], register: Register) -> Option<NumberInput> {
    match operations.get(register)? {
        NumberInstruction::Constant(value) => Some(NumberInput::Constant(*value)),
        NumberInstruction::LoadLocal(local) if *local < MAX_DENSE_LOCALS => {
            Some(NumberInput::Local(*local))
        }
        _ => None,
    }
}

fn is_counter(operations: &[NumberInstruction], register: Register, counter_local: usize) -> bool {
    matches!(
        operations.get(register),
        Some(NumberInstruction::LoadLocal(local)) if *local == counter_local
    )
}

fn native_counter(value: f64) -> bool {
    value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= f64::from(i32::MAX)
}

fn native_step(value: f64) -> bool {
    value.is_finite() && value > 0.0 && value.fract() == 0.0 && value <= 9_007_199_254_740_991.0
}

fn deopt_or_decline(made_progress: bool) -> Option<DynamicProgramRun> {
    made_progress.then_some(DynamicProgramRun {
        deoptimized: true,
        made_progress: true,
    })
}

#[cfg(test)]
thread_local! {
    static PACKED_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_iteration() {
    PACKED_ITERATIONS.set(PACKED_ITERATIONS.get() + 1);
}

#[cfg(not(test))]
fn record_iteration() {}

#[cfg(test)]
fn reset_test_counters() {
    PACKED_ITERATIONS.set(0);
}

#[cfg(test)]
fn test_iterations() -> usize {
    PACKED_ITERATIONS.get()
}

#[cfg(test)]
mod tests {
    use crate::bytecode::vm_numeric_mutation_loop::{
        NumericMutationLoopKind, NumericMutationLoopPlan,
    };
    use crate::{Value, bytecode::compiler, eval};
    use qjs_parser::parse_script;

    use super::{reset_test_counters, test_iterations};

    #[test]
    fn runs_packed_clear_set_and_toggle_with_dynamic_layout_inputs() {
        for (assignment, initial, expected) in [
            ("&= ~(bit << (j & mask))", "-1", "-16"),
            ("|= bit << (j & mask)", "0", "15"),
            ("^= bit << (j & mask)", "0", "15"),
        ] {
            reset_test_counters();
            let source = format!(
                "function run(words, j, m, step, shift, mask, bit) {{ for (; j < m; j += step) words[j >> shift] {assignment}; return j + ':' + words[0]; }} run([{initial}], 0, 4, 1, 5, 31, 1);"
            );
            assert_eq!(
                eval(&source),
                Ok(Value::String(format!("4:{expected}").into()))
            );
            assert_eq!(test_iterations(), 3, "{assignment}");
        }
    }

    #[test]
    fn replays_the_first_non_number_word_after_committing_prior_updates() {
        reset_test_counters();
        assert_eq!(
            eval(
                "var calls = 0; var marker = { valueOf: function () { calls++; return -1; } }; function run(words) { var j = 0; for (; j < 34; j += 1) words[j >> 5] &= ~(1 << (j & 31)); return j + ':' + words[0] + ':' + words[1] + ':' + calls; } run([-1, marker]);",
            ),
            Ok(Value::String("34:0:-4:1".into()))
        );
        assert_eq!(test_iterations(), 32);
    }

    #[test]
    fn compiles_dynamic_packed_bitset_plan() {
        let script = parse_script(
            "function run(words, j, m, step, shift, mask, bit) { for (; j < m; j += step) words[j >> shift] &= ~(bit << (j & mask)); return j + ':' + words[0]; }",
        )
        .expect("source should parse");
        let bytecode = compiler::compile_script(&script).expect("source should compile");
        let function = bytecode
            .code
            .iter()
            .find_map(|operation| match operation {
                crate::bytecode::ir::Op::NewFunction { bytecode, .. } => Some(bytecode),
                _ => None,
            })
            .expect("function should compile");
        let plans = NumericMutationLoopPlan::compile_all(function);
        assert_eq!(plans.len(), 1, "{:#?}", function.code);
        let NumericMutationLoopKind::Dense(dense) = &plans[0].kind else {
            panic!("expected dense loop plan: {plans:#?}");
        };
        assert!(dense.has_packed_bitset_mutation());
    }
}
