use std::rc::Rc;

use qjs_ast::{BinaryOp, UpdateOp};

use crate::{
    Function, NativeFunction, Value,
    function::{CallEnv, is_direct_leaf_function},
    value::OwnDataPropertyRead,
};

use super::{
    ir::{Bytecode, Op},
    vm_numeric_leaf::{math_binary, math_unary},
    vm_props::fast_number_binary,
};

const MAX_NUMERIC_CONTROL_LOCALS: usize = 32;
const MAX_NUMERIC_CONTROL_STACK: usize = 16;
const MAX_NUMERIC_CONTROL_OPS: usize = 96;
const MAX_NUMERIC_CONTROL_CALL_ARGS: usize = 3;
const MAX_NUMERIC_CONTROL_CALL_DEPTH: usize = 8;

/// A compact, acyclic numeric bytecode subset with branches and calls.
///
/// The regular numeric leaf tier deliberately keeps its hot straight-line
/// representation minimal. This sibling plan handles the distinct shape where
/// a numeric helper has bounded control flow or calls another helper, while
/// retaining only tokens for globals, captures, and `Math` members until their
/// live values are checked at execution time.
#[derive(Clone, Debug)]
pub(super) struct NumericControlLeafPlan {
    ops: Vec<NumericControlOp>,
    globals: Vec<Rc<str>>,
    keys: Vec<Rc<str>>,
    hoisted_slots: u32,
}

#[derive(Clone, Debug)]
enum NumericControlOp {
    Nop,
    LoadConst(ControlValue),
    LoadLocal(usize),
    LoadLocalOrUndefined(usize),
    StoreLocal(usize),
    Dup,
    Pop,
    ToNumeric,
    Update(UpdateOp),
    Binary(BinaryOp),
    LoadGlobal(u8),
    GetNamed(u8),
    Call(u8),
    CallResolved(u8),
    JumpIfFalse(usize),
    Jump(usize),
    Return,
}

#[derive(Clone, Copy, Debug)]
enum ControlValue {
    Uninitialized,
    Undefined,
    Number(f64),
    Boolean(bool),
    Captured(u8),
    Global(u8),
    MathMember { global: u8, key: u8 },
}

