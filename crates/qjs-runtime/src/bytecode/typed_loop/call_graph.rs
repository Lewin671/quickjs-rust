//! Entry-prepared pure-Number call graphs for typed loops.
//!
//! A loop may call a small acyclic graph of ordinary Number-only helpers. The
//! graph is lowered and every live function, capture, realm, and `Math` guard is
//! resolved once when the typed loop is entered. Iterations then execute only
//! scalar instructions; a failed guard declines before the loop has performed
//! fast-path work.

use std::{collections::VecDeque, rc::Rc};

use qjs_ast::BinaryOp;

use crate::{Function, Value, function::is_direct_leaf_function, value::OwnDataPropertyRead};

use super::super::{
    ir::{Bytecode, Op},
    vm_numeric_leaf::{math_binary, math_unary},
};
use super::Typed;

const MAX_GRAPH_OPS: usize = 128;
const MAX_GRAPH_CALLS: usize = 8;
const MAX_GRAPH_DEPTH: usize = 8;
const MAX_PREPARED_GRAPH_NODES: usize = 32;
const MAX_GRAPH_PARAMETERS: usize = 2;
const MAX_GRAPH_LOCALS: usize = 32;
const MAX_GRAPH_STACK: usize = 16;

/// Immutable bytecode shape for a bounded pure numeric call graph.
#[derive(Clone, Debug)]
struct CallGraphPlan {
    ops: Vec<GraphOp>,
    source_to_op: Vec<Option<usize>>,
    parameter_count: usize,
    capture_uses: Vec<CaptureUse>,
    uses_math_global: bool,
    math_properties: Vec<Rc<str>>,
    math_uses: Vec<MathUse>,
    call_sites: Vec<CallSite>,
}

