use qjs_ast::{BinaryOp, UpdateOp};

use crate::{Value, value::OwnDataPropertyRead};

use super::{
    ir::{Bytecode, NamedPropertyCache, Op},
    vm::Vm,
    vm_numeric_leaf::NumericLoopCall,
};

#[derive(Clone, Copy, Debug)]
enum NumericLoopArgument {
    Counter,
    Constant(f64),
}

#[derive(Clone, Copy, Debug)]
enum NumericLoopArguments {
    None,
    One(NumericLoopArgument),
    Two(NumericLoopArgument, NumericLoopArgument),
}

impl NumericLoopArguments {
    fn len(self) -> usize {
        match self {
            Self::None => 0,
            Self::One(_) => 1,
            Self::Two(_, _) => 2,
        }
    }

    fn values(self, counter: f64) -> [f64; 2] {
        let value = |argument| match argument {
            NumericLoopArgument::Counter => counter,
            NumericLoopArgument::Constant(value) => value,
        };
        match self {
            Self::None => [0.0, 0.0],
            Self::One(first) => [value(first), 0.0],
            Self::Two(first, second) => [value(first), value(second)],
        }
    }
}

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
    GlobalCall {
        name: String,
        arguments: NumericLoopArguments,
    },
    GlobalMethodCall {
        receiver_name: String,
        key: std::rc::Rc<str>,
        arguments: NumericLoopArguments,
    },
    LocalCall {
        callee_slot: usize,
        arguments: NumericLoopArguments,
    },
    MethodCall {
        receiver_slot: usize,
        key: std::rc::Rc<str>,
        arguments: NumericLoopArguments,
    },
}

