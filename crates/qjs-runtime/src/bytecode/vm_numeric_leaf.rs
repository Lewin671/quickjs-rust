use qjs_ast::{BinaryOp, FunctionParams, UpdateOp};

use crate::{Value, function::Upvalue};

use super::{
    ir::{Bytecode, Op},
    vm_props::fast_number_binary,
};

const MAX_FAST_LOCALS: usize = 32;
const MAX_FAST_STACK: usize = 16;

#[derive(Clone, Copy, Debug)]
enum FastValue {
    Uninitialized,
    Undefined,
    Number(f64),
    Boolean(bool),
}

#[derive(Clone, Copy, Debug)]
enum AbstractValue {
    Known(FastValue),
    Materialized,
}

#[derive(Clone, Debug)]
enum FastOp {
    LoadConst(FastValue),
    LoadLocal(usize),
    LoadLocalOrUndefined(usize),
    StoreLocal {
        slot: usize,
        upvalue_index: Option<usize>,
    },
    Dup,
    Pop,
    ToNumeric,
    Update(UpdateOp),
    Binary(BinaryOp),
    BinaryConstRight(BinaryOp, f64),
    UpdateUpvalueConstReturn {
        slot: usize,
        upvalue_index: usize,
        op: BinaryOp,
        right: f64,
    },
    Return,
    ReturnConst(FastValue),
}

/// Compact, prevalidated form of the straight-line numeric bytecode subset.
///
/// Besides avoiding the general `Op` representation in every call, building
/// the plan propagates primitive constants through ordinary local slots. This
/// removes repeated `var local = <number>` setup and turns a dynamic value
/// followed by a constant binary operand into one immediate micro-op.
#[derive(Clone, Debug)]
pub(super) struct NumericLeafPlan {
    ops: Vec<FastOp>,
    hoisted_slots: u32,
    writes_received_upvalues: bool,
}