#[derive(Clone, Debug)]
enum GraphOp {
    LoadNumber(f64),
    LoadParameter(usize),
    LoadCapture(usize),
    LoadMath,
    GetMathProperty(usize),
    Dup,
    Pop,
    Binary(BinaryOp),
    CallCaptured {
        call_index: usize,
        argument_count: u8,
    },
    CallMath {
        property: usize,
        argument_count: u8,
    },
    /// The target remains a bytecode instruction index so compilation can
    /// process forward branch merge points in any worklist order.
    Jump(usize),
    /// Like `Jump`, but the branch leaves its tested scalar on the stack, just
    /// as the bytecode VM does. Each branch's following `Pop` owns it.
    JumpIfFalse(usize),
    Return,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GraphScalar {
    Number,
    Boolean,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GraphStackValue {
    Scalar(GraphScalar),
    Capture(usize),
    MathObject,
    MathProperty(usize),
}

#[derive(Clone, Copy, Debug, Default)]
struct CaptureUse {
    number: bool,
    call_arity: Option<u8>,
}

#[derive(Clone, Copy, Debug, Default)]
struct MathUse {
    number: bool,
    call_arity: Option<u8>,
}

#[derive(Clone, Copy, Debug)]
struct CallSite {
    capture: usize,
    argument_count: u8,
}

/// A fully checked graph entry. Nested functions and captured values are owned,
/// so no cell or function identity is reread inside the hot loop.
pub(super) struct PreparedCallGraph {
    plan: CallGraphPlan,
    context: PreparedContext,
    calls: Vec<PreparedCallGraph>,
}

struct PreparedContext {
    captures: Vec<Value>,
    math_properties: Vec<Value>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum RuntimeValue {
    Empty,
    Number(f64),
    Boolean(bool),
    Capture(usize),
    MathObject,
    MathProperty(usize),
}

#[cfg(test)]
std::thread_local! {
    static EVAL_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_eval_hits() {
    EVAL_HITS.with(|hits| hits.set(0));
}

#[cfg(test)]
pub(super) fn eval_hits() -> usize {
    EVAL_HITS.with(std::cell::Cell::get)
}

impl CallGraphPlan {
    pub(super) fn compile(bytecode: &Bytecode) -> Option<Self> {
        if bytecode.locals.len() > MAX_GRAPH_LOCALS
            || bytecode.code.len() > MAX_GRAPH_OPS
            || bytecode.parameter_slots().len() > MAX_GRAPH_PARAMETERS
            || bytecode
                .locals
                .iter()
                .any(|local| local.sloppy_global_fallback)
            || !matches!(bytecode.code.first(), Some(Op::FunctionPrologueEnd))
        {
            return None;
        }

        let parameter_count = bytecode.parameter_slots().len();
        let mut states = vec![None; bytecode.code.len()];
        states[0] = Some(Vec::new());
        let mut queue = VecDeque::from([0]);
        let mut ops = Vec::with_capacity(bytecode.code.len());
        let mut source_to_op = vec![None; bytecode.code.len()];
        let mut capture_uses = vec![CaptureUse::default(); bytecode.received_upvalue_slots().len()];
        let mut uses_math_global = false;
        let mut math_properties = Vec::new();
        let mut math_uses = Vec::new();
        let mut call_sites = Vec::new();

        while let Some(ip) = queue.pop_front() {
            let mut stack = states.get(ip)?.as_ref()?.clone();
            match bytecode.code.get(ip)? {
                Op::FunctionPrologueEnd if ip == 0 && stack.is_empty() => {
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack)?;
                }
                Op::LoadConst(index) => {
                    let Value::Number(value) = bytecode.constants.get(*index)? else {
                        return None;
                    };
                    push_static(&mut stack, GraphStackValue::Scalar(GraphScalar::Number))?;
                    emit_graph_op(&mut source_to_op, ip, &mut ops, GraphOp::LoadNumber(*value))?;
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack)?;
                }
                Op::LoadLocal(slot) => {
                    let (stack_value, graph_op) = if let Some(parameter) = bytecode
                        .parameter_slots()
                        .iter()
                        .rposition(|candidate| candidate == slot)
                    {
                        (
                            GraphStackValue::Scalar(GraphScalar::Number),
                            GraphOp::LoadParameter(parameter),
                        )
                    } else if let Some(capture) = bytecode
                        .received_upvalue_slots()
                        .iter()
                        .position(|candidate| candidate == slot)
                    {
                        (
                            GraphStackValue::Capture(capture),
                            GraphOp::LoadCapture(capture),
                        )
                    } else {
                        return None;
                    };
                    push_static(&mut stack, stack_value)?;
                    emit_graph_op(&mut source_to_op, ip, &mut ops, graph_op)?;
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack)?;
                }
                Op::LoadGlobal(name) if name == "Math" => {
                    uses_math_global = true;
                    push_static(&mut stack, GraphStackValue::MathObject)?;
                    emit_graph_op(&mut source_to_op, ip, &mut ops, GraphOp::LoadMath)?;
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack)?;
                }
                Op::GetPropNamed { key, .. } => {
                    if stack.pop()? != GraphStackValue::MathObject {
                        return None;
                    }
                    let property = intern_math_property(&mut math_properties, &mut math_uses, key);
                    push_static(&mut stack, GraphStackValue::MathProperty(property))?;
                    emit_graph_op(
                        &mut source_to_op,
                        ip,
                        &mut ops,
                        GraphOp::GetMathProperty(property),
                    )?;
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack)?;
                }
                Op::Dup => {
                    let value = *stack.last()?;
                    push_static(&mut stack, value)?;
                    emit_graph_op(&mut source_to_op, ip, &mut ops, GraphOp::Dup)?;
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack)?;
                }
                Op::Pop => {
                    stack.pop()?;
                    emit_graph_op(&mut source_to_op, ip, &mut ops, GraphOp::Pop)?;
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack)?;
                }
                Op::Binary(binary) => {
                    let right = stack.pop()?;
                    let left = stack.pop()?;
                    require_number(right, &mut capture_uses, &mut math_uses)?;
                    require_number(left, &mut capture_uses, &mut math_uses)?;
                    let result = graph_binary(0.0, *binary, 0.0)?;
                    let scalar = match result {
                        RuntimeValue::Number(_) => GraphScalar::Number,
                        RuntimeValue::Boolean(_) => GraphScalar::Boolean,
                        RuntimeValue::Empty
                        | RuntimeValue::Capture(_)
                        | RuntimeValue::MathObject
                        | RuntimeValue::MathProperty(_) => return None,
                    };
                    push_static(&mut stack, GraphStackValue::Scalar(scalar))?;
                    emit_graph_op(&mut source_to_op, ip, &mut ops, GraphOp::Binary(*binary))?;
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack)?;
                }
                Op::Call(argument_count) if (1..=2).contains(argument_count) => {
                    let argument_count = u8::try_from(*argument_count).ok()?;
                    let start = stack.len().checked_sub(usize::from(argument_count) + 1)?;
                    let capture = match stack.get(start)? {
                        GraphStackValue::Capture(capture) => *capture,
                        _ => return None,
                    };
                    for value in stack.iter().skip(start + 1).copied() {
                        require_number(value, &mut capture_uses, &mut math_uses)?;
                    }
                    mark_capture_call(&mut capture_uses, capture, argument_count)?;
                    stack.truncate(start);
                    push_static(&mut stack, GraphStackValue::Scalar(GraphScalar::Number))?;
                    if call_sites.len() == MAX_GRAPH_CALLS {
                        return None;
                    }
                    let call_index = call_sites.len();
                    call_sites.push(CallSite {
                        capture,
                        argument_count,
                    });
                    emit_graph_op(
                        &mut source_to_op,
                        ip,
                        &mut ops,
                        GraphOp::CallCaptured {
                            call_index,
                            argument_count,
                        },
                    )?;
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack)?;
                }
                Op::CallResolved(argument_count) if (1..=2).contains(argument_count) => {
                    let argument_count = u8::try_from(*argument_count).ok()?;
                    let start = stack.len().checked_sub(usize::from(argument_count) + 2)?;
                    if stack.get(start)? != &GraphStackValue::MathObject {
                        return None;
                    }
                    let property = match stack.get(start + 1)? {
                        GraphStackValue::MathProperty(property) => *property,
                        _ => return None,
                    };
                    for value in stack.iter().skip(start + 2).copied() {
                        require_number(value, &mut capture_uses, &mut math_uses)?;
                    }
                    mark_math_call(&mut math_uses, property, argument_count)?;
                    stack.truncate(start);
                    push_static(&mut stack, GraphStackValue::Scalar(GraphScalar::Number))?;
                    emit_graph_op(
                        &mut source_to_op,
                        ip,
                        &mut ops,
                        GraphOp::CallMath {
                            property,
                            argument_count,
                        },
                    )?;
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack)?;
                }
                Op::Jump(target) if *target > ip && *target < bytecode.code.len() => {
                    emit_graph_op(&mut source_to_op, ip, &mut ops, GraphOp::Jump(*target))?;
                    enqueue_state(&mut states, &mut queue, *target, stack)?;
                }
                Op::JumpIfFalse(target) if *target > ip && *target < bytecode.code.len() => {
                    require_truthy(*stack.last()?, &mut capture_uses, &mut math_uses)?;
                    emit_graph_op(
                        &mut source_to_op,
                        ip,
                        &mut ops,
                        GraphOp::JumpIfFalse(*target),
                    )?;
                    enqueue_state(&mut states, &mut queue, ip.checked_add(1)?, stack.clone())?;
                    enqueue_state(&mut states, &mut queue, *target, stack)?;
                }
                Op::Return => {
                    require_number(stack.pop()?, &mut capture_uses, &mut math_uses)?;
                    if !stack.is_empty() {
                        return None;
                    }
                    emit_graph_op(&mut source_to_op, ip, &mut ops, GraphOp::Return)?;
                }
                _ => return None,
            }
        }

        if !ops.iter().any(|op| matches!(op, GraphOp::Return)) {
            return None;
        }
        // The data-flow worklist deliberately explores both successors before
        // either arm is complete, so its discovery order is not executable
        // fallthrough order. Store the finished graph in bytecode order; then
        // `ip + 1` means ordinary fallthrough and only explicit jumps need the
        // source-to-op map.
        let mut ordered_ops = Vec::with_capacity(ops.len());
        let mut ordered_source_to_op = vec![None; source_to_op.len()];
        for (source, discovered) in source_to_op.iter().copied().enumerate() {
            let Some(discovered) = discovered else {
                continue;
            };
            ordered_source_to_op[source] = Some(ordered_ops.len());
            ordered_ops.push(ops.get(discovered)?.clone());
        }
        ops = ordered_ops;
        source_to_op = ordered_source_to_op;

        for op in &ops {
            let target = match op {
                GraphOp::Jump(target) | GraphOp::JumpIfFalse(target) => Some(*target),
                _ => None,
            };
            if let Some(target) = target {
                source_to_op.get(target)?.as_ref()?;
            }
        }

        Some(Self {
            ops,
            source_to_op,
            parameter_count,
            capture_uses,
            uses_math_global,
            math_properties,
            math_uses,
            call_sites,
        })
    }

    fn jump_to_op(&self, source: usize) -> Option<usize> {
        self.source_to_op.get(source).copied().flatten()
    }

    fn execute(
        &self,
        context: &PreparedContext,
        calls: &[PreparedCallGraph],
        arguments: &[f64],
    ) -> Option<f64> {
        if arguments.len() != self.parameter_count {
            return None;
        }

        let mut stack = [RuntimeValue::Empty; MAX_GRAPH_STACK];
        let mut stack_len = 0;
        let mut ip = 0;
        loop {
            match self.ops.get(ip)? {
                GraphOp::LoadNumber(value) => {
                    push_runtime(&mut stack, &mut stack_len, RuntimeValue::Number(*value))?;
                    ip += 1;
                }
                GraphOp::LoadParameter(index) => {
                    push_runtime(
                        &mut stack,
                        &mut stack_len,
                        RuntimeValue::Number(*arguments.get(*index)?),
                    )?;
                    ip += 1;
                }
                GraphOp::LoadCapture(index) => {
                    push_runtime(&mut stack, &mut stack_len, RuntimeValue::Capture(*index))?;
                    ip += 1;
                }
                GraphOp::LoadMath => {
                    push_runtime(&mut stack, &mut stack_len, RuntimeValue::MathObject)?;
                    ip += 1;
                }
                GraphOp::GetMathProperty(property) => {
                    if pop_runtime(&stack, &mut stack_len)? != RuntimeValue::MathObject {
                        return None;
                    }
                    push_runtime(
                        &mut stack,
                        &mut stack_len,
                        RuntimeValue::MathProperty(*property),
                    )?;
                    ip += 1;
                }
                GraphOp::Dup => {
                    let value = *stack.get(stack_len.checked_sub(1)?)?;
                    push_runtime(&mut stack, &mut stack_len, value)?;
                    ip += 1;
                }
                GraphOp::Pop => {
                    pop_runtime(&stack, &mut stack_len)?;
                    ip += 1;
                }
                GraphOp::Binary(binary) => {
                    let right = runtime_number(context, pop_runtime(&stack, &mut stack_len)?)?;
                    let left = runtime_number(context, pop_runtime(&stack, &mut stack_len)?)?;
                    let value = graph_binary(left, *binary, right)?;
                    push_runtime(&mut stack, &mut stack_len, value)?;
                    ip += 1;
                }
                GraphOp::CallCaptured {
                    call_index,
                    argument_count,
                } => {
                    let mut numbers = [0.0; MAX_GRAPH_PARAMETERS];
                    for index in (0..usize::from(*argument_count)).rev() {
                        numbers[index] =
                            runtime_number(context, pop_runtime(&stack, &mut stack_len)?)?;
                    }
                    let site = self.call_sites.get(*call_index)?;
                    if site.argument_count != *argument_count
                        || pop_runtime(&stack, &mut stack_len)?
                            != RuntimeValue::Capture(site.capture)
                    {
                        return None;
                    }
                    let call = calls.get(*call_index)?;
                    let value = call.eval_numbers(&numbers[..usize::from(*argument_count)])?;
                    push_runtime(&mut stack, &mut stack_len, RuntimeValue::Number(value))?;
                    ip += 1;
                }
                GraphOp::CallMath {
                    property,
                    argument_count,
                } => {
                    let mut numbers = [0.0; MAX_GRAPH_PARAMETERS];
                    for index in (0..usize::from(*argument_count)).rev() {
                        numbers[index] =
                            runtime_number(context, pop_runtime(&stack, &mut stack_len)?)?;
                    }
                    if pop_runtime(&stack, &mut stack_len)? != RuntimeValue::MathProperty(*property)
                        || pop_runtime(&stack, &mut stack_len)? != RuntimeValue::MathObject
                    {
                        return None;
                    }
                    let Value::Function(function) = context.math_properties.get(*property)? else {
                        return None;
                    };
                    let native = function.native?;
                    let value = match *argument_count {
                        1 => math_unary(native, numbers[0])?,
                        2 => math_binary(native, numbers[0], numbers[1])?,
                        _ => return None,
                    };
                    push_runtime(&mut stack, &mut stack_len, RuntimeValue::Number(value))?;
                    ip += 1;
                }
                GraphOp::Jump(target) => ip = self.jump_to_op(*target)?,
                GraphOp::JumpIfFalse(target) => {
                    if !runtime_truthy(context, *stack.get(stack_len.checked_sub(1)?)?)? {
                        ip = self.jump_to_op(*target)?;
                    } else {
                        ip += 1;
                    }
                }
                GraphOp::Return => {
                    let value = runtime_number(context, pop_runtime(&stack, &mut stack_len)?)?;
                    return (stack_len == 0).then_some(value);
                }
            }
        }
    }
}