impl NumericControlLeafPlan {
    pub(super) fn compile(bytecode: &Bytecode) -> Option<Self> {
        if bytecode.is_global_scope()
            || bytecode.locals.len() > MAX_NUMERIC_CONTROL_LOCALS
            || bytecode.code.len() > MAX_NUMERIC_CONTROL_OPS
            || bytecode.uses_lexical_this()
            || bytecode.needs_arguments_object()
            || bytecode.contains_direct_eval()
            || bytecode.contains_with()
            || bytecode
                .locals
                .iter()
                .any(|local| local.sloppy_global_fallback)
        {
            return None;
        }

        let mut globals = Vec::new();
        let mut keys = Vec::new();
        let mut ops = Vec::with_capacity(bytecode.code.len());
        let mut hoisted_slots = 0_u32;
        for (slot, local) in bytecode.locals.iter().enumerate() {
            if local.hoisted {
                hoisted_slots |= 1 << slot;
            }
        }
        let mut has_control_or_call = false;

        for (ip, source) in bytecode.code.iter().enumerate() {
            let op = match source {
                Op::FunctionPrologueEnd => NumericControlOp::Nop,
                Op::LoadConst(index) => NumericControlOp::LoadConst(ControlValue::from_value(
                    bytecode.constants.get(*index)?,
                )?),
                Op::LoadLocal(slot) => {
                    (*slot < bytecode.locals.len()).then_some(NumericControlOp::LoadLocal(*slot))?
                }
                Op::LoadLocalOrUndefined(slot) => (*slot < bytecode.locals.len())
                    .then_some(NumericControlOp::LoadLocalOrUndefined(*slot))?,
                Op::StoreLocal(slot) | Op::AssignLocal(slot) => {
                    if *slot >= bytecode.locals.len()
                        || !bytecode.local_is_mutable(*slot)
                        || bytecode.received_upvalue_slots().contains(slot)
                    {
                        return None;
                    }
                    NumericControlOp::StoreLocal(*slot)
                }
                Op::Dup => NumericControlOp::Dup,
                Op::Pop => NumericControlOp::Pop,
                Op::ToNumeric => NumericControlOp::ToNumeric,
                Op::Update(update) => NumericControlOp::Update(*update),
                Op::Binary(binary) => NumericControlOp::Binary(*binary),
                Op::LoadGlobal(name) => {
                    if name == "this" {
                        return None;
                    }
                    has_control_or_call = true;
                    NumericControlOp::LoadGlobal(intern_control_name(&mut globals, name)?)
                }
                Op::GetPropNamed { key, .. } => {
                    NumericControlOp::GetNamed(intern_control_name(&mut keys, key)?)
                }
                Op::Call(argc) => {
                    let argc = u8::try_from(*argc).ok()?;
                    if argc as usize > MAX_NUMERIC_CONTROL_CALL_ARGS {
                        return None;
                    }
                    has_control_or_call = true;
                    NumericControlOp::Call(argc)
                }
                Op::CallResolved(argc) => {
                    let argc = u8::try_from(*argc).ok()?;
                    if argc as usize > MAX_NUMERIC_CONTROL_CALL_ARGS {
                        return None;
                    }
                    has_control_or_call = true;
                    NumericControlOp::CallResolved(argc)
                }
                Op::JumpIfFalse(target) => {
                    if *target <= ip || *target >= bytecode.code.len() {
                        return None;
                    }
                    has_control_or_call = true;
                    NumericControlOp::JumpIfFalse(*target)
                }
                Op::Jump(target) => {
                    if *target <= ip || *target >= bytecode.code.len() {
                        return None;
                    }
                    has_control_or_call = true;
                    NumericControlOp::Jump(*target)
                }
                Op::Return => NumericControlOp::Return,
                _ => return None,
            };
            ops.push(op);
        }

        has_control_or_call.then_some(Self {
            ops,
            globals,
            keys,
            hoisted_slots,
        })
    }
}

fn intern_control_name(names: &mut Vec<Rc<str>>, name: &str) -> Option<u8> {
    if let Some(index) = names
        .iter()
        .position(|candidate| candidate.as_ref() == name)
    {
        return u8::try_from(index).ok();
    }
    let index = u8::try_from(names.len()).ok()?;
    names.push(Rc::from(name));
    Some(index)
}

impl ControlValue {
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
            Self::Undefined => Some(Value::Undefined),
            Self::Number(value) => Some(Value::Number(value)),
            Self::Boolean(value) => Some(Value::Boolean(value)),
            Self::Uninitialized | Self::Captured(_) | Self::Global(_) | Self::MathMember { .. } => {
                None
            }
        }
    }

    fn truthy(self) -> Option<bool> {
        match self {
            Self::Undefined => Some(false),
            Self::Number(value) => Some(value != 0.0 && !value.is_nan()),
            Self::Boolean(value) => Some(value),
            Self::Uninitialized | Self::Captured(_) | Self::Global(_) | Self::MathMember { .. } => {
                None
            }
        }
    }
}

/// Executes an acyclic, pure numeric call graph without constructing child
/// VMs. The compiled plan carries only symbolic references to globals,
/// captured values, and `Math` members; each is resolved immediately before
/// use so rebinding or accessor semantics decline to the ordinary VM before
/// any observable operation occurs.
pub(crate) fn try_eval_numeric_control_leaf(
    function: &Function,
    bytecode: &Bytecode,
    arguments: &[Value],
    env: &CallEnv,
) -> Option<Value> {
    let plan = numeric_control_function_eligible(function, bytecode)?;

    let mut control_arguments = [ControlValue::Undefined; MAX_NUMERIC_CONTROL_CALL_ARGS];
    for (index, argument) in arguments.iter().enumerate() {
        *control_arguments.get_mut(index)? = ControlValue::from_value(argument)?;
    }
    run_numeric_control_leaf(
        function,
        bytecode,
        plan,
        &control_arguments[..arguments.len()],
        env,
        0,
    )?
    .into_value()
}

