//! Escape-checked scalar replacement for allocation-only counted loops.
//!
//! A plan is admitted only when every literal input is a numeric constant, the
//! newly allocated object or array is read exclusively through statically
//! resolved own data slots, and its local never escapes the loop. The first
//! iteration still executes canonically; at the backedge, the remaining
//! iterations evaluate the same numeric expression without materializing dead
//! intermediate containers. Any dynamic scope, closure, accessor-capable read,
//! hole, spread, out-of-range index, or later use of the container rejects the
//! plan before execution.

use qjs_ast::{BinaryOp, UpdateOp};

use crate::{Value, value::ObjectLiteralShape};

use super::{
    ir::{ArrayElementKind, Bytecode, Op},
    vm::Vm,
};

#[derive(Clone, Copy, Debug)]
enum LiteralKind {
    Object,
    Array,
}

#[derive(Clone, Copy, Debug)]
enum ScalarOp {
    Accumulator,
    Constant(f64),
    Add,
}

#[derive(Clone, Debug)]
enum LiteralLayout {
    Object {
        shape: std::rc::Rc<ObjectLiteralShape>,
        inputs: Vec<f64>,
    },
    Array(Vec<f64>),
}

impl LiteralLayout {
    fn kind(&self) -> LiteralKind {
        match self {
            Self::Object { .. } => LiteralKind::Object,
            Self::Array(_) => LiteralKind::Array,
        }
    }

    fn named_value(&self, key: &str) -> Option<f64> {
        let Self::Object { shape, inputs } = self else {
            return None;
        };
        inputs.get(shape.final_input_index(key)?).copied()
    }

    fn indexed_value(&self, index: usize) -> Option<f64> {
        let Self::Array(values) = self else {
            return None;
        };
        values.get(index).copied()
    }
}

/// A counted loop whose per-iteration container allocation is proven dead.
#[derive(Clone, Debug)]
pub(super) struct AllocationLoopPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    counter_slot: usize,
    limit_slot: usize,
    accumulator_slot: usize,
    block_result_slot: usize,
    loop_result_slot: usize,
    receiver_slot: usize,
    literal_kind: LiteralKind,
    expression: Vec<ScalarOp>,
}

impl AllocationLoopPlan {
    pub(super) fn compile_all(bytecode: &Bytecode) -> Vec<Self> {
        if bytecode.contains_direct_eval() || bytecode.contains_with() {
            return Vec::new();
        }
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
        let mut literal_inputs = Vec::new();
        while let Some(Op::LoadConst(index)) = code.get(cursor) {
            literal_inputs.push(number_constant(bytecode, *index)?);
            cursor += 1;
        }
        let layout = match code.get(cursor)? {
            Op::NewObjectDataLiteral { shape } if shape.input_len() == literal_inputs.len() => {
                LiteralLayout::Object {
                    shape: shape.clone(),
                    inputs: literal_inputs,
                }
            }
            Op::NewArray { elements }
                if elements.len() == literal_inputs.len()
                    && elements
                        .iter()
                        .all(|element| matches!(element, ArrayElementKind::Expr)) =>
            {
                LiteralLayout::Array(literal_inputs)
            }
            _ => return None,
        };
        cursor += 1;
        let Op::StoreLocal(receiver_slot) = code.get(cursor)? else {
            return None;
        };
        let allocation_store = cursor;
        cursor += 1;
        if !matches!(
            (code.get(cursor), code.get(cursor + 1)),
            (Some(Op::LoadConst(_)), Some(Op::Pop))
        ) {
            return None;
        }
        cursor += 2;

        let expression_end = tail.checked_sub(5)?;
        let (
            Op::Dup,
            Op::AssignLocal(accumulator_slot),
            Op::Dup,
            Op::StoreLocal(expression_block_result_slot),
            Op::StoreLocal(expression_loop_result_slot),
        ) = (
            code.get(expression_end)?,
            code.get(expression_end + 1)?,
            code.get(expression_end + 2)?,
            code.get(expression_end + 3)?,
            code.get(expression_end + 4)?,
        )
        else {
            return None;
        };
        if cursor >= expression_end
            || expression_block_result_slot != block_result_slot
            || expression_loop_result_slot != loop_result_slot
        {
            return None;
        }

        let mut expression = Vec::new();
        let mut stack_depth = 0_usize;
        let mut accumulator_reads = 0_usize;
        let mut literal_reads = 0_usize;
        let mut allowed_receiver_reads = Vec::new();
        for (ip, op) in code.iter().enumerate().take(expression_end).skip(cursor) {
            match op {
                Op::LoadLocal(slot) if slot == accumulator_slot => {
                    expression.push(ScalarOp::Accumulator);
                    accumulator_reads += 1;
                    stack_depth += 1;
                }
                Op::LoadConst(index) => {
                    expression.push(ScalarOp::Constant(number_constant(bytecode, *index)?));
                    stack_depth += 1;
                }
                Op::GetPropNamed { key, cache } if cache.local_slot() == Some(*receiver_slot) => {
                    expression.push(ScalarOp::Constant(layout.named_value(key)?));
                    literal_reads += 1;
                    stack_depth += 1;
                    allowed_receiver_reads.push(ip);
                }
                Op::GetPropIndex(encoded) => {
                    let (index, slot) = decoded_index_receiver(*encoded)?;
                    if slot != *receiver_slot {
                        return None;
                    }
                    expression.push(ScalarOp::Constant(layout.indexed_value(index)?));
                    literal_reads += 1;
                    stack_depth += 1;
                    allowed_receiver_reads.push(ip);
                }
                Op::Binary(BinaryOp::Add) if stack_depth >= 2 => {
                    expression.push(ScalarOp::Add);
                    stack_depth -= 1;
                }
                _ => return None,
            }
        }
        if stack_depth != 1 || accumulator_reads != 1 || literal_reads == 0 {
            return None;
        }
        if receiver_is_observable(
            bytecode,
            *receiver_slot,
            allocation_store,
            &allowed_receiver_reads,
        ) {
            return None;
        }
        if [
            *counter_slot,
            *limit_slot,
            *accumulator_slot,
            *block_result_slot,
            *loop_result_slot,
        ]
        .contains(receiver_slot)
        {
            return None;
        }

        Some(Self {
            header,
            backedge,
            exit: *exit,
            counter_slot: *counter_slot,
            limit_slot: *limit_slot,
            accumulator_slot: *accumulator_slot,
            block_result_slot: *block_result_slot,
            loop_result_slot: *loop_result_slot,
            receiver_slot: *receiver_slot,
            literal_kind: layout.kind(),
            expression,
        })
    }