impl PreparedCallGraph {
    pub(super) fn prepare(function: &Function) -> Option<Self> {
        let mut active = Vec::with_capacity(MAX_GRAPH_DEPTH);
        let mut remaining = MAX_PREPARED_GRAPH_NODES;
        Self::prepare_inner(function, &mut active, &mut remaining)
    }

    fn prepare_inner(
        function: &Function,
        active: &mut Vec<Function>,
        remaining: &mut usize,
    ) -> Option<Self> {
        if active.len() == MAX_GRAPH_DEPTH
            || active.iter().any(|candidate| candidate.ptr_eq(function))
            || function.native.is_some()
            || !is_direct_leaf_function(&Value::Function(function.clone()))
            || function.params.rest.is_some()
            || function
                .params
                .positional
                .iter()
                .any(|parameter| parameter.default.is_some())
        {
            return None;
        }
        *remaining = remaining.checked_sub(1)?;
        let bytecode = function.bytecode.as_ref()?;
        let plan = CallGraphPlan::compile(bytecode)?;
        if bytecode.parameter_slots().len() != plan.parameter_count
            || bytecode.received_upvalue_slots().len() != plan.capture_uses.len()
            || function.params.positional.len() != plan.parameter_count
            || function.upvalues.len() != plan.capture_uses.len()
        {
            return None;
        }

        active.push(function.clone());
        let prepared = (|| {
            let context = PreparedContext::prepare(function, &plan)?;
            let mut calls = Vec::with_capacity(plan.call_sites.len());
            for site in &plan.call_sites {
                let Value::Function(callee) = context.captures.get(site.capture)? else {
                    return None;
                };
                if callee.params.rest.is_some()
                    || callee.params.positional.len() != usize::from(site.argument_count)
                    || callee
                        .params
                        .positional
                        .iter()
                        .any(|parameter| parameter.default.is_some())
                {
                    return None;
                }
                calls.push(Self::prepare_inner(callee, active, remaining)?);
            }
            Some(Self {
                plan,
                context,
                calls,
            })
        })();
        active.pop();
        prepared
    }