fn numeric_control_function_eligible<'a>(
    function: &Function,
    bytecode: &'a Bytecode,
) -> Option<&'a NumericControlLeafPlan> {
    if function.native.is_some()
        || function.has_dynamic_function_realm
        || function.has_dynamic_function_realm_override.get()
        || !function.module_imports.is_empty()
        || function.params.positional.len() > MAX_NUMERIC_CONTROL_CALL_ARGS
        || bytecode.parameter_slots().len() != function.params.positional.len()
        || bytecode.received_upvalue_slots().len() != function.upvalues.len()
        || bytecode.reads_arguments()
    {
        return None;
    }
    bytecode.numeric_control_leaf_plan.as_ref()
}

fn run_numeric_control_leaf(
    function: &Function,
    bytecode: &Bytecode,
    plan: &NumericControlLeafPlan,
    arguments: &[ControlValue],
    env: &CallEnv,
    depth: usize,
) -> Option<ControlValue> {
    if depth >= MAX_NUMERIC_CONTROL_CALL_DEPTH
        || numeric_control_function_eligible(function, bytecode).is_none()
    {
        return None;
    }

    let mut locals = [ControlValue::Uninitialized; MAX_NUMERIC_CONTROL_LOCALS];
    let mut hoisted_slots = plan.hoisted_slots;
    while hoisted_slots != 0 {
        let slot = hoisted_slots.trailing_zeros() as usize;
        locals[slot] = ControlValue::Undefined;
        hoisted_slots &= hoisted_slots - 1;
    }

    for (index, &slot) in bytecode.parameter_slots().iter().enumerate() {
        *locals.get_mut(slot)? = arguments
            .get(index)
            .copied()
            .unwrap_or(ControlValue::Undefined);
    }
    for (index, &slot) in bytecode.received_upvalue_slots().iter().enumerate() {
        *locals.get_mut(slot)? = ControlValue::Captured(u8::try_from(index).ok()?);
    }

    let mut stack = [ControlValue::Uninitialized; MAX_NUMERIC_CONTROL_STACK];
    let mut stack_len = 0;
    let mut ip = 0;
    while let Some(op) = plan.ops.get(ip) {
        match op {
            NumericControlOp::Nop => ip += 1,
            NumericControlOp::LoadConst(value) => {
                push_control(&mut stack, &mut stack_len, *value)?;
                ip += 1;
            }
            NumericControlOp::LoadLocal(slot) => {
                let value = *locals.get(*slot)?;
                if matches!(value, ControlValue::Uninitialized) {
                    return None;
                }
                ensure_control_value_loaded(value, function, plan, env)?;
                push_control(&mut stack, &mut stack_len, value)?;
                ip += 1;
            }
            NumericControlOp::LoadLocalOrUndefined(slot) => {
                let value = match *locals.get(*slot)? {
                    ControlValue::Uninitialized => ControlValue::Undefined,
                    value => value,
                };
                ensure_control_value_loaded(value, function, plan, env)?;
                push_control(&mut stack, &mut stack_len, value)?;
                ip += 1;
            }
            NumericControlOp::StoreLocal(slot) => {
                *locals.get_mut(*slot)? = pop_control(&stack, &mut stack_len)?;
                ip += 1;
            }
            NumericControlOp::Dup => {
                let value = *stack.get(stack_len.checked_sub(1)?)?;
                push_control(&mut stack, &mut stack_len, value)?;
                ip += 1;
            }
            NumericControlOp::Pop => {
                let value = pop_control(&stack, &mut stack_len)?;
                ensure_control_value_loaded(value, function, plan, env)?;
                ip += 1;
            }
            NumericControlOp::ToNumeric => {
                let value = pop_control(&stack, &mut stack_len)?;
                let value = resolve_control_value(value, function, plan, env)?;
                if !matches!(value, ControlValue::Number(_)) {
                    return None;
                }
                push_control(&mut stack, &mut stack_len, value)?;
                ip += 1;
            }
            NumericControlOp::Update(op) => {
                let ControlValue::Number(value) = resolve_control_value(
                    pop_control(&stack, &mut stack_len)?,
                    function,
                    plan,
                    env,
                )?
                else {
                    return None;
                };
                let value = match op {
                    UpdateOp::Increment => value + 1.0,
                    UpdateOp::Decrement => value - 1.0,
                };
                push_control(&mut stack, &mut stack_len, ControlValue::Number(value))?;
                ip += 1;
            }
            NumericControlOp::Binary(op) => {
                let ControlValue::Number(right) = resolve_control_value(
                    pop_control(&stack, &mut stack_len)?,
                    function,
                    plan,
                    env,
                )?
                else {
                    return None;
                };
                let ControlValue::Number(left) = resolve_control_value(
                    pop_control(&stack, &mut stack_len)?,
                    function,
                    plan,
                    env,
                )?
                else {
                    return None;
                };
                push_control(
                    &mut stack,
                    &mut stack_len,
                    direct_control_number_binary(left, *op, right)?,
                )?;
                ip += 1;
            }
            NumericControlOp::LoadGlobal(index) => {
                let value = ControlValue::Global(*index);
                ensure_control_value_loaded(value, function, plan, env)?;
                push_control(&mut stack, &mut stack_len, value)?;
                ip += 1;
            }
            NumericControlOp::GetNamed(key) => {
                let ControlValue::Global(global) = pop_control(&stack, &mut stack_len)? else {
                    return None;
                };
                let _ = control_math_value(env, plan, global, *key)?;
                push_control(
                    &mut stack,
                    &mut stack_len,
                    ControlValue::MathMember { global, key: *key },
                )?;
                ip += 1;
            }
            NumericControlOp::Call(argc) => {
                let arguments = pop_control_arguments(&stack, &mut stack_len, *argc as usize)?;
                let callee = pop_control(&stack, &mut stack_len)?;
                let value = call_numeric_control_callee(
                    callee,
                    None,
                    &arguments[..*argc as usize],
                    function,
                    plan,
                    env,
                    depth,
                )?;
                push_control(&mut stack, &mut stack_len, value)?;
                ip += 1;
            }
            NumericControlOp::CallResolved(argc) => {
                let arguments = pop_control_arguments(&stack, &mut stack_len, *argc as usize)?;
                let callee = pop_control(&stack, &mut stack_len)?;
                let receiver = pop_control(&stack, &mut stack_len)?;
                let value = call_numeric_control_callee(
                    callee,
                    Some(receiver),
                    &arguments[..*argc as usize],
                    function,
                    plan,
                    env,
                    depth,
                )?;
                push_control(&mut stack, &mut stack_len, value)?;
                ip += 1;
            }
            NumericControlOp::JumpIfFalse(target) => {
                let value = resolve_control_value(
                    *stack.get(stack_len.checked_sub(1)?)?,
                    function,
                    plan,
                    env,
                )?;
                ip = if value.truthy()? { ip + 1 } else { *target };
            }
            NumericControlOp::Jump(target) => ip = *target,
            NumericControlOp::Return => {
                let value = if stack_len == 0 {
                    ControlValue::Undefined
                } else {
                    pop_control(&stack, &mut stack_len)?
                };
                return resolve_control_value(value, function, plan, env);
            }
        }
    }
    None
}