impl NumericLeafPlan {
    fn compile(bytecode: &Bytecode) -> Option<Self> {
        if bytecode.locals.len() > MAX_FAST_LOCALS
            || bytecode
                .locals
                .iter()
                .any(|local| local.sloppy_global_fallback)
        {
            return None;
        }

        let mut locals = [AbstractValue::Known(FastValue::Uninitialized); MAX_FAST_LOCALS];
        let mut hoisted_slots = 0_u32;
        for (slot, local) in bytecode.locals.iter().enumerate() {
            if local.hoisted {
                locals[slot] = AbstractValue::Known(FastValue::Undefined);
                hoisted_slots |= 1 << slot;
            }
        }
        for &slot in bytecode.parameter_slots() {
            *locals.get_mut(slot)? = AbstractValue::Materialized;
        }
        for &slot in bytecode.received_upvalue_slots() {
            *locals.get_mut(slot)? = AbstractValue::Materialized;
        }

        let mut ops = Vec::with_capacity(bytecode.code.len());
        let mut stack = Vec::with_capacity(MAX_FAST_STACK);
        let mut writes_received_upvalues = false;
        for op in &bytecode.code {
            match op {
                Op::FunctionPrologueEnd => {}
                Op::LoadConst(index) => {
                    let value = FastValue::from_value(bytecode.constants.get(*index)?)?;
                    push_abstract(&mut stack, AbstractValue::Known(value))?;
                }
                Op::LoadLocal(slot) => match *locals.get(*slot)? {
                    AbstractValue::Known(FastValue::Uninitialized) => return None,
                    AbstractValue::Known(value) => {
                        push_abstract(&mut stack, AbstractValue::Known(value))?;
                    }
                    AbstractValue::Materialized => {
                        materialize_deferred(&mut stack, &mut ops)?;
                        ops.push(FastOp::LoadLocal(*slot));
                        push_abstract(&mut stack, AbstractValue::Materialized)?;
                    }
                },
                Op::LoadLocalOrUndefined(slot) => match *locals.get(*slot)? {
                    AbstractValue::Known(FastValue::Uninitialized) => {
                        push_abstract(&mut stack, AbstractValue::Known(FastValue::Undefined))?;
                    }
                    AbstractValue::Known(value) => {
                        push_abstract(&mut stack, AbstractValue::Known(value))?;
                    }
                    AbstractValue::Materialized => {
                        materialize_deferred(&mut stack, &mut ops)?;
                        ops.push(FastOp::LoadLocalOrUndefined(*slot));
                        push_abstract(&mut stack, AbstractValue::Materialized)?;
                    }
                },
                Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                    if !bytecode.local_is_mutable(*slot) {
                        return None;
                    }
                    let upvalue_index = bytecode
                        .received_upvalue_slots()
                        .iter()
                        .position(|received_slot| received_slot == slot);
                    if upvalue_index.is_some() {
                        writes_received_upvalues = true;
                        materialize_deferred(&mut stack, &mut ops)?;
                        stack.pop()?;
                        ops.push(FastOp::StoreLocal {
                            slot: *slot,
                            upvalue_index,
                        });
                        *locals.get_mut(*slot)? = AbstractValue::Materialized;
                    } else {
                        match stack.pop()? {
                            AbstractValue::Known(value) => {
                                *locals.get_mut(*slot)? = AbstractValue::Known(value);
                            }
                            AbstractValue::Materialized => {
                                ops.push(FastOp::StoreLocal {
                                    slot: *slot,
                                    upvalue_index: None,
                                });
                                *locals.get_mut(*slot)? = AbstractValue::Materialized;
                            }
                        }
                    }
                }
                Op::Dup => match *stack.last()? {
                    AbstractValue::Known(value) => {
                        push_abstract(&mut stack, AbstractValue::Known(value))?;
                    }
                    AbstractValue::Materialized => {
                        ops.push(FastOp::Dup);
                        push_abstract(&mut stack, AbstractValue::Materialized)?;
                    }
                },
                Op::Pop => match stack.pop()? {
                    AbstractValue::Known(_) => {}
                    AbstractValue::Materialized => ops.push(FastOp::Pop),
                },
                Op::ToNumeric => match *stack.last()? {
                    AbstractValue::Known(FastValue::Number(_)) => {}
                    AbstractValue::Known(_) => return None,
                    AbstractValue::Materialized => ops.push(FastOp::ToNumeric),
                },
                Op::Update(update) => match stack.last_mut()? {
                    AbstractValue::Known(FastValue::Number(value)) => {
                        *value = match update {
                            UpdateOp::Increment => *value + 1.0,
                            UpdateOp::Decrement => *value - 1.0,
                        };
                    }
                    AbstractValue::Known(_) => return None,
                    AbstractValue::Materialized => ops.push(FastOp::Update(*update)),
                },
                Op::Binary(binary) => {
                    let len = stack.len();
                    let left = *stack.get(len.checked_sub(2)?)?;
                    let right = *stack.last()?;
                    match (left, right) {
                        (
                            AbstractValue::Known(FastValue::Number(left)),
                            AbstractValue::Known(FastValue::Number(right)),
                        ) => {
                            stack.truncate(len - 2);
                            let value = direct_number_binary(left, *binary, right)?;
                            push_abstract(&mut stack, AbstractValue::Known(value))?;
                        }
                        (
                            AbstractValue::Materialized,
                            AbstractValue::Known(FastValue::Number(right)),
                        ) => {
                            stack.truncate(len - 2);
                            ops.push(FastOp::BinaryConstRight(*binary, right));
                            push_abstract(&mut stack, AbstractValue::Materialized)?;
                        }
                        _ => {
                            materialize_deferred(&mut stack, &mut ops)?;
                            stack.truncate(len - 2);
                            ops.push(FastOp::Binary(*binary));
                            push_abstract(&mut stack, AbstractValue::Materialized)?;
                        }
                    }
                }
                Op::Return => {
                    let value = if stack.is_empty() {
                        AbstractValue::Known(FastValue::Undefined)
                    } else {
                        stack.pop()?
                    };
                    match value {
                        AbstractValue::Known(value) => ops.push(FastOp::ReturnConst(value)),
                        AbstractValue::Materialized => ops.push(FastOp::Return),
                    }
                    if compact_terminal_upvalue_update(&mut ops) {
                        writes_received_upvalues = false;
                    }
                    return Some(Self {
                        ops,
                        hoisted_slots,
                        writes_received_upvalues,
                    });
                }
                _ => return None,
            }
        }
        None
    }
}

