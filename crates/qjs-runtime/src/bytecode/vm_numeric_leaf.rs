use std::rc::Rc;

use qjs_ast::{BinaryOp, FunctionParams, UpdateOp};

use crate::{
    Function, NativeFunction, Value,
    function::{Upvalue, is_direct_leaf_function},
    value::OwnDataPropertyRead,
};

use super::{
    ir::{Bytecode, Op},
    named_property_cache::NamedPropertyCache,
    vm_props::fast_number_binary,
};

const MAX_FAST_LOCALS: usize = 32;
const MAX_FAST_STACK: usize = 16;

mod arithmetic_program;

use arithmetic_program::NumericLeafArithmeticProgram;

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

/// Prevalidated direct-method bodies that read statically named properties
/// from `this` and, for numeric expressions, plain object parameters.
///
/// The evaluator admits ordinary object own-data properties only. Any receiver
/// or parameter that needs coercion or exotic/prototype/accessor semantics
/// declines before running user code, so the normal direct-leaf VM still
/// handles it exactly.
#[derive(Clone, Debug)]
pub(super) enum ThisPropertyLeafPlan {
    Read(ThisPropertyReadPlan),
    Numeric(ThisPropertyNumericPlan),
}

#[derive(Clone, Debug)]
pub(super) struct ThisPropertyReadPlan {
    key: Rc<str>,
    cache: NamedPropertyCache,
}

#[derive(Clone, Debug)]
pub(super) struct ThisPropertyNumericPlan {
    ops: Vec<ThisPropertyNumericOp>,
}

#[derive(Clone, Debug)]
enum ThisPropertyNumericOp {
    Read {
        source: ThisPropertySource,
        key: Rc<str>,
        cache: NamedPropertyCache,
    },
    Constant(f64),
    Binary(BinaryOp),
    Return,
}

#[derive(Clone, Copy, Debug)]
enum ThisPropertySource {
    Receiver,
    Argument(usize),
}

#[derive(Clone, Copy, Debug)]
enum ThisPropertyStackValue {
    Receiver,
    Number,
}

#[derive(Clone, Debug)]
enum NumericLeafShortcut {
    /// `return <literal>` — the whole body.
    Constant(FastValue),
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
    /// A straight-line Number-only leaf body compiled into a raw scalar stack
    /// program. Any non-Number argument declines before running an operation.
    ArithmeticProgram(NumericLeafArithmeticProgram),
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
    /// A `Math` intrinsic whose whole effect is a floating-point computation of
    /// its first argument.
    MathUnary(NativeFunction),
    /// The same for one of two arguments.
    MathBinary(NativeFunction),
    /// The callee returns the same number every time.
    Constant(f64),
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

impl ThisPropertyLeafPlan {
    pub(super) fn compile(
        constants: &[Value],
        ops: &[Op],
        global_scope: bool,
        parameter_slots: &[usize],
    ) -> Option<Self> {
        if global_scope {
            return None;
        }
        ThisPropertyReadPlan::compile(ops)
            .map(Self::Read)
            .or_else(|| {
                ThisPropertyNumericPlan::compile(constants, ops, parameter_slots).map(Self::Numeric)
            })
    }

    fn eval(&self, this_value: &Value, arguments: &[Value]) -> Option<Value> {
        match self {
            Self::Read(plan) => plan.eval(this_value),
            Self::Numeric(plan) => plan.eval(this_value, arguments),
        }
    }
}

impl ThisPropertyReadPlan {
    fn compile(ops: &[Op]) -> Option<Self> {
        let [
            Op::FunctionPrologueEnd,
            Op::LoadGlobal(this_name),
            Op::GetPropNamed { key, cache },
            Op::Return,
            ..,
        ] = ops
        else {
            return None;
        };
        if this_name != "this" {
            return None;
        }
        Some(Self {
            key: Rc::clone(key),
            cache: cache.clone(),
        })
    }