    // This boundary is intentionally never inlined into the ordinary typed-loop
    // dispatcher. The first rejected integration widened and outlined its
    // existing arithmetic helper, charging every non-graph loop for this tier.
    #[inline(never)]
    pub(super) fn eval(&self, first: Typed, second: Typed, arity: u8) -> Option<Typed> {
        if usize::from(arity) != self.plan.parameter_count {
            return None;
        }
        let mut numbers = [0.0; MAX_GRAPH_PARAMETERS];
        if arity > 0 {
            numbers[0] = first.number()?;
        }
        if arity > 1 {
            numbers[1] = second.number()?;
        }
        let value = self
            .eval_numbers(&numbers[..usize::from(arity)])
            .map(Typed::Number)?;
        #[cfg(test)]
        EVAL_HITS.with(|hits| hits.set(hits.get() + 1));
        Some(value)
    }

    fn eval_numbers(&self, arguments: &[f64]) -> Option<f64> {
        self.plan.execute(&self.context, &self.calls, arguments)
    }
}

impl PreparedContext {
    fn prepare(function: &Function, plan: &CallGraphPlan) -> Option<Self> {
        let captures = function
            .upvalues
            .iter()
            .map(crate::function::Upvalue::get)
            .collect::<Vec<_>>();
        if captures.len() != plan.capture_uses.len() {
            return None;
        }
        for (value, usage) in captures.iter().zip(&plan.capture_uses) {
            if usage.number && !matches!(value, Value::Number(_)) {
                return None;
            }
        }

        let mut math_properties = Vec::with_capacity(plan.math_properties.len());
        if plan.uses_math_global {
            if function.has_dynamic_function_realm
                || function.has_dynamic_function_realm_override.get()
            {
                return None;
            }
            let Value::Object(math) = function.realm.as_ref()?.get_value("Math")? else {
                return None;
            };
            for key in &plan.math_properties {
                let OwnDataPropertyRead::Data(value) = math.own_data_property_read(key) else {
                    return None;
                };
                math_properties.push(value);
            }
        }
        if math_properties.len() != plan.math_properties.len() {
            return None;
        }
        for (value, usage) in math_properties.iter().zip(&plan.math_uses) {
            if usage.number && !matches!(value, Value::Number(_)) {
                return None;
            }
            if let Some(arity) = usage.call_arity {
                let Value::Function(function) = value else {
                    return None;
                };
                let native = function.native?;
                match arity {
                    1 if math_unary(native, 0.0).is_some() => {}
                    2 if math_binary(native, 0.0, 0.0).is_some() => {}
                    _ => return None,
                }
            }
        }

        Some(Self {
            captures,
            math_properties,
        })
    }
}