impl FastValue {
    fn from_value(value: &Value) -> Option<Self> {
        match value {
            Value::Undefined => Some(Self::Undefined),
            Value::Number(value) => Some(Self::Number(*value)),
            Value::Boolean(value) => Some(Self::Boolean(*value)),
            _ => None,
        }
    }

    fn into_value(self) -> Option<Value> {
        match self {
            Self::Uninitialized => None,
            Self::Undefined => Some(Value::Undefined),
            Self::Number(value) => Some(Value::Number(value)),
            Self::Boolean(value) => Some(Value::Boolean(value)),
        }
    }
}

/// Executes a side-effect-free numeric leaf without constructing a nested VM.
///
/// The fixed-size scratch frame admits only local loads/stores and primitive
/// numeric operations. Received upvalue writes are delayed until a supported
/// `Return`, so an unsupported value or opcode can fall back to the full VM
/// without duplicating observable work.
pub(crate) fn try_eval_numeric_leaf(
    bytecode: &Bytecode,
    params: &FunctionParams,
    arguments: &[Value],
    upvalues: &[Upvalue],
) -> Option<Value> {
    let plan = bytecode
        .numeric_leaf_plan
        .get_or_init(|| NumericLeafPlan::compile(bytecode))
        .as_ref()?;
    if plan.writes_received_upvalues {
        return try_eval_numeric_leaf_bytecode(bytecode, params, arguments, upvalues);
    }

    let mut locals = [FastValue::Uninitialized; MAX_FAST_LOCALS];
    let mut hoisted_slots = plan.hoisted_slots;
    while hoisted_slots != 0 {
        let slot = hoisted_slots.trailing_zeros() as usize;
        locals[slot] = FastValue::Undefined;
        hoisted_slots &= hoisted_slots - 1;
    }

    if bytecode.parameter_slots().len() != params.positional.len() {
        return None;
    }
    for (index, &slot) in bytecode.parameter_slots().iter().enumerate() {
        locals[slot] = match arguments.get(index) {
            Some(value) => FastValue::from_value(value)?,
            None => FastValue::Undefined,
        };
    }

    let received_upvalue_slots = bytecode.received_upvalue_slots();
    if received_upvalue_slots.len() != upvalues.len() {
        return None;
    }
    for (&slot, upvalue) in received_upvalue_slots.iter().zip(upvalues) {
        locals[slot] = FastValue::from_value(&upvalue.get())?;
    }

    let mut assigned_upvalues = 0_u32;
    let mut stack = [FastValue::Uninitialized; MAX_FAST_STACK];
    let mut stack_len = 0;

    for op in &plan.ops {
        match op {
            FastOp::LoadConst(value) => push(&mut stack, &mut stack_len, *value)?,
            FastOp::LoadLocal(slot) => {
                let value = *locals.get(*slot)?;
                if matches!(value, FastValue::Uninitialized) {
                    return None;
                }
                push(&mut stack, &mut stack_len, value)?;
            }
            FastOp::LoadLocalOrUndefined(slot) => {
                let value = match *locals.get(*slot)? {
                    FastValue::Uninitialized => FastValue::Undefined,
                    value => value,
                };
                push(&mut stack, &mut stack_len, value)?;
            }
            FastOp::StoreLocal {
                slot,
                upvalue_index,
            } => {
                let value = pop(&stack, &mut stack_len)?;
                *locals.get_mut(*slot)? = value;
                if let Some(index) = upvalue_index {
                    assigned_upvalues |= 1 << index;
                }
            }
            FastOp::Dup => {
                let value = *stack.get(stack_len.checked_sub(1)?)?;
                push(&mut stack, &mut stack_len, value)?;
            }
            FastOp::Pop => {
                pop(&stack, &mut stack_len)?;
            }
            FastOp::ToNumeric => {
                if !matches!(stack.get(stack_len.checked_sub(1)?)?, FastValue::Number(_)) {
                    return None;
                }
            }
            FastOp::Update(op) => {
                let value = match pop(&stack, &mut stack_len)? {
                    FastValue::Number(value) => match op {
                        UpdateOp::Increment => FastValue::Number(value + 1.0),
                        UpdateOp::Decrement => FastValue::Number(value - 1.0),
                    },
                    _ => return None,
                };
                push(&mut stack, &mut stack_len, value)?;
            }
            FastOp::Binary(op) => {
                let right = pop(&stack, &mut stack_len)?;
                let left = pop(&stack, &mut stack_len)?;
                let (FastValue::Number(left), FastValue::Number(right)) = (left, right) else {
                    return None;
                };
                let value = direct_number_binary(left, *op, right)?;
                push(&mut stack, &mut stack_len, value)?;
            }
            FastOp::BinaryConstRight(op, right) => {
                let FastValue::Number(left) = pop(&stack, &mut stack_len)? else {
                    return None;
                };
                let value = direct_number_binary(left, *op, *right)?;
                push(&mut stack, &mut stack_len, value)?;
            }
            FastOp::UpdateUpvalueConstReturn {
                slot,
                upvalue_index,
                op,
                right,
            } => {
                let FastValue::Number(left) = *locals.get(*slot)? else {
                    return None;
                };
                let value = direct_number_binary(left, *op, *right)?;
                let value = value.into_value()?;
                upvalues.get(*upvalue_index)?.set(value.clone());
                return Some(value);
            }
            FastOp::Return => {
                let value = pop(&stack, &mut stack_len)?;
                commit_upvalues(received_upvalue_slots, upvalues, &locals, assigned_upvalues)?;
                return value.into_value();
            }
            FastOp::ReturnConst(value) => {
                commit_upvalues(received_upvalue_slots, upvalues, &locals, assigned_upvalues)?;
                return value.into_value();
            }
        }
    }
    None
}

