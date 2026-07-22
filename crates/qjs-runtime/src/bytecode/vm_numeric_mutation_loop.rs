use std::rc::Rc;

use qjs_ast::{BinaryOp, UpdateOp};

use crate::{Value, value::OwnDataPropertyWrite};

use super::{
    ir::{Bytecode, Op},
    vm::Vm,
};

#[derive(Clone, Copy, Debug)]
enum NumericMutationOp {
    Add,
    Subtract,
}

impl NumericMutationOp {
    fn apply(self, value: f64, constant: f64) -> f64 {
        match self {
            Self::Add => value + constant,
            Self::Subtract => value - constant,
        }
    }
}

#[derive(Clone, Debug)]
struct NumericMutation {
    source: usize,
    target: usize,
    operation: NumericMutationOp,
    constant: f64,
}

/// A counted loop that scalar-replaces writable numeric fields on one ordinary
/// object. Every source iteration still performs each recurrence step; only
/// the unobservable intermediate property storage is sunk to the loop exit.
#[derive(Clone, Debug)]
pub(super) struct NumericMutationLoopPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    counter_slot: usize,
    limit_slot: usize,
    accumulator_slot: usize,
    block_result_slot: usize,
    loop_result_slot: usize,
    receiver_slot: usize,
    fields: Vec<Rc<str>>,
    mutations: Vec<NumericMutation>,
    checksum_field: usize,
}