    fn eval(&self, this_value: &Value) -> Option<Value> {
        let Value::Object(receiver) = this_value else {
            return None;
        };
        if let Some(value) = self.cache.get(receiver) {
            return Some(value);
        }
        match receiver.own_data_property_read(&self.key) {
            OwnDataPropertyRead::Data(value) => {
                self.cache.update(receiver, &self.key, &value);
                Some(value)
            }
            OwnDataPropertyRead::Missing | OwnDataPropertyRead::NeedsSlowPath => None,
        }
    }
}

impl ThisPropertyNumericPlan {
    fn compile(constants: &[Value], ops: &[Op], parameter_slots: &[usize]) -> Option<Self> {
        let [Op::FunctionPrologueEnd, body @ ..] = ops else {
            return None;
        };
        let mut stack = Vec::with_capacity(MAX_FAST_STACK);
        let mut plan_ops = Vec::with_capacity(body.len());
        for op in body {
            match op {
                Op::LoadGlobal(name) if name == "this" => {
                    push_this_property_stack(&mut stack, ThisPropertyStackValue::Receiver)?;
                }
                Op::GetPropNamed { key, cache } => {
                    let source = match cache.local_slot() {
                        Some(slot) => ThisPropertySource::Argument(
                            parameter_slots
                                .iter()
                                .rposition(|candidate| *candidate == slot)?,
                        ),
                        None => match stack.pop()? {
                            ThisPropertyStackValue::Receiver => ThisPropertySource::Receiver,
                            ThisPropertyStackValue::Number => return None,
                        },
                    };
                    push_this_property_stack(&mut stack, ThisPropertyStackValue::Number)?;
                    plan_ops.push(ThisPropertyNumericOp::Read {
                        source,
                        key: Rc::clone(key),
                        cache: cache.clone(),
                    });
                }
                Op::LoadConst(index) => {
                    let Value::Number(value) = constants.get(*index)? else {
                        return None;
                    };
                    push_this_property_stack(&mut stack, ThisPropertyStackValue::Number)?;
                    plan_ops.push(ThisPropertyNumericOp::Constant(*value));
                }
                Op::Binary(binary) if this_property_numeric_binary(*binary) => {
                    let (
                        Some(ThisPropertyStackValue::Number),
                        Some(ThisPropertyStackValue::Number),
                    ) = (stack.pop(), stack.pop())
                    else {
                        return None;
                    };
                    push_this_property_stack(&mut stack, ThisPropertyStackValue::Number)?;
                    plan_ops.push(ThisPropertyNumericOp::Binary(*binary));
                }
                Op::Return => {
                    if !matches!(stack.as_slice(), [ThisPropertyStackValue::Number]) {
                        return None;
                    }
                    plan_ops.push(ThisPropertyNumericOp::Return);
                    return Some(Self { ops: plan_ops });
                }
                _ => return None,
            }
        }
        None
    }

    fn eval(&self, this_value: &Value, arguments: &[Value]) -> Option<Value> {
        let mut stack = [0.0; MAX_FAST_STACK];
        let mut stack_len = 0;
        for op in &self.ops {
            match op {
                ThisPropertyNumericOp::Read { source, key, cache } => {
                    let receiver = match source {
                        ThisPropertySource::Receiver => this_value,
                        ThisPropertySource::Argument(index) => arguments.get(*index)?,
                    };
                    push_number(
                        &mut stack,
                        &mut stack_len,
                        own_data_property_number(receiver, key, cache)?,
                    )?;
                }
                ThisPropertyNumericOp::Constant(value) => {
                    push_number(&mut stack, &mut stack_len, *value)?;
                }
                ThisPropertyNumericOp::Binary(op) => {
                    let right = pop_number(&stack, &mut stack_len)?;
                    let left = pop_number(&stack, &mut stack_len)?;
                    push_number(&mut stack, &mut stack_len, number_binary(left, *op, right)?)?;
                }
                ThisPropertyNumericOp::Return => {
                    return Some(Value::Number(pop_number(&stack, &mut stack_len)?));
                }
            }
        }
        None
    }
}

fn push_this_property_stack(
    stack: &mut Vec<ThisPropertyStackValue>,
    value: ThisPropertyStackValue,
) -> Option<()> {
    (stack.len() < MAX_FAST_STACK).then_some(())?;
    stack.push(value);
    Some(())
}

fn this_property_numeric_binary(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::Pow
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr
            | BinaryOp::BitwiseAnd
            | BinaryOp::BitwiseXor
            | BinaryOp::BitwiseOr
    )
}