/// Original direct executor retained for received-upvalue writes. Its delayed
/// write mask is faster for that narrow stateful shape than the compact plan,
/// while preserving transactional fallback after an unsupported later op.
fn try_eval_numeric_leaf_bytecode(
    bytecode: &Bytecode,
    params: &FunctionParams,
    arguments: &[Value],
    upvalues: &[Upvalue],
) -> Option<Value> {
    let mut locals = [FastValue::Uninitialized; MAX_FAST_LOCALS];
    for (slot, local) in bytecode.locals.iter().enumerate() {
        if local.hoisted {
            locals[slot] = FastValue::Undefined;
        }
    }
    if bytecode.parameter_slots().len() != params.positional.len() {
        return None;
    }
    for (index, &slot) in bytecode.parameter_slots().iter().enumerate() {
        locals[slot] = match arguments.get(index) {
            Some(value) => FastValue::from_value(value)?,
            None => FastValue::Undefined,
        };
    }
    let received_upvalue_slots = bytecode.received_upvalue_slots();
    if received_upvalue_slots.len() != upvalues.len() {
        return None;
    }
    for (&slot, upvalue) in received_upvalue_slots.iter().zip(upvalues) {
        locals[slot] = FastValue::from_value(&upvalue.get())?;
    }

    let mut assigned_upvalues = 0_u32;
    let mut stack = [FastValue::Uninitialized; MAX_FAST_STACK];
    let mut stack_len = 0;
    for op in &bytecode.code {
        match op {
            Op::FunctionPrologueEnd => {}
            Op::LoadConst(index) => {
                let value = FastValue::from_value(bytecode.constants.get(*index)?)?;
                push(&mut stack, &mut stack_len, value)?;
            }
            Op::LoadLocal(slot) => {
                let value = *locals.get(*slot)?;
                if matches!(value, FastValue::Uninitialized) {
                    return None;
                }
                push(&mut stack, &mut stack_len, value)?;
            }
            Op::LoadLocalOrUndefined(slot) => {
                let value = match *locals.get(*slot)? {
                    FastValue::Uninitialized => FastValue::Undefined,
                    value => value,
                };
                push(&mut stack, &mut stack_len, value)?;
            }
            Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                if !bytecode.local_is_mutable(*slot) {
                    return None;
                }
                let value = pop(&stack, &mut stack_len)?;
                *locals.get_mut(*slot)? = value;
                if let Some(index) = received_upvalue_slots
                    .iter()
                    .position(|received_slot| received_slot == slot)
                {
                    assigned_upvalues |= 1 << index;
                }
            }
            Op::Dup => {
                let value = *stack.get(stack_len.checked_sub(1)?)?;
                push(&mut stack, &mut stack_len, value)?;
            }
            Op::Pop => {
                pop(&stack, &mut stack_len)?;
            }
            Op::ToNumeric => {
                if !matches!(stack.get(stack_len.checked_sub(1)?)?, FastValue::Number(_)) {
                    return None;
                }
            }
            Op::Update(op) => {
                let value = match pop(&stack, &mut stack_len)? {
                    FastValue::Number(value) => match op {
                        UpdateOp::Increment => FastValue::Number(value + 1.0),
                        UpdateOp::Decrement => FastValue::Number(value - 1.0),
                    },
                    _ => return None,
                };
                push(&mut stack, &mut stack_len, value)?;
            }
            Op::Binary(op) => {
                let right = pop(&stack, &mut stack_len)?;
                let left = pop(&stack, &mut stack_len)?;
                let (FastValue::Number(left), FastValue::Number(right)) = (left, right) else {
                    return None;
                };
                let value = direct_number_binary(left, *op, right)?;
                push(&mut stack, &mut stack_len, value)?;
            }
            Op::Return => {
                let value = if stack_len == 0 {
                    FastValue::Undefined
                } else {
                    pop(&stack, &mut stack_len)?
                };
                commit_upvalues(received_upvalue_slots, upvalues, &locals, assigned_upvalues)?;
                return value.into_value();
            }
            _ => return None,
        }
    }
    None
}

