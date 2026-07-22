use qjs_ast::BinaryOp;

use crate::{Value, to_int32_number};

use super::{
    Bytecode, LoopWriteTarget, NumericLoopPlan, Op, RealmGlobalLoopWrite, Vm, set_local_number,
};

/// One pure local phi selected from a bitwise counter predicate at the start
/// of a numeric loop body.
#[derive(Clone, Debug)]
pub(super) struct NumericLoopSelector {
    target_slot: usize,
    consequent_slot: usize,
    alternate_slot: usize,
    mask: f64,
    expected: f64,
}

/// Runtime values for both selector arms. Both values are captured and all
/// dependent terms are prepared before the first accelerated iteration.
#[derive(Clone, Debug)]
pub(super) struct PreparedNumericLoopSelector {
    target_slot: usize,
    consequent: Value,
    alternate: Value,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct NumericLoopSlotOverride<'a> {
    slot: usize,
    value: &'a Value,
}

impl NumericLoopSelector {
    pub(super) fn compile(
        bytecode: &Bytecode,
        cursor: usize,
        counter_slot: usize,
        compact_completion: bool,
        expected_block_result_slot: Option<usize>,
        loop_result_slot: Option<usize>,
    ) -> Option<(Self, usize, usize)> {
        let code = &bytecode.code;
        let (
            Op::LoadLocal(condition_counter_slot),
            Op::LoadConst(mask_index),
            Op::Binary(BinaryOp::BitwiseAnd),
            Op::LoadConst(expected_index),
            Op::Binary(BinaryOp::StrictEq),
            Op::JumpIfFalse(alternate_start),
            Op::Pop,
            Op::LoadLocal(consequent_slot),
            Op::Jump(join),
            Op::Pop,
            Op::LoadLocal(alternate_slot),
            Op::Dup,
            Op::AssignLocal(target_slot),
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
        )
        else {
            return None;
        };
        if *condition_counter_slot != counter_slot
            || *alternate_start != cursor + 9
            || *join != cursor + 11
            || target_slot == consequent_slot
            || target_slot == alternate_slot
            || !bytecode.local_is_mutable(*target_slot)
        {
            return None;
        }

        let (block_result_slot, next_cursor) = if compact_completion {
            let Op::StoreLocal(block_result_slot) = code.get(cursor + 13)? else {
                return None;
            };
            if expected_block_result_slot.is_some_and(|slot| slot != *block_result_slot)
                || loop_result_slot.is_some()
            {
                return None;
            }
            (*block_result_slot, cursor + 14)
        } else {
            let (
                Op::Dup,
                Op::StoreLocal(block_result_slot),
                Op::StoreLocal(selection_loop_result_slot),
            ) = (
                code.get(cursor + 13)?,
                code.get(cursor + 14)?,
                code.get(cursor + 15)?,
            )
            else {
                return None;
            };
            if expected_block_result_slot != Some(*block_result_slot)
                || loop_result_slot != Some(*selection_loop_result_slot)
            {
                return None;
            }
            (*block_result_slot, cursor + 16)
        };

        Some((
            Self {
                target_slot: *target_slot,
                consequent_slot: *consequent_slot,
                alternate_slot: *alternate_slot,
                mask: number_constant(bytecode, *mask_index)?,
                expected: number_constant(bytecode, *expected_index)?,
            },
            next_cursor,
            block_result_slot,
        ))
    }

    /// The selector target is independently materialized after the scalar
    /// loop. Neither source may alias scalarized state; equal source slots are
    /// safe because admitted terms are read-only.
    pub(super) fn slots_are_disjoint(&self, scalar_slots: &[usize]) -> bool {
        self.target_slot != self.consequent_slot
            && self.target_slot != self.alternate_slot
            && !scalar_slots.contains(&self.target_slot)
            && !scalar_slots.contains(&self.consequent_slot)
            && !scalar_slots.contains(&self.alternate_slot)
    }

