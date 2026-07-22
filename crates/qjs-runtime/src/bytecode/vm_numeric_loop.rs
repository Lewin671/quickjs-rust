use qjs_ast::{BinaryOp, UpdateOp};

mod selector;

use crate::{NativeFunction, Value, value::OwnDataPropertyRead};

use super::{
    ir::{Bytecode, NamedPropertyCache, Op, decode_index_receiver},
    vm::Vm,
    vm_numeric_leaf::NumericLoopCall,
    vm_props::RealmGlobalLoopWrite,
};
use selector::{NumericLoopSelector, NumericLoopSlotOverride};

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static NUMERIC_LOOP_ENTRY_HITS: Cell<usize> = const { Cell::new(0) };
    static REALM_GLOBAL_LOOP_BATCH_COMMITS: Cell<usize> = const { Cell::new(0) };
}

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
        key: std::rc::Rc<str>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum NumericLoopWrite {
    Local { slot: usize },
    RealmGlobal { slot: usize, name: String },
}

impl NumericLoopWrite {
    fn compile(op: &Op, expected_slot: usize) -> Option<Self> {
        match op {
            Op::AssignLocal(slot) if *slot == expected_slot => Some(Self::Local { slot: *slot }),
            Op::StoreGlobalSloppy { slot, name } if *slot == expected_slot => {
                Some(Self::RealmGlobal {
                    slot: *slot,
                    name: name.clone(),
                })
            }
            _ => None,
        }
    }

    fn prepare(&self, vm: &Vm<'_>) -> Option<LoopWriteTarget> {
        match self {
            Self::Local { slot } if vm.slot_is_authoritative(*slot) => {
                Some(LoopWriteTarget::Local { slot: *slot })
            }
            Self::RealmGlobal { slot, name } => vm
                .prepare_realm_global_loop_write(*slot, name)
                .map(LoopWriteTarget::RealmGlobal),
            Self::Local { .. } => None,
        }
    }

    fn slot(&self) -> usize {
        match self {
            Self::Local { slot } | Self::RealmGlobal { slot, .. } => *slot,
        }
    }
}

#[derive(Clone, Debug)]
enum LoopWriteTarget {
    Local { slot: usize },
    RealmGlobal(RealmGlobalLoopWrite),
}

impl LoopWriteTarget {
    fn realm_cell(&self) -> Option<crate::function::Upvalue> {
        match self {
            Self::Local { .. } => None,
            Self::RealmGlobal(target) => Some(target.cell()),
        }
    }

    fn number(&self, vm: &Vm<'_>) -> Option<f64> {
        let value = match self {
            Self::Local { slot } => vm.locals.get(*slot)?.as_ref()?.clone(),
            Self::RealmGlobal(target) => target.cell().get(),
        };
        match value {
            Value::Number(value) => Some(value),
            _ => None,
        }
    }
}

/// Prevalidated counted loop whose body only adds stable numeric reads.
#[derive(Clone, Debug)]
pub(super) struct NumericLoopPlan {
    header: usize,
    backedge: usize,
    exit: usize,
    counter_write: NumericLoopWrite,
    limit_slot: usize,
    accumulator_write: NumericLoopWrite,
    block_result_slot: usize,
    loop_result_slot: Option<usize>,
    selector: Option<NumericLoopSelector>,
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
        if !matches!(code.get(*exit), Some(Op::Pop)) || backedge < header + 16 {
            return None;
        }

        let (mut cursor, mut block_result_slot, compact_completion) =
            match (code.get(header + 5), code.get(header + 6)) {
                (Some(Op::LoadConst(_)), Some(Op::StoreLocal(slot))) => {
                    (header + 7, Some(*slot), false)
                }
                _ => (header + 5, None, true),
            };