fn enqueue_state(
    states: &mut [Option<Vec<GraphStackValue>>],
    queue: &mut VecDeque<usize>,
    index: usize,
    incoming: Vec<GraphStackValue>,
) -> Option<()> {
    let state = states.get_mut(index)?;
    if let Some(existing) = state {
        return (existing == &incoming).then_some(());
    }
    *state = Some(incoming);
    queue.push_back(index);
    Some(())
}

fn push_static(stack: &mut Vec<GraphStackValue>, value: GraphStackValue) -> Option<()> {
    (stack.len() < MAX_GRAPH_STACK).then_some(())?;
    stack.push(value);
    Some(())
}

fn emit_graph_op(
    source_to_op: &mut [Option<usize>],
    source: usize,
    ops: &mut Vec<GraphOp>,
    op: GraphOp,
) -> Option<()> {
    (ops.len() < MAX_GRAPH_OPS).then_some(())?;
    *source_to_op.get_mut(source)? = Some(ops.len());
    ops.push(op);
    Some(())
}

fn intern_math_property(
    properties: &mut Vec<Rc<str>>,
    usages: &mut Vec<MathUse>,
    key: &Rc<str>,
) -> usize {
    if let Some(index) = properties.iter().position(|candidate| candidate == key) {
        return index;
    }
    properties.push(Rc::clone(key));
    usages.push(MathUse::default());
    properties.len() - 1
}