fn own_data_property_number(value: &Value, key: &str, cache: &NamedPropertyCache) -> Option<f64> {
    let Value::Object(receiver) = value else {
        return None;
    };
    if let Some(Value::Number(value)) = cache.get(receiver) {
        return Some(value);
    }
    match receiver.own_data_property_read(key) {
        OwnDataPropertyRead::Data(Value::Number(value)) => {
            cache.update(receiver, key, &Value::Number(value));
            Some(value)
        }
        OwnDataPropertyRead::Data(_)
        | OwnDataPropertyRead::Missing
        | OwnDataPropertyRead::NeedsSlowPath => None,
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
        if let [FastOp::ReturnConst(value)] = core {
            return Some(Self::Constant(*value));
        }
        // An empty chain is `return a`, the most common leaf body there is.
        if let [FastOp::LoadLocal(slot), middle @ .., FastOp::Return] = core
            && let Some(argument_index) = parameter_index(bytecode, *slot)
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
        NumericLeafArithmeticProgram::compile(ops, bytecode).map(Self::ArithmeticProgram)
    }

    fn eval(&self, arguments: &[Value], upvalues: &[Upvalue]) -> Option<Value> {
        if let Self::Constant(value) = self {
            return value.into_value();
        }
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
            // Handled above, before the numeric accessors.
            Self::Constant(value) => value.into_value(),
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
            Self::ArithmeticProgram(program) => program.eval(arguments).map(Value::Number),
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

/// A `Math` function of one argument whose entire effect is a floating-point
/// computation. Anything that can observe an object, allocate, or depend on
/// state stays out.
pub(super) fn math_unary(native: NativeFunction, argument: f64) -> Option<f64> {
    let value = match native {
        NativeFunction::MathAbs => argument.abs(),
        NativeFunction::MathSqrt => argument.sqrt(),
        NativeFunction::MathFloor => argument.floor(),
        NativeFunction::MathCeil => argument.ceil(),
        NativeFunction::MathTrunc => argument.trunc(),
        NativeFunction::MathRound => crate::math::round_number(argument),
        NativeFunction::MathSin => argument.sin(),
        NativeFunction::MathCos => argument.cos(),
        NativeFunction::MathTan => argument.tan(),
        NativeFunction::MathExp => argument.exp(),
        NativeFunction::MathLog => argument.ln(),
        // The remaining pure computations, each mapped to the same `f64` method
        // the native implementation calls, so the results are identical.
        NativeFunction::MathAcos => argument.acos(),
        NativeFunction::MathAcosh => argument.acosh(),
        NativeFunction::MathAsin => argument.asin(),
        NativeFunction::MathAsinh => argument.asinh(),
        NativeFunction::MathAtan => argument.atan(),
        NativeFunction::MathAtanh => argument.atanh(),
        NativeFunction::MathCbrt => argument.cbrt(),
        NativeFunction::MathCosh => argument.cosh(),
        NativeFunction::MathExpm1 => argument.exp_m1(),
        NativeFunction::MathLog1p => argument.ln_1p(),
        NativeFunction::MathLog10 => argument.log10(),
        NativeFunction::MathLog2 => argument.log2(),
        NativeFunction::MathSinh => argument.sinh(),
        NativeFunction::MathTanh => argument.tanh(),
        // `sign` returns its argument unchanged for NaN and for either zero, so
        // both signed zeros survive.
        NativeFunction::MathSign => {
            if argument.is_nan() || argument == 0.0 {
                argument
            } else if argument.is_sign_negative() {
                -1.0
            } else {
                1.0
            }
        }
        NativeFunction::MathFround => f64::from(argument as f32),
        _ => return None,
    };
    Some(value)
}

/// A `Math` function of two arguments whose entire effect is a floating-point
/// computation. `max`/`min` follow the spec's NaN and signed-zero rules rather
/// than Rust's, which disagree on both.
pub(super) fn math_binary(native: NativeFunction, left: f64, right: f64) -> Option<f64> {
    let value = match native {
        NativeFunction::MathPow => crate::operations::number_exponentiate(left, right),
        NativeFunction::MathAtan2 => left.atan2(right),
        NativeFunction::MathMax => {
            if left.is_nan() || right.is_nan() {
                f64::NAN
            } else if right > left || (right == 0.0 && left == 0.0 && right.is_sign_positive()) {
                right
            } else {
                left
            }
        }
        NativeFunction::MathMin => {
            if left.is_nan() || right.is_nan() {
                f64::NAN
            } else if right < left || (right == 0.0 && left == 0.0 && right.is_sign_negative()) {
                right
            } else {
                left
            }
        }
        _ => return None,
    };
    Some(value)
}

impl NumericLoopCall {
    pub(super) fn prepare(
        function: &Function,
        argument_count: usize,
        caller_cells: &[Option<Upvalue>],
        forbidden_cells: &[Upvalue],
    ) -> Option<Self> {
        if let Some(native) = function.native {
            if argument_count >= 2 && math_binary(native, 0.0, 0.0).is_some() {
                return Some(Self::MathBinary(native));
            }
            if argument_count >= 1 && math_unary(native, 0.0).is_some() {
                return Some(Self::MathUnary(native));
            }
            // Any other native is outside this tier: it may observe the world.
            return None;
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
            NumericLeafShortcut::Constant(FastValue::Number(value)) => Some(Self::Constant(*value)),
            NumericLeafShortcut::Constant(_) => None,
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
            // This path supports general direct calls but is intentionally not
            // a counted-loop scalar plan: its multi-step stack shape needs a
            // per-invocation evaluator rather than one reusable scalar state.
            NumericLeafShortcut::ArithmeticProgram(_) => None,
            _ => None,
        }
    }

    /// Whether repeated evaluation cannot stage a captured write whose commit
    /// would depend on a selector arm's per-iteration ordering.
    pub(super) fn is_read_only(&self) -> bool {
        match self {
            Self::MathUnary(_)
            | Self::MathBinary(_)
            | Self::Constant(_)
            | Self::ArgumentAddConstants { .. }
            | Self::ArgumentConstChain { .. }
            | Self::ArgumentCapturedBinary { .. }
            | Self::ArgumentArgumentBinary { .. } => true,
            Self::UpdateCapturedConstReturn { .. } => false,
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
            Self::MathUnary(native) => {
                math_unary(*native, first_argument).expect("validated numeric intrinsic")
            }
            Self::MathBinary(native) => math_binary(*native, first_argument, second_argument)
                .expect("validated numeric intrinsic"),
            Self::Constant(value) => *value,
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

/// Evaluates the smallest receiver-property method shape without constructing
/// a child VM. The bytecode-owned plan and named-property cache are immutable
/// to source code; their checks still validate the receiver's live layout.
pub(crate) fn try_eval_this_property_leaf(
    bytecode: &Bytecode,
    this_value: &Value,
    arguments: &[Value],
) -> Option<Value> {
    // A receiver-property body can only shortcut an already-object receiver.
    // The bytecode shape was preclassified during compilation, so ordinary
    // direct calls do not pay an additional lazy-plan probe here.
    if !matches!(this_value, Value::Object(_)) {
        return None;
    }
    bytecode
        .this_property_leaf_plan
        .as_ref()?
        .eval(this_value, arguments)
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

fn push_number(stack: &mut [f64; MAX_FAST_STACK], stack_len: &mut usize, value: f64) -> Option<()> {
    *stack.get_mut(*stack_len)? = value;
    *stack_len += 1;
    Some(())
}

fn pop_number(stack: &[f64; MAX_FAST_STACK], stack_len: &mut usize) -> Option<f64> {
    *stack_len = stack_len.checked_sub(1)?;
    stack.get(*stack_len).copied()
}

fn direct_number_binary(left: f64, op: BinaryOp, right: f64) -> Option<FastValue> {
    let value = match op {
        BinaryOp::Add => FastValue::Number(left + right),
        BinaryOp::Sub => FastValue::Number(left - right),
        BinaryOp::Mul => FastValue::Number(left * right),
        BinaryOp::Div => FastValue::Number(left / right),
        BinaryOp::Rem => FastValue::Number(crate::operations::number_remainder(left, right)),
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
        BinaryOp::Rem => Some(crate::operations::number_remainder(left, right)),
        _ => match direct_number_binary(left, op, right)? {
            FastValue::Number(value) => Some(value),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{bytecode::compiler, eval};

    fn numeric_plan_for_first_function(source: &str) -> Option<NumericLeafPlan> {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        let script_bytecode = compiler::compile_script(&script).expect("source should compile");
        let function_bytecode = script_bytecode.code.iter().find_map(|op| match op {
            Op::NewFunction { bytecode, .. } => Some(bytecode),
            _ => None,
        })?;
        NumericLeafPlan::compile(function_bytecode)
    }

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
    fn arithmetic_program_compiles_multi_argument_formula() {
        let plan = numeric_plan_for_first_function(
            "function A(i, j) { return 1 / ((i + j) * (i + j + 1) / 2 + i + 1); }",
        )
        .expect("formula should be admitted");
        let Some(NumericLeafShortcut::ArithmeticProgram(program)) = plan.shortcut.as_ref() else {
            panic!("formula should use an arithmetic program: {plan:#?}");
        };

        assert_eq!(
            program.eval(&[Value::Number(2.0), Value::Number(3.0)]),
            Some(1.0 / 18.0)
        );
        assert_eq!(
            program.eval(&[Value::Boolean(true), Value::Number(3.0)]),
            None,
            "non-Number input must decline before arithmetic"
        );
    }

    #[test]
    fn arithmetic_program_preserves_number_edge_cases() {
        let plan = numeric_plan_for_first_function(
            "function quotient(left, right) { return (left * 1) / (right + 0); }",
        )
        .expect("formula should be admitted");
        let Some(NumericLeafShortcut::ArithmeticProgram(program)) = plan.shortcut.as_ref() else {
            panic!("formula should use an arithmetic program: {plan:#?}");
        };

        let negative_zero = program
            .eval(&[Value::Number(-0.0), Value::Number(1.0)])
            .expect("Number arguments should evaluate");
        assert_eq!(negative_zero, 0.0);
        assert!(negative_zero.is_sign_negative());
        assert!(
            program
                .eval(&[Value::Number(f64::NAN), Value::Number(1.0)])
                .expect("Number arguments should evaluate")
                .is_nan()
        );
        assert!(
            program
                .eval(&[Value::Number(f64::INFINITY), Value::Number(1.0)])
                .expect("Number arguments should evaluate")
                .is_infinite()
        );

        let remainder = numeric_plan_for_first_function(
            "function remainder(left, right) { return (left * 1) % right; }",
        )
        .expect("remainder should be admitted");
        let Some(NumericLeafShortcut::ArithmeticProgram(program)) = remainder.shortcut.as_ref()
        else {
            panic!("remainder should use an arithmetic program: {remainder:#?}");
        };
        let negative_zero = program
            .eval(&[Value::Number(-0.0), Value::Number(1.0)])
            .expect("Number arguments should evaluate");
        assert_eq!(negative_zero, 0.0);
        assert!(negative_zero.is_sign_negative());
        assert!(
            program
                .eval(&[Value::Number(f64::INFINITY), Value::Number(1.0)])
                .expect("Number arguments should evaluate")
                .is_nan()
        );
    }

    #[test]
    fn arithmetic_program_declines_mutation_control_flow_and_coercion() {
        let mutation = numeric_plan_for_first_function(
            "function mutate(left, right) { left += right; return left * right; }",
        )
        .expect("ordinary numeric leaf should still be admitted");
        assert!(
            !matches!(
                mutation.shortcut,
                Some(NumericLeafShortcut::ArithmeticProgram(_))
            ),
            "local writes must not enter the raw arithmetic program"
        );
        assert!(
            numeric_plan_for_first_function(
                "function branch(left, right) { if (left) return right; return left + right; }",
            )
            .is_none(),
            "control flow must not enter the numeric leaf plan"
        );

        let sum = numeric_plan_for_first_function(
            "function sum(left, right) { return (left + right) * 2; }",
        )
        .expect("formula should be admitted");
        assert!(matches!(
            sum.shortcut,
            Some(NumericLeafShortcut::ArithmeticProgram(_))
        ));
        assert_eq!(
            eval(
                "var calls = 0; \
                 function sum(left, right) { return (left + right) * 2; } \
                 var source = { valueOf: function() { calls++; return 2; } }; \
                 sum(source, 3) + ':' + calls;"
            ),
            Ok(Value::String("10:1".to_owned().into()))
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

    #[test]
    fn this_property_plan_classifies_plain_reads_and_numeric_expressions() {
        let script = qjs_parser::parse_script(
            "var receiver = { value: 7, read: function() { return this.value; } };",
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
        let plan = function_bytecode
            .this_property_leaf_plan
            .as_ref()
            .expect("plain receiver read should be admitted");
        assert!(matches!(plan, ThisPropertyLeafPlan::Read(_)));
        let receiver =
            crate::ObjectRef::new(HashMap::from([("value".to_owned(), Value::Number(7.0))]));
        assert_eq!(
            plan.eval(&Value::Object(receiver), &[]),
            Some(Value::Number(7.0))
        );

        let script = qjs_parser::parse_script(
            "var receiver = { value: 7, read: function() { return this.value + 1; } };",
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
        let plan = function_bytecode
            .this_property_leaf_plan
            .as_ref()
            .expect("numeric receiver expression should be admitted");
        assert!(matches!(plan, ThisPropertyLeafPlan::Numeric(_)));
        let receiver =
            crate::ObjectRef::new(HashMap::from([("value".to_owned(), Value::Number(7.0))]));
        assert_eq!(
            plan.eval(&Value::Object(receiver), &[]),
            Some(Value::Number(8.0))
        );
    }

    #[test]
    fn this_property_numeric_plan_accepts_receiver_and_argument_field_dot_product() {
        let script = qjs_parser::parse_script(
            "var dot = function(other) { return this.x * other.x + this.y * other.y + this.z * other.z; };",
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
        let plan = function_bytecode
            .this_property_leaf_plan
            .as_ref()
            .expect("dot product should be admitted");
        assert!(
            matches!(plan, ThisPropertyLeafPlan::Numeric(_)),
            "{plan:#?}"
        );
        let receiver = crate::ObjectRef::new(HashMap::from([
            ("x".to_owned(), Value::Number(1.0)),
            ("y".to_owned(), Value::Number(2.0)),
            ("z".to_owned(), Value::Number(3.0)),
        ]));
        let argument = crate::ObjectRef::new(HashMap::from([
            ("x".to_owned(), Value::Number(4.0)),
            ("y".to_owned(), Value::Number(5.0)),
            ("z".to_owned(), Value::Number(6.0)),
        ]));
        assert_eq!(
            plan.eval(&Value::Object(receiver), &[Value::Object(argument)]),
            Some(Value::Number(32.0))
        );
    }

    #[test]
    fn this_property_numeric_plan_uses_the_last_duplicate_parameter_position() {
        let script = qjs_parser::parse_script(
            "var multiply = function(other, other) { return this.x * other.x; };",
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
        let plan = function_bytecode
            .this_property_leaf_plan
            .as_ref()
            .expect("duplicate-parameter property expression should be admitted");
        let receiver = crate::ObjectRef::new(HashMap::from([("x".to_owned(), Value::Number(2.0))]));
        let first_argument =
            crate::ObjectRef::new(HashMap::from([("x".to_owned(), Value::Number(3.0))]));
        let last_argument =
            crate::ObjectRef::new(HashMap::from([("x".to_owned(), Value::Number(4.0))]));
        assert_eq!(
            plan.eval(
                &Value::Object(receiver),
                &[Value::Object(first_argument), Value::Object(last_argument)]
            ),
            Some(Value::Number(8.0))
        );
    }
}
