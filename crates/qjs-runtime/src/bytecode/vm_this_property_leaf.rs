//! Closed-form evaluation of receiver-property function bodies.
//!
//! A method whose whole body reads own data properties of its receiver or its
//! arguments, combines them with constants using ordinary numeric operators,
//! and returns the result, can be answered without building a frame at all.
//! This is the tier that makes `function (value) { return value + this.step; }`
//! cost a property read instead of a call.
//!
//! Every admission is conservative: the plan is compiled from the body's
//! opcodes, and at evaluation time any receiver, property, or argument that is
//! not an ordinary number declines to the general direct-leaf path, which then
//! produces exactly the result it would have produced on its own.

use std::rc::Rc;

use qjs_ast::BinaryOp;

use crate::{Value, value::OwnDataPropertyRead};

use super::ir::{Bytecode, Op};
use super::named_property_cache::NamedPropertyCache;
use super::vm_numeric_leaf::{MAX_FAST_STACK, number_binary, pop_number, push_number};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;
    use std::collections::HashMap;

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