#[derive(Clone, Debug)]
enum PreparedNumericLoopTerm {
    Stable(f64),
    Call {
        call: NumericLoopCall,
        arguments: NumericLoopArguments,
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
            let Op::LoadLocal(term_accumulator_slot) = code.get(cursor)? else {
                return None;
            };
            let (term, suffix) = NumericLoopTerm::compile(bytecode, cursor + 1, *counter_slot)?;
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
            if assigned_accumulator_slot != term_accumulator_slot
                || term_block_result_slot != block_result_slot
                || term_loop_result_slot != loop_result_slot
                || accumulator_slot.is_some_and(|slot| slot != *term_accumulator_slot)
            {
                return None;
            }
            accumulator_slot = Some(*term_accumulator_slot);
            terms.push(term);
            cursor = suffix + 6;
        }
        if cursor != tail
            || terms.is_empty()
            || terms.len() > 1 && terms.iter().any(NumericLoopTerm::is_call)
        {
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
        let Some(limit) = local_number_read(vm, self.limit_slot) else {
            return false;
        };
        let Some(mut accumulator) = local_number(vm, self.accumulator_slot) else {
            return false;
        };
        let mut terms = Vec::with_capacity(self.terms.len());
        for term in &self.terms {
            let Some(term) = term.prepare(vm) else {
                return false;
            };
            terms.push(term);
        }

        while counter < limit {
            for term in &mut terms {
                accumulator += term.eval(counter);
            }
            counter += 1.0;
        }
        for term in terms {
            term.commit();
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
    fn compile(bytecode: &Bytecode, cursor: usize, counter_slot: usize) -> Option<(Self, usize)> {
        let code = &bytecode.code;
        match code.get(cursor)? {
            Op::GetPropNamed { cache, .. } => Some((
                Self::NamedProperty {
                    receiver_slot: cache.local_slot()?,
                    cache: cache.clone(),
                },
                cursor + 1,
            )),
            Op::GetPropIndex(encoded) if usize::BITS > u32::BITS => {
                let receiver_slot = (encoded >> u32::BITS).checked_sub(1)?;
                Some((
                    Self::DenseIndex {
                        receiver_slot,
                        index: encoded & u32::MAX as usize,
                    },
                    cursor + 1,
                ))
            }
            Op::LoadGlobal(receiver_name)
                if matches!(code.get(cursor + 1), Some(Op::Dup))
                    && matches!(code.get(cursor + 2), Some(Op::GetPropNamed { .. })) =>
            {
                let Op::GetPropNamed { key, .. } = code.get(cursor + 2)? else {
                    unreachable!("guarded named method read");
                };
                let (arguments, suffix) =
                    compile_call_arguments(bytecode, cursor + 3, counter_slot, true)?;
                Some((
                    Self::GlobalMethodCall {
                        receiver_name: receiver_name.clone(),
                        key: key.clone(),
                        arguments,
                    },
                    suffix,
                ))
            }
            Op::LoadGlobal(name) => {
                let (arguments, suffix) =
                    compile_call_arguments(bytecode, cursor + 1, counter_slot, false)?;
                Some((
                    Self::GlobalCall {
                        name: name.clone(),
                        arguments,
                    },
                    suffix,
                ))
            }
            Op::LoadLocal(receiver_slot)
                if matches!(code.get(cursor + 1), Some(Op::Dup))
                    && matches!(code.get(cursor + 2), Some(Op::GetPropNamed { .. })) =>
            {
                let Op::GetPropNamed { key, .. } = code.get(cursor + 2)? else {
                    unreachable!("guarded named method read");
                };
                let (arguments, suffix) =
                    compile_call_arguments(bytecode, cursor + 3, counter_slot, true)?;
                Some((
                    Self::MethodCall {
                        receiver_slot: *receiver_slot,
                        key: key.clone(),
                        arguments,
                    },
                    suffix,
                ))
            }
            Op::LoadLocal(callee_slot) => {
                let (arguments, suffix) =
                    compile_call_arguments(bytecode, cursor + 1, counter_slot, false)?;
                Some((
                    Self::LocalCall {
                        callee_slot: *callee_slot,
                        arguments,
                    },
                    suffix,
                ))
            }
            _ => None,
        }
    }

    fn is_call(&self) -> bool {
        matches!(
            self,
            Self::GlobalCall { .. }
                | Self::GlobalMethodCall { .. }
                | Self::LocalCall { .. }
                | Self::MethodCall { .. }
        )
    }

    fn prepare(&self, vm: &mut Vm<'_>) -> Option<PreparedNumericLoopTerm> {
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
                    Value::Number(value) => Some(PreparedNumericLoopTerm::Stable(value)),
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
                    Value::Number(value) => Some(PreparedNumericLoopTerm::Stable(value)),
                    _ => None,
                }
            }
            Self::GlobalCall { name, arguments } => {
                let Value::Function(function) = vm.env.get(name)? else {
                    return None;
                };
                Self::prepare_call(function, arguments, vm)
            }
            Self::GlobalMethodCall {
                receiver_name,
                key,
                arguments,
            } => {
                let Value::Object(object) = vm.env.get(receiver_name)? else {
                    return None;
                };
                let OwnDataPropertyRead::Data(Value::Function(function)) =
                    object.own_data_property_read(key)
                else {
                    return None;
                };
                Self::prepare_call(function, arguments, vm)
            }
            Self::LocalCall {
                callee_slot,
                arguments,
            } => {
                if !vm.slot_is_authoritative(*callee_slot) {
                    return None;
                }
                let Some(Some(Value::Function(function))) = vm.locals.get(*callee_slot) else {
                    return None;
                };
                Self::prepare_call(function.clone(), arguments, vm)
            }
            Self::MethodCall {
                receiver_slot,
                key,
                arguments,
            } => {
                if !vm.slot_is_authoritative(*receiver_slot) {
                    return None;
                }
                let Some(Some(Value::Object(object))) = vm.locals.get(*receiver_slot) else {
                    return None;
                };
                let OwnDataPropertyRead::Data(Value::Function(function)) =
                    object.own_data_property_read(key)
                else {
                    return None;
                };
                Self::prepare_call(function, arguments, vm)
            }
        }
    }

    fn prepare_call(
        function: crate::Function,
        arguments: &NumericLoopArguments,
        vm: &Vm<'_>,
    ) -> Option<PreparedNumericLoopTerm> {
        Some(PreparedNumericLoopTerm::Call {
            call: NumericLoopCall::prepare(&function, arguments.len(), &vm.local_upvalues)?,
            arguments: *arguments,
        })
    }
}