    fn try_run(&self, vm: &mut Vm<'_>) -> bool {
        if vm.direct_eval_with_stack
            || [
                self.counter_slot,
                self.limit_slot,
                self.accumulator_slot,
                self.block_result_slot,
                self.loop_result_slot,
                self.receiver_slot,
            ]
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
        let receiver_matches = matches!(
            (self.literal_kind, vm.locals.get(self.receiver_slot)),
            (LiteralKind::Object, Some(Some(Value::Object(_))))
                | (LiteralKind::Array, Some(Some(Value::Array(_))))
        );
        if !receiver_matches {
            return false;
        }

        let mut scalar_stack = Vec::with_capacity(self.expression.len());
        while counter < limit {
            scalar_stack.clear();
            for operation in &self.expression {
                match operation {
                    ScalarOp::Accumulator => scalar_stack.push(accumulator),
                    ScalarOp::Constant(value) => scalar_stack.push(*value),
                    ScalarOp::Add => {
                        let right = scalar_stack.pop().expect("validated scalar RHS");
                        let left = scalar_stack.pop().expect("validated scalar LHS");
                        scalar_stack.push(left + right);
                    }
                }
            }
            accumulator = scalar_stack.pop().expect("validated scalar result");
            counter += 1.0;
        }

        set_local_number(vm, self.counter_slot, counter);
        set_local_number(vm, self.accumulator_slot, accumulator);
        set_local_number(vm, self.block_result_slot, accumulator);
        set_local_number(vm, self.loop_result_slot, accumulator);
        vm.ip = self.exit + 1;
        true
    }
}

fn number_constant(bytecode: &Bytecode, index: usize) -> Option<f64> {
    match bytecode.constants.get(index)? {
        Value::Number(value) => Some(*value),
        _ => None,
    }
}

fn decoded_index_receiver(encoded: usize) -> Option<(usize, usize)> {
    if usize::BITS <= u32::BITS {
        return None;
    }
    let receiver = (encoded >> u32::BITS).checked_sub(1)?;
    Some((encoded & u32::MAX as usize, receiver))
}