fn require_number(
    value: GraphStackValue,
    captures: &mut [CaptureUse],
    math: &mut [MathUse],
) -> Option<()> {
    match value {
        GraphStackValue::Scalar(GraphScalar::Number) => Some(()),
        GraphStackValue::Capture(index) => mark_capture_number(captures, index),
        GraphStackValue::MathProperty(index) => mark_math_number(math, index),
        GraphStackValue::Scalar(GraphScalar::Boolean) | GraphStackValue::MathObject => None,
    }
}

fn require_truthy(
    value: GraphStackValue,
    captures: &mut [CaptureUse],
    math: &mut [MathUse],
) -> Option<()> {
    match value {
        GraphStackValue::Scalar(GraphScalar::Number | GraphScalar::Boolean) => Some(()),
        GraphStackValue::Capture(index) => mark_capture_number(captures, index),
        GraphStackValue::MathProperty(index) => mark_math_number(math, index),
        GraphStackValue::MathObject => None,
    }
}

fn mark_capture_number(captures: &mut [CaptureUse], index: usize) -> Option<()> {
    let usage = captures.get_mut(index)?;
    usage.call_arity.is_none().then_some(())?;
    usage.number = true;
    Some(())
}

fn mark_math_number(math: &mut [MathUse], index: usize) -> Option<()> {
    let usage = math.get_mut(index)?;
    usage.call_arity.is_none().then_some(())?;
    usage.number = true;
    Some(())
}

