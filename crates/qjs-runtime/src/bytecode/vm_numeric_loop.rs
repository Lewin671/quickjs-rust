use qjs_ast::{BinaryOp, UpdateOp};

use crate::{NativeFunction, Value, value::OwnDataPropertyRead};

use super::{
    ir::{Bytecode, NamedPropertyCache, Op, decode_index_receiver},
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
    LocalRead {
        slot: usize,
    },
    GlobalRead {
        name: String,
    },
    NamedProperty {
        receiver_slot: usize,
        cache: NamedPropertyCache,
    },
    ComputedProperty {
        receiver_slot: usize,
        key_slot: usize,
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
    StringSliceLength {
        receiver_slot: usize,
        arguments: NumericLoopArguments,
    },
}

#[derive(Clone, Debug)]
enum PreparedNumericLoopTerm {
    Stable(f64),
    DenseArrayIndexOf {
        array: crate::ArrayRef,
        arguments: NumericLoopArguments,
    },
    Call {
        call: NumericLoopCall,
        arguments: NumericLoopArguments,
    },
    StringSliceLength {
        value: std::rc::Rc<String>,
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
            let accumulator_first = match code.get(cursor) {
                Some(Op::LoadLocal(term_accumulator_slot)) => NumericLoopTerm::compile(
                    bytecode,
                    cursor + 1,
                    *counter_slot,
                    *term_accumulator_slot,
                )
                .map(|(term, suffix)| (*term_accumulator_slot, term, suffix)),
                _ => None,
            };
            let (term_accumulator_slot, term, suffix) = accumulator_first.or_else(|| {
                let (term, suffix) =
                    NumericLoopTerm::compile_reordered_call(bytecode, cursor, *counter_slot)?;
                let Op::LoadLocal(term_accumulator_slot) = code.get(suffix)? else {
                    return None;
                };
                Some((*term_accumulator_slot, term, suffix + 1))
            })?;
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
            if term_accumulator_slot == *counter_slot
                || assigned_accumulator_slot != &term_accumulator_slot
                || term_block_result_slot != block_result_slot
                || term_loop_result_slot != loop_result_slot
                || accumulator_slot.is_some_and(|slot| slot != term_accumulator_slot)
            {
                return None;
            }
            accumulator_slot = Some(term_accumulator_slot);
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
    fn compile_reordered_call(
        bytecode: &Bytecode,
        cursor: usize,
        counter_slot: usize,
    ) -> Option<(Self, usize)> {
        let (global_name, local_slot) = match bytecode.code.get(cursor)? {
            Op::LoadGlobal(name) => (Some(name.clone()), None),
            Op::LoadLocal(callee_slot) => (None, Some(*callee_slot)),
            _ => return None,
        };
        let (arguments, suffix) =
            compile_call_arguments(bytecode, cursor + 1, counter_slot, false)?;
        let term = match (global_name, local_slot) {
            (Some(name), None) => Self::GlobalCall { name, arguments },
            (None, Some(callee_slot)) => Self::LocalCall {
                callee_slot,
                arguments,
            },
            _ => unreachable!("reordered call has exactly one callee route"),
        };
        Some((term, suffix))
    }

    fn compile(
        bytecode: &Bytecode,
        cursor: usize,
        counter_slot: usize,
        accumulator_slot: usize,
    ) -> Option<(Self, usize)> {
        let code = &bytecode.code;
        match code.get(cursor)? {
            Op::LoadLocal(slot)
                if *slot != counter_slot
                    && *slot != accumulator_slot
                    && matches!(code.get(cursor + 1), Some(Op::Binary(BinaryOp::Add))) =>
            {
                Some((Self::LocalRead { slot: *slot }, cursor + 1))
            }
            Op::GetPropNamed { cache, .. } => Some((
                Self::NamedProperty {
                    receiver_slot: cache.local_slot()?,
                    cache: cache.clone(),
                },
                cursor + 1,
            )),
            Op::LoadLocal(receiver_slot)
                if matches!(code.get(cursor + 1), Some(Op::LoadLocal(_)))
                    && matches!(code.get(cursor + 2), Some(Op::GetProp)) =>
            {
                let Op::LoadLocal(key_slot) = code.get(cursor + 1)? else {
                    unreachable!("guarded computed-property key load");
                };
                if *key_slot == counter_slot || *key_slot == accumulator_slot {
                    return None;
                }
                Some((
                    Self::ComputedProperty {
                        receiver_slot: *receiver_slot,
                        key_slot: *key_slot,
                    },
                    cursor + 3,
                ))
            }
            Op::GetPropIndex(encoded) => {
                let (index, receiver_slot) = decode_index_receiver(*encoded);
                let receiver_slot = receiver_slot?;
                Some((
                    Self::DenseIndex {
                        receiver_slot,
                        index,
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
            Op::LoadGlobal(name)
                if matches!(code.get(cursor + 1), Some(Op::Binary(BinaryOp::Add))) =>
            {
                Some((Self::GlobalRead { name: name.clone() }, cursor + 1))
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
                    && matches!(
                        code.get(cursor + 2),
                        Some(Op::GetPropNamed { key, .. }) if key.as_ref() == "slice"
                    ) =>
            {
                let (arguments, suffix) =
                    compile_call_arguments(bytecode, cursor + 3, counter_slot, true)?;
                if arguments.len() != 2
                    || !matches!(
                        code.get(suffix),
                        Some(Op::GetPropNamed { key, .. }) if key.as_ref() == "length"
                    )
                {
                    return None;
                }
                Some((
                    Self::StringSliceLength {
                        receiver_slot: *receiver_slot,
                        arguments,
                    },
                    suffix + 1,
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
                | Self::StringSliceLength { .. }
        )
    }

    fn prepare(&self, vm: &mut Vm<'_>) -> Option<PreparedNumericLoopTerm> {
        match self {
            Self::LocalRead { slot } => {
                if !slot_is_stable_read(vm, *slot) {
                    return None;
                }
                local_number_read(vm, *slot).map(PreparedNumericLoopTerm::Stable)
            }
            Self::GlobalRead { name } => {
                if vm.bytecode.local_slot(name).is_some()
                    || vm.env.module_import_value(name).is_some()
                    || vm.env.is_immutable_function_name(name)
                {
                    return None;
                }
                match vm.env.get(name) {
                    Some(Value::Number(value)) => Some(PreparedNumericLoopTerm::Stable(value)),
                    _ => None,
                }
            }
            Self::NamedProperty {
                receiver_slot,
                cache,
            } => {
                if !slot_is_stable_read(vm, *receiver_slot) {
                    return None;
                }
                let Some(Value::Object(object)) = vm.local_slot_value(*receiver_slot) else {
                    return None;
                };
                match cache.get(&object)? {
                    Value::Number(value) => Some(PreparedNumericLoopTerm::Stable(value)),
                    _ => None,
                }
            }
            Self::ComputedProperty {
                receiver_slot,
                key_slot,
            } => {
                if !slot_is_stable_read(vm, *receiver_slot) || !slot_is_stable_read(vm, *key_slot) {
                    return None;
                }
                let receiver = vm.local_slot_value(*receiver_slot)?;
                let key = vm.local_slot_value(*key_slot)?;
                match (&receiver, &key) {
                    (Value::Object(object), Value::String(key)) => {
                        match object.own_data_property_read(key) {
                            OwnDataPropertyRead::Data(Value::Number(value)) => {
                                Some(PreparedNumericLoopTerm::Stable(value))
                            }
                            _ => None,
                        }
                    }
                    (Value::Array(array), Value::Number(number)) => {
                        let index = super::vm_props::array_index_from_number(*number)?;
                        match array.direct_dense_index_value(index)? {
                            Value::Number(value) => Some(PreparedNumericLoopTerm::Stable(value)),
                            _ => None,
                        }
                    }
                    _ => None,
                }
            }
            Self::DenseIndex {
                receiver_slot,
                index,
            } => {
                if !slot_is_stable_read(vm, *receiver_slot) {
                    return None;
                }
                let Some(Value::Array(array)) = vm.local_slot_value(*receiver_slot) else {
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
                if !slot_is_stable_read(vm, *callee_slot) {
                    return None;
                }
                let Some(Value::Function(function)) = vm.local_slot_value(*callee_slot) else {
                    return None;
                };
                Self::prepare_call(function, arguments, vm)
            }
            Self::MethodCall {
                receiver_slot,
                key,
                arguments,
            } => {
                if !slot_is_stable_read(vm, *receiver_slot) {
                    return None;
                }
                if let Some(term) =
                    Self::prepare_dense_array_index_of(*receiver_slot, key, *arguments, vm)
                {
                    return Some(term);
                }
                let Some(Value::Object(object)) = vm.local_slot_value(*receiver_slot) else {
                    return None;
                };
                let OwnDataPropertyRead::Data(Value::Function(function)) =
                    object.own_data_property_read(key)
                else {
                    return None;
                };
                Self::prepare_call(function, arguments, vm)
            }
            Self::StringSliceLength {
                receiver_slot,
                arguments,
            } => {
                if !vm.slot_is_authoritative(*receiver_slot) {
                    return None;
                }
                let Some(Some(Value::String(value))) = vm.locals.get(*receiver_slot) else {
                    return None;
                };
                let prototype = crate::string_prototype(&vm.realm_env())?;
                let property = prototype.own_property("slice")?;
                if property.is_accessor() {
                    return None;
                }
                let Value::Function(function) = property.value else {
                    return None;
                };
                if function.native_kind() != Some(NativeFunction::StringPrototypeSlice) {
                    return None;
                }
                Some(PreparedNumericLoopTerm::StringSliceLength {
                    value: value.clone(),
                    arguments: *arguments,
                })
            }
        }
    }

    fn prepare_dense_array_index_of(
        receiver_slot: usize,
        key: &str,
        arguments: NumericLoopArguments,
        vm: &Vm<'_>,
    ) -> Option<PreparedNumericLoopTerm> {
        if key != "indexOf" || arguments.len() == 0 {
            return None;
        }
        let Some(Some(Value::Array(array))) = vm.locals.get(receiver_slot) else {
            return None;
        };
        if !array.uses_default_prototype() || array.property(key).is_some() {
            return None;
        }
        let prototype = crate::array_prototype(&vm.realm_env())?;
        let OwnDataPropertyRead::Data(Value::Function(function)) =
            prototype.own_data_property_read(key)
        else {
            return None;
        };
        if function.native != Some(NativeFunction::ArrayPrototypeIndexOf)
            || array.direct_dense_index_of_number(0.0, 0).is_none()
        {
            return None;
        }
        Some(PreparedNumericLoopTerm::DenseArrayIndexOf {
            array: array.clone(),
            arguments,
        })
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
            Self::DenseArrayIndexOf { array, arguments } => {
                let [search, from_index] = arguments.values(counter);
                let length = array.len();
                let start = if arguments.len() < 2 || from_index.is_nan() {
                    0
                } else if from_index >= length as f64 {
                    length
                } else if from_index >= 0.0 {
                    from_index.trunc() as usize
                } else {
                    (length as f64 + from_index.trunc()).max(0.0) as usize
                };
                array
                    .direct_dense_index_of_number(search, start)
                    .expect("prepared dense array search stays side-effect free")
            }
            Self::Call { call, arguments } => {
                let [first, second] = arguments.values(counter);
                call.eval(first, second)
            }
            Self::StringSliceLength { value, arguments } => {
                let [start, end] = arguments.values(counter);
                crate::string::numeric_string_slice_code_unit_len(value, start, end) as f64
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

fn slot_is_stable_read(vm: &Vm<'_>, slot: usize) -> bool {
    vm.slot_is_authoritative(slot) || vm.slot_is_realm_binding(slot)
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
    fn recognizes_named_property_accumulation_loop() {
        let bytecode = nested_function(
            "function sum(n) { var o = { a: 1, b: 2 }; var s = 0; for (var i = 0; i < n; i++) { s += o.a; s += o.b; } return s; }",
        );
        let plans = NumericLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].terms.len(), 2);
    }

    #[test]
    fn recognizes_stable_local_read_accumulation_loop() {
        let bytecode = nested_function(
            "function sum(n) { var first = 1, second = 2, s = 0; for (var i = 0; i < n; i++) { s += first; s += second; } return s; }",
        );
        let plans = NumericLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].terms.len(), 2);
        assert!(
            plans[0]
                .terms
                .iter()
                .all(|term| matches!(term, NumericLoopTerm::LocalRead { .. }))
        );
    }

    #[test]
    fn recognizes_stable_global_read_accumulation_loop() {
        let source = "var value = 2; function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += value; } return s; }";
        let bytecode = nested_function(source);
        let plans = NumericLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1);
        let [NumericLoopTerm::LocalRead { slot }] = plans[0].terms.as_slice() else {
            panic!("read-only global should compile as a realm-cell local read");
        };
        assert!(bytecode.local_is_from_env(*slot));
        assert_eq!(eval(&format!("{source} sum(4);")), Ok(Value::Number(8.0)));
    }

    #[test]
    fn rejects_mutating_local_read_terms() {
        for source in [
            "function sum(n) { var s = 1; for (var i = 0; i < n; i++) { s += s; } return s; }",
            "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s += i; } return s; }",
        ] {
            let bytecode = nested_function(source);
            assert!(
                NumericLoopPlan::compile_all(&bytecode).is_empty(),
                "{source}"
            );
        }
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
    fn recognizes_computed_object_and_array_accumulation_loops() {
        for (source, term_count) in [
            (
                "function sum(n) { var o = { a: 1, b: 2 }; var x = 'a', y = 'b'; var s = 0; for (var i = 0; i < n; i++) { s += o[x]; s += o[y]; } return s; }",
                2,
            ),
            (
                "function sum(n) { var a = [1, 2, 3]; var x = 0, y = 1, z = 2; var s = 0; for (var i = 0; i < n; i++) { s += a[x]; s += a[y]; s += a[z]; } return s; }",
                3,
            ),
        ] {
            let bytecode = nested_function(source);
            let plans = NumericLoopPlan::compile_all(&bytecode);
            assert_eq!(plans.len(), 1, "{source}");
            assert_eq!(plans[0].terms.len(), term_count, "{source}");
        }
    }

    #[test]
    fn computed_accessors_keep_the_observable_loop_path() {
        assert_eq!(
            eval(
                "function run(n) { var reads = 0, o = {}, key = 'a', sum = 0; Object.defineProperty(o, 'a', { get: function () { reads += 1; return 2; } }); for (var i = 0; i < n; i++) { sum += o[key]; } return sum + ':' + reads; } run(4);"
            ),
            Ok(Value::String("8:4".to_owned().into()))
        );
    }

    #[test]
    fn rejects_computed_keys_mutated_by_the_loop() {
        for source in [
            "function sum(n) { var a = [1, 2, 3]; var s = 0; for (var i = 0; i < n; i++) { s += a[i]; } return s; }",
            "function sum(n) { var o = { 0: 1, 1: 2 }; var s = 0; for (var i = 0; i < n; i++) { s += o[s]; } return s; }",
        ] {
            let bytecode = nested_function(source);
            assert!(
                NumericLoopPlan::compile_all(&bytecode).is_empty(),
                "{source}"
            );
        }
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
    fn recognizes_reordered_numeric_global_call() {
        let source = "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; }";
        let bytecode = nested_function(source);
        assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1, "{source}");
        assert_eq!(
            eval(
                "function leaf(value) { return value + 1; } function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; } sum(1000);"
            ),
            Ok(Value::Number(500500.0))
        );
    }

    #[test]
    fn reordered_non_numeric_and_mutating_calls_keep_the_observable_path() {
        assert_eq!(
            eval(
                "function leaf(value) { return 'x' + value; } function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; } sum(3);"
            ),
            Ok(Value::String("x2x1x00".to_owned().into()))
        );
        assert_eq!(
            eval(
                "function leaf(value) { if (value === 1) { leaf = function (next) { return next + 10; }; } return value + 1; } function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; } sum(3);"
            ),
            Ok(Value::Number(15.0))
        );
    }

    #[test]
    fn recognizes_reordered_numeric_local_call_and_rejects_counter_accumulation() {
        let source = "function sum(n) { var offset = 1; var leaf = function (value) { return value + offset; }; var s = 0; for (var i = 0; i < n; i++) { s = leaf(i) + s; } return s; }";
        let bytecode = nested_function(source);
        assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1, "{source}");
        assert_eq!(
            eval(&format!("{source} sum(1000);")),
            Ok(Value::Number(500500.0))
        );

        let counter_source =
            "function sum(n) { for (var i = 0; i < n; i++) { i = leaf(i) + i; } return i; }";
        let counter_bytecode = nested_function(counter_source);
        assert!(
            NumericLoopPlan::compile_all(&counter_bytecode).is_empty(),
            "{counter_source}"
        );
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
    fn recognizes_dense_array_index_of_calls() {
        let bytecode = nested_function(
            "function sum(n) { var array = [1, 2, 3, 4]; var s = 0; for (var i = 0; i < n; i++) { s += array.indexOf(3); } return s; }",
        );
        assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1);
    }

    #[test]
    fn recognizes_numeric_string_slice_length_calls() {
        let bytecode = nested_function(
            "function sum(n) { var text = 'the quick brown fox'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 4).length; } return s; }",
        );
        assert_eq!(NumericLoopPlan::compile_all(&bytecode).len(), 1);
        assert_eq!(
            eval(
                "function sum(n) { var text = 'the quick brown fox'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 4).length; } return s; } sum(1000);"
            ),
            Ok(Value::Number(3000.0))
        );
        assert_eq!(
            eval(
                "function sum(n) { var text = '😀x'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(0, 1).length; } return s; } sum(4);"
            ),
            Ok(Value::Number(4.0))
        );
        assert_eq!(
            eval(
                "function sum(n) { var text = '😀x'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(i, 3).length; } return s; } sum(4);"
            ),
            Ok(Value::Number(6.0))
        );
        assert_eq!(
            eval(
                "function sum(n) { var text = '\\u{F0000}x'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(i, 3).length; } return s; } sum(4);"
            ),
            Ok(Value::Number(6.0))
        );
        assert_eq!(
            eval(
                "function sum(n) { var text = 'abcdef'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(-3, -1).length; } return s; } sum(4);"
            ),
            Ok(Value::Number(8.0))
        );
    }

    #[test]
    fn overridden_string_slice_keeps_the_observable_loop_path() {
        assert_eq!(
            eval(
                "String.prototype.slice = function () { return { length: 7 }; }; function sum(n) { var text = 'abc'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 2).length; } return s; } sum(4);"
            ),
            Ok(Value::Number(28.0))
        );
        assert_eq!(
            eval(
                "var reads = 0; var slice = String.prototype.slice; Object.defineProperty(String.prototype, 'slice', { get: function () { reads += 1; return slice; } }); function sum(n) { var text = 'abc'; var s = 0; for (var i = 0; i < n; i++) { s += text.slice(1, 2).length; } return s + ':' + reads; } sum(4);"
            ),
            Ok(Value::String("4:4".to_owned().into()))
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
