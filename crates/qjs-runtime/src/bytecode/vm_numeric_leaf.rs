use qjs_ast::{BinaryOp, BindingPattern, FunctionParams, UpdateOp};

use crate::{Value, function::Upvalue};

use super::{
    ir::{Bytecode, Op},
    vm_props::fast_number_binary,
};

const MAX_FAST_LOCALS: usize = 32;
const MAX_FAST_STACK: usize = 64;
const NO_UPVALUE: usize = usize::MAX;

#[derive(Clone, Copy)]
enum FastValue {
    Uninitialized,
    Undefined,
    Number(f64),
    Boolean(bool),
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
    if bytecode.locals.len() > MAX_FAST_LOCALS
        || bytecode
            .locals
            .iter()
            .any(|local| local.sloppy_global_fallback)
    {
        return None;
    }

    let mut locals = [FastValue::Uninitialized; MAX_FAST_LOCALS];
    for (slot, local) in bytecode.locals.iter().enumerate() {
        if local.hoisted {
            locals[slot] = FastValue::Undefined;
        }
    }

    for (index, element) in params.positional.iter().enumerate() {
        let BindingPattern::Identifier { name, .. } = &element.binding else {
            return None;
        };
        let slot = local_slot(bytecode, name)?;
        locals[slot] = match arguments.get(index) {
            Some(value) => FastValue::from_value(value)?,
            None => FastValue::Undefined,
        };
    }

    let mut upvalue_slots = [NO_UPVALUE; MAX_FAST_LOCALS];
    let mut received_count = 0;
    for (name, upvalue) in bytecode.received_upvalue_names().zip(upvalues) {
        let slot = local_slot(bytecode, name)?;
        locals[slot] = FastValue::from_value(&upvalue.get())?;
        upvalue_slots[slot] = received_count;
        received_count += 1;
    }
    if received_count != upvalues.len() {
        return None;
    }

    let mut assigned_upvalues = [false; MAX_FAST_LOCALS];
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
                if upvalue_slots[*slot] != NO_UPVALUE {
                    assigned_upvalues[*slot] = true;
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
                for slot in 0..bytecode.locals.len() {
                    if !assigned_upvalues[slot] {
                        continue;
                    }
                    let index = upvalue_slots[slot];
                    upvalues.get(index)?.set(locals[slot].into_value()?);
                }
                return value.into_value();
            }
            _ => return None,
        }
    }
    None
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

fn local_slot(bytecode: &Bytecode, name: &str) -> Option<usize> {
    bytecode.locals.iter().position(|local| local.name == name)
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