fn push_abstract(stack: &mut Vec<AbstractValue>, value: AbstractValue) -> Option<()> {
    if stack.len() == MAX_FAST_STACK {
        return None;
    }
    stack.push(value);
    Some(())
}

fn compact_terminal_upvalue_update(ops: &mut Vec<FastOp>) -> bool {
    let [
        FastOp::LoadLocal(load_slot),
        FastOp::BinaryConstRight(op, right),
        FastOp::Dup,
        FastOp::StoreLocal {
            slot: store_slot,
            upvalue_index: Some(upvalue_index),
        },
        FastOp::LoadLocal(return_slot),
        FastOp::Return,
    ] = ops.as_slice()
    else {
        return false;
    };
    if load_slot != store_slot || load_slot != return_slot {
        return false;
    }
    let compact = FastOp::UpdateUpvalueConstReturn {
        slot: *load_slot,
        upvalue_index: *upvalue_index,
        op: *op,
        right: *right,
    };
    ops.clear();
    ops.push(compact);
    true
}

fn materialize_deferred(stack: &mut [AbstractValue], ops: &mut Vec<FastOp>) -> Option<()> {
    for value in stack {
        if let AbstractValue::Known(known) = value {
            if matches!(known, FastValue::Uninitialized) {
                return None;
            }
            ops.push(FastOp::LoadConst(*known));
            *value = AbstractValue::Materialized;
        }
    }
    Some(())
}

