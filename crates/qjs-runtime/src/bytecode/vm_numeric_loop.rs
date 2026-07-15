use qjs_ast::{BinaryOp, UpdateOp};

use crate::Value;

use super::{
    ir::{Bytecode, NamedPropertyCache, Op},
    vm::Vm,
};

#[derive(Clone, Debug)]
enum NumericLoopTerm {
    NamedProperty {
        receiver_slot: usize,
        cache: NamedPropertyCache,
    },
    DenseIndex {
        receiver_slot: usize,
        index: usize,
    },
}

/// Prevalidated counted loop whose body only adds stable numeric reads.
#[derive(Clone, Debug)]
pub(super) struct NumericLoopPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    counter_slot: usize,
    limit_slot: usize,
    accumulator_slot: usize,
    block_result_slot: usize,
    loop_result_slot: usize,
    terms: Vec<NumericLoopTerm>,
}

impl NumericLoopPlan {
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
        if !matches!(code.get(*exit), Some(Op::Pop)) || backedge < header + 16 {
            return None;
        }

        let tail = backedge.checked_sub(8)?;
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
            || tail_header != &header
            || tail_block_result_slot != block_result_slot
            || tail_counter_slot != counter_slot
            || assigned_counter_slot != counter_slot
        {
            return None;
        }

        let mut cursor = header + 7;
        let mut accumulator_slot = None;
        let mut terms = Vec::new();
        while cursor < tail {
            let (
                Op::LoadLocal(term_accumulator_slot),
                read,
                Op::Binary(BinaryOp::Add),
                Op::Dup,
                Op::AssignLocal(assigned_accumulator_slot),
                Op::Dup,
                Op::StoreLocal(term_block_result_slot),
                Op::StoreLocal(term_loop_result_slot),
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
            if assigned_accumulator_slot != term_accumulator_slot
                || term_block_result_slot != block_result_slot
                || term_loop_result_slot != loop_result_slot
                || accumulator_slot.is_some_and(|slot| slot != *term_accumulator_slot)
            {
                return None;
            }
            accumulator_slot = Some(*term_accumulator_slot);
            terms.push(NumericLoopTerm::compile(read)?);
            cursor += 8;
        }
        if cursor != tail || terms.is_empty() {
            return None;
        }

        Some(Self {
            header,
            backedge,
            exit: *exit,
            counter_slot: *counter_slot,
            limit_slot: *limit_slot,
            accumulator_slot: accumulator_slot?,
            block_result_slot: *block_result_slot,
            loop_result_slot: *loop_result_slot,
            terms,
        })
    }

    fn try_run(&self, vm: &mut Vm<'_>) -> bool {
        if vm.direct_eval_with_stack {
            return false;
        }
        let required_slots = [
            self.counter_slot,
            self.limit_slot,
            self.accumulator_slot,
            self.block_result_slot,
            self.loop_result_slot,
        ];
        if required_slots
            .into_iter()
            .any(|slot| !vm.slot_is_authoritative(slot))
        {
            return false;
        }
        let Some(mut counter) = local_number(vm, self.counter_slot) else {
            return false;
        };
        let Some(limit) = local_number(vm, self.limit_slot) else {
            return false;
        };
        let Some(mut accumulator) = local_number(vm, self.accumulator_slot) else {
            return false;
        };
        let Some(terms) = self
            .terms
            .iter()
            .map(|term| term.number(vm))
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };

        while counter < limit {
            for term in &terms {
                accumulator += term;
            }
            counter += 1.0;
        }

        set_local_number(vm, self.counter_slot, counter);
        set_local_number(vm, self.accumulator_slot, accumulator);
        set_local_number(vm, self.block_result_slot, accumulator);
        set_local_number(vm, self.loop_result_slot, accumulator);
        // A normal failing test leaves its boolean on the operand stack for
        // the exit Pop. The trace has already proved the same `counter < limit`
        // result, so resume immediately after that Pop.
        vm.ip = self.exit + 1;
        true
    }
}

impl NumericLoopTerm {
    fn compile(op: &Op) -> Option<Self> {
        match op {
            Op::GetPropNamed { cache, .. } => Some(Self::NamedProperty {
                receiver_slot: cache.local_slot()?,
                cache: cache.clone(),
            }),
            Op::GetPropIndex(encoded) if usize::BITS > u32::BITS => {
                let receiver_slot = (encoded >> u32::BITS).checked_sub(1)?;
                Some(Self::DenseIndex {
                    receiver_slot,
                    index: encoded & u32::MAX as usize,
                })
            }
            _ => None,
        }
    }

    fn number(&self, vm: &Vm<'_>) -> Option<f64> {
        match self {
            Self::NamedProperty {
                receiver_slot,
                cache,
            } => {
                if !vm.slot_is_authoritative(*receiver_slot) {
                    return None;
                }
                let Some(Some(Value::Object(object))) = vm.locals.get(*receiver_slot) else {
                    return None;
                };
                match cache.get(object)? {
                    Value::Number(value) => Some(value),
                    _ => None,
                }
            }
            Self::DenseIndex {
                receiver_slot,
                index,
            } => {
                if !vm.slot_is_authoritative(*receiver_slot) {
                    return None;
                }
                let Some(Some(Value::Array(array))) = vm.locals.get(*receiver_slot) else {
                    return None;
                };
                match array.direct_dense_index_value(*index)? {
                    Value::Number(value) => Some(value),
                    _ => None,
                }
            }
        }
    }
}

pub(super) fn try_run_numeric_loop(vm: &mut Vm<'_>, header: usize, backedge: usize) -> bool {
    let plan = vm
        .numeric_loop_plans
        .iter()
        .find(|plan| plan.header == header && plan.backedge == backedge)
        .cloned();
    plan.is_some_and(|plan| plan.try_run(vm))
}

fn local_number(vm: &Vm<'_>, slot: usize) -> Option<f64> {
    match vm.locals.get(slot)? {
        Some(Value::Number(value)) => Some(*value),
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
    fn recognizes_named_property_accumulation_loop() {
        let bytecode = nested_function(
            "function sum(n) { var o = { a: 1, b: 2 }; var s = 0; for (var i = 0; i < n; i++) { s += o.a; s += o.b; } return s; }",
        );
        let plans = NumericLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].terms.len(), 2);
    }

    #[test]
    fn recognizes_dense_array_accumulation_loop() {
        let bytecode = nested_function(
            "function sum(n) { var a = [1, 2, 3]; var s = 0; for (var i = 0; i < n; i++) { s += a[0]; s += a[1]; s += a[2]; } return s; }",
        );
        let plans = NumericLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].terms.len(), 3);
    }

    #[test]
    fn rejects_loop_bodies_with_observable_calls() {
        let bytecode = nested_function(
            "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += Number(i); } return s; }",
        );
        assert!(NumericLoopPlan::compile_all(&bytecode).is_empty());
    }
}
