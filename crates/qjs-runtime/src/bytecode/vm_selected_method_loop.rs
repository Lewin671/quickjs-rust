use std::rc::Rc;

use qjs_ast::{BinaryOp, UpdateOp};

use crate::{Value, to_int32_number, value::OwnDataPropertyRead};

use super::{
    ir::{Bytecode, Op},
    vm::Vm,
    vm_numeric_leaf::NumericLoopCall,
};

#[derive(Clone, Copy, Debug)]
enum SelectedLoopArgument {
    Counter,
    Constant(f64),
}

#[derive(Clone, Copy, Debug)]
enum SelectedLoopArguments {
    None,
    One(SelectedLoopArgument),
    Two(SelectedLoopArgument, SelectedLoopArgument),
}

impl SelectedLoopArguments {
    fn len(self) -> usize {
        match self {
            Self::None => 0,
            Self::One(_) => 1,
            Self::Two(_, _) => 2,
        }
    }

    #[inline(always)]
    fn values(self, counter: f64) -> [f64; 2] {
        let value = |argument| match argument {
            SelectedLoopArgument::Counter => counter,
            SelectedLoopArgument::Constant(value) => value,
        };
        match self {
            Self::None => [0.0, 0.0],
            Self::One(first) => [value(first), 0.0],
            Self::Two(first, second) => [value(first), value(second)],
        }
    }
}

/// Prevalidated counted loop with one pure local phi and one numeric method
/// call. This plan is deliberately separate from `NumericLoopPlan`: its two
/// branch-call states never enlarge the common prepared-term enum or alter the
/// existing numeric-loop executor's inner-loop code generation.
#[derive(Clone, Debug)]
pub(super) struct SelectedMethodLoopPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    counter_slot: usize,
    limit_slot: usize,
    accumulator_slot: usize,
    block_result_slot: usize,
    loop_result_slot: usize,
    receiver_slot: usize,
    when_equal_slot: usize,
    when_not_equal_slot: usize,
    mask: f64,
    expected: f64,
    key: Rc<str>,
    arguments: SelectedLoopArguments,
}