        let full_tail = backedge.checked_sub(8).and_then(|tail| {
            let (
                Op::LoadLocal(tail_block_result_slot),
                Op::StoreLocal(loop_result_slot),
                Op::LoadLocal(tail_counter_slot),
                Op::ToNumeric,
                Op::Dup,
                Op::Update(UpdateOp::Increment),
                assigned_counter,
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
                || Some(*tail_block_result_slot) != block_result_slot
                || tail_counter_slot != counter_slot
            {
                return None;
            }
            Some((
                tail,
                NumericLoopWrite::compile(assigned_counter, *counter_slot)?,
                Some(*loop_result_slot),
            ))
        });
        let compact_tail = || {
            let tail = backedge.checked_sub(6)?;
            let (
                Op::LoadLocal(tail_counter_slot),
                Op::ToNumeric,
                Op::Dup,
                Op::Update(UpdateOp::Increment),
                assigned_counter,
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
            )
            else {
                return None;
            };
            if !compact_completion
                || tail + 6 != backedge
                || tail_header != &header
                || tail_counter_slot != counter_slot
            {
                return None;
            }
            Some((
                tail,
                NumericLoopWrite::compile(assigned_counter, *counter_slot)?,
                None,
            ))
        };
        let (tail, counter_write, loop_result_slot) = full_tail.or_else(compact_tail)?;

        let selector = NumericLoopSelector::compile(
            bytecode,
            cursor,
            *counter_slot,
            compact_completion,
            block_result_slot,
            loop_result_slot,
        );
        let selector = selector.map(|(selector, next_cursor, selector_block_result_slot)| {
            cursor = next_cursor;
            block_result_slot.get_or_insert(selector_block_result_slot);
            selector
        });

        let mut accumulator_slot = None;
        let mut accumulator_write = None;
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
            let (assigned_accumulator, term_block_result_slot, term_loop_result_slot, next_cursor) =
                if compact_completion {
                    let (
                        Op::Binary(BinaryOp::Add),
                        Op::Dup,
                        assigned_accumulator,
                        Op::StoreLocal(term_block_result_slot),
                    ) = (
                        code.get(suffix)?,
                        code.get(suffix + 1)?,
                        code.get(suffix + 2)?,
                        code.get(suffix + 3)?,
                    )
                    else {
                        return None;
                    };
                    (
                        assigned_accumulator,
                        *term_block_result_slot,
                        None,
                        suffix + 4,
                    )
                } else {
                    let (
                        Op::Binary(BinaryOp::Add),
                        Op::Dup,
                        assigned_accumulator,
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
                    (
                        assigned_accumulator,
                        *term_block_result_slot,
                        Some(*term_loop_result_slot),
                        suffix + 6,
                    )
                };
            if term_accumulator_slot == *counter_slot
                || block_result_slot.is_some_and(|slot| slot != term_block_result_slot)
                || term_loop_result_slot != loop_result_slot
                || accumulator_slot.is_some_and(|slot| slot != term_accumulator_slot)
            {
                return None;
            }
            block_result_slot.get_or_insert(term_block_result_slot);
            let term_write =
                NumericLoopWrite::compile(assigned_accumulator, term_accumulator_slot)?;
            if accumulator_write
                .as_ref()
                .is_some_and(|existing| existing != &term_write)
            {
                return None;
            }
            accumulator_slot = Some(term_accumulator_slot);
            accumulator_write.get_or_insert(term_write);
            terms.push(term);
            cursor = next_cursor;
        }
        if cursor != tail
            || terms.is_empty()
            || terms.len() > 1 && terms.iter().any(NumericLoopTerm::is_call)
        {
            return None;
        }

        let accumulator_slot = accumulator_slot?;
        let accumulator_write = accumulator_write?;
        let block_result_slot = block_result_slot?;
        let mut mutable_slots = vec![*counter_slot, accumulator_slot, block_result_slot];
        if let Some(loop_result_slot) = loop_result_slot {
            mutable_slots.push(loop_result_slot);
        }
        if mutable_slots
            .iter()
            .enumerate()
            .any(|(index, slot)| mutable_slots[..index].contains(slot))
            || mutable_slots.contains(limit_slot)
            || terms.iter().any(|term| term.reads_any_slot(&mutable_slots))
        {
            return None;
        }
        if let Some(selector) = &selector {
            let mut scalar_slots = mutable_slots.clone();
            scalar_slots.push(*limit_slot);
            if !selector.slots_are_disjoint(&scalar_slots) {
                return None;
            }
        }

        Some(Self {
            header,
            backedge,
            exit: *exit,
            counter_write,
            limit_slot: *limit_slot,
            accumulator_write,
            block_result_slot,
            loop_result_slot,
            selector,
            terms,
        })
    }