fn resolve_control_value(
    value: ControlValue,
    function: &Function,
    plan: &NumericControlLeafPlan,
    env: &CallEnv,
) -> Option<ControlValue> {
    match value {
        ControlValue::Captured(index) => {
            ControlValue::from_value(&function.upvalues.get(usize::from(index))?.get())
        }
        ControlValue::Global(index) => ControlValue::from_value(
            &env.get_realm(plan.globals.get(usize::from(index))?.as_ref())?,
        ),
        ControlValue::MathMember { global, key } => {
            ControlValue::from_value(&control_math_value(env, plan, global, key)?)
        }
        primitive => Some(primitive),
    }
}

/// Validates the observable read represented by a deferred token without
/// materializing its non-primitive value. This keeps the compact token form
/// for calls and named members, while an expression statement such as `f;`
/// still throws for an unbound or TDZ name before its value is discarded.
fn ensure_control_value_loaded(
    value: ControlValue,
    function: &Function,
    plan: &NumericControlLeafPlan,
    env: &CallEnv,
) -> Option<()> {
    let value = match value {
        ControlValue::Captured(index) => function.upvalues.get(usize::from(index))?.get(),
        ControlValue::Global(index) => {
            env.get_realm(plan.globals.get(usize::from(index))?.as_ref())?
        }
        ControlValue::MathMember { global, key } => control_math_value(env, plan, global, key)?,
        ControlValue::Uninitialized => return None,
        ControlValue::Undefined | ControlValue::Number(_) | ControlValue::Boolean(_) => {
            return Some(());
        }
    };
    (!value.is_uninitialized_lexical_marker()).then_some(())
}