    pub(super) fn prepare(
        &self,
        vm: &Vm<'_>,
        scalar_slots: &[usize],
    ) -> Option<PreparedNumericLoopSelector> {
        let selector_slots = [self.target_slot, self.consequent_slot, self.alternate_slot];
        if selector_slots
            .into_iter()
            .any(|slot| !vm.slot_is_authoritative(slot))
        {
            return None;
        }

        // Distinct bytecode slots should not route to one shared binding cell.
        // This is normally implied by authoritative local storage, but keeping
        // the pointer proof here makes the scalar/source/target contract
        // explicit if frame routing evolves.
        let mut slots = scalar_slots.to_vec();
        slots.extend(selector_slots);
        for (index, &slot) in slots.iter().enumerate() {
            let Some(Some(cell)) = vm.local_upvalues.get(slot) else {
                continue;
            };
            for &previous_slot in &slots[..index] {
                if previous_slot == slot {
                    continue;
                }
                if vm
                    .local_upvalues
                    .get(previous_slot)
                    .and_then(Option::as_ref)
                    .is_some_and(|previous| previous.ptr_eq(cell))
                {
                    return None;
                }
            }
        }

        Some(PreparedNumericLoopSelector {
            target_slot: self.target_slot,
            consequent: vm.local_slot_value(self.consequent_slot)?,
            alternate: vm.local_slot_value(self.alternate_slot)?,
        })
    }

    pub(super) fn slots(&self) -> [usize; 3] {
        [self.target_slot, self.consequent_slot, self.alternate_slot]
    }

    #[inline(always)]
    pub(super) fn matches(&self, counter: f64) -> bool {
        f64::from(to_int32_number(counter) & to_int32_number(self.mask)) == self.expected
    }
}

impl NumericLoopPlan {
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_run_selected(
        &self,
        vm: &mut Vm<'_>,
        selector: &NumericLoopSelector,
        prepared_selector: PreparedNumericLoopSelector,
        write_targets: Vec<LoopWriteTarget>,
        forbidden_cells: &[crate::function::Upvalue],
        forbidden_realm_writes: &[RealmGlobalLoopWrite],
        mut counter: f64,
        limit: f64,
        mut accumulator: f64,
    ) -> bool {
        let consequent_override = prepared_selector.consequent_override();
        let alternate_override = prepared_selector.alternate_override();
        let mut consequent_terms = Vec::with_capacity(self.terms.len());
        let mut alternate_terms = Vec::with_capacity(self.terms.len());
        for term in &self.terms {
            let Some(consequent) = term.prepare_with_slot_override(
                vm,
                forbidden_cells,
                forbidden_realm_writes,
                Some(&consequent_override),
            ) else {
                return false;
            };
            let Some(alternate) = term.prepare_with_slot_override(
                vm,
                forbidden_cells,
                forbidden_realm_writes,
                Some(&alternate_override),
            ) else {
                return false;
            };
            if !consequent.is_read_only() || !alternate.is_read_only() {
                return false;
            }
            consequent_terms.push(consequent);
            alternate_terms.push(alternate);
        }

        #[cfg(test)]
        super::NUMERIC_LOOP_ENTRY_HITS.with(|hits| hits.set(hits.get() + 1));

        let mut last_selection = None;
        while counter < limit {
            let matches = selector.matches(counter);
            let terms = if matches {
                &mut consequent_terms
            } else {
                &mut alternate_terms
            };
            for term in terms {
                accumulator += term.eval(counter);
            }
            last_selection = Some(matches);
            counter += 1.0;
        }

        let mut values = vec![counter, accumulator, accumulator];
        if self.loop_result_slot.is_some() {
            values.push(accumulator);
        }
        let realm_writes = write_targets
            .iter()
            .zip(values.iter().copied())
            .filter_map(|(target, value)| match target {
                LoopWriteTarget::Local { .. } => None,
                LoopWriteTarget::RealmGlobal(target) => Some((target.clone(), value)),
            })
            .collect::<Vec<_>>();
        if !vm.commit_realm_global_loop_writes(&realm_writes) {
            return false;
        }
        #[cfg(test)]
        if !realm_writes.is_empty() {
            super::REALM_GLOBAL_LOOP_BATCH_COMMITS.with(|commits| commits.set(commits.get() + 1));
        }

        for (target, value) in write_targets.into_iter().zip(values) {
            if let LoopWriteTarget::Local { slot } = target {
                set_local_number(vm, slot, value);
            }
        }
        if let Some(matches) = last_selection {
            vm.locals[prepared_selector.target_slot()] =
                Some(prepared_selector.selected_value(matches));
        }
        vm.ip = self.exit + 1;
        true
    }
}