fn receiver_is_observable(
    bytecode: &Bytecode,
    receiver_slot: usize,
    allocation_store: usize,
    allowed_reads: &[usize],
) -> bool {
    bytecode.code.iter().enumerate().any(|(ip, op)| {
        if ip == allocation_store || allowed_reads.contains(&ip) {
            return false;
        }
        match op {
            Op::LoadLocal(slot)
            | Op::LoadLocalOrUndefined(slot)
            | Op::AppendStringLiteralLocal { slot, .. } => *slot == receiver_slot,
            Op::GetPropNamed { cache, .. } => cache.local_slot() == Some(receiver_slot),
            Op::GetPropIndex(encoded) => {
                decoded_index_receiver(*encoded).is_some_and(|(_, slot)| slot == receiver_slot)
            }
            // A nested closure or class can capture a local without first
            // loading it onto the operand stack. Rejecting all such bodies is
            // deliberately conservative and keeps the escape proof local.
            Op::NewFunction { .. } | Op::NewClass { .. } => true,
            _ => false,
        }
    })
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

#[inline(always)]
pub(super) fn try_run_allocation_loop(vm: &mut Vm<'_>, header: usize, backedge: usize) -> bool {
    let plan = vm
        .bytecode
        .allocation_loop_plans
        .get_or_init(|| AllocationLoopPlan::compile_all(vm.bytecode))
        .iter()
        .find(|plan| plan.header == header && plan.backedge == backedge)
        .cloned();
    plan.is_some_and(|plan| plan.try_run(vm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;
    use crate::eval;

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

    fn plan_count(source: &str) -> usize {
        AllocationLoopPlan::compile_all(&nested_function(source)).len()
    }

    #[test]
    fn recognizes_varied_dead_object_and_array_literals() {
        assert_eq!(
            plan_count(
                "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var record = { x: 4, y: 5, z: 6 }; sum += record.z + record.x; } return sum; }"
            ),
            1
        );
        assert_eq!(
            plan_count(
                "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var values = [4, 5, 6, 7, 8]; sum += values[4] + values[0]; } return sum; }"
            ),
            1
        );
        assert_eq!(
            plan_count(
                "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var record = { x: 1, x: 4, y: 5 }; sum += record.x + record.y; } return sum; }"
            ),
            1
        );
    }

    #[test]
    fn rejects_identity_escape_and_observable_property_paths() {
        for source in [
            "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var record = { x: 1 }; sum += record.x; } return sum + record.x; }",
            "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var record = { x: 1 }; sum += record.x; } return function () { return record; }; }",
            "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var record = { get x() { return 1; } }; sum += record.x; } return sum; }",
            "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var record = { [key]: 1 }; sum += record.x; } return sum; }",
            "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var values = [1, , 3]; sum += values[1]; } return sum; }",
            "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var values = [1]; sum += values[2]; } return sum; }",
            "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var record = { x: 1 }; sum += record.x; } eval('record'); return sum; }",
        ] {
            assert_eq!(plan_count(source), 0, "unexpected plan for {source}");
        }
    }

    #[test]
    fn scalar_replacement_preserves_results_and_duplicate_keys() {
        let result = eval(
            "function objectSum(n) { var sum = 0; for (var i = 0; i < n; i++) { var record = { x: 1, x: 4, y: 5 }; sum += record.x + record.y; } return sum; } function arraySum(n) { var sum = 0; for (var i = 0; i < n; i++) { var values = [4, 5, 6, 7, 8]; sum += values[4] + values[0]; } return sum; } objectSum(0) + objectSum(1) + objectSum(1000) + arraySum(1000);",
        )
        .expect("scalar-replaced loops should evaluate");
        assert_eq!(result, Value::Number(21_009.0));
    }

    #[test]
    fn mapped_arguments_parameter_slots_force_runtime_fallback() {
        let source = "function run(record, i, n) { var sum = 0; for (; i < n; i++) { var record = { x: 1, y: 2 }; sum += record.x + record.y; } return [arguments[0], arguments[1], sum]; } var original = { x: 9 }; var result = run(original, 0, 3); result[0] !== original && result[0].x === 1 && result[0].y === 2 && result[1] === 3 && result[2] === 9;";
        assert_eq!(
            plan_count(source),
            1,
            "compile-time shape should stay eligible"
        );

        let result = eval(source).expect("mapped arguments should retain canonical updates");
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn rejected_literals_keep_accessors_holes_descriptors_and_identity_observable() {
        let result = eval(
            "var getterCalls = 0; function getterLoop(n) { var sum = 0; for (var i = 0; i < n; i++) { var record = { get x() { getterCalls++; return 2; } }; sum += record.x; } return sum; } Object.defineProperty(Array.prototype, '1', { value: 7, configurable: true }); function holeLoop(n) { var sum = 0; for (var i = 0; i < n; i++) { var values = [1, , 3]; sum += values[1]; } return sum; } function identityLoop() { var first; var last; for (var i = 0; i < 2; i++) { var record = { x: 1 }; if (i === 0) first = record; last = record; } return first === last; } getterLoop(4) + getterCalls + holeLoop(3) + (identityLoop() ? 1000 : 0);",
        )
        .expect("observable literals should use canonical execution");
        assert_eq!(result, Value::Number(33.0));
    }
}