fn control_math_value(
    env: &CallEnv,
    plan: &NumericControlLeafPlan,
    global: u8,
    key: u8,
) -> Option<Value> {
    if plan.globals.get(usize::from(global))?.as_ref() != "Math" {
        return None;
    }
    let Value::Object(math) = env.get_realm("Math")? else {
        return None;
    };
    match math.own_data_property_read(plan.keys.get(usize::from(key))?.as_ref()) {
        OwnDataPropertyRead::Data(value) => Some(value),
        OwnDataPropertyRead::Missing | OwnDataPropertyRead::NeedsSlowPath => None,
    }
}

fn call_numeric_control_callee(
    callee: ControlValue,
    receiver: Option<ControlValue>,
    arguments: &[ControlValue],
    caller: &Function,
    caller_plan: &NumericControlLeafPlan,
    env: &CallEnv,
    depth: usize,
) -> Option<ControlValue> {
    let mut resolved_arguments = [ControlValue::Undefined; MAX_NUMERIC_CONTROL_CALL_ARGS];
    for (index, argument) in arguments.iter().enumerate() {
        let value = resolve_control_value(*argument, caller, caller_plan, env)?;
        if !matches!(value, ControlValue::Number(_)) {
            return None;
        }
        *resolved_arguments.get_mut(index)? = value;
    }

    match callee {
        ControlValue::MathMember { global, key } if matches!(receiver, Some(ControlValue::Global(receiver_global)) if receiver_global == global) =>
        {
            let Value::Function(math_function) = control_math_value(env, caller_plan, global, key)?
            else {
                return None;
            };
            let native = math_function.native?;
            let ControlValue::Number(first) = resolved_arguments[0] else {
                return None;
            };
            if let Some(value) = math_unary(native, first) {
                return Some(ControlValue::Number(value));
            }
            let ControlValue::Number(second) = resolved_arguments[1] else {
                return None;
            };
            let value = match native {
                NativeFunction::MathPow | NativeFunction::MathAtan2 => {
                    math_binary(native, first, second)
                }
                NativeFunction::MathMax | NativeFunction::MathMin if arguments.len() == 2 => {
                    math_binary(native, first, second)
                }
                _ => None,
            }?;
            Some(ControlValue::Number(value))
        }
        ControlValue::Captured(index) | ControlValue::Global(index) if receiver.is_none() => {
            let callee = match callee {
                ControlValue::Captured(_) => caller.upvalues.get(usize::from(index))?.get(),
                ControlValue::Global(_) => {
                    env.get_realm(caller_plan.globals.get(usize::from(index))?.as_ref())?
                }
                _ => unreachable!("guarded by the enclosing match"),
            };
            let Value::Function(callee) = callee else {
                return None;
            };
            if !is_direct_leaf_function(&Value::Function(callee.clone())) {
                return None;
            }
            let bytecode = callee.bytecode.as_ref()?;
            let plan = numeric_control_function_eligible(&callee, bytecode)?;
            run_numeric_control_leaf(
                &callee,
                bytecode,
                plan,
                &resolved_arguments[..arguments.len()],
                env,
                depth + 1,
            )
        }
        _ => None,
    }
}