fn commit_upvalues(
    received_upvalue_slots: &[usize],
    upvalues: &[Upvalue],
    locals: &[FastValue; MAX_FAST_LOCALS],
    assigned_upvalues: u32,
) -> Option<()> {
    for (index, &slot) in received_upvalue_slots.iter().enumerate() {
        if assigned_upvalues & (1 << index) != 0 {
            upvalues.get(index)?.set(locals[slot].into_value()?);
        }
    }
    Some(())
}

fn push(
    stack: &mut [FastValue; MAX_FAST_STACK],
    stack_len: &mut usize,
    value: FastValue,
) -> Option<()> {
    *stack.get_mut(*stack_len)? = value;
    *stack_len += 1;
    Some(())
}

fn pop(stack: &[FastValue; MAX_FAST_STACK], stack_len: &mut usize) -> Option<FastValue> {
    *stack_len = stack_len.checked_sub(1)?;
    stack.get(*stack_len).copied()
}

fn direct_number_binary(left: f64, op: BinaryOp, right: f64) -> Option<FastValue> {
    let value = match op {
        BinaryOp::Add => FastValue::Number(left + right),
        BinaryOp::Sub => FastValue::Number(left - right),
        BinaryOp::Mul => FastValue::Number(left * right),
        BinaryOp::Div => FastValue::Number(left / right),
        BinaryOp::Rem => FastValue::Number(left % right),
        BinaryOp::Eq | BinaryOp::StrictEq => FastValue::Boolean(left == right),
        BinaryOp::Ne | BinaryOp::StrictNe => FastValue::Boolean(left != right),
        BinaryOp::Lt => FastValue::Boolean(left < right),
        BinaryOp::Le => FastValue::Boolean(left <= right),
        BinaryOp::Gt => FastValue::Boolean(left > right),
        BinaryOp::Ge => FastValue::Boolean(left >= right),
        _ => {
            let value = fast_number_binary(&Value::Number(left), op, &Value::Number(right))?;
            FastValue::from_value(&value)?
        }
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;

    #[test]
    fn plan_propagates_constant_locals_into_immediate_binary_ops() {
        let script = qjs_parser::parse_script(
            "function add(value) { var a = 1, b = 2, c = 3; return value + a + b + c; }",
        )
        .expect("source should parse");
        let script_bytecode = compiler::compile_script(&script).expect("source should compile");
        let function_bytecode = script_bytecode
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(bytecode),
                _ => None,
            })
            .expect("function bytecode should be nested in the script");
        let plan = NumericLeafPlan::compile(function_bytecode).expect("leaf should be admitted");

        assert_eq!(
            plan.ops
                .iter()
                .filter(|op| matches!(op, FastOp::BinaryConstRight(BinaryOp::Add, _)))
                .count(),
            3
        );
        assert!(
            plan.ops
                .iter()
                .all(|op| !matches!(op, FastOp::StoreLocal { .. })),
            "unexpected materialized setup in {:#?}",
            plan.ops
        );
    }

    #[test]
    fn captured_counter_plan_shape() {
        let script = qjs_parser::parse_script(
            "function make() { var captured = 0; return function() { captured += 1; return captured; }; }",
        )
        .expect("source should parse");
        let script_bytecode = compiler::compile_script(&script).expect("source should compile");
        let outer = script_bytecode
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(bytecode),
                _ => None,
            })
            .expect("outer function should be compiled");
        let inner = outer
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(bytecode),
                _ => None,
            })
            .expect("inner function should be compiled");
        let plan = NumericLeafPlan::compile(inner).expect("counter should be admitted");
        assert!(matches!(
            plan.ops.as_slice(),
            [FastOp::UpdateUpvalueConstReturn {
                slot: 0,
                upvalue_index: 0,
                op: BinaryOp::Add,
                right: 1.0,
            }]
        ));
    }
}
