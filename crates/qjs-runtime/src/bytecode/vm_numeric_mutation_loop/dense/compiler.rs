use std::collections::{BTreeMap, BTreeSet};

use super::*;

#[derive(Clone, Debug)]
enum AbstractValue {
    Local(usize),
    Number(Register),
    Key(Register),
    DirectThis,
    OwnData { source: OwnDataSource, read: usize },
    MathObject,
    MathRoundFunction,
    Other,
}

struct Translator<'a> {
    bytecode: &'a Bytecode,
    stack: Vec<AbstractValue>,
    locals: BTreeMap<usize, AbstractValue>,
    operations: Vec<NumberInstruction>,
    writes: Vec<LocalWrite>,
    written_slots: BTreeSet<usize>,
    number_slots: BTreeSet<usize>,
    receiver_sources: Vec<ArraySource>,
    number_sources: Vec<OwnDataSource>,
    own_data_reads: Vec<bool>,
    store_count: usize,
    uses_math_round: bool,
}

impl<'a> Translator<'a> {
    fn new(bytecode: &'a Bytecode) -> Self {
        Self {
            bytecode,
            stack: Vec::new(),
            locals: BTreeMap::new(),
            operations: Vec::new(),
            writes: Vec::new(),
            written_slots: BTreeSet::new(),
            number_slots: BTreeSet::new(),
            receiver_sources: Vec::new(),
            number_sources: Vec::new(),
            own_data_reads: Vec::new(),
            store_count: 0,
            uses_math_round: false,
        }
    }

    fn emit(&mut self, operation: NumberInstruction) -> Option<Register> {
        if self.operations.len() >= MAX_DENSE_OPS {
            return None;
        }
        let register = self.operations.len();
        self.operations.push(operation);
        Some(register)
    }

    fn pop(&mut self) -> Option<AbstractValue> {
        self.stack.pop()
    }

    fn own_data(&mut self, source: OwnDataSource) -> AbstractValue {
        let read = self.own_data_reads.len();
        self.own_data_reads.push(false);
        AbstractValue::OwnData { source, read }
    }

    fn number(&mut self, value: AbstractValue) -> Option<Register> {
        match value {
            AbstractValue::Number(register) | AbstractValue::Key(register) => Some(register),
            AbstractValue::Local(slot) => {
                self.number_slots.insert(slot);
                self.emit(NumberInstruction::LoadLocal(slot))
            }
            AbstractValue::OwnData { source, read } => {
                self.own_data_reads[read] = true;
                let source = match self
                    .number_sources
                    .iter()
                    .position(|existing| existing == &source)
                {
                    Some(index) => index,
                    None => {
                        if self.number_sources.len() >= MAX_DENSE_LOCALS {
                            return None;
                        }
                        let index = self.number_sources.len();
                        self.number_sources.push(source);
                        index
                    }
                };
                self.emit(NumberInstruction::LoadInvariant(source))
            }
            AbstractValue::DirectThis
            | AbstractValue::MathObject
            | AbstractValue::MathRoundFunction
            | AbstractValue::Other => None,
        }
    }

    fn receiver(&mut self, value: &AbstractValue) -> Option<usize> {
        let source = match value {
            AbstractValue::Local(slot) => ArraySource::Local(*slot),
            AbstractValue::OwnData { source, read } => {
                self.own_data_reads[*read] = true;
                ArraySource::OwnData(source.clone())
            }
            AbstractValue::Number(_)
            | AbstractValue::Key(_)
            | AbstractValue::DirectThis
            | AbstractValue::MathObject
            | AbstractValue::MathRoundFunction
            | AbstractValue::Other => return None,
        };
        if let Some(receiver) = self
            .receiver_sources
            .iter()
            .position(|existing| existing == &source)
        {
            return Some(receiver);
        }
        if self.receiver_sources.len() >= MAX_DENSE_RECEIVERS {
            return None;
        }
        let receiver = self.receiver_sources.len();
        self.receiver_sources.push(source);
        Some(receiver)
    }