fn pop_control_arguments(
    stack: &[ControlValue; MAX_NUMERIC_CONTROL_STACK],
    stack_len: &mut usize,
    argc: usize,
) -> Option<[ControlValue; MAX_NUMERIC_CONTROL_CALL_ARGS]> {
    let mut arguments = [ControlValue::Undefined; MAX_NUMERIC_CONTROL_CALL_ARGS];
    for index in (0..argc).rev() {
        *arguments.get_mut(index)? = pop_control(stack, stack_len)?;
    }
    Some(arguments)
}

fn push_control(
    stack: &mut [ControlValue; MAX_NUMERIC_CONTROL_STACK],
    stack_len: &mut usize,
    value: ControlValue,
) -> Option<()> {
    *stack.get_mut(*stack_len)? = value;
    *stack_len += 1;
    Some(())
}

fn pop_control(
    stack: &[ControlValue; MAX_NUMERIC_CONTROL_STACK],
    stack_len: &mut usize,
) -> Option<ControlValue> {
    *stack_len = stack_len.checked_sub(1)?;
    stack.get(*stack_len).copied()
}

fn direct_control_number_binary(left: f64, op: BinaryOp, right: f64) -> Option<ControlValue> {
    match op {
        BinaryOp::Add => Some(ControlValue::Number(left + right)),
        BinaryOp::Sub => Some(ControlValue::Number(left - right)),
        BinaryOp::Mul => Some(ControlValue::Number(left * right)),
        BinaryOp::Div => Some(ControlValue::Number(left / right)),
        BinaryOp::Rem => Some(ControlValue::Number(crate::operations::number_remainder(
            left, right,
        ))),
        BinaryOp::Eq | BinaryOp::StrictEq => Some(ControlValue::Boolean(left == right)),
        BinaryOp::Ne | BinaryOp::StrictNe => Some(ControlValue::Boolean(left != right)),
        BinaryOp::Lt => Some(ControlValue::Boolean(left < right)),
        BinaryOp::Le => Some(ControlValue::Boolean(left <= right)),
        BinaryOp::Gt => Some(ControlValue::Boolean(left > right)),
        BinaryOp::Ge => Some(ControlValue::Boolean(left >= right)),
        _ => ControlValue::from_value(&fast_number_binary(
            &Value::Number(left),
            op,
            &Value::Number(right),
        )?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;

    #[test]
    fn numeric_control_leaf_classifies_branches_and_math_members() {
        let script = qjs_parser::parse_script(
            "function clamp(value) { return value < 0 ? 0 : value > 1 ? 1 : value; }",
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

        assert!(
            function_bytecode.numeric_control_leaf_plan.is_some(),
            "control-flow leaf should be classified: {:#?}",
            function_bytecode.code
        );
        assert_eq!(
            crate::eval(
                "function clamp(value) { return value < 0 ? 0 : value > 1 ? 1 : value; } 1 / clamp(-0);",
            ),
            Ok(Value::Number(f64::NEG_INFINITY))
        );
    }

    #[test]
    fn numeric_control_leaf_runs_captured_numeric_call_graphs() {
        assert_eq!(
            crate::eval(
                "function make() { function log2(value) { return Math.log(value) / Math.LN2; } return function(value) { return log2(value) < 0 ? Math.pow(value, 2) : log2(value); }; } var probe = make(); probe(.5);",
            ),
            Ok(Value::Number(0.25))
        );
    }

    #[test]
    fn numeric_control_leaf_falls_back_after_math_rebinding() {
        assert_eq!(
            crate::eval(
                "function probe(value) { return Math.log(value) + 1; } Math.log = function(value) { return value + 4; }; probe(3);",
            ),
            Ok(Value::Number(8.0))
        );
    }

    #[test]
    fn numeric_control_leaf_keeps_unbound_discarded_globals_observable() {
        assert_eq!(
            crate::eval(
                "(function () { try { (function () { missing; }()); } catch (error) { return error.name + ':' + typeof missing; } return 'leaked'; }());",
            ),
            Ok(Value::String("ReferenceError:undefined".to_owned().into()))
        );
    }
}