impl SelectedMethodLoopPlan {
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
        if !matches!(code.get(*exit), Some(Op::Pop)) || backedge < header + 32 {
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

        let cursor = header + 7;
        let (
            Op::LoadLocal(condition_counter_slot),
            Op::LoadConst(mask_index),
            Op::Binary(BinaryOp::BitwiseAnd),
            Op::LoadConst(expected_index),
            Op::Binary(BinaryOp::StrictEq),
            Op::JumpIfFalse(else_start),
            Op::Pop,
            Op::LoadLocal(when_equal_slot),
            Op::Jump(join),
            Op::Pop,
            Op::LoadLocal(when_not_equal_slot),
            Op::Dup,
            Op::AssignLocal(receiver_slot),
            Op::Dup,
            Op::StoreLocal(selection_block_result_slot),
            Op::StoreLocal(selection_loop_result_slot),
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
        )
        else {
            return None;
        };
        if condition_counter_slot != counter_slot
            || *else_start != cursor + 9
            || *join != cursor + 11
            || receiver_slot == when_equal_slot
            || receiver_slot == when_not_equal_slot
            || selection_block_result_slot != block_result_slot
            || selection_loop_result_slot != loop_result_slot
        {
            return None;
        }

        let term = cursor + 16;
        let (
            Op::LoadLocal(accumulator_slot),
            Op::LoadLocal(method_receiver_slot),
            Op::Dup,
            Op::GetPropNamed { key, .. },
        ) = (
            code.get(term)?,
            code.get(term + 1)?,
            code.get(term + 2)?,
            code.get(term + 3)?,
        )
        else {
            return None;
        };
        if method_receiver_slot != receiver_slot {
            return None;
        }
        let (arguments, suffix) = compile_arguments(bytecode, term + 4, *counter_slot, true)?;
        let (
            Op::Binary(BinaryOp::Add),
            Op::Dup,
            Op::AssignLocal(assigned_accumulator_slot),
            Op::Dup,
            Op::StoreLocal(term_block_result_slot),
            Op::StoreLocal(term_loop_result_slot),
        ) = (
            code.get(suffix)?,
            code.get(suffix + 1)?,
            code.get(suffix + 2)?,
            code.get(suffix + 3)?,
            code.get(suffix + 4)?,
            code.get(suffix + 5)?,
        )
        else {
            return None;
        };
        // The scalar executor evolves these slots independently. If two of
        // them name one JS binding, separating that binding can change both
        // the loop test and the value observed by the body. The two branch
        // sources may alias each other because admitted calls are read-only,
        // but neither source may alias independently scalarized state.
        let scalarized_slots = [
            *counter_slot,
            *limit_slot,
            *accumulator_slot,
            *block_result_slot,
            *loop_result_slot,
            *receiver_slot,
        ];
        let scalarized_slots_are_distinct = scalarized_slots
            .iter()
            .enumerate()
            .all(|(index, slot)| !scalarized_slots[..index].contains(slot));
        let sources_are_disjoint = [*when_equal_slot, *when_not_equal_slot]
            .into_iter()
            .all(|slot| !scalarized_slots.contains(&slot));
        if suffix + 6 != tail
            || assigned_accumulator_slot != accumulator_slot
            || term_block_result_slot != block_result_slot
            || term_loop_result_slot != loop_result_slot
            || !scalarized_slots_are_distinct
            || !sources_are_disjoint
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
            when_equal_slot: *when_equal_slot,
            when_not_equal_slot: *when_not_equal_slot,
            mask: number_constant(bytecode, *mask_index)?,
            expected: number_constant(bytecode, *expected_index)?,
            key: key.clone(),
            arguments,
        })
    }

    #[inline(never)]
    fn try_run(&self, vm: &mut Vm<'_>) -> bool {
        if vm.direct_eval_with_stack
            || [
                self.counter_slot,
                self.accumulator_slot,
                self.block_result_slot,
                self.loop_result_slot,
                self.receiver_slot,
                self.when_equal_slot,
                self.when_not_equal_slot,
            ]
            .into_iter()
            .any(|slot| !vm.slot_is_authoritative(slot))
        {
            return false;
        }
        let Some(mut counter) = local_number(vm, self.counter_slot) else {
            return false;
        };
        let Some(limit) = local_number_read(vm, self.limit_slot) else {
            return false;
        };
        let Some(mut accumulator) = local_number(vm, self.accumulator_slot) else {
            return false;
        };
        let Some(when_equal_value) = vm.local_slot_value(self.when_equal_slot) else {
            return false;
        };
        let Some(when_not_equal_value) = vm.local_slot_value(self.when_not_equal_slot) else {
            return false;
        };
        let Some(mut when_equal) =
            prepare_call(&when_equal_value, &self.key, self.arguments.len(), vm)
        else {
            return false;
        };
        let Some(mut when_not_equal) =
            prepare_call(&when_not_equal_value, &self.key, self.arguments.len(), vm)
        else {
            return false;
        };
        if !when_equal.is_read_only() || !when_not_equal.is_read_only() {
            return false;
        }

        let mut last_selected_counter = None;
        while counter < limit {
            let [first, second] = self.arguments.values(counter);
            accumulator += if self.matches(counter) {
                when_equal.eval(first, second)
            } else {
                when_not_equal.eval(first, second)
            };
            last_selected_counter = Some(counter);
            counter += 1.0;
        }
        when_equal.commit();
        when_not_equal.commit();

        if let Some(selected_counter) = last_selected_counter {
            vm.locals[self.receiver_slot] = Some(if self.matches(selected_counter) {
                when_equal_value
            } else {
                when_not_equal_value
            });
        }
        set_local_number(vm, self.counter_slot, counter);
        set_local_number(vm, self.accumulator_slot, accumulator);
        set_local_number(vm, self.block_result_slot, accumulator);
        set_local_number(vm, self.loop_result_slot, accumulator);
        vm.ip = self.exit + 1;
        true
    }

    #[inline(always)]
    fn matches(&self, counter: f64) -> bool {
        f64::from(to_int32_number(counter) & to_int32_number(self.mask)) == self.expected
    }
}

pub(super) fn try_run_selected_method_loop(
    vm: &mut Vm<'_>,
    header: usize,
    backedge: usize,
) -> bool {
    let plan = vm
        .bytecode
        .selected_method_loop_plans
        .get_or_init(|| SelectedMethodLoopPlan::compile_all(vm.bytecode))
        .iter()
        .find(|plan| plan.header == header && plan.backedge == backedge)
        .cloned();
    plan.is_some_and(|plan| plan.try_run(vm))
}