    fn translate(&mut self, op: &Op) -> Option<()> {
        match op {
            Op::LoadLocal(slot) => self.stack.push(
                self.locals
                    .get(slot)
                    .cloned()
                    .unwrap_or(AbstractValue::Local(*slot)),
            ),
            Op::LoadGlobal(name) if name == "this" && !self.bytecode.is_global_scope() => {
                self.stack.push(AbstractValue::DirectThis);
            }
            Op::LoadGlobal(name) if name == "Math" => {
                self.stack.push(AbstractValue::MathObject);
            }
            Op::LoadConst(index) => match self.bytecode.constants.get(*index)? {
                Value::Number(value) => {
                    let register = self.emit(NumberInstruction::Constant(*value))?;
                    self.stack.push(AbstractValue::Number(register));
                }
                _ => self.stack.push(AbstractValue::Other),
            },
            Op::Dup => self.stack.push(self.stack.last()?.clone()),
            Op::Pop => {
                if matches!(
                    self.pop()?,
                    AbstractValue::DirectThis
                        | AbstractValue::MathObject
                        | AbstractValue::MathRoundFunction
                ) {
                    return None;
                }
            }
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                let mut value = self.pop()?;
                self.written_slots.insert(*slot);
                let register = match &value {
                    AbstractValue::Number(register) | AbstractValue::Key(register) => {
                        Some(*register)
                    }
                    AbstractValue::Local(_)
                        if !self.bytecode.local_is_compiler_temporary(*slot) =>
                    {
                        let register = self.number(value.clone())?;
                        value = AbstractValue::Number(register);
                        Some(register)
                    }
                    AbstractValue::OwnData { .. }
                        if self.bytecode.local_is_compiler_temporary(*slot) =>
                    {
                        None
                    }
                    AbstractValue::DirectThis
                    | AbstractValue::OwnData { .. }
                    | AbstractValue::MathObject
                    | AbstractValue::MathRoundFunction => return None,
                    AbstractValue::Other if !self.bytecode.local_is_compiler_temporary(*slot) => {
                        return None;
                    }
                    AbstractValue::Local(_) | AbstractValue::Other => None,
                };
                if let Some(register) = register {
                    if let Some(write) = self.writes.iter_mut().find(|write| write.local == *slot) {
                        write.value = register;
                    } else if self.writes.len() >= MAX_DENSE_WRITES {
                        return None;
                    } else {
                        self.writes.push(LocalWrite {
                            local: *slot,
                            value: register,
                        });
                    }
                }
                self.locals.insert(*slot, value);
            }
            Op::RequireObjectCoercible => {
                if matches!(self.stack.last(), Some(AbstractValue::Other) | None) {
                    return None;
                }
            }
            Op::ToNumeric => {
                let value = self.pop()?;
                let register = self.number(value)?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::ToPropertyKeyForAccess => {
                let value = self.pop()?;
                let register = self.number(value)?;
                self.stack.push(AbstractValue::Key(register));
            }
            Op::Binary(operation) if supported_binary(*operation) => {
                let right = self.pop()?;
                let left = self.pop()?;
                let right = self.number(right)?;
                let left = self.number(left)?;
                let register = self.emit(NumberInstruction::Binary {
                    operation: *operation,
                    left,
                    right,
                })?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::Unary(operation)
                if matches!(
                    operation,
                    UnaryOp::Plus | UnaryOp::Minus | UnaryOp::BitwiseNot
                ) =>
            {
                let value = self.pop()?;
                let value = self.number(value)?;
                let register = self.emit(NumberInstruction::Unary {
                    operation: *operation,
                    value,
                })?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::Update(operation) => {
                let value = self.pop()?;
                let value = self.number(value)?;
                let register = self.emit(NumberInstruction::Update {
                    operation: *operation,
                    value,
                })?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::GetPropNamed { key, cache } => {
                let value = if let Some(slot) = cache.local_slot() {
                    self.own_data(OwnDataSource {
                        owner: OwnDataOwner::Local(slot),
                        key: key.clone(),
                    })
                } else {
                    match self.pop()? {
                        AbstractValue::DirectThis => self.own_data(OwnDataSource {
                            owner: OwnDataOwner::DirectThis,
                            key: key.clone(),
                        }),
                        AbstractValue::MathObject if key.as_ref() == "round" => {
                            AbstractValue::MathRoundFunction
                        }
                        _ => return None,
                    }
                };
                self.stack.push(value);
            }
            Op::CallResolved(1) => {
                let argument = self.pop()?;
                let callee = self.pop()?;
                let receiver = self.pop()?;
                if !matches!(callee, AbstractValue::MathRoundFunction)
                    || !matches!(receiver, AbstractValue::MathObject)
                {
                    return None;
                }
                let value = self.number(argument)?;
                let register = self.emit(NumberInstruction::MathRound { value })?;
                self.uses_math_round = true;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::GetProp => {
                let key = self.pop()?;
                let object = self.pop()?;
                let index = self.number(key)?;
                let receiver = self.receiver(&object)?;
                let register = self.emit(NumberInstruction::DenseLoad { receiver, index })?;
                self.stack.push(AbstractValue::Number(register));
            }
            Op::SetProp { .. } => {
                if self.store_count >= MAX_DENSE_STORES {
                    return None;
                }
                let value = self.pop()?;
                let key = self.pop()?;
                let object = self.pop()?;
                let value = self.number(value)?;
                let index = self.number(key)?;
                let receiver = self.receiver(&object)?;
                self.emit(NumberInstruction::DenseStore {
                    receiver,
                    index,
                    value,
                })?;
                self.store_count += 1;
                self.stack.push(AbstractValue::Number(value));
            }
            _ => return None,
        }
        Some(())
    }
}

fn instruction_uses_register(operation: &NumberInstruction, target: Register) -> bool {
    match operation {
        NumberInstruction::Constant(_)
        | NumberInstruction::LoadLocal(_)
        | NumberInstruction::LoadInvariant(_) => false,
        NumberInstruction::DenseLoad { index, .. } => *index == target,
        NumberInstruction::DenseStore { index, value, .. }
        | NumberInstruction::Binary {
            left: index,
            right: value,
            ..
        } => *index == target || *value == target,
        NumberInstruction::Unary { value, .. }
        | NumberInstruction::Update { value, .. }
        | NumberInstruction::MathRound { value } => *value == target,
    }
}

fn remap_removed_register(register: &mut Register, removed: Register) -> bool {
    if *register == removed {
        return false;
    }
    if *register > removed {
        *register -= 1;
    }
    true
}

fn remap_instruction_registers(operation: &mut NumberInstruction, removed: Register) -> bool {
    match operation {
        NumberInstruction::Constant(_)
        | NumberInstruction::LoadLocal(_)
        | NumberInstruction::LoadInvariant(_) => true,
        NumberInstruction::DenseLoad { index, .. } => remap_removed_register(index, removed),
        NumberInstruction::DenseStore { index, value, .. }
        | NumberInstruction::Binary {
            left: index,
            right: value,
            ..
        } => remap_removed_register(index, removed) && remap_removed_register(value, removed),
        NumberInstruction::Unary { value, .. }
        | NumberInstruction::Update { value, .. }
        | NumberInstruction::MathRound { value } => remap_removed_register(value, removed),
    }
}

fn sink_unique_store(
    operations: &mut Vec<NumberInstruction>,
    writes: &mut [LocalWrite],
    store_count: usize,
) -> Option<Option<SunkDenseStore>> {
    if store_count != 1 {
        return Some(None);
    }
    let (position, mut store) =
        operations
            .iter()
            .enumerate()
            .find_map(|(position, operation)| match operation {
                NumberInstruction::DenseStore {
                    receiver,
                    index,
                    value,
                } => Some((
                    position,
                    SunkDenseStore {
                        receiver: *receiver,
                        index: *index,
                        value: *value,
                    },
                )),
                _ => None,
            })?;
    if operations[position + 1..].iter().any(|operation| {
        matches!(
            operation,
            NumberInstruction::DenseLoad { .. } | NumberInstruction::DenseStore { .. }
        )
    }) {
        return Some(None);
    }

    let store_result_is_used = operations.iter().enumerate().any(|(index, operation)| {
        index != position && instruction_uses_register(operation, position)
    }) || writes.iter().any(|write| write.value == position);
    debug_assert!(
        !store_result_is_used,
        "SetProp keeps its input value as the expression result"
    );
    if store_result_is_used {
        return None;
    }

    operations.remove(position);
    if !operations
        .iter_mut()
        .all(|operation| remap_instruction_registers(operation, position))
        || !writes
            .iter_mut()
            .all(|write| remap_removed_register(&mut write.value, position))
        || !remap_removed_register(&mut store.index, position)
        || !remap_removed_register(&mut store.value, position)
    {
        return None;
    }
    Some(Some(store))
}

pub(super) fn compile_dynamic(
    bytecode: &Bytecode,
    header: usize,
    backedge: usize,
) -> Option<(usize, DynamicDensePlan)> {
    let code = &bytecode.code;
    let Op::LoadLocal(counter_slot) = code.get(header)? else {
        return None;
    };
    let less_than_header = (|| {
        let (limit, cursor) = match code.get(header + 1)? {
            Op::LoadLocal(slot) => (DynamicLimit::LocalNumber(*slot), header + 2),
            Op::GetPropNamed { key, cache } if key.as_ref() == "length" => (
                DynamicLimit::LocalArrayLength(cache.local_slot()?),
                header + 2,
            ),
            Op::GetPropNamed { key, cache } => (
                DynamicLimit::OwnDataNumber(OwnDataSource {
                    owner: OwnDataOwner::Local(cache.local_slot()?),
                    key: key.clone(),
                }),
                header + 2,
            ),
            Op::LoadGlobal(name)
                if name == "this"
                    && !bytecode.is_global_scope()
                    && matches!(
                        code.get(header + 2),
                        Some(Op::GetPropNamed { cache, .. }) if cache.local_slot().is_none()
                    ) =>
            {
                let Op::GetPropNamed { key, .. } = code.get(header + 2)? else {
                    unreachable!("guarded direct-this named property read");
                };
                (
                    DynamicLimit::OwnDataNumber(OwnDataSource {
                        owner: OwnDataOwner::DirectThis,
                        key: key.clone(),
                    }),
                    header + 3,
                )
            }
            _ => return None,
        };
        let (Op::Binary(BinaryOp::Lt), Op::JumpIfFalse(exit), Op::Pop) = (
            code.get(cursor)?,
            code.get(cursor + 1)?,
            code.get(cursor + 2)?,
        ) else {
            return None;
        };
        Some((DynamicControl::LessThan(limit), cursor + 3, *exit))
    })();
    let at_least_zero_header = (|| {
        let Op::LoadConst(zero_constant) = code.get(header + 1)? else {
            return None;
        };
        let Some(Value::Number(zero)) = bytecode.constants.get(*zero_constant) else {
            return None;
        };
        if *zero != 0.0 {
            return None;
        }
        let comparison = if matches!(code.get(header + 2), Some(Op::Unary(UnaryOp::Minus))) {
            header + 3
        } else {
            header + 2
        };
        let (Op::Binary(BinaryOp::Ge), Op::JumpIfFalse(exit), Op::Pop) = (
            code.get(comparison)?,
            code.get(comparison + 1)?,
            code.get(comparison + 2)?,
        ) else {
            return None;
        };

        let tail = backedge.checked_sub(6)?;
        let (
            Op::LoadLocal(tail_counter_slot),
            Op::ToNumeric,
            Op::Dup,
            Op::Update(UpdateOp::Decrement),
            Op::AssignLocal(assigned_counter_slot),
            Op::Pop,
        ) = (
            code.get(tail)?,
            code.get(tail + 1)?,
            code.get(tail + 2)?,
            code.get(tail + 3)?,
            code.get(tail + 4)?,
            code.get(tail + 5)?,
        )
        else {
            return None;
        };
        let body_start = comparison + 3;
        if tail < body_start
            || tail_counter_slot != counter_slot
            || assigned_counter_slot != counter_slot
            || code[body_start..tail].iter().any(|op| {
                matches!(
                    op,
                    Op::StoreLocal(slot) | Op::AssignLocal(slot) if slot == counter_slot
                )
            })
        {
            return None;
        }
        Some((DynamicControl::AtLeastZero, body_start, *exit))
    })();
    let countdown_header = (|| {
        let (
            Op::ToNumeric,
            Op::Dup,
            Op::Update(UpdateOp::Decrement),
            Op::AssignLocal(assigned_counter_slot),
            Op::JumpIfFalse(exit),
            Op::Pop,
        ) = (
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
        if assigned_counter_slot != counter_slot {
            return None;
        }
        Some((DynamicControl::Countdown, header + 7, *exit))
    })();
    let (control, body_start, exit) = less_than_header
        .or(at_least_zero_header)
        .or(countdown_header)?;
    if exit <= backedge
        || !matches!(code.get(exit), Some(Op::Pop))
        || !matches!(code.get(backedge), Some(Op::Jump(target)) if *target == header)
    {
        return None;
    }

    let mut translator = Translator::new(bytecode);
    for op in &code[body_start..backedge] {
        translator.translate(op)?;
    }
    if !translator.stack.is_empty()
        || translator.own_data_reads.iter().any(|used| !used)
        || !translator
            .operations
            .iter()
            .any(|operation| matches!(operation, NumberInstruction::DenseLoad { .. }))
    {
        return None;
    }
    let limit = control.limit();
    let limit_slot = limit.and_then(DynamicLimit::required_slot);
    let limit_number_slot = limit.and_then(DynamicLimit::number_slot);
    let writes_array_length_source =
        limit.and_then(DynamicLimit::array_length_slot).is_some_and(|slot| {
            translator.operations.iter().any(|operation| {
                let NumberInstruction::DenseStore { receiver, .. } = operation else {
                    return false;
                };
                matches!(translator.receiver_sources.get(*receiver), Some(ArraySource::Local(receiver)) if *receiver == slot)
            })
        });
    if limit_slot.is_some_and(|limit_slot| counter_slot == &limit_slot)
        || translator.receiver_sources.iter().any(|receiver| {
            receiver.local_slot().is_some_and(|receiver| {
                counter_slot == &receiver
                    || limit_number_slot == Some(receiver)
                    || translator.number_slots.contains(&receiver)
                    || translator.written_slots.contains(&receiver)
            })
        })
        || limit_slot.is_some_and(|slot| translator.written_slots.contains(&slot))
        || limit_slot.is_some_and(|slot| translator.number_slots.contains(&slot))
        || translator.number_sources.iter().any(|source| {
            source.owner.local_slot().is_some_and(|slot| {
                counter_slot == &slot
                    || limit_number_slot == Some(slot)
                    || translator.number_slots.contains(&slot)
                    || translator.written_slots.contains(&slot)
            })
        })
        || writes_array_length_source
        || match control {
            DynamicControl::LessThan(_) | DynamicControl::AtLeastZero => {
                !translator.written_slots.contains(counter_slot)
            }
            DynamicControl::Countdown => translator.written_slots.contains(counter_slot),
        }
    {
        return None;
    }
    let sunk_store = sink_unique_store(
        &mut translator.operations,
        &mut translator.writes,
        translator.store_count,
    )?;
    translator.number_slots.insert(*counter_slot);
    if let Some(limit_slot) = limit_number_slot {
        translator.number_slots.insert(limit_slot);
    }
    let mut local_slots = translator.number_slots;
    local_slots.extend(translator.writes.iter().map(|write| write.local));
    let local_slots: Vec<_> = local_slots.into_iter().collect();
    if local_slots.len() > MAX_DENSE_LOCALS {
        return None;
    }
    let local_index = |slot| local_slots.binary_search(&slot).ok();
    for operation in &mut translator.operations {
        if let NumberInstruction::LoadLocal(slot) = operation {
            *slot = local_index(*slot)?;
        }
    }
    for write in &mut translator.writes {
        write.local = local_index(write.local)?;
    }

    Some((
        exit,
        DynamicDensePlan {
            counter_local: local_index(*counter_slot)?,
            control: match control {
                DynamicControl::LessThan(DynamicLimit::LocalNumber(slot)) => {
                    DynamicControl::LessThan(DynamicLimit::LocalNumber(local_index(slot)?))
                }
                DynamicControl::LessThan(DynamicLimit::LocalArrayLength(slot)) => {
                    DynamicControl::LessThan(DynamicLimit::LocalArrayLength(slot))
                }
                DynamicControl::LessThan(DynamicLimit::OwnDataNumber(source)) => {
                    DynamicControl::LessThan(DynamicLimit::OwnDataNumber(source))
                }
                DynamicControl::AtLeastZero => DynamicControl::AtLeastZero,
                DynamicControl::Countdown => DynamicControl::Countdown,
            },
            receiver_sources: translator.receiver_sources,
            number_sources: translator.number_sources,
            local_slots,
            operations: translator.operations,
            writes: translator.writes,
            store_count: translator.store_count,
            sunk_store,
            uses_math_round: translator.uses_math_round,
            header,
        },
    ))
}

fn supported_binary(operation: BinaryOp) -> bool {
    matches!(
        operation,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseXor
            | BinaryOp::BitwiseOr
    )
}