fn mark_capture_call(captures: &mut [CaptureUse], index: usize, arity: u8) -> Option<()> {
    let usage = captures.get_mut(index)?;
    (!usage.number && usage.call_arity.is_none_or(|existing| existing == arity)).then_some(())?;
    usage.call_arity = Some(arity);
    Some(())
}

fn mark_math_call(math: &mut [MathUse], index: usize, arity: u8) -> Option<()> {
    let usage = math.get_mut(index)?;
    (!usage.number && usage.call_arity.is_none_or(|existing| existing == arity)).then_some(())?;
    usage.call_arity = Some(arity);
    Some(())
}

fn push_runtime(
    stack: &mut [RuntimeValue; MAX_GRAPH_STACK],
    stack_len: &mut usize,
    value: RuntimeValue,
) -> Option<()> {
    *stack.get_mut(*stack_len)? = value;
    *stack_len += 1;
    Some(())
}

fn pop_runtime(
    stack: &[RuntimeValue; MAX_GRAPH_STACK],
    stack_len: &mut usize,
) -> Option<RuntimeValue> {
    *stack_len = stack_len.checked_sub(1)?;
    stack.get(*stack_len).copied()
}

fn runtime_number(context: &PreparedContext, value: RuntimeValue) -> Option<f64> {
    match value {
        RuntimeValue::Number(value) => Some(value),
        RuntimeValue::Capture(index) => match context.captures.get(index)? {
            Value::Number(value) => Some(*value),
            _ => None,
        },
        RuntimeValue::MathProperty(index) => match context.math_properties.get(index)? {
            Value::Number(value) => Some(*value),
            _ => None,
        },
        RuntimeValue::Empty | RuntimeValue::Boolean(_) | RuntimeValue::MathObject => None,
    }
}

fn runtime_truthy(context: &PreparedContext, value: RuntimeValue) -> Option<bool> {
    match value {
        RuntimeValue::Boolean(value) => Some(value),
        value => {
            let number = runtime_number(context, value)?;
            Some(number != 0.0 && !number.is_nan())
        }
    }
}