impl NumericMutationLoopPlan {
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
        if !matches!(code.get(*exit), Some(Op::Pop)) {
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
        let mut receiver_slot = None;
        let mut fields = Vec::new();
        let mut mutations = Vec::new();
        while let Some((mutation, next, slot)) = compile_mutation(
            bytecode,
            cursor,
            *block_result_slot,
            *loop_result_slot,
            &mut fields,
        ) {
            if receiver_slot.is_some_and(|current| current != slot) {
                return None;
            }
            receiver_slot = Some(slot);
            mutations.push(mutation);
            cursor = next;
        }
        if mutations.is_empty() || mutations.len() > 8 {
            return None;
        }

        let (
            Op::LoadLocal(accumulator_slot),
            Op::GetPropNamed { key, cache },
            Op::Binary(BinaryOp::Add),
            Op::Dup,
            Op::AssignLocal(assigned_accumulator_slot),
            Op::Dup,
            Op::StoreLocal(accumulator_block_result_slot),
            Op::StoreLocal(accumulator_loop_result_slot),
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
        let receiver_slot = receiver_slot?;
        if cursor + 8 != tail
            || cache.local_slot() != Some(receiver_slot)
            || assigned_accumulator_slot != accumulator_slot
            || accumulator_block_result_slot != block_result_slot
            || accumulator_loop_result_slot != loop_result_slot
        {
            return None;
        }
        let checksum_field = field_index(&mut fields, key);

        Some(Self {
            header,
            backedge,
            exit: *exit,
            counter_slot: *counter_slot,
            limit_slot: *limit_slot,
            accumulator_slot: *accumulator_slot,
            block_result_slot: *block_result_slot,
            loop_result_slot: *loop_result_slot,
            receiver_slot,
            fields,
            mutations,
            checksum_field,
        })
    }

    fn try_run(&self, vm: &mut Vm<'_>) -> bool {
        if vm.direct_eval_with_stack {
            return false;
        }
        for slot in [
            self.counter_slot,
            self.limit_slot,
            self.accumulator_slot,
            self.block_result_slot,
            self.loop_result_slot,
            self.receiver_slot,
        ] {
            if !vm.slot_is_authoritative(slot) {
                return false;
            }
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
        let Some(Some(Value::Object(object))) = vm.locals.get(self.receiver_slot) else {
            return false;
        };
        if vm.is_global_object(&Value::Object(object.clone()))
            || crate::symbol::is_symbol_primitive(object)
            || crate::typed_array::is_typed_array_object(object)
            || object.is_module_namespace_exotic()
        {
            return false;
        }
        let mut values = Vec::with_capacity(self.fields.len());
        for field in &self.fields {
            let Some(value) = object.writable_own_data_number(field) else {
                return false;
            };
            values.push(value);
        }
        let object = object.clone();

        while counter < limit {
            for mutation in &self.mutations {
                values[mutation.target] = mutation
                    .operation
                    .apply(values[mutation.source], mutation.constant);
            }
            accumulator += values[self.checksum_field];
            counter += 1.0;
        }
        for mutation in &self.mutations {
            let key = &self.fields[mutation.target];
            let value = Value::Number(values[mutation.target]);
            if !matches!(
                object.write_existing_own_data_property(key, &value),
                OwnDataPropertyWrite::Written
            ) {
                return false;
            }
        }

        set_local_number(vm, self.counter_slot, counter);
        set_local_number(vm, self.accumulator_slot, accumulator);
        set_local_number(vm, self.block_result_slot, accumulator);
        set_local_number(vm, self.loop_result_slot, accumulator);
        vm.ip = self.exit + 1;
        true
    }
}

fn compile_mutation(
    bytecode: &Bytecode,
    cursor: usize,
    block_result_slot: usize,
    loop_result_slot: usize,
    fields: &mut Vec<Rc<str>>,
) -> Option<(NumericMutation, usize, usize)> {
    let code = &bytecode.code;
    let (
        Op::LoadLocal(receiver_slot),
        Op::GetPropNamed {
            key: source_key,
            cache,
        },
        Op::LoadConst(constant_index),
        Op::Binary(operation),
        Op::SetPropNamed {
            key: target_key, ..
        },
        Op::Dup,
        Op::StoreLocal(write_block_result_slot),
        Op::StoreLocal(write_loop_result_slot),
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
    if cache.local_slot() != Some(*receiver_slot)
        || write_block_result_slot != &block_result_slot
        || write_loop_result_slot != &loop_result_slot
    {
        return None;
    }
    let Value::Number(constant) = bytecode.constants.get(*constant_index)? else {
        return None;
    };
    let operation = match operation {
        BinaryOp::Add => NumericMutationOp::Add,
        BinaryOp::Sub => NumericMutationOp::Subtract,
        _ => return None,
    };
    let source = field_index(fields, source_key);
    let target = field_index(fields, target_key);
    Some((
        NumericMutation {
            source,
            target,
            operation,
            constant: *constant,
        },
        cursor + 8,
        *receiver_slot,
    ))
}

fn field_index(fields: &mut Vec<Rc<str>>, key: &Rc<str>) -> usize {
    if let Some(index) = fields.iter().position(|field| field == key) {
        return index;
    }
    fields.push(key.clone());
    fields.len() - 1
}

fn local_number(vm: &Vm<'_>, slot: usize) -> Option<f64> {
    match vm.local_slot_value(slot)? {
        Value::Number(value) => Some(value),
        _ => None,
    }
}

fn set_local_number(vm: &mut Vm<'_>, slot: usize, value: f64) {
    vm.locals[slot] = Some(Value::Number(value));
}

pub(super) fn try_run_numeric_mutation_loop(
    vm: &mut Vm<'_>,
    header: usize,
    backedge: usize,
) -> bool {
    vm.numeric_mutation_loop_plans
        .iter()
        .find(|plan| plan.header == header && plan.backedge == backedge)
        .cloned()
        .is_some_and(|plan| plan.try_run(vm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;
    use crate::{Value, eval};

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
    fn recognizes_named_numeric_recurrence() {
        let bytecode = nested_function(
            "function run(n) { var o = { a: 0, b: 0, c: 0 }; var sum = 0; for (var i = 0; i < n; i++) { o.a = o.c + 1; o.b = o.a + 1; o.c = o.b - 1; sum += o.c; } return sum; }",
        );
        assert_eq!(NumericMutationLoopPlan::compile_all(&bytecode).len(), 1);
    }

    #[test]
    fn commits_scalar_replaced_fields_at_loop_exit() {
        assert_eq!(
            eval(
                "function run(n) { var o = { a: 0, b: 0, c: 0 }; var sum = 0; for (var i = 0; i < n; i++) { o.a = o.c + 1; o.b = o.a + 1; o.c = o.b - 1; sum += o.c; } return sum + ':' + o.a + ':' + o.b + ':' + o.c; } run(1000);"
            ),
            Ok(Value::String("500500:1000:1001:1000".to_owned().into()))
        );
    }

    #[test]
    fn accessors_keep_the_observable_loop_path() {
        assert_eq!(
            eval(
                "function run(n) { var value = 0, writes = 0; var o = { b: 0, c: 0 }; Object.defineProperty(o, 'a', { get: function () { return value; }, set: function (next) { value = next; writes += 1; } }); var sum = 0; for (var i = 0; i < n; i++) { o.a = o.c + 1; o.b = o.a + 1; o.c = o.b - 1; sum += o.c; } return sum + ':' + writes; } run(4);"
            ),
            Ok(Value::String("10:4".to_owned().into()))
        );
    }

    #[test]
    fn read_only_fields_keep_sloppy_assignment_semantics() {
        assert_eq!(
            eval(
                "function run(n) { var o = { a: 0, b: 0, c: 0 }; Object.defineProperty(o, 'b', { writable: false }); var sum = 0; for (var i = 0; i < n; i++) { o.a = o.c + 1; o.b = o.a + 1; o.c = o.b - 1; sum += o.c; } return sum + ':' + o.a + ':' + o.b + ':' + o.c; } run(4);"
            ),
            Ok(Value::String("-4:0:0:-1".to_owned().into()))
        );
    }
}