fn compile_call_arguments(
    bytecode: &Bytecode,
    mut cursor: usize,
    counter_slot: usize,
    resolved: bool,
) -> Option<(NumericLoopArguments, usize)> {
    let mut arguments = NumericLoopArguments::None;
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
                (NumericLoopArgument::Counter, cursor + 1)
            }
            Op::LoadConst(index) => match bytecode.constants.get(*index)? {
                Value::Number(value)
                    if matches!(
                        bytecode.code.get(cursor + 1),
                        Some(Op::Unary(qjs_ast::UnaryOp::Minus))
                    ) =>
                {
                    (NumericLoopArgument::Constant(-*value), cursor + 2)
                }
                Value::Number(value) => (NumericLoopArgument::Constant(*value), cursor + 1),
                _ => return None,
            },
            _ => return None,
        };
        arguments = match arguments {
            NumericLoopArguments::None => NumericLoopArguments::One(argument),
            NumericLoopArguments::One(first) => NumericLoopArguments::Two(first, argument),
            NumericLoopArguments::Two(_, _) => return None,
        };
        cursor = next_cursor;
    }
}

impl PreparedNumericLoopTerm {
    fn eval(&mut self, counter: f64) -> f64 {
        match self {
            Self::Stable(value) => *value,
            Self::Call { call, arguments } => {
                let [first, second] = arguments.values(counter);
                call.eval(first, second)
            }
        }
    }

    fn commit(self) {
        if let Self::Call { call, .. } = self {
            call.commit();
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
    fn leaves_callable_admission_to_runtime_guards() {
        let bytecode = nested_function(
            "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += Number(i); } return s; }",
        );
        assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1);
    }

    #[test]
    fn recognizes_numeric_global_local_and_method_calls() {
        for source in [
            "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += leaf(i); } return s; }",
            "function sum(n) { var f = makeLeaf(); var s = 0; for (var i = 0; i < n; i++) { s += f(i); } return s; }",
            "function sum(n) { var o = { f: leaf }; var s = 0; for (var i = 0; i < n; i++) { s += o.f(i); } return s; }",
            "function runMethodCall(iterations) { var receiver = { addOne: function (value) { return value + 1; } }; var checksum = 0; for (var i = 0; i < iterations; i++) { checksum += receiver.addOne(i); } return { operations: iterations, checksum: checksum }; }",
            "function sum(n) { var f = makeWriter(); var s = 0; for (var i = 0; i < n; i++) { s += f(); } return s; }",
            "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += leaf(i, 2); } return s; }",
            "function sum(n) { var f = makeLeaf(); var s = 0; for (var i = 0; i < n; i++) { s += f(i, 3); } return s; }",
            "function sum(n) { var o = { f: leaf }; var s = 0; for (var i = 0; i < n; i++) { s += o.f(i, 4); } return s; }",
        ] {
            let bytecode = nested_function(source);
            assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1, "{source}");
        }
    }

    #[test]
    fn recognizes_numeric_global_object_method_calls() {
        let bytecode = nested_function(
            "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += Math.abs(-1); } return s; }",
        );
        assert_eq!(
            NumericLoopPlan::compile_all(&bytecode).len(),
            1,
            "{:?}",
            bytecode.code
        );
    }

    #[test]
    fn rejects_non_numeric_call_constants() {
        let bytecode = nested_function(
            "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += leaf(i, 'x'); } return s; }",
        );
        assert!(NumericLoopPlan::compile_all(&bytecode).is_empty());
    }
}