/// Arithmetic private to the prepared graph evaluator.
///
/// Keep this separate from `execute::typed_binary`: changing that existing
/// helper's visibility was enough to make LLVM outline every ordinary typed
/// numeric operation in the rejected predecessor. The duplicated match is
/// deliberately small, semantics-identical, and isolated behind `eval`'s
/// out-of-line boundary.
fn graph_binary(left: f64, op: BinaryOp, right: f64) -> Option<RuntimeValue> {
    let value = match op {
        BinaryOp::Add => RuntimeValue::Number(left + right),
        BinaryOp::Sub => RuntimeValue::Number(left - right),
        BinaryOp::Mul => RuntimeValue::Number(left * right),
        BinaryOp::Div => RuntimeValue::Number(left / right),
        BinaryOp::Rem => RuntimeValue::Number(crate::operations::number_remainder(left, right)),
        BinaryOp::Pow => RuntimeValue::Number(crate::operations::number_exponentiate(left, right)),
        BinaryOp::Shl => RuntimeValue::Number(f64::from(
            crate::conversion::to_int32_number(left)
                << (crate::conversion::to_uint32_number(right) & 0x1f),
        )),
        BinaryOp::Shr => RuntimeValue::Number(f64::from(
            crate::conversion::to_int32_number(left)
                >> (crate::conversion::to_uint32_number(right) & 0x1f),
        )),
        BinaryOp::UShr => RuntimeValue::Number(f64::from(
            crate::conversion::to_uint32_number(left)
                >> (crate::conversion::to_uint32_number(right) & 0x1f),
        )),
        BinaryOp::BitwiseAnd => RuntimeValue::Number(f64::from(
            crate::conversion::to_int32_number(left) & crate::conversion::to_int32_number(right),
        )),
        BinaryOp::BitwiseOr => RuntimeValue::Number(f64::from(
            crate::conversion::to_int32_number(left) | crate::conversion::to_int32_number(right),
        )),
        BinaryOp::BitwiseXor => RuntimeValue::Number(f64::from(
            crate::conversion::to_int32_number(left) ^ crate::conversion::to_int32_number(right),
        )),
        BinaryOp::Lt => RuntimeValue::Boolean(left < right),
        BinaryOp::Le => RuntimeValue::Boolean(left <= right),
        BinaryOp::Gt => RuntimeValue::Boolean(left > right),
        BinaryOp::Ge => RuntimeValue::Boolean(left >= right),
        BinaryOp::Eq | BinaryOp::StrictEq => RuntimeValue::Boolean(left == right),
        BinaryOp::Ne | BinaryOp::StrictNe => RuntimeValue::Boolean(left != right),
        _ => return None,
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::{RuntimeValue, graph_binary};
    use qjs_ast::BinaryOp;

    fn number(value: Option<RuntimeValue>) -> f64 {
        let Some(RuntimeValue::Number(value)) = value else {
            panic!("expected a graph Number");
        };
        value
    }

    #[test]
    fn graph_binary_preserves_typed_loop_number_semantics() {
        assert_eq!(number(graph_binary(5.0, BinaryOp::Add, 2.0)), 7.0);
        assert_eq!(number(graph_binary(5.0, BinaryOp::Sub, 2.0)), 3.0);
        assert_eq!(number(graph_binary(5.0, BinaryOp::Mul, 2.0)), 10.0);
        assert_eq!(number(graph_binary(5.0, BinaryOp::Div, 2.0)), 2.5);
        assert_eq!(
            number(graph_binary(-5.0, BinaryOp::Rem, 2.0)),
            crate::operations::number_remainder(-5.0, 2.0)
        );
        assert_eq!(
            number(graph_binary(-2.0, BinaryOp::Pow, 3.0)),
            crate::operations::number_exponentiate(-2.0, 3.0)
        );
        assert!(number(graph_binary(-0.0, BinaryOp::Mul, 2.0)).is_sign_negative());

        assert_eq!(number(graph_binary(-1.0, BinaryOp::Shl, 1.0)), -2.0);
        assert_eq!(number(graph_binary(-2.0, BinaryOp::Shr, 1.0)), -1.0);
        assert_eq!(
            number(graph_binary(-1.0, BinaryOp::UShr, 1.0)),
            2_147_483_647.0
        );
        assert_eq!(number(graph_binary(6.0, BinaryOp::BitwiseAnd, 3.0)), 2.0);
        assert_eq!(number(graph_binary(6.0, BinaryOp::BitwiseOr, 3.0)), 7.0);
        assert_eq!(number(graph_binary(6.0, BinaryOp::BitwiseXor, 3.0)), 5.0);

        assert_eq!(
            graph_binary(1.0, BinaryOp::Lt, 2.0),
            Some(RuntimeValue::Boolean(true))
        );
        assert_eq!(
            graph_binary(2.0, BinaryOp::Le, 2.0),
            Some(RuntimeValue::Boolean(true))
        );
        assert_eq!(
            graph_binary(3.0, BinaryOp::Gt, 2.0),
            Some(RuntimeValue::Boolean(true))
        );
        assert_eq!(
            graph_binary(2.0, BinaryOp::Ge, 2.0),
            Some(RuntimeValue::Boolean(true))
        );
        assert_eq!(
            graph_binary(-0.0, BinaryOp::StrictEq, 0.0),
            Some(RuntimeValue::Boolean(true))
        );
        assert_eq!(
            graph_binary(f64::NAN, BinaryOp::Eq, f64::NAN),
            Some(RuntimeValue::Boolean(false))
        );
        assert_eq!(
            graph_binary(f64::NAN, BinaryOp::Ne, f64::NAN),
            Some(RuntimeValue::Boolean(true))
        );
    }
}