    fn try_run(&self, vm: &mut Vm<'_>) -> bool {
        if vm.direct_eval_with_stack {
            return false;
        }
        let Some(write_targets) = (|| {
            let mut targets = vec![
                self.counter_write.prepare(vm)?,
                self.accumulator_write.prepare(vm)?,
                NumericLoopWrite::Local {
                    slot: self.block_result_slot,
                }
                .prepare(vm)?,
            ];
            if let Some(loop_result_slot) = self.loop_result_slot {
                targets.push(
                    NumericLoopWrite::Local {
                        slot: loop_result_slot,
                    }
                    .prepare(vm)?,
                );
            }
            Some(targets)
        })() else {
            return false;
        };
        for (index, target) in write_targets.iter().enumerate() {
            let LoopWriteTarget::RealmGlobal(target) = target else {
                continue;
            };
            if write_targets[..index].iter().any(|previous| {
                let LoopWriteTarget::RealmGlobal(previous) = previous else {
                    return false;
                };
                target.name() == previous.name() || target.cell().ptr_eq(&previous.cell())
            }) {
                return false;
            }
        }
        let mut forbidden_cells = write_targets
            .iter()
            .filter_map(LoopWriteTarget::realm_cell)
            .collect::<Vec<_>>();
        let prepared_selector = if let Some(selector) = &self.selector {
            let mut scalar_slots = vec![
                self.counter_write.slot(),
                self.limit_slot,
                self.accumulator_write.slot(),
                self.block_result_slot,
            ];
            if let Some(loop_result_slot) = self.loop_result_slot {
                scalar_slots.push(loop_result_slot);
            }
            let Some(prepared) = selector.prepare(vm, &scalar_slots) else {
                return false;
            };
            for slot in scalar_slots.iter().copied().chain(selector.slots()) {
                let Some(Some(cell)) = vm.local_upvalues.get(slot) else {
                    continue;
                };
                if !forbidden_cells
                    .iter()
                    .any(|forbidden| forbidden.ptr_eq(cell))
                {
                    forbidden_cells.push(cell.clone());
                }
            }
            Some(prepared)
        } else {
            None
        };
        let forbidden_realm_writes = write_targets
            .iter()
            .filter_map(|target| match target {
                LoopWriteTarget::Local { .. } => None,
                LoopWriteTarget::RealmGlobal(target) => Some(target.clone()),
            })
            .collect::<Vec<_>>();
        let Some(mut counter) = write_targets.first().and_then(|target| target.number(vm)) else {
            return false;
        };
        let Some(limit) = local_number_read(vm, self.limit_slot) else {
            return false;
        };
        let Some(mut accumulator) = write_targets.get(1).and_then(|target| target.number(vm))
        else {
            return false;
        };
        if let (Some(selector), Some(prepared_selector)) = (&self.selector, prepared_selector) {
            return self.try_run_selected(
                vm,
                selector,
                prepared_selector,
                write_targets,
                &forbidden_cells,
                &forbidden_realm_writes,
                counter,
                limit,
                accumulator,
            );
        }
        let mut terms = Vec::with_capacity(self.terms.len());
        for term in &self.terms {
            let Some(term) = term.prepare(vm, &forbidden_cells, &forbidden_realm_writes) else {
                return false;
            };
            terms.push(term);
        }

        #[cfg(test)]
        NUMERIC_LOOP_ENTRY_HITS.with(|hits| hits.set(hits.get() + 1));

        while counter < limit {
            for term in &mut terms {
                accumulator += term.eval(counter);
            }
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
            REALM_GLOBAL_LOOP_BATCH_COMMITS.with(|commits| commits.set(commits.get() + 1));
        }

        for (target, value) in write_targets.into_iter().zip(values) {
            if let LoopWriteTarget::Local { slot } = target {
                set_local_number(vm, slot, value);
            }
        }
        for term in terms {
            term.commit();
        }
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
            Op::GetPropNamed { key, cache, .. } => Some((
                Self::NamedProperty {
                    receiver_slot: cache.local_slot()?,
                    key: key.clone(),
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

    fn reads_any_slot(&self, mutable_slots: &[usize]) -> bool {
        let aliases = |slot: usize| mutable_slots.contains(&slot);
        match self {
            Self::LocalRead { slot }
            | Self::NamedProperty {
                receiver_slot: slot,
                ..
            }
            | Self::DenseIndex {
                receiver_slot: slot,
                ..
            }
            | Self::LocalCall {
                callee_slot: slot, ..
            }
            | Self::MethodCall {
                receiver_slot: slot,
                ..
            }
            | Self::StringSliceLength {
                receiver_slot: slot,
                ..
            } => aliases(*slot),
            Self::ComputedProperty {
                receiver_slot,
                key_slot,
            } => aliases(*receiver_slot) || aliases(*key_slot),
            Self::GlobalRead { .. } | Self::GlobalCall { .. } | Self::GlobalMethodCall { .. } => {
                false
            }
        }
    }

    fn prepare(
        &self,
        vm: &mut Vm<'_>,
        forbidden_cells: &[crate::function::Upvalue],
        forbidden_realm_writes: &[RealmGlobalLoopWrite],
    ) -> Option<PreparedNumericLoopTerm> {
        self.prepare_with_slot_override(vm, forbidden_cells, forbidden_realm_writes, None)
    }

    fn prepare_with_slot_override(
        &self,
        vm: &mut Vm<'_>,
        forbidden_cells: &[crate::function::Upvalue],
        forbidden_realm_writes: &[RealmGlobalLoopWrite],
        slot_override: Option<&NumericLoopSlotOverride<'_>>,
    ) -> Option<PreparedNumericLoopTerm> {
        match self {
            Self::LocalRead { slot } => local_number_read_with_override(vm, *slot, slot_override)
                .map(PreparedNumericLoopTerm::Stable),
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
                key,
                cache,
            } => {
                let Some(Value::Object(object)) =
                    stable_slot_value_with_override(vm, *receiver_slot, slot_override)
                else {
                    return None;
                };
                if forbidden_realm_writes
                    .iter()
                    .any(|write| write.aliases_property(&object, key))
                {
                    return None;
                }
                if slot_override
                    .and_then(|slot_override| slot_override.value_for(*receiver_slot))
                    .is_some()
                {
                    match object.own_data_property_read(key) {
                        OwnDataPropertyRead::Data(Value::Number(value)) => {
                            Some(PreparedNumericLoopTerm::Stable(value))
                        }
                        _ => None,
                    }
                } else {
                    match cache.get(&object)? {
                        Value::Number(value) => Some(PreparedNumericLoopTerm::Stable(value)),
                        _ => None,
                    }
                }
            }
            Self::ComputedProperty {
                receiver_slot,
                key_slot,
            } => {
                let receiver = stable_slot_value_with_override(vm, *receiver_slot, slot_override)?;
                let key = stable_slot_value_with_override(vm, *key_slot, slot_override)?;
                match (&receiver, &key) {
                    (Value::Object(object), Value::String(key)) => {
                        if forbidden_realm_writes
                            .iter()
                            .any(|write| write.aliases_property(object, key))
                        {
                            return None;
                        }
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
                let Some(Value::Array(array)) =
                    stable_slot_value_with_override(vm, *receiver_slot, slot_override)
                else {
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
                Self::prepare_call(function, arguments, vm, forbidden_cells)
            }
            Self::GlobalMethodCall {
                receiver_name,
                key,
                arguments,
            } => {
                let Value::Object(object) = vm.env.get(receiver_name)? else {
                    return None;
                };
                if forbidden_realm_writes
                    .iter()
                    .any(|write| write.aliases_property(&object, key))
                {
                    return None;
                }
                let OwnDataPropertyRead::Data(Value::Function(function)) =
                    object.own_data_property_read(key)
                else {
                    return None;
                };
                Self::prepare_call(function, arguments, vm, forbidden_cells)
            }
            Self::LocalCall {
                callee_slot,
                arguments,
            } => {
                let Some(Value::Function(function)) =
                    stable_slot_value_with_override(vm, *callee_slot, slot_override)
                else {
                    return None;
                };
                Self::prepare_call(function, arguments, vm, forbidden_cells)
            }
            Self::MethodCall {
                receiver_slot,
                key,
                arguments,
            } => {
                if let Some(term) = Self::prepare_dense_array_index_of(
                    *receiver_slot,
                    key,
                    *arguments,
                    vm,
                    slot_override,
                ) {
                    return Some(term);
                }
                let Some(Value::Object(object)) =
                    stable_slot_value_with_override(vm, *receiver_slot, slot_override)
                else {
                    return None;
                };
                if forbidden_realm_writes
                    .iter()
                    .any(|write| write.aliases_property(&object, key))
                {
                    return None;
                }
                let OwnDataPropertyRead::Data(Value::Function(function)) =
                    object.own_data_property_read(key)
                else {
                    return None;
                };
                Self::prepare_call(function, arguments, vm, forbidden_cells)
            }
            Self::StringSliceLength {
                receiver_slot,
                arguments,
            } => {
                if slot_override
                    .and_then(|slot_override| slot_override.value_for(*receiver_slot))
                    .is_none()
                    && !vm.slot_is_authoritative(*receiver_slot)
                {
                    return None;
                }
                let Some(Value::String(value)) =
                    stable_slot_value_with_override(vm, *receiver_slot, slot_override)
                else {
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
                    value,
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
        slot_override: Option<&NumericLoopSlotOverride<'_>>,
    ) -> Option<PreparedNumericLoopTerm> {
        if key != "indexOf" || arguments.len() == 0 {
            return None;
        }
        let Some(Value::Array(array)) =
            stable_slot_value_with_override(vm, receiver_slot, slot_override)
        else {
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
        forbidden_cells: &[crate::function::Upvalue],
    ) -> Option<PreparedNumericLoopTerm> {
        Some(PreparedNumericLoopTerm::Call {
            call: NumericLoopCall::prepare(
                &function,
                arguments.len(),
                &vm.local_upvalues,
                forbidden_cells,
            )?,
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
    fn is_read_only(&self) -> bool {
        match self {
            Self::Call { call, .. } => call.is_read_only(),
            Self::Stable(_) | Self::DenseArrayIndexOf { .. } | Self::StringSliceLength { .. } => {
                true
            }
        }
    }

    // This sits directly around every admitted loop term. Keep it in the
    // scalar loop even as selector-only helpers grow elsewhere in the module;
    // outlining this match adds a call to ordinary one-term loops as well.
    #[inline(always)]
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

fn local_number_read(vm: &Vm<'_>, slot: usize) -> Option<f64> {
    if vm.bytecode.is_global_scope() && vm.slot_is_realm_binding(slot) {
        vm.prepare_realm_global_loop_read(slot)?;
    }
    match vm.local_slot_value(slot)? {
        Value::Number(value) => Some(value),
        _ => None,
    }
}

fn local_number_read_with_override(
    vm: &Vm<'_>,
    slot: usize,
    slot_override: Option<&NumericLoopSlotOverride<'_>>,
) -> Option<f64> {
    if let Some(value) = slot_override.and_then(|slot_override| slot_override.value_for(slot)) {
        return match value {
            Value::Number(value) => Some(value),
            _ => None,
        };
    }
    local_number_read(vm, slot)
}

fn stable_slot_value(vm: &Vm<'_>, slot: usize) -> Option<Value> {
    if vm.slot_is_authoritative(slot) {
        return vm.local_slot_value(slot);
    }
    if !vm.slot_is_realm_binding(slot) {
        return None;
    }
    // Nested functions already receive an exact realm-cell route for captured
    // script globals. Preserve that established stable-read contract; only a
    // root script can coexist with the new transactional realm-global writes
    // and therefore needs the stronger property/cell revalidation.
    if vm.bytecode.is_global_scope() {
        vm.prepare_realm_global_loop_read(slot)?;
    }
    vm.local_slot_value(slot)
}

fn stable_slot_value_with_override(
    vm: &Vm<'_>,
    slot: usize,
    slot_override: Option<&NumericLoopSlotOverride<'_>>,
) -> Option<Value> {
    slot_override
        .and_then(|slot_override| slot_override.value_for(slot))
        .or_else(|| stable_slot_value(vm, slot))
}

fn set_local_number(vm: &mut Vm<'_>, slot: usize, value: f64) {
    vm.locals[slot] = Some(Value::Number(value));
}

#[cfg(test)]
mod tests;