fn compile_arguments(
    bytecode: &Bytecode,
    mut cursor: usize,
    counter_slot: usize,
    resolved: bool,
) -> Option<(SelectedLoopArguments, usize)> {
    let mut arguments = SelectedLoopArguments::None;
    loop {
        let call_count = match bytecode.code.get(cursor)? {
            Op::Call(count) if !resolved => Some(*count),
            Op::CallResolved(count) if resolved => Some(*count),
            _ => None,
        };
        if let Some(call_count) = call_count {
            return (call_count == arguments.len()).then_some((arguments, cursor + 1));
        }
        let (argument, next_cursor) = match bytecode.code.get(cursor)? {
            Op::LoadLocal(slot) if *slot == counter_slot => {
                (SelectedLoopArgument::Counter, cursor + 1)
            }
            Op::LoadConst(index) => match bytecode.constants.get(*index)? {
                Value::Number(value)
                    if matches!(
                        bytecode.code.get(cursor + 1),
                        Some(Op::Unary(qjs_ast::UnaryOp::Minus))
                    ) =>
                {
                    (SelectedLoopArgument::Constant(-*value), cursor + 2)
                }
                Value::Number(value) => (SelectedLoopArgument::Constant(*value), cursor + 1),
                _ => return None,
            },
            _ => return None,
        };
        arguments = match arguments {
            SelectedLoopArguments::None => SelectedLoopArguments::One(argument),
            SelectedLoopArguments::One(first) => SelectedLoopArguments::Two(first, argument),
            SelectedLoopArguments::Two(_, _) => return None,
        };
        cursor = next_cursor;
    }
}

