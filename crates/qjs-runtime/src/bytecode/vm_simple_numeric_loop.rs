use qjs_ast::{BinaryOp, UpdateOp};

use crate::{Value, to_int32_number, to_uint32_number};

use super::{
    ir::{Bytecode, Op},
    vm::Vm,
};

#[derive(Clone, Copy, Debug)]
enum NumericSource {
    Constant(f64),
    Local(usize),
}

impl NumericSource {
    fn compile(bytecode: &Bytecode, op: &Op) -> Option<Self> {
        match op {
            Op::LoadConst(index) => match bytecode.constants.get(*index)? {
                Value::Number(value) => Some(Self::Constant(*value)),
                _ => None,
            },
            Op::LoadLocal(slot) => Some(Self::Local(*slot)),
            _ => None,
        }
    }

    fn prepare(
        self,
        vm: &Vm<'_>,
        counter_slot: usize,
        accumulator_slot: usize,
    ) -> Option<PreparedNumericSource> {
        match self {
            Self::Constant(value) => Some(PreparedNumericSource::Stable(value)),
            Self::Local(slot) if slot == counter_slot => Some(PreparedNumericSource::Counter),
            Self::Local(slot) if slot == accumulator_slot => {
                Some(PreparedNumericSource::Accumulator)
            }
            Self::Local(slot) => {
                if !slot_is_stable(vm, slot) {
                    return None;
                }
                Some(PreparedNumericSource::Stable(local_number(vm, slot)?))
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum PreparedNumericSource {
    Counter,
    Accumulator,
    Stable(f64),
}

impl PreparedNumericSource {
    fn value(self, counter: f64, accumulator: f64) -> f64 {
        match self {
            Self::Counter => counter,
            Self::Accumulator => accumulator,
            Self::Stable(value) => value,
        }
    }
}

#[derive(Clone, Debug)]
enum LoopStore {
    Local { slot: usize },
    Global { slot: usize, name: String },
}

impl LoopStore {
    fn compile(bytecode: &Bytecode, op: &Op, expected_slot: usize) -> Option<Self> {
        match op {
            Op::AssignLocal(slot) if *slot == expected_slot => Some(Self::Local { slot: *slot }),
            Op::StoreGlobalSloppy(name) | Op::StoreGlobalStrict(name)
                if bytecode.local_slot(name) == Some(expected_slot) =>
            {
                Some(Self::Global {
                    slot: expected_slot,
                    name: name.clone(),
                })
            }
            Op::StoreLocalOrGlobalSloppy { slot, name } if *slot == expected_slot => {
                Some(Self::Global {
                    slot: *slot,
                    name: name.clone(),
                })
            }
            _ => None,
        }
    }

    fn can_sink(&self, vm: &Vm<'_>) -> bool {
        match self {
            Self::Local { slot } => vm.slot_is_authoritative(*slot),
            Self::Global { slot, name } => {
                let Some(property) = vm.global_this_own_property(name) else {
                    return false;
                };
                if property.accessor || !property.writable {
                    return false;
                }
                if vm.slot_is_realm_binding(*slot) {
                    return true;
                }
                vm.bytecode.local_is_sloppy_global_fallback(*slot)
                    && vm
                        .local_slot_value(*slot)
                        .is_some_and(|value| value == property.value)
            }
        }
    }

    fn commit(&self, vm: &mut Vm<'_>, value: f64) {
        let value = Value::Number(value);
        match self {
            Self::Local { slot } => vm.locals[*slot] = Some(value),
            Self::Global { slot, name } => {
                vm.invalidate_array_prototype_cache(name);
                vm.locals[*slot] = Some(value.clone());
                if let Some(upvalue) = vm.local_upvalues.get(*slot).and_then(Option::as_ref) {
                    upvalue.set(value.clone());
                }
                vm.env.insert_realm(name.clone(), value.clone());
                if let Some(Value::Object(global_this)) = vm.env.global_this()
                    && global_this.has_own_property(name)
                {
                    global_this.set(name.clone(), value.clone());
                }
                vm.write_through_module_live_binding_slot(*slot, &value);
                vm.sync_marked_dynamic_global(name);
                if vm.bytecode.local_is_sloppy_global_fallback(*slot) {
                    vm.record_sloppy_global_name(name);
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
enum AccumulatorRead {
    Local { slot: usize },
    Global { slot: usize, name: String },
}

impl AccumulatorRead {
    fn compile(bytecode: &Bytecode, op: &Op) -> Option<Self> {
        match op {
            Op::LoadLocal(slot) => Some(Self::Local { slot: *slot }),
            Op::LoadGlobal(name) => Some(Self::Global {
                slot: bytecode.local_slot(name)?,
                name: name.clone(),
            }),
            _ => None,
        }
    }

    fn slot(&self) -> usize {
        match self {
            Self::Local { slot } | Self::Global { slot, .. } => *slot,
        }
    }

    fn number(&self, vm: &Vm<'_>) -> Option<f64> {
        let slot = self.slot();
        let value = local_number(vm, slot)?;
        match self {
            Self::Local { .. } => Some(value),
            Self::Global { name, .. } => match vm.global_this_own_property(name)? {
                property
                    if !property.accessor
                        && property.writable
                        && property.value == Value::Number(value) =>
                {
                    Some(value)
                }
                _ => None,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum NumericComparison {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

impl NumericComparison {
    fn compile(op: BinaryOp) -> Option<Self> {
        match op {
            BinaryOp::Lt => Some(Self::Less),
            BinaryOp::Le => Some(Self::LessEqual),
            BinaryOp::Gt => Some(Self::Greater),
            BinaryOp::Ge => Some(Self::GreaterEqual),
            _ => None,
        }
    }

    fn test(self, left: f64, right: f64) -> bool {
        match self {
            Self::Less => left < right,
            Self::LessEqual => left <= right,
            Self::Greater => left > right,
            Self::GreaterEqual => left >= right,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum NumericRecurrence {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Power,
    ShiftLeft,
    ShiftRight,
    UnsignedShiftRight,
    BitwiseAnd,
    BitwiseXor,
    BitwiseOr,
}

impl NumericRecurrence {
    fn compile(op: BinaryOp) -> Option<Self> {
        match op {
            BinaryOp::Add => Some(Self::Add),
            BinaryOp::Sub => Some(Self::Subtract),
            BinaryOp::Mul => Some(Self::Multiply),
            BinaryOp::Div => Some(Self::Divide),
            BinaryOp::Rem => Some(Self::Remainder),
            BinaryOp::Pow => Some(Self::Power),
            BinaryOp::Shl => Some(Self::ShiftLeft),
            BinaryOp::Shr => Some(Self::ShiftRight),
            BinaryOp::UShr => Some(Self::UnsignedShiftRight),
            BinaryOp::BitwiseAnd => Some(Self::BitwiseAnd),
            BinaryOp::BitwiseXor => Some(Self::BitwiseXor),
            BinaryOp::BitwiseOr => Some(Self::BitwiseOr),
            _ => None,
        }
    }

    fn apply(self, left: f64, right: f64) -> f64 {
        match self {
            Self::Add => left + right,
            Self::Subtract => left - right,
            Self::Multiply => left * right,
            Self::Divide => left / right,
            Self::Remainder => left % right,
            Self::Power => crate::operations::number_exponentiate(left, right),
            Self::ShiftLeft => f64::from(to_int32_number(left) << (to_uint32_number(right) & 0x1f)),
            Self::ShiftRight => {
                f64::from(to_int32_number(left) >> (to_uint32_number(right) & 0x1f))
            }
            Self::UnsignedShiftRight => {
                f64::from(to_uint32_number(left) >> (to_uint32_number(right) & 0x1f))
            }
            Self::BitwiseAnd => f64::from(to_int32_number(left) & to_int32_number(right)),
            Self::BitwiseXor => f64::from(to_int32_number(left) ^ to_int32_number(right)),
            Self::BitwiseOr => f64::from(to_int32_number(left) | to_int32_number(right)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum CounterUpdate {
    Increment,
    Decrement,
}

impl CounterUpdate {
    fn apply(self, value: f64) -> f64 {
        match self {
            Self::Increment => value + 1.0,
            Self::Decrement => value - 1.0,
        }
    }
}

/// Prevalidated straight-line numeric recurrence inside a counted loop.
///
/// The plan is independent of source names and iteration counts. It scalarizes
/// only number-typed locals or writable ordinary global data properties; any
/// dynamic binding, accessor, non-number, `with`, or direct-eval state falls
/// back to the ordinary bytecode loop before one iteration is consumed.
#[derive(Clone, Debug)]
pub(super) struct SimpleNumericLoopPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    counter_slot: usize,
    accumulator_slot: usize,
    accumulator_read: AccumulatorRead,
    loop_result_slot: usize,
    limit: NumericSource,
    operand: NumericSource,
    comparison: NumericComparison,
    recurrence: NumericRecurrence,
    update: CounterUpdate,
    accumulator_store: LoopStore,
    counter_store: LoopStore,
}

impl SimpleNumericLoopPlan {
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
        let Op::LoadLocal(counter_slot) = code.get(header)? else {
            return None;
        };
        let limit = NumericSource::compile(bytecode, code.get(header + 1)?)?;
        let Op::Binary(comparison) = code.get(header + 2)? else {
            return None;
        };
        let comparison = NumericComparison::compile(*comparison)?;
        let Op::JumpIfFalse(exit) = code.get(header + 3)? else {
            return None;
        };
        if !matches!(code.get(header + 4), Some(Op::Pop)) {
            return None;
        }

        let cursor = header + 5;
        let accumulator_read = AccumulatorRead::compile(bytecode, code.get(cursor)?)?;
        let accumulator_slot = accumulator_read.slot();
        if accumulator_slot == *counter_slot {
            return None;
        }
        let operand = NumericSource::compile(bytecode, code.get(cursor + 1)?)?;
        let Op::Binary(recurrence) = code.get(cursor + 2)? else {
            return None;
        };
        let recurrence = NumericRecurrence::compile(*recurrence)?;
        if !matches!(code.get(cursor + 3), Some(Op::Dup)) {
            return None;
        }
        let accumulator_store =
            LoopStore::compile(bytecode, code.get(cursor + 4)?, accumulator_slot)?;
        let Op::StoreLocal(loop_result_slot) = code.get(cursor + 5)? else {
            return None;
        };

        let tail = cursor + 6;
        if !matches!(code.get(tail), Some(Op::LoadLocal(slot)) if slot == counter_slot)
            || !matches!(code.get(tail + 1), Some(Op::ToNumeric))
            || !matches!(code.get(tail + 2), Some(Op::Dup))
        {
            return None;
        }
        let update = match code.get(tail + 3)? {
            Op::Update(UpdateOp::Increment) => CounterUpdate::Increment,
            Op::Update(UpdateOp::Decrement) => CounterUpdate::Decrement,
            _ => return None,
        };
        let counter_store = LoopStore::compile(bytecode, code.get(tail + 4)?, *counter_slot)?;
        if !matches!(code.get(tail + 5), Some(Op::Pop))
            || !matches!(code.get(tail + 6), Some(Op::Jump(target)) if target == &header)
            || tail + 6 != backedge
            || !matches!(code.get(*exit), Some(Op::Pop))
        {
            return None;
        }

        Some(Self {
            header,
            backedge,
            exit: *exit,
            counter_slot: *counter_slot,
            accumulator_slot,
            accumulator_read,
            loop_result_slot: *loop_result_slot,
            limit,
            operand,
            comparison,
            recurrence,
            update,
            accumulator_store,
            counter_store,
        })
    }

    fn try_run(&self, vm: &mut Vm<'_>) -> bool {
        if vm.direct_eval_with_stack
            || !slot_is_trace_local(vm, self.loop_result_slot)
            || !self.accumulator_store.can_sink(vm)
            || !self.counter_store.can_sink(vm)
        {
            return false;
        }
        let Some(mut counter) = local_number(vm, self.counter_slot) else {
            return false;
        };
        let Some(mut accumulator) = self.accumulator_read.number(vm) else {
            return false;
        };
        let Some(limit) = self
            .limit
            .prepare(vm, self.counter_slot, self.accumulator_slot)
        else {
            return false;
        };
        let Some(operand) = self
            .operand
            .prepare(vm, self.counter_slot, self.accumulator_slot)
        else {
            return false;
        };
        if !self
            .comparison
            .test(counter, limit.value(counter, accumulator))
        {
            return false;
        }

        loop {
            accumulator = self
                .recurrence
                .apply(accumulator, operand.value(counter, accumulator));
            counter = self.update.apply(counter);
            if !self
                .comparison
                .test(counter, limit.value(counter, accumulator))
            {
                break;
            }
        }

        self.accumulator_store.commit(vm, accumulator);
        self.counter_store.commit(vm, counter);
        vm.locals[self.loop_result_slot] = Some(Value::Number(accumulator));
        vm.ip = self.exit + 1;
        true
    }
}

pub(super) fn try_run_simple_numeric_loop(vm: &mut Vm<'_>, header: usize, backedge: usize) -> bool {
    let plan = vm
        .bytecode
        .simple_numeric_loop_plans
        .get_or_init(|| SimpleNumericLoopPlan::compile_all(vm.bytecode))
        .iter()
        .find(|plan| plan.header == header && plan.backedge == backedge)
        .cloned();
    plan.is_some_and(|plan| plan.try_run(vm))
}

fn slot_is_stable(vm: &Vm<'_>, slot: usize) -> bool {
    vm.slot_is_authoritative(slot) || vm.slot_is_realm_binding(slot)
}

fn slot_is_trace_local(vm: &Vm<'_>, slot: usize) -> bool {
    vm.slot_is_authoritative(slot)
        || vm
            .bytecode
            .local_name_at(slot)
            .is_some_and(super::vm_bindings::is_compiler_temporary)
}

fn local_number(vm: &Vm<'_>, slot: usize) -> Option<f64> {
    match vm.local_slot_value(slot)? {
        Value::Number(value) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;
    use crate::{Value, eval};

    fn compile(source: &str) -> Bytecode {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        compiler::compile_script(&script).expect("source should compile")
    }

    fn nested_function(source: &str) -> Bytecode {
        let bytecode = compile(source);
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
    fn recognizes_local_and_global_numeric_recurrences() {
        let local = nested_function(
            "function fold(limit, value) { for (var i = 0; i < limit; i++) value = value & i; return value; }",
        );
        assert_eq!(SimpleNumericLoopPlan::compile_all(&local).len(), 1);

        let global = compile(
            "folded = 4294967296; for (var cursor = 0; cursor < 600000; cursor++) folded = folded & cursor;",
        );
        assert_eq!(SimpleNumericLoopPlan::compile_all(&global).len(), 1);
    }

    #[test]
    fn executes_global_bitwise_recurrence_and_commits_both_bindings() {
        assert_eq!(
            eval(
                "folded = 4294967296; for (var cursor = 0; cursor < 600000; cursor++) folded = folded & cursor; folded + ':' + cursor;"
            ),
            Ok(Value::String("0:600000".to_owned().into()))
        );
    }

    #[test]
    fn executes_local_arithmetic_and_decrement_recurrences() {
        assert_eq!(
            eval(
                "function addFold(limit, value) { for (var i = 0; i < limit; i++) value = value + i; return value; } addFold(10, 1);"
            ),
            Ok(Value::Number(46.0))
        );
        assert_eq!(
            eval(
                "function xorFold(value) { for (var i = 5; i > 0; i--) value = value ^ i; return value; } xorFold(1);"
            ),
            Ok(Value::Number(0.0))
        );
    }

    #[test]
    fn non_number_values_deopt_before_the_first_iteration() {
        assert_eq!(
            eval(
                "function append(limit, value) { for (var i = 0; i < limit; i++) value = value + i; return value; } append(3, 'x');"
            ),
            Ok(Value::String("x012".to_owned().into()))
        );
    }

    #[test]
    fn global_accessors_deopt_and_preserve_each_observable_read() {
        assert_eq!(
            eval(
                "var gets = 0; Object.defineProperty(globalThis, 'folded', { configurable: true, get() { gets++; return 7; } }); for (var cursor = 0; cursor < 3; cursor++) folded = folded & cursor; gets + ':' + cursor;"
            ),
            Ok(Value::String("3:3".to_owned().into()))
        );
    }

    #[test]
    fn dynamic_accumulator_limits_and_zero_iteration_loops_match_bytecode() {
        assert_eq!(
            eval(
                "function shrink(value) { for (var i = 1; i < value; i++) value = value - 1; return value; } shrink(10);"
            ),
            Ok(Value::Number(5.0))
        );
        assert_eq!(
            eval(
                "function untouched(limit, value) { for (var i = 0; i < limit; i++) value = value ^ i; return value + ':' + i; } untouched(0, 7);"
            ),
            Ok(Value::String("7:0".to_owned().into()))
        );
    }
}
