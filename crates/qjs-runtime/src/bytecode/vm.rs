use std::collections::HashMap;

use qjs_ast::BinaryOp;

use crate::{
    GLOBAL_THIS_BINDING, ObjectRef, RuntimeError, Value, error_value, initialize_builtins,
    is_truthy, operations,
};

use super::ir::{Bytecode, Op};
use super::util::{stack_underflow, typeof_value};

#[derive(Clone)]
enum Slot {
    Uninitialized,
    Value(Value),
}

pub(super) fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode);
    vm.run()
}

struct Vm<'a> {
    bytecode: &'a Bytecode,
    ip: usize,
    stack: Vec<Value>,
    locals: Vec<Slot>,
    globals: HashMap<String, Value>,
}

impl<'a> Vm<'a> {
    fn new(bytecode: &'a Bytecode) -> Self {
        let mut globals = HashMap::new();
        let global_this = Value::Object(ObjectRef::new(HashMap::new()));
        globals.insert("this".to_owned(), global_this.clone());
        globals.insert(GLOBAL_THIS_BINDING.to_owned(), global_this.clone());
        globals.insert("undefined".to_owned(), Value::Undefined);
        initialize_builtins(&mut globals, &global_this);
        Self {
            bytecode,
            ip: 0,
            stack: Vec::with_capacity(64),
            locals: bytecode
                .locals
                .iter()
                .map(|local| {
                    if local.hoisted {
                        Slot::Value(Value::Undefined)
                    } else {
                        Slot::Uninitialized
                    }
                })
                .collect(),
            globals,
        }
    }

    fn run(&mut self) -> Result<Value, RuntimeError> {
        loop {
            let op = self
                .bytecode
                .code
                .get(self.ip)
                .cloned()
                .ok_or_else(|| RuntimeError {
                    message: "bytecode instruction pointer out of bounds".to_owned(),
                })?;
            self.ip += 1;
            match op {
                Op::LoadConst(index) => {
                    self.stack
                        .push(self.bytecode.constants.get(index).cloned().ok_or_else(|| {
                            RuntimeError {
                                message: "bytecode constant index out of bounds".to_owned(),
                            }
                        })?)
                }
                Op::LoadLocal(slot) => self.stack.push(self.load_local(slot)?),
                Op::StoreLocal(slot) => {
                    let value = self.pop()?;
                    self.store_local(slot, value)?;
                }
                Op::LoadGlobal(name) => {
                    let value = self
                        .globals
                        .get(&name)
                        .cloned()
                        .ok_or_else(|| RuntimeError {
                            message: format!("undefined identifier `{name}`"),
                        })?;
                    self.stack.push(value);
                }
                Op::Pop => {
                    self.pop()?;
                }
                Op::Dup => {
                    let value = self.stack.last().cloned().ok_or_else(stack_underflow)?;
                    self.stack.push(value);
                }
                Op::Typeof => {
                    let value = self.pop()?;
                    self.stack.push(Value::String(typeof_value(value)));
                }
                Op::Unary(op) => {
                    let value = self.pop()?;
                    self.stack.push(operations::eval_unary(op, value)?);
                }
                Op::Binary(op) => self.eval_binary(op)?,
                Op::Jump(target) => self.ip = target,
                Op::JumpIfFalse(target) => {
                    if !is_truthy(self.stack.last().ok_or_else(stack_underflow)?) {
                        self.ip = target;
                    }
                }
                Op::JumpIfTrue(target) => {
                    if is_truthy(self.stack.last().ok_or_else(stack_underflow)?) {
                        self.ip = target;
                    }
                }
                Op::JumpIfNotNullish(target) => {
                    if !matches!(self.stack.last(), Some(Value::Null | Value::Undefined)) {
                        self.ip = target;
                    }
                }
                Op::Return => return Ok(self.stack.pop().unwrap_or(Value::Undefined)),
                Op::Throw => {
                    let value = self.pop()?;
                    return Err(RuntimeError {
                        message: format!("throw statement executed: {}", error_value(value)),
                    });
                }
            }
        }
    }

    fn eval_binary(&mut self, op: BinaryOp) -> Result<(), RuntimeError> {
        let right = self.pop()?;
        let left = self.pop()?;
        if let Some(value) = fast_number_binary(&left, op, &right) {
            self.stack.push(value);
            return Ok(());
        }
        self.stack
            .push(operations::eval_binary(left, op, right, &self.globals)?);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(stack_underflow)
    }

    fn load_local(&self, slot: usize) -> Result<Value, RuntimeError> {
        match self.locals.get(slot) {
            Some(Slot::Value(value)) => Ok(value.clone()),
            Some(Slot::Uninitialized) => Err(RuntimeError {
                message: format!("undefined identifier `{}`", self.bytecode.locals[slot].name),
            }),
            None => Err(RuntimeError {
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    fn store_local(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        *local = Slot::Value(value);
        Ok(())
    }
}

fn fast_number_binary(left: &Value, op: BinaryOp, right: &Value) -> Option<Value> {
    let (Value::Number(left), Value::Number(right)) = (left, right) else {
        return None;
    };
    let value = match op {
        BinaryOp::Add => Value::Number(left + right),
        BinaryOp::Sub => Value::Number(left - right),
        BinaryOp::Mul => Value::Number(left * right),
        BinaryOp::Div => Value::Number(left / right),
        BinaryOp::Rem => Value::Number(left % right),
        BinaryOp::Pow => Value::Number(left.powf(*right)),
        BinaryOp::Lt => Value::Boolean(left < right),
        BinaryOp::Le => Value::Boolean(left <= right),
        BinaryOp::Gt => Value::Boolean(left > right),
        BinaryOp::Ge => Value::Boolean(left >= right),
        BinaryOp::StrictEq => Value::Boolean(left == right),
        BinaryOp::StrictNe => Value::Boolean(left != right),
        _ => return None,
    };
    Some(value)
}