fn prepare_call(
    receiver: &Value,
    key: &str,
    argument_count: usize,
    vm: &Vm<'_>,
) -> Option<NumericLoopCall> {
    let Value::Object(object) = receiver else {
        return None;
    };
    let OwnDataPropertyRead::Data(Value::Function(function)) = object.own_data_property_read(key)
    else {
        return None;
    };
    NumericLoopCall::prepare(&function, argument_count, &vm.local_upvalues)
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
    fn recognizes_and_runs_bitwise_selected_numeric_method_calls() {
        let source = "function run(n) { var first = { add: function (value) { return value + 1; } }; var second = { add: function (value) { return value + 10; } }; var receiver; var sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 3) === 2 ? first : second; sum += receiver.add(i); } return sum + ':' + (receiver === first ? 'first' : receiver === second ? 'second' : 'none'); }";
        let bytecode = nested_function(source);
        assert_eq!(SelectedMethodLoopPlan::compile_all(&bytecode).len(), 1);

        for (count, expected) in [
            (0, "0:none"),
            (1, "10:second"),
            (2, "21:second"),
            (3, "24:first"),
            (6, "66:second"),
            (1_000, "507250:second"),
        ] {
            assert_eq!(
                eval(&format!("{source} run({count});")),
                Ok(Value::String(expected.to_owned().into())),
                "count={count}"
            );
        }
    }

    #[test]
    fn rejects_selected_receiver_property_reads_before_snapshotting() {
        let source = "function run(n) { var first = { value: 1 }; var second = { value: 2 }; var receiver; var sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.value; } return sum; }";
        assert!(SelectedMethodLoopPlan::compile_all(&nested_function(source)).is_empty());
        assert_eq!(
            eval(&format!("{source} run(1000);")),
            Ok(Value::Number(1500.0))
        );
    }

    #[test]
    fn accessors_and_captured_writes_keep_observable_order() {
        assert_eq!(
            eval(
                "function run(n) { var reads = 0; var first = {}, second = {}; function add(value) { return value + 1; } Object.defineProperty(first, 'add', { get: function () { reads += 1; return add; } }); Object.defineProperty(second, 'add', { get: function () { reads += 1; return add; } }); var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.add(i); } return sum + ':' + reads; } run(8);"
            ),
            Ok(Value::String("36:8".to_owned().into()))
        );
        assert_eq!(
            eval(
                "function run(n) { var writes = 0; var first = { add: function (value) { writes += 1; return value + 1; } }; var second = { add: function (value) { writes += 10; return value + 2; } }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.add(i); } return sum + ':' + writes; } run(8);"
            ),
            Ok(Value::String("40:44".to_owned().into()))
        );
    }

    #[test]
    fn evolving_captured_locals_force_fallback() {
        assert_eq!(
            eval(
                "function run(n) { var first = { add: function (value) { return value + i; } }; var second = { add: function (value) { return value + i; } }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.add(i); } return sum; } run(6);"
            ),
            Ok(Value::Number(30.0))
        );
        assert_eq!(
            eval(
                "function run(n) { var first = { add: function (value) { return value + sum; } }; var second = { add: function (value) { return value + sum; } }; var receiver, sum = 1; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.add(i); } return sum; } run(4);"
            ),
            Ok(Value::Number(27.0))
        );
    }

    #[test]
    fn counter_accumulator_alias_forces_fallback() {
        let source = "function run(n) { var first = { add: function (value) { return value + 1; } }; var second = { add: function (value) { return value + 1; } }; var receiver; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; i += receiver.add(i); } return i; }";
        assert!(SelectedMethodLoopPlan::compile_all(&nested_function(source)).is_empty());
        assert_eq!(eval(&format!("{source} run(10);")), Ok(Value::Number(14.0)));
    }

    #[test]
    fn limit_accumulator_alias_forces_fallback() {
        let source = "function run() { var first = { add: function (value) { return value - 2; } }; var second = { add: function (value) { return value - 2; } }; var receiver; var limit = 5; var i = 0; for (; i < limit; i++) { receiver = (i & 1) === 0 ? first : second; limit += receiver.add(i); } return limit + ':' + i; }";
        assert!(SelectedMethodLoopPlan::compile_all(&nested_function(source)).is_empty());
        assert_eq!(
            eval(&format!("{source} run();")),
            Ok(Value::String("2:2".to_owned().into()))
        );
    }

    #[test]
    fn rejects_every_accumulator_state_alias() {
        let source = "function run(n) { var first = { add: function (value) { return value + 1; } }; var second = { add: function (value) { return value + 10; } }; var receiver; var sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 3) === 2 ? first : second; sum += receiver.add(i); } return sum; }";
        let bytecode = nested_function(source);
        let plan = SelectedMethodLoopPlan::compile_all(&bytecode)
            .pop()
            .expect("baseline loop should have one selected-method plan");
        for alias in [
            plan.counter_slot,
            plan.limit_slot,
            plan.block_result_slot,
            plan.loop_result_slot,
            plan.receiver_slot,
            plan.when_equal_slot,
            plan.when_not_equal_slot,
        ] {
            let mut aliased = bytecode.clone();
            for op in &mut aliased.code[plan.header..=plan.backedge] {
                match op {
                    Op::LoadLocal(slot) | Op::AssignLocal(slot)
                        if *slot == plan.accumulator_slot =>
                    {
                        *slot = alias;
                    }
                    _ => {}
                }
            }
            assert!(
                SelectedMethodLoopPlan::compile_all(&aliased).is_empty(),
                "accumulator alias with slot {alias} must be rejected"
            );
        }
    }

    #[test]
    fn allows_equal_read_only_branch_sources() {
        let source = "function run(n) { var source = { add: function (value) { return value + 1; } }; var receiver; var sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? source : source; sum += receiver.add(i); } return sum; }";
        assert_eq!(
            SelectedMethodLoopPlan::compile_all(&nested_function(source)).len(),
            1
        );
        assert_eq!(eval(&format!("{source} run(6);")), Ok(Value::Number(21.0)));
    }

    #[test]
    fn selected_calls_support_two_arguments_and_read_only_captures() {
        let two_arguments = "function run(n) { var first = { f: function (left, right) { return left + right; } }; var second = { f: function (left, right) { return left - right; } }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i, 2); } return sum; }";
        assert_eq!(
            SelectedMethodLoopPlan::compile_all(&nested_function(two_arguments)).len(),
            1
        );
        assert_eq!(
            eval(&format!("{two_arguments} run(6);")),
            Ok(Value::Number(15.0))
        );

        let captures = "function run(n) { var left = 1, right = 10; var first = { f: function (value) { return value + left; } }; var second = { f: function (value) { return value + right; } }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }";
        assert_eq!(
            SelectedMethodLoopPlan::compile_all(&nested_function(captures)).len(),
            1
        );
        assert_eq!(
            eval(&format!("{captures} run(4);")),
            Ok(Value::Number(28.0))
        );
    }

    #[test]
    fn changing_a_method_during_the_first_iteration_forces_fallback() {
        assert_eq!(
            eval(
                "function run(n) { var first = {}, second = {}; first.add = function (value) { second.add = function (next) { return next + 100; }; return value + 1; }; second.add = function (value) { return value + 2; }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.add(i); } return sum; } run(4);"
            ),
            Ok(Value::Number(208.0))
        );
    }
}
