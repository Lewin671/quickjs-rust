use qjs_ast::{BinaryOp, UpdateOp};

use crate::{Value, to_int32_number};

use super::{
    ir::{Bytecode, Op},
    vm::Vm,
};

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
}

/// Prevalidated counted loop whose body is pure local control flow.
#[derive(Clone, Copy, Debug)]
pub(super) struct ControlLoopPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    counter_slot: usize,
    limit_slot: usize,
    kind: ControlLoopKind,
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

    fn compile(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<Self> {
        Self::compile_empty(bytecode, header, backedge)
            .or_else(|| Self::compile_bitwise_branch(bytecode, header, backedge))
    }

    fn compile_header(bytecode: &Bytecode, header: usize) -> Option<(usize, usize, usize, usize)> {
        let code = &bytecode.code;
        let (
            Op::LoadLocal(counter_slot),
            Op::LoadLocal(limit_slot),
            Op::Binary(BinaryOp::Lt),
            Op::JumpIfFalse(exit),
            Op::Pop,
            Op::LoadConst(_),
            Op::StoreLocal(block_result_slot),
        ) = (
            code.get(header)?,
            code.get(header + 1)?,
            code.get(header + 2)?,
            code.get(header + 3)?,
            code.get(header + 4)?,
            code.get(header + 5)?,
            code.get(header + 6)?,
        )
        else {
            return None;
        };
        matches!(code.get(*exit), Some(Op::Pop)).then_some((
            *counter_slot,
            *limit_slot,
            *exit,
            *block_result_slot,
        ))
    }

    fn compile_empty(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<Self> {
        let (counter_slot, limit_slot, exit, block_result_slot) =
            Self::compile_header(bytecode, header)?;
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
            code.get(header + 7)?,
            code.get(header + 8)?,
            code.get(header + 9)?,
            code.get(header + 10)?,
            code.get(header + 11)?,
            code.get(header + 12)?,
            code.get(header + 13)?,
        )
        else {
            return None;
        };
        if backedge != header + 13
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
            limit_slot,
            kind: ControlLoopKind::Empty { block_result_slot },
        })
    }

    fn compile_bitwise_branch(bytecode: &Bytecode, header: usize, backedge: usize) -> Option<Self> {
        let (counter_slot, limit_slot, exit, block_result_slot) =
            Self::compile_header(bytecode, header)?;
        let code = &bytecode.code;
        let cursor = header + 7;
        let (
            Op::LoadLocal(condition_counter_slot),
            Op::LoadConst(mask_index),
            Op::Binary(BinaryOp::BitwiseAnd),
            Op::LoadConst(expected_index),
            Op::Binary(BinaryOp::StrictEq),
            Op::JumpIfFalse(else_start),
            Op::Pop,
            Op::LoadConst(_),
            Op::StoreLocal(loop_result_slot),
            Op::LoadConst(_),
            Op::StoreLocal(then_result_slot),
            Op::LoadLocal(accumulator_slot),
            Op::LoadConst(then_delta_index),
            Op::Binary(BinaryOp::Add),
            Op::Dup,
            Op::AssignLocal(then_accumulator_slot),
            Op::Dup,
            Op::StoreLocal(stored_then_result_slot),
            Op::StoreLocal(then_loop_result_slot),
            Op::LoadLocal(loaded_then_result_slot),
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
            code.get(cursor + 11)?,
            code.get(cursor + 12)?,
            code.get(cursor + 13)?,
            code.get(cursor + 14)?,
            code.get(cursor + 15)?,
            code.get(cursor + 16)?,
            code.get(cursor + 17)?,
            code.get(cursor + 18)?,
            code.get(cursor + 19)?,
            code.get(cursor + 20)?,
        )
        else {
            return None;
        };
        if condition_counter_slot != &counter_slot
            || else_start != &(cursor + 21)
            || then_accumulator_slot != accumulator_slot
            || stored_then_result_slot != then_result_slot
            || then_loop_result_slot != loop_result_slot
            || loaded_then_result_slot != then_result_slot
            || join != &(cursor + 35)
        {
            return None;
        }

        let else_cursor = *else_start;
        let (
            Op::Pop,
            Op::LoadConst(_),
            Op::StoreLocal(else_loop_result_slot),
            Op::LoadConst(_),
            Op::StoreLocal(else_result_slot),
            Op::LoadLocal(else_accumulator_slot),
            Op::LoadConst(else_delta_index),
            Op::Binary(BinaryOp::Add),
            Op::Dup,
            Op::AssignLocal(assigned_else_accumulator_slot),
            Op::Dup,
            Op::StoreLocal(stored_else_result_slot),
            Op::StoreLocal(stored_else_loop_result_slot),
            Op::LoadLocal(loaded_else_result_slot),
            Op::Dup,
            Op::StoreLocal(stored_block_result_slot),
            Op::StoreLocal(join_loop_result_slot),
        ) = (
            code.get(else_cursor)?,
            code.get(else_cursor + 1)?,
            code.get(else_cursor + 2)?,
            code.get(else_cursor + 3)?,
            code.get(else_cursor + 4)?,
            code.get(else_cursor + 5)?,
            code.get(else_cursor + 6)?,
            code.get(else_cursor + 7)?,
            code.get(else_cursor + 8)?,
            code.get(else_cursor + 9)?,
            code.get(else_cursor + 10)?,
            code.get(else_cursor + 11)?,
            code.get(else_cursor + 12)?,
            code.get(else_cursor + 13)?,
            code.get(else_cursor + 14)?,
            code.get(else_cursor + 15)?,
            code.get(else_cursor + 16)?,
        )
        else {
            return None;
        };
        if else_loop_result_slot != loop_result_slot
            || else_accumulator_slot != accumulator_slot
            || assigned_else_accumulator_slot != accumulator_slot
            || stored_else_result_slot != else_result_slot
            || stored_else_loop_result_slot != loop_result_slot
            || loaded_else_result_slot != else_result_slot
            || stored_block_result_slot != &block_result_slot
            || join_loop_result_slot != loop_result_slot
        {
            return None;
        }

        let tail = else_cursor + 17;
        let (
            Op::LoadLocal(tail_block_result_slot),
            Op::StoreLocal(tail_loop_result_slot),
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
            || tail_loop_result_slot != loop_result_slot
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
            limit_slot,
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

    fn try_run(self, vm: &mut Vm<'_>) -> bool {
        if vm.direct_eval_with_stack || !vm.slot_is_authoritative(self.counter_slot) {
            return false;
        }
        let Some(mut counter) = local_number(vm, self.counter_slot) else {
            return false;
        };
        let Some(limit) = local_number_read(vm, self.limit_slot) else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;

    fn nested_function(source: &str) -> Bytecode {
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
    }
}
