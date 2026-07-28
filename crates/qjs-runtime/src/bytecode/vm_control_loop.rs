use qjs_ast::{BinaryOp, UpdateOp};

use crate::{Value, to_int32_number, to_uint32_number};

use super::{
    ir::{Bytecode, Op},
    vm::Vm,
};

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static BITWISE_CONDITIONAL_SHIFT_HITS: Cell<usize> = const { Cell::new(0) };
}

#[derive(Clone, Copy, Debug)]
enum ControlLoopKind {
    Empty {
        block_result_slot: usize,
    },
    BitwiseBranch {
        accumulator_slot: usize,
        block_result_slot: usize,
        loop_result_slot: usize,
        mask: f64,
        expected: f64,
        then_delta: f64,
        else_delta: f64,
    },
    BitwiseConditionalShift {
        input_slot: usize,
        accumulator_slot: usize,
        block_result_slot: usize,
        loop_result_slot: usize,
        shift: u32,
    },
}

/// Prevalidated counted loop whose body is pure local control flow.
#[derive(Clone, Copy, Debug)]
pub(super) struct ControlLoopPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    counter_slot: usize,
    limit: ControlLoopLimit,
    kind: ControlLoopKind,
}

struct ControlLoopHeader {
    counter_slot: usize,
    limit: ControlLoopLimit,
    exit: usize,
    body_start: usize,
    seeded_block_result_slot: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
enum ControlLoopLimit {
    Local(usize),
    Constant(f64),
}

impl ControlLoopLimit {
    fn compile(bytecode: &Bytecode, op: &Op) -> Option<Self> {
        match op {
            Op::LoadLocal(slot) => Some(Self::Local(*slot)),
            Op::LoadConst(index) => Some(Self::Constant(number_constant(bytecode, *index)?)),
            _ => None,
        }
    }

    fn value(self, vm: &Vm<'_>) -> Option<f64> {
        match self {
            Self::Local(slot) => local_number_read(vm, slot),
            Self::Constant(value) => Some(value),
        }
    }

    fn references(self, slot: usize) -> bool {
        matches!(self, Self::Local(candidate) if candidate == slot)
    }
}

impl ControlLoopPlan {
    pub(super) fn compile_all(bytecode: &Bytecode) -> Vec<Self> {
        bytecode
            .code
            .iter()
            .enumerate()
            .filter_map(|(backedge, op)| match op {
                Op::Jump(header) if *header < backedge => {
                    Self::compile(bytecode, *header, backedge)
                }
                _ => None,
            })
            .collect()
    }

    pub(super) fn contains_instruction(&self, ip: usize) -> bool {
        (self.header..=self.backedge).contains(&ip)
    }

