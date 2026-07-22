use qjs_ast::{BinaryOp, FunctionParams, UpdateOp};

use crate::{
    Function, NativeFunction, Value,
    function::{Upvalue, is_direct_leaf_function},
};

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
    shortcut: Option<NumericLeafShortcut>,
    hoisted_slots: u32,
    writes_received_upvalues: bool,
}

#[derive(Clone, Debug)]
enum NumericLeafShortcut {
    ArgumentConstChain {
        argument_index: usize,
        operations: Vec<(BinaryOp, f64)>,
    },
    ArgumentUpvalueBinary {
        argument_index: usize,
        upvalue_index: usize,
        op: BinaryOp,
    },
    ArgumentArgumentBinary {
        left_argument_index: usize,
        right_argument_index: usize,
        op: BinaryOp,
    },
    UpvalueArgumentBinary {
        upvalue_index: usize,
        argument_index: usize,
        op: BinaryOp,
    },
    UpdateUpvalueConstReturn {
        upvalue_index: usize,
        op: BinaryOp,
        right: f64,
    },
}

/// A numeric leaf call reduced to scalar state for a counted-loop trace.
///
/// Read-only captures are snapshotted because the admitted loop body contains
/// no other observable operation. A compact captured update keeps its scalar
/// value locally and commits the shared cell after the loop; callers reject
/// cells owned by the active frame before constructing this plan.
#[derive(Clone, Debug)]
pub(super) enum NumericLoopCall {
    MathAbs,
    ArgumentAddConstants {
        argument_index: usize,
        constants: Vec<f64>,
    },
    ArgumentConstChain {
        argument_index: usize,
        operations: Vec<(BinaryOp, f64)>,
    },
    ArgumentCapturedBinary {
        argument_index: usize,
        captured: f64,
        op: BinaryOp,
        argument_left: bool,
    },
    ArgumentArgumentBinary {
        left_argument_index: usize,
        right_argument_index: usize,
        op: BinaryOp,
    },
    UpdateCapturedConstReturn {
        upvalue: Upvalue,
        value: f64,
        op: BinaryOp,
        right: f64,
    },
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
                    let shortcut = NumericLeafShortcut::compile(&ops, bytecode);
                    return Some(Self {
                        ops,
                        shortcut,
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

impl NumericLeafShortcut {
    fn compile(ops: &[FastOp], bytecode: &Bytecode) -> Option<Self> {
        // Function prologues can leave primitive constants below the eventual
        // return value. They are side-effect free and never consumed by these
        // terminal shapes, so exclude that dead prefix from recognition.
        let core = &ops[ops
            .iter()
            .position(|op| !matches!(op, FastOp::LoadConst(_)))?..];
        if let [FastOp::LoadLocal(slot), middle @ .., FastOp::Return] = core
            && let Some(argument_index) = parameter_index(bytecode, *slot)
            && !middle.is_empty()
            && middle
                .iter()
                .all(|op| matches!(op, FastOp::BinaryConstRight(_, _)))
        {
            let operations = middle
                .iter()
                .map(|op| match op {
                    FastOp::BinaryConstRight(op, right) => (*op, *right),
                    _ => unreachable!("guarded constant-chain operation"),
                })
                .collect();
            return Some(Self::ArgumentConstChain {
                argument_index,
                operations,
            });
        }
        if let [
            FastOp::LoadLocal(left),
            FastOp::LoadLocal(right),
            FastOp::Binary(op),
            FastOp::Return,
        ] = core
        {
            if let (Some(left_argument_index), Some(right_argument_index)) = (
                parameter_index(bytecode, *left),
                parameter_index(bytecode, *right),
            ) {
                return Some(Self::ArgumentArgumentBinary {
                    left_argument_index,
                    right_argument_index,
                    op: *op,
                });
            }
            if let (Some(argument_index), Some(upvalue_index)) = (
                parameter_index(bytecode, *left),
                upvalue_index(bytecode, *right),
            ) {
                return Some(Self::ArgumentUpvalueBinary {
                    argument_index,
                    upvalue_index,
                    op: *op,
                });
            }
            if let (Some(upvalue_index), Some(argument_index)) = (
                upvalue_index(bytecode, *left),
                parameter_index(bytecode, *right),
            ) {
                return Some(Self::UpvalueArgumentBinary {
                    upvalue_index,
                    argument_index,
                    op: *op,
                });
            }
        }
        if let [
            FastOp::UpdateUpvalueConstReturn {
                upvalue_index,
                op,
                right,
                ..
            },
        ] = core
        {
            return Some(Self::UpdateUpvalueConstReturn {
                upvalue_index: *upvalue_index,
                op: *op,
                right: *right,
            });
        }
        None
    }

    fn eval(&self, arguments: &[Value], upvalues: &[Upvalue]) -> Option<Value> {
        let argument_number = |index: usize| -> Option<f64> {
            match arguments.get(index)? {
                Value::Number(value) => Some(*value),
                _ => None,
            }
        };
        let upvalue_number = |index: usize| -> Option<f64> {
            upvalues.get(index)?.with_value(|value| match value {
                Value::Number(value) => Some(*value),
                _ => None,
            })
        };
        match self {
            Self::ArgumentConstChain {
                argument_index,
                operations,
            } => {
                let mut value = FastValue::Number(argument_number(*argument_index)?);
                for (op, right) in operations {
                    let FastValue::Number(left) = value else {
                        return None;
                    };
                    value = direct_number_binary(left, *op, *right)?;
                }
                value.into_value()
            }
            Self::ArgumentUpvalueBinary {
                argument_index,
                upvalue_index,
                op,
            } => direct_number_binary(
                argument_number(*argument_index)?,
                *op,
                upvalue_number(*upvalue_index)?,
            )?
            .into_value(),
            Self::ArgumentArgumentBinary {
                left_argument_index,
                right_argument_index,
                op,
            } => direct_number_binary(
                argument_number(*left_argument_index)?,
                *op,
                argument_number(*right_argument_index)?,
            )?
            .into_value(),
            Self::UpvalueArgumentBinary {
                upvalue_index,
                argument_index,
                op,
            } => direct_number_binary(
                upvalue_number(*upvalue_index)?,
                *op,
                argument_number(*argument_index)?,
            )?
            .into_value(),
            Self::UpdateUpvalueConstReturn {
                upvalue_index,
                op,
                right,
            } => {
                let value = direct_number_binary(upvalue_number(*upvalue_index)?, *op, *right)?
                    .into_value()?;
                upvalues.get(*upvalue_index)?.set(value.clone());
                Some(value)
            }
        }
    }
}

impl NumericLoopCall {
    pub(super) fn prepare(
        function: &Function,
        argument_count: usize,
        caller_cells: &[Option<Upvalue>],
        forbidden_cells: &[Upvalue],
    ) -> Option<Self> {
        if function.native == Some(NativeFunction::MathAbs) && argument_count >= 1 {
            return Some(Self::MathAbs);
        }
        if argument_count > 2 || !is_direct_leaf_function(&Value::Function(function.clone())) {
            return None;
        }
        let bytecode = function.bytecode.as_ref()?;
        if bytecode.parameter_slots().len() != function.params.positional.len()
            || bytecode.received_upvalue_slots().len() != function.upvalues.len()
            || function.upvalues.iter().any(|upvalue| {
                forbidden_cells
                    .iter()
                    .any(|forbidden| forbidden.ptr_eq(upvalue))
            })
        {
            return None;
        }
        let shortcut = bytecode
            .numeric_leaf_plan
            .get_or_init(|| NumericLeafPlan::compile(bytecode))
            .as_ref()?
            .shortcut
            .as_ref()?;
        let captured_number = |index: usize| -> Option<f64> {
            function
                .upvalues
                .get(index)?
                .with_value(|value| match value {
                    Value::Number(value) => Some(*value),
                    _ => None,
                })
        };
        match shortcut {
            NumericLeafShortcut::ArgumentConstChain {
                argument_index,
                operations,
            } if *argument_index < argument_count
                && operations
                    .iter()
                    .all(|(op, right)| number_binary(0.0, *op, *right).is_some()) =>
            {
                if operations.iter().all(|(op, _)| *op == BinaryOp::Add) {
                    Some(Self::ArgumentAddConstants {
                        argument_index: *argument_index,
                        constants: operations.iter().map(|(_, right)| *right).collect(),
                    })
                } else {
                    Some(Self::ArgumentConstChain {
                        argument_index: *argument_index,
                        operations: operations.clone(),
                    })
                }
            }
            NumericLeafShortcut::ArgumentUpvalueBinary {
                argument_index,
                upvalue_index,
                op,
            } if *argument_index < argument_count => {
                let captured = captured_number(*upvalue_index)?;
                number_binary(0.0, *op, captured)?;
                Some(Self::ArgumentCapturedBinary {
                    argument_index: *argument_index,
                    captured,
                    op: *op,
                    argument_left: true,
                })
            }
            NumericLeafShortcut::UpvalueArgumentBinary {
                upvalue_index,
                argument_index,
                op,
            } if *argument_index < argument_count => {
                let captured = captured_number(*upvalue_index)?;
                number_binary(captured, *op, 0.0)?;
                Some(Self::ArgumentCapturedBinary {
                    argument_index: *argument_index,
                    captured,
                    op: *op,
                    argument_left: false,
                })
            }
            NumericLeafShortcut::ArgumentArgumentBinary {
                left_argument_index,
                right_argument_index,
                op,
            } if *left_argument_index < argument_count
                && *right_argument_index < argument_count =>
            {
                number_binary(0.0, *op, 0.0)?;
                Some(Self::ArgumentArgumentBinary {
                    left_argument_index: *left_argument_index,
                    right_argument_index: *right_argument_index,
                    op: *op,
                })
            }
            NumericLeafShortcut::UpdateUpvalueConstReturn {
                upvalue_index,
                op,
                right,
            } if argument_count == 0 => {
                let upvalue = function.upvalues.get(*upvalue_index)?.clone();
                if caller_cells
                    .iter()
                    .flatten()
                    .any(|caller| caller.ptr_eq(&upvalue))
                {
                    return None;
                }
                let value = captured_number(*upvalue_index)?;
                number_binary(value, *op, *right)?;
                Some(Self::UpdateCapturedConstReturn {
                    upvalue,
                    value,
                    op: *op,
                    right: *right,
                })
            }
            _ => None,
        }
    }

    // This executes once per admitted loop iteration. Small unrelated layout
    // changes can otherwise make LLVM outline it, adding a call to the hottest
    // part of the numeric-loop path.
    #[inline(always)]
    pub(super) fn eval(&mut self, first_argument: f64, second_argument: f64) -> f64 {
        let argument = |index: usize| {
            if index == 0 {
                first_argument
            } else {
                second_argument
            }
        };
        match self {
            Self::MathAbs => first_argument.abs(),
            Self::ArgumentAddConstants {
                argument_index,
                constants,
            } => {
                let mut value = argument(*argument_index);
                for constant in constants {
                    value += *constant;
                }
                value
            }
            Self::ArgumentConstChain {
                argument_index,
                operations,
            } => {
                let mut value = argument(*argument_index);
                for (op, right) in operations {
                    value = number_binary(value, *op, *right)
                        .expect("validated numeric-result shortcut");
                }
                value
            }
            Self::ArgumentCapturedBinary {
                argument_index,
                captured,
                op,
                argument_left,
            } => {
                let argument = argument(*argument_index);
                let (left, right) = if *argument_left {
                    (argument, *captured)
                } else {
                    (*captured, argument)
                };
                number_binary(left, *op, right).expect("validated numeric-result shortcut")
            }
            Self::ArgumentArgumentBinary {
                left_argument_index,
                right_argument_index,
                op,
            } => number_binary(
                argument(*left_argument_index),
                *op,
                argument(*right_argument_index),
            )
            .expect("validated numeric-result shortcut"),
            Self::UpdateCapturedConstReturn {
                value, op, right, ..
            } => {
                *value =
                    number_binary(*value, *op, *right).expect("validated numeric-result shortcut");
                *value
            }
        }
    }

    pub(super) fn commit(self) {
        if let Self::UpdateCapturedConstReturn { upvalue, value, .. } = self {
            upvalue.set(Value::Number(value));
        }
    }
}

fn parameter_index(bytecode: &Bytecode, slot: usize) -> Option<usize> {
    bytecode
        .parameter_slots()
        .iter()
        .rposition(|candidate| *candidate == slot)
}

fn upvalue_index(bytecode: &Bytecode, slot: usize) -> Option<usize> {
    bytecode
        .received_upvalue_slots()
        .iter()
        .position(|candidate| *candidate == slot)
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

    if bytecode.parameter_slots().len() == params.positional.len()
        && bytecode.received_upvalue_slots().len() == upvalues.len()
        && let Some(value) = plan
            .shortcut
            .as_ref()
            .and_then(|shortcut| shortcut.eval(arguments, upvalues))
    {
        return Some(value);
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
        locals[slot] = upvalue.with_value(FastValue::from_value)?;
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
        locals[slot] = upvalue.with_value(FastValue::from_value)?;
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

// Keep the scalar-result arithmetic local to call-shaped plans. Inlining the
// wider `direct_number_binary` helper bloats unrelated fast loops, while this
// adapter only needs the five operations that always produce a Number.
#[inline(always)]
fn number_binary(left: f64, op: BinaryOp, right: f64) -> Option<f64> {
    match op {
        BinaryOp::Add => Some(left + right),
        BinaryOp::Sub => Some(left - right),
        BinaryOp::Mul => Some(left * right),
        BinaryOp::Div => Some(left / right),
        BinaryOp::Rem => Some(left % right),
        _ => match direct_number_binary(left, op, right)? {
            FastValue::Number(value) => Some(value),
            _ => None,
        },
    }
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
        assert!(
            matches!(
                plan.shortcut,
                Some(NumericLeafShortcut::ArgumentConstChain {
                    argument_index: 0,
                    ..
                })
            ),
            "unexpected plan: {plan:#?}"
        );
    }

    #[test]
    fn two_argument_plan_uses_argument_binary_shortcut() {
        let script = qjs_parser::parse_script("function add(left, right) { return left + right; }")
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

        assert!(matches!(
            plan.shortcut,
            Some(NumericLeafShortcut::ArgumentArgumentBinary {
                left_argument_index: 0,
                right_argument_index: 1,
                op: BinaryOp::Add,
            })
        ));
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
        assert!(matches!(
            plan.shortcut,
            Some(NumericLeafShortcut::UpdateUpvalueConstReturn {
                upvalue_index: 0,
                op: BinaryOp::Add,
                right: 1.0,
            })
        ));
    }

    #[test]
    fn captured_reader_plan_uses_argument_upvalue_shortcut() {
        let script = qjs_parser::parse_script(
            "function make() { var captured = 7; return function(value) { return value + captured; }; }",
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
        let plan = NumericLeafPlan::compile(inner).expect("reader should be admitted");
        assert!(matches!(
            plan.shortcut,
            Some(NumericLeafShortcut::ArgumentUpvalueBinary {
                argument_index: 0,
                upvalue_index: 0,
                op: BinaryOp::Add,
            })
        ));
    }
}