impl PreparedNumericLoopSelector {
    pub(super) fn consequent_override(&self) -> NumericLoopSlotOverride<'_> {
        NumericLoopSlotOverride {
            slot: self.target_slot,
            value: &self.consequent,
        }
    }

    pub(super) fn alternate_override(&self) -> NumericLoopSlotOverride<'_> {
        NumericLoopSlotOverride {
            slot: self.target_slot,
            value: &self.alternate,
        }
    }

    pub(super) fn selected_value(&self, matches: bool) -> Value {
        if matches {
            self.consequent.clone()
        } else {
            self.alternate.clone()
        }
    }

    pub(super) fn target_slot(&self) -> usize {
        self.target_slot
    }
}

impl NumericLoopSlotOverride<'_> {
    pub(super) fn value_for(&self, slot: usize) -> Option<Value> {
        (self.slot == slot).then(|| self.value.clone())
    }
}

fn number_constant(bytecode: &Bytecode, index: usize) -> Option<f64> {
    match bytecode.constants.get(index)? {
        Value::Number(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::bytecode::{
        compiler,
        vm_numeric_loop::{NUMERIC_LOOP_ENTRY_HITS, NumericLoopPlan, NumericLoopTerm},
    };
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

    fn selector_plan_count(source: &str) -> usize {
        NumericLoopPlan::compile_all(&nested_function(source))
            .iter()
            .filter(|plan| plan.selector.is_some())
            .count()
    }

    fn reset_hits() {
        NUMERIC_LOOP_ENTRY_HITS.with(|hits| hits.set(0));
    }

    fn hits() -> usize {
        NUMERIC_LOOP_ENTRY_HITS.with(Cell::get)
    }

    fn assert_traced(source: &str, expression: &str, expected: Value) {
        assert_eq!(selector_plan_count(source), 1, "{source}");
        reset_hits();
        assert_eq!(eval(&format!("{source} {expression}")), Ok(expected));
        assert_eq!(hits(), 1, "{source}");
    }

    fn assert_runtime_fallback(source: &str, expression: &str, expected: Value) {
        assert_eq!(selector_plan_count(source), 1, "{source}");
        reset_hits();
        assert_eq!(eval(&format!("{source} {expression}")), Ok(expected));
        assert_eq!(hits(), 0, "{source}");
    }

    #[test]
    fn selected_method_loop_compiles_and_preserves_zero_one_and_many_iterations() {
        let source = "function run(n) { var first = { f: function (value, offset) { return value + offset; } }; var second = { f: function (value, offset) { return value - offset; } }; var receiver; var sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 3) === 2 ? first : second; sum += receiver.f(i, 2); } return sum + ':' + (receiver === first ? 'first' : receiver === second ? 'second' : 'none'); }";
        assert_eq!(selector_plan_count(source), 1);
        for (count, expected, expected_hits) in [
            (0, "0:none", 0),
            (1, "-2:second", 1),
            (2, "-3:second", 1),
            (3, "1:first", 1),
            (6, "7:second", 1),
        ] {
            reset_hits();
            assert_eq!(
                eval(&format!("{source} run({count});")),
                Ok(Value::String(expected.to_owned().into())),
                "count={count}"
            );
            assert_eq!(hits(), expected_hits, "count={count}");
        }
    }

    #[test]
    fn selected_method_loop_supports_other_predicates_and_equal_sources() {
        let different_predicate = "function run(n) { var first = { f: function (value) { return value + 3; } }; var second = { f: function (value) { return value + 20; } }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 7) === 4 ? first : second; sum += receiver.f(i); } return sum; }";
        assert_traced(different_predicate, "run(9);", Value::Number(199.0));

        let equal_sources = "function run(n) { var source = { f: function (value) { return value + 1; } }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? source : source; sum += receiver.f(i); } return sum; }";
        assert_traced(equal_sources, "run(6);", Value::Number(21.0));

        let read_only_captures = "function run(n) { var left = 3, right = 20; var first = { f: function (value) { return value + left; } }; var second = { f: function (value) { return value + right; } }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }";
        assert_traced(read_only_captures, "run(6);", Value::Number(84.0));
    }

    #[test]
    fn selector_override_is_shared_by_local_property_and_dense_reads() {
        for source in [
            "function run(n) { var first = 2, second = 5, selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected; } return sum; }",
            "function run(n) { var first = { value: 2 }, second = { value: 5 }, selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected.value; } return sum; }",
            "function run(n) { var first = [2], second = [5], selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected[0]; } return sum; }",
        ] {
            assert_traced(source, "run(6);", Value::Number(21.0));
        }
    }

    #[test]
    fn selector_override_covers_computed_call_slice_and_array_builtin_terms() {
        let computed = "function run(n) { var first = { a: 2 }, second = { a: 5 }, key = 'a', selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected[key]; } return sum; }";
        let computed_plan = NumericLoopPlan::compile_all(&nested_function(computed))
            .pop()
            .expect("computed selector loop should compile");
        assert!(matches!(
            computed_plan.terms.as_slice(),
            [NumericLoopTerm::ComputedProperty { .. }]
        ));
        assert_traced(computed, "run(6);", Value::Number(21.0));

        let local_call = "function run(n) { var first = function (value) { return value + 1; }, second = function (value) { return value + 10; }, selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected(i); } return sum; }";
        let local_call_plan = NumericLoopPlan::compile_all(&nested_function(local_call))
            .pop()
            .expect("local-call selector loop should compile");
        assert!(matches!(
            local_call_plan.terms.as_slice(),
            [NumericLoopTerm::LocalCall { .. }]
        ));
        assert_traced(local_call, "run(6);", Value::Number(48.0));

        let string_slice = "function run(n) { var first = 'abcd', second = 'x', selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected.slice(1, 3).length; } return sum; }";
        let string_plan = NumericLoopPlan::compile_all(&nested_function(string_slice))
            .pop()
            .expect("string-slice selector loop should compile");
        assert!(matches!(
            string_plan.terms.as_slice(),
            [NumericLoopTerm::StringSliceLength { .. }]
        ));
        assert_traced(string_slice, "run(6);", Value::Number(6.0));

        let array_index_of = "function run(n) { var first = [1, 3], second = [3, 1], selected, sum = 0; for (var i = 0; i < n; i++) { selected = (i & 1) === 0 ? first : second; sum += selected.indexOf(3); } return sum; }";
        let array_plan = NumericLoopPlan::compile_all(&nested_function(array_index_of))
            .pop()
            .expect("array-indexOf selector loop should compile");
        assert!(matches!(
            array_plan.terms.as_slice(),
            [NumericLoopTerm::MethodCall { .. }]
        ));
        assert_traced(array_index_of, "run(6);", Value::Number(3.0));
    }

    #[test]
    fn accessor_inherited_proxy_and_non_callable_methods_fall_back() {
        let accessor = "function run(n) { var reads = 0, first = {}, second = {}; function f(value) { return value + 1; } Object.defineProperty(first, 'f', { get: function () { reads += 1; return f; } }); Object.defineProperty(second, 'f', { get: function () { reads += 1; return f; } }); var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum + ':' + reads; }";
        assert_runtime_fallback(accessor, "run(4);", Value::String("10:4".to_owned().into()));

        let inherited = "function run(n) { var proto = { f: function (value) { return value + 1; } }; var first = Object.create(proto), second = Object.create(proto), receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }";
        assert_runtime_fallback(inherited, "run(4);", Value::Number(10.0));

        let proxied = "function run(n) { var first = new Proxy({ f: function (value) { return value + 1; } }, {}), second = new Proxy({ f: function (value) { return value + 1; } }, {}), receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }";
        assert_runtime_fallback(proxied, "run(4);", Value::Number(10.0));

        let non_callable = "function run(n) { var first = { f: function (value) { return value + 1; } }, second = { f: 1 }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }";
        assert_runtime_fallback(
            non_callable,
            "try { run(4); 'missed'; } catch (error) { 'caught'; }",
            Value::String("caught".to_owned().into()),
        );
    }

    #[test]
    fn non_numeric_methods_and_captured_writes_fall_back() {
        let non_numeric = "function run(n) { var first = { f: function () { return 'a'; } }, second = { f: function () { return 'b'; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(); } return sum; }";
        assert_runtime_fallback(
            non_numeric,
            "run(4);",
            Value::String("0abab".to_owned().into()),
        );

        let captured_write = "function run(n) { var writes = 0; var first = { f: function () { writes += 1; return writes; } }, second = { f: function () { writes += 10; return writes; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(); } return sum + ':' + writes; }";
        assert_runtime_fallback(
            captured_write,
            "run(4);",
            Value::String("46:22".to_owned().into()),
        );
    }

    #[test]
    fn calls_observing_scalarized_state_or_mutating_an_arm_fall_back() {
        for (source, expected) in [
            (
                "function run(n) { var first = { f: function (value) { return value + i; } }, second = { f: function (value) { return value + i; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }",
                Value::Number(12.0),
            ),
            (
                "function run(n) { var first = { f: function (value) { return value + sum; } }, second = { f: function (value) { return value + sum; } }, receiver, sum = 1; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }",
                Value::Number(27.0),
            ),
            (
                "function run(n) { var first = {}, second = {}; first.f = function (value) { second.f = function (next) { return next + 100; }; return value + 1; }; second.f = function (value) { return value + 2; }; var receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }",
                Value::Number(208.0),
            ),
        ] {
            assert_runtime_fallback(source, "run(4);", expected);
        }

        let receiver_capture = "function run(n) { var first, second, receiver, sum = 0; first = { f: function (value) { return value + (receiver === first ? 1 : 10); } }; second = { f: function (value) { return value + (receiver === first ? 1 : 10); } }; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }";
        assert_runtime_fallback(receiver_capture, "run(4);", Value::Number(28.0));
    }

    #[test]
    fn selector_slot_aliases_and_per_iteration_scopes_are_rejected() {
        for source in [
            "function run(n) { var first = { f: function (value) { return value + 1; } }, second = { f: function (value) { return value + 1; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? sum : second; sum += receiver.f(i); } return sum; }",
            "function run(n) { var first = { f: function (value) { return value + 1; } }, second = { f: function (value) { return value + 1; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? n : second; sum += receiver.f(i); } return sum; }",
            "function run(n) { var first = { f: function (value) { return value + 1; } }, second = { f: function (value) { return value + 1; } }, receiver, sum = 0, closures = []; for (let i = 0; i < n; i++) { let snapshot = i; closures.push(function () { return snapshot; }); receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum + closures[0](); }",
        ] {
            assert_eq!(selector_plan_count(source), 0, "{source}");
        }
    }

    #[test]
    fn direct_eval_and_with_scopes_disable_selector_execution() {
        for source in [
            "function run(n) { eval(''); var first = { f: function (value) { return value + 1; } }, second = { f: function (value) { return value + 1; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }",
            "function run(n) { with ({}) {} var first = { f: function (value) { return value + 1; } }, second = { f: function (value) { return value + 1; } }, receiver, sum = 0; for (var i = 0; i < n; i++) { receiver = (i & 1) === 0 ? first : second; sum += receiver.f(i); } return sum; }",
        ] {
            assert_runtime_fallback(source, "run(4);", Value::Number(10.0));
        }
    }
}