    fn compile(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<Self> {
        Self::compile_empty(bytecode, header, backedge)
            .or_else(|| Self::compile_bitwise_branch(bytecode, header, backedge))
            .or_else(|| Self::compile_bitwise_conditional_shift(bytecode, header, backedge))
    }

    fn compile_header(bytecode: &Bytecode, header: usize) -> Option<ControlLoopHeader> {
        let code = &bytecode.code;
        let (
            Op::LoadLocal(counter_slot),
            limit_op,
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
        let limit = ControlLoopLimit::compile(bytecode, limit_op)?;
        // The block-result seed prologue is emitted only where a statement
        // completion value is observable; a function body starts its loop body
        // directly and names the same slot at its first completion store.
        let (body_start, seeded_block_result_slot) =
            match (code.get(header + 5), code.get(header + 6)) {
                (Some(Op::LoadConst(_)), Some(Op::StoreLocal(slot))) => (header + 7, Some(*slot)),
                _ => (header + 5, None),
            };
        matches!(code.get(*exit), Some(Op::Pop)).then_some(ControlLoopHeader {
            counter_slot: *counter_slot,
            limit,
            exit: *exit,
            body_start,
            seeded_block_result_slot,
        })
    }

    /// Skips the two-instruction `undefined` seed a statement list emits for a
    /// result slot when completion values are observable. Function bodies omit
    /// it, so the sequence is optional.
    fn skip_optional_seed(code: &[Op], cursor: usize) -> usize {
        match (code.get(cursor), code.get(cursor + 1)) {
            (Some(Op::LoadConst(_)), Some(Op::StoreLocal(_))) => cursor + 2,
            _ => cursor,
        }
    }

    /// Matches the stores that follow a value-producing statement: the block
    /// and loop result slots where completion values are observable, and only
    /// the block slot inside a function body. Returns the cursor past the
    /// stores and the block slot written.
    /// Matches an arm's constant accumulator update. Where the arm's value is
    /// unobservable the statement is compiled without the duplication and
    /// without a completion store, so the following `LoadLocal` names the
    /// arm's result slot instead.
    fn match_arm_accumulator_update(
        code: &[Op],
        cursor: usize,
    ) -> Option<(usize, usize, usize, usize)> {
        let (
            Op::LoadLocal(accumulator_slot),
            Op::LoadConst(delta_index),
            Op::Binary(BinaryOp::Add),
        ) = (
            code.get(cursor)?,
            code.get(cursor + 1)?,
            code.get(cursor + 2)?,
        )
        else {
            return None;
        };
        let (assigned_slot, end, stored_result_slot) =
            match (code.get(cursor + 3)?, code.get(cursor + 4)) {
                (Op::Dup, Some(Op::AssignLocal(slot))) => {
                    let (end, block) = Self::match_completion_stores(code, cursor + 5, None)?;
                    (*slot, end, Some(block))
                }
                (Op::AssignLocal(slot), _) => (*slot, cursor + 4, None),
                _ => return None,
            };
        if assigned_slot != *accumulator_slot {
            return None;
        }
        let Op::LoadLocal(loaded_result_slot) = code.get(end)? else {
            return None;
        };
        if stored_result_slot.is_some_and(|slot| slot != *loaded_result_slot) {
            return None;
        }
        Some((*accumulator_slot, *delta_index, end, *loaded_result_slot))
    }

    fn match_completion_stores(
        code: &[Op],
        cursor: usize,
        expected_loop_result_slot: Option<usize>,
    ) -> Option<(usize, usize)> {
        if let (Some(Op::Dup), Some(Op::StoreLocal(block)), Some(Op::StoreLocal(loop_slot))) =
            (code.get(cursor), code.get(cursor + 1), code.get(cursor + 2))
            && expected_loop_result_slot.is_none_or(|slot| slot == *loop_slot)
        {
            return Some((cursor + 3, *block));
        }
        match code.get(cursor)? {
            Op::StoreLocal(block) => Some((cursor + 1, *block)),
            _ => None,
        }
    }

    fn compile_empty(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<Self> {
        let ControlLoopHeader {
            counter_slot,
            limit,
            exit,
            body_start,
            seeded_block_result_slot,
        } = Self::compile_header(bytecode, header)?;
        // Keep the pre-existing empty-loop tier's local-limit admission
        // unchanged. Constant limits are part of the new shifting-recursion
        // shape below, not an incidental widening of this older plan.
        if !matches!(limit, ControlLoopLimit::Local(_)) {
            return None;
        }
        let block_result_slot = seeded_block_result_slot?;
        let code = &bytecode.code;
        let (
            Op::LoadLocal(tail_counter_slot),
            Op::ToNumeric,
            Op::Dup,
            Op::Update(UpdateOp::Increment),
            Op::AssignLocal(assigned_counter_slot),
            Op::Pop,
            Op::Jump(tail_header),
        ) = (
            code.get(body_start)?,
            code.get(body_start + 1)?,
            code.get(body_start + 2)?,
            code.get(body_start + 3)?,
            code.get(body_start + 4)?,
            code.get(body_start + 5)?,
            code.get(body_start + 6)?,
        )
        else {
            return None;
        };
        if backedge != body_start + 6
            || tail_header != &header
            || tail_counter_slot != &counter_slot
            || assigned_counter_slot != &counter_slot
        {
            return None;
        }
        Some(Self {
            header,
            backedge,
            exit,
            counter_slot,
            limit,
            kind: ControlLoopKind::Empty { block_result_slot },
        })
    }

    fn compile_bitwise_branch(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<Self> {
        let ControlLoopHeader {
            counter_slot,
            limit,
            exit,
            body_start,
            seeded_block_result_slot,
        } = Self::compile_header(bytecode, header)?;
        // See `compile_empty`: this matcher intentionally keeps its original
        // local-limit contract while the new recurrence plan admits constants.
        if !matches!(limit, ControlLoopLimit::Local(_)) {
            return None;
        }
        let code = &bytecode.code;
        let cursor = body_start;
        let (
            Op::LoadLocal(condition_counter_slot),
            Op::LoadConst(mask_index),
            Op::Binary(BinaryOp::BitwiseAnd),
            Op::LoadConst(expected_index),
            Op::Binary(BinaryOp::StrictEq),
            Op::JumpIfFalse(else_start),
            Op::Pop,
        ) = (
            code.get(cursor)?,
            code.get(cursor + 1)?,
            code.get(cursor + 2)?,
            code.get(cursor + 3)?,
            code.get(cursor + 4)?,
            code.get(cursor + 5)?,
            code.get(cursor + 6)?,
        )
        else {
            return None;
        };
        if condition_counter_slot != &counter_slot {
            return None;
        }

        // Each arm optionally seeds the loop and branch result slots, applies a
        // constant delta to the accumulator, records the statement completion,
        // and reloads the branch result as the arm's value.
        let mut then_cursor = Self::skip_optional_seed(code, cursor + 7);
        then_cursor = Self::skip_optional_seed(code, then_cursor);
        let (accumulator_slot, then_delta_index, then_cursor, _then_result_slot) =
            Self::match_arm_accumulator_update(code, then_cursor)?;
        let accumulator_slot = &accumulator_slot;
        let then_delta_index = &then_delta_index;
        let Op::Jump(join) = code.get(then_cursor + 1)? else {
            return None;
        };
        if else_start != &(then_cursor + 2) {
            return None;
        }

        let mut else_cursor = *else_start;
        if !matches!(code.get(else_cursor), Some(Op::Pop)) {
            return None;
        }
        else_cursor = Self::skip_optional_seed(code, else_cursor + 1);
        else_cursor = Self::skip_optional_seed(code, else_cursor);
        let (else_accumulator_slot, else_delta_index, else_cursor, _else_result_slot) =
            Self::match_arm_accumulator_update(code, else_cursor)?;
        let else_delta_index = &else_delta_index;
        if &else_accumulator_slot != accumulator_slot || join != &(else_cursor + 1) {
            return None;
        }

        // The join records the `if` statement's own completion value.
        let (tail, block_result_slot) = Self::match_completion_stores(code, *join, None)?;
        if seeded_block_result_slot.is_some_and(|slot| slot != block_result_slot) {
            return None;
        }
        let (
            Op::LoadLocal(tail_block_result_slot),
            Op::StoreLocal(loop_result_slot),
            Op::LoadLocal(tail_counter_slot),
            Op::ToNumeric,
            Op::Dup,
            Op::Update(UpdateOp::Increment),
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
        if tail + 8 != backedge
            || tail_block_result_slot != &block_result_slot
            || tail_counter_slot != &counter_slot
            || assigned_counter_slot != &counter_slot
            || tail_header != &header
        {
            return None;
        }

        Some(Self {
            header,
            backedge,
            exit,
            counter_slot,
            limit,
            kind: ControlLoopKind::BitwiseBranch {
                accumulator_slot: *accumulator_slot,
                block_result_slot,
                loop_result_slot: *loop_result_slot,
                mask: number_constant(bytecode, *mask_index)?,
                expected: number_constant(bytecode, *expected_index)?,
                then_delta: number_constant(bytecode, *then_delta_index)?,
                else_delta: number_constant(bytecode, *else_delta_index)?,
            },
        })
    }

    /// Matches a pure local bit test paired with a left-shifting recurrence:
    ///
    /// ```text
    /// while (counter < limit) {
    ///     if (input & counter) accumulator++;
    ///     counter <<= constant;
    /// }
    /// ```
    ///
    /// This is a semantic bytecode shape, not a source-level population-count
    /// special case. The run-time guard below proves the shifting recurrence
    /// exits before batching the local writes.
    fn compile_bitwise_conditional_shift(
        bytecode: &Bytecode,
        header: usize,
        backedge: usize,
    ) -> Option<Self> {
        let ControlLoopHeader {
            counter_slot,
            limit,
            exit,
            body_start,
            seeded_block_result_slot,
        } = Self::compile_header(bytecode, header)?;
        if seeded_block_result_slot.is_some() {
            return None;
        }
        let code = &bytecode.code;
        let cursor = body_start;
        let (
            Op::LoadLocal(input_slot),
            Op::LoadLocal(condition_counter_slot),
            Op::Binary(BinaryOp::BitwiseAnd),
            Op::JumpIfFalse(false_start),
            Op::Pop,
            Op::LoadLocal(accumulator_slot),
            Op::ToNumeric,
            Op::Dup,
            Op::Update(UpdateOp::Increment),
            Op::AssignLocal(assigned_accumulator_slot),
            Op::Jump(join),
        ) = (
            code.get(cursor)?,
            code.get(cursor + 1)?,
            code.get(cursor + 2)?,
            code.get(cursor + 3)?,
            code.get(cursor + 4)?,
            code.get(cursor + 5)?,
            code.get(cursor + 6)?,
            code.get(cursor + 7)?,
            code.get(cursor + 8)?,
            code.get(cursor + 9)?,
            code.get(cursor + 10)?,
        )
        else {
            return None;
        };
        if condition_counter_slot != &counter_slot || assigned_accumulator_slot != accumulator_slot
        {
            return None;
        }
        let (Op::Pop, Op::LoadConst(undefined_index), Op::StoreLocal(block_result_slot)) = (
            code.get(*false_start)?,
            code.get(false_start + 1)?,
            code.get(false_start + 2)?,
        ) else {
            return None;
        };
        if join != &(*false_start + 2)
            || !matches!(
                bytecode.constants.get(*undefined_index),
                Some(Value::Undefined)
            )
        {
            return None;
        }
        let tail = *join + 1;
        let (
            Op::LoadLocal(tail_counter_slot),
            Op::LoadConst(shift_index),
            Op::Binary(BinaryOp::Shl),
            Op::AssignLocal(assigned_counter_slot),
            Op::LoadLocal(tail_block_result_slot),
            Op::StoreLocal(loop_result_slot),
            Op::Jump(tail_header),
        ) = (
            code.get(tail)?,
            code.get(tail + 1)?,
            code.get(tail + 2)?,
            code.get(tail + 3)?,
            code.get(tail + 4)?,
            code.get(tail + 5)?,
            code.get(tail + 6)?,
        )
        else {
            return None;
        };
        if tail_counter_slot != &counter_slot
            || assigned_counter_slot != &counter_slot
            || tail_block_result_slot != block_result_slot
            || tail_header != &header
            || tail + 6 != backedge
            || !bytecode.local_is_compiler_temporary(*block_result_slot)
            || !bytecode.local_is_compiler_temporary(*loop_result_slot)
        {
            return None;
        }
        let shift = to_uint32_number(number_constant(bytecode, *shift_index)?) & 0x1f;
        if shift == 0
            || [*input_slot, counter_slot, *accumulator_slot]
                .into_iter()
                .any(|slot| limit.references(slot))
            || *input_slot == counter_slot
            || *input_slot == *accumulator_slot
            || counter_slot == *accumulator_slot
            || [*block_result_slot, *loop_result_slot]
                .into_iter()
                .any(|slot| [*input_slot, counter_slot, *accumulator_slot].contains(&slot))
            || block_result_slot == loop_result_slot
        {
            return None;
        }

        Some(Self {
            header,
            backedge,
            exit,
            counter_slot,
            limit,
            kind: ControlLoopKind::BitwiseConditionalShift {
                input_slot: *input_slot,
                accumulator_slot: *accumulator_slot,
                block_result_slot: *block_result_slot,
                loop_result_slot: *loop_result_slot,
                shift,
            },
        })
    }

    fn try_run(self, vm: &mut Vm<'_>) -> bool {
        if vm.direct_eval_with_stack || !vm.slot_is_authoritative(self.counter_slot) {
            return false;
        }
        let Some(mut counter) = local_number(vm, self.counter_slot) else {
            return false;
        };
        let Some(limit) = self.limit.value(vm) else {
            return false;
        };
        match self.kind {
            ControlLoopKind::Empty { block_result_slot } => {
                if !vm.slot_is_authoritative(block_result_slot) {
                    return false;
                }
                while counter < limit {
                    counter += 1.0;
                }
                set_local_number(vm, self.counter_slot, counter);
            }
            ControlLoopKind::BitwiseBranch {
                accumulator_slot,
                block_result_slot,
                loop_result_slot,
                mask,
                expected,
                then_delta,
                else_delta,
            } => {
                if [accumulator_slot, block_result_slot, loop_result_slot]
                    .into_iter()
                    .any(|slot| !vm.slot_is_authoritative(slot))
                {
                    return false;
                }
                let Some(mut accumulator) = local_number(vm, accumulator_slot) else {
                    return false;
                };
                while counter < limit {
                    let masked = f64::from(to_int32_number(counter) & to_int32_number(mask));
                    accumulator += if masked == expected {
                        then_delta
                    } else {
                        else_delta
                    };
                    counter += 1.0;
                }
                set_local_number(vm, self.counter_slot, counter);
                set_local_number(vm, accumulator_slot, accumulator);
                set_local_number(vm, block_result_slot, accumulator);
                set_local_number(vm, loop_result_slot, accumulator);
            }
            ControlLoopKind::BitwiseConditionalShift {
                input_slot,
                accumulator_slot,
                block_result_slot,
                loop_result_slot,
                shift,
            } => {
                if vm.bytecode.contains_direct_eval()
                    || vm.bytecode.contains_with()
                    || [
                        input_slot,
                        accumulator_slot,
                        block_result_slot,
                        loop_result_slot,
                    ]
                    .into_iter()
                    .any(|slot| !vm.slot_is_authoritative(slot))
                {
                    return false;
                }
                let Some(input) = local_number_read(vm, input_slot) else {
                    return false;
                };
                let Some(mut accumulator) = local_number(vm, accumulator_slot) else {
                    return false;
                };
                #[cfg(test)]
                BITWISE_CONDITIONAL_SHIFT_HITS.with(|hits| hits.set(hits.get() + 1));
                // At a backedge the source loop has already completed at least
                // one iteration. If its next header check fails, preserving the
                // existing completion slots and skipping the exit `Pop` is the
                // exact ordinary continuation.
                if counter < limit {
                    if !counter.is_finite()
                        || counter.fract() != 0.0
                        || counter <= 0.0
                        || !limit.is_finite()
                        || limit <= 0.0
                    {
                        return false;
                    }
                    let input_word = to_int32_number(input);
                    let mut last_completion = Value::Undefined;
                    // A positive signed 32-bit value shifted left by a
                    // nonzero count either reaches the positive limit or
                    // becomes non-positive within 32 transitions. Refusing
                    // the latter leaves every potentially non-terminating
                    // recurrence to ordinary execution without publication.
                    for _ in 0..32 {
                        if counter >= limit {
                            break;
                        }
                        if (input_word & to_int32_number(counter)) != 0 {
                            last_completion = Value::Number(accumulator);
                            accumulator += 1.0;
                        }
                        counter = f64::from(to_int32_number(counter) << shift);
                    }
                    if counter < limit {
                        return false;
                    }
                    set_local_number(vm, accumulator_slot, accumulator);
                    set_local_value(vm, block_result_slot, last_completion.clone());
                    set_local_value(vm, loop_result_slot, last_completion);
                }
                set_local_number(vm, self.counter_slot, counter);
            }
        }
        vm.ip = self.exit + 1;
        true
    }
}

pub(super) fn try_run_control_loop(vm: &mut Vm<'_>, header: usize, backedge: usize) -> bool {
    vm.control_loop_plans
        .iter()
        .find(|plan| plan.header == header && plan.backedge == backedge)
        .copied()
        .is_some_and(|plan| plan.try_run(vm))
}

fn number_constant(bytecode: &Bytecode, index: usize) -> Option<f64> {
    match bytecode.constants.get(index)? {
        Value::Number(value) => Some(*value),
        _ => None,
    }
}

fn local_number(vm: &Vm<'_>, slot: usize) -> Option<f64> {
    match vm.locals.get(slot)? {
        Some(Value::Number(value)) => Some(*value),
        _ => None,
    }
}

fn local_number_read(vm: &Vm<'_>, slot: usize) -> Option<f64> {
    match vm.local_slot_value(slot)? {
        Value::Number(value) => Some(value),
        _ => None,
    }
}

fn set_local_number(vm: &mut Vm<'_>, slot: usize, value: f64) {
    vm.locals[slot] = Some(Value::Number(value));
}

fn set_local_value(vm: &mut Vm<'_>, slot: usize, value: Value) {
    vm.locals[slot] = Some(value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Value, bytecode::compiler, eval};

    pub(super) fn nested_function(source: &str) -> Bytecode {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        let bytecode = compiler::compile_script(&script).expect("source should compile");
        bytecode
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
                _ => None,
            })
            .expect("function bytecode should be nested in the script")
    }

    #[test]
    fn recognizes_empty_and_bitwise_branch_loops() {
        let empty =
            nested_function("function run(n) { var i; for (i = 0; i < n; i++) {} return i; }");
        assert!(matches!(
            ControlLoopPlan::compile_all(&empty).as_slice(),
            [ControlLoopPlan {
                kind: ControlLoopKind::Empty { .. },
                ..
            }]
        ));

        let branch = nested_function(
            "function run(n) { var s = 0; for (var i = 0; i < n; i++) { if ((i & 1) === 0) { s += 1; } else { s += 2; } } return s; }",
        );
        assert!(matches!(
            ControlLoopPlan::compile_all(&branch).as_slice(),
            [ControlLoopPlan {
                kind: ControlLoopKind::BitwiseBranch { .. },
                ..
            }]
        ));

        let conditional_shift = nested_function(
            "function bitsinbyte(b) { var m = 1, c = 0; while (m < 0x100) { if (b & m) c++; m <<= 1; } return c; }",
        );
        assert!(matches!(
            ControlLoopPlan::compile_all(&conditional_shift).as_slice(),
            [ControlLoopPlan {
                kind: ControlLoopKind::BitwiseConditionalShift { .. },
                ..
            }]
        ));
    }

    fn reset_conditional_shift_hits() {
        BITWISE_CONDITIONAL_SHIFT_HITS.with(|hits| hits.set(0));
    }

    fn conditional_shift_hits() -> usize {
        BITWISE_CONDITIONAL_SHIFT_HITS.with(Cell::get)
    }

    #[test]
    fn bitwise_conditional_shift_preserves_numeric_results_and_deopts() {
        reset_conditional_shift_hits();
        assert_eq!(
            eval(
                "function bitsinbyte(b) { var m = 1, c = 0; while (m < 0x100) { if (b & m) c++; m <<= 1; } return c + ':' + m; } bitsinbyte(173);",
            ),
            Ok(Value::String("5:256".to_owned().into()))
        );
        assert_eq!(conditional_shift_hits(), 1);

        reset_conditional_shift_hits();
        assert_eq!(
            eval(
                "function stride(b) { var m = 1, c = 0; while (m < 64) { if (b & m) c++; m <<= 2; } return c + ':' + m; } stride(85);",
            ),
            Ok(Value::String("3:64".to_owned().into()))
        );
        assert_eq!(conditional_shift_hits(), 1);

        // JavaScript masks the shift count to five bits, and a false branch
        // must retain the numeric accumulator exactly (including `-0`).
        reset_conditional_shift_hits();
        assert_eq!(
            eval(
                "function maskedShift(b) { var m = 1, c = 0; while (m < 16) { if (b & m) c++; m <<= 33; } return c + ':' + m; } maskedShift(11);",
            ),
            Ok(Value::String("3:16".to_owned().into()))
        );
        assert_eq!(conditional_shift_hits(), 1);

        reset_conditional_shift_hits();
        assert_eq!(
            eval(
                "function falseOnly(b) { var m = 1, c = -0; while (m < 255.5) { if (b & m) c++; m <<= 1; } return Object.is(c, -0) + ':' + c + ':' + m; } falseOnly(Infinity);",
            ),
            Ok(Value::String("true:0:256".to_owned().into()))
        );
        assert_eq!(conditional_shift_hits(), 1);

        // A captured accumulator could be observed by an escaped closure, so
        // the plan must retain the generic loop even though its bytecode shape
        // is otherwise identical.
        reset_conditional_shift_hits();
        assert_eq!(
            eval(
                "function captured(b) { var m = 1, c = 0; function read() { return c; } while (m < 16) { if (b & m) c++; m <<= 1; } return read() + ':' + m; } captured(11);",
            ),
            Ok(Value::String("3:16".to_owned().into()))
        );
        assert_eq!(conditional_shift_hits(), 0);

        let non_progressing = nested_function(
            "function stuck(b) { var m = 1, c = 0; while (m < 8) { if (b & m) c++; m <<= 0; } return c; }",
        );
        assert!(ControlLoopPlan::compile_all(&non_progressing).is_empty());
    }
}
