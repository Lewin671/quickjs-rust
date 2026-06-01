use std::collections::HashMap;

use crate::{
    ArrayRef, Function, GLOBAL_THIS_BINDING, ObjectRef, RuntimeError, Value, call_function,
    constructor_prototype, initialize_builtins, is_truthy, object_prototype, operations,
    to_property_key,
};

use super::ir::{Bytecode, Op};
use super::util::{stack_underflow, typeof_value};
use super::vm_props::{delete_property, get_property, set_property};
use super::vm_try::TryFrame;

#[derive(Clone)]
pub(super) enum Slot {
    Uninitialized,
    Value(Value),
}

pub(super) fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode);
    vm.run()
}

pub(super) fn eval_function_bytecode(
    bytecode: &Bytecode,
    env: HashMap<String, Value>,
) -> Result<(Value, HashMap<String, Value>), RuntimeError> {
    let mut vm = Vm::new_with_globals(bytecode, env);
    let value = vm.run()?;
    Ok((value, vm.current_env()))
}

pub(super) struct Vm<'a> {
    pub(super) bytecode: &'a Bytecode,
    pub(super) ip: usize,
    pub(super) stack: Vec<Value>,
    pub(super) locals: Vec<Slot>,
    pub(super) globals: HashMap<String, Value>,
    pub(super) try_stack: Vec<TryFrame>,
    pub(super) pending_throw: Option<Value>,
    pub(super) pending_return: Option<Value>,
}

impl<'a> Vm<'a> {
    fn new(bytecode: &'a Bytecode) -> Self {
        let mut globals = HashMap::new();
        let global_this = Value::Object(ObjectRef::new(HashMap::new()));
        globals.insert("this".to_owned(), global_this.clone());
        globals.insert(GLOBAL_THIS_BINDING.to_owned(), global_this.clone());
        globals.insert("undefined".to_owned(), Value::Undefined);
        initialize_builtins(&mut globals, &global_this);
        Self::new_with_globals(bytecode, globals)
    }

    fn new_with_globals(bytecode: &'a Bytecode, globals: HashMap<String, Value>) -> Self {
        Self {
            bytecode,
            ip: 0,
            stack: Vec::with_capacity(64),
            locals: Self::initial_slots(bytecode, &globals),
            globals,
            try_stack: Vec::new(),
            pending_throw: None,
            pending_return: None,
        }
    }

    fn initial_slots(bytecode: &Bytecode, globals: &HashMap<String, Value>) -> Vec<Slot> {
        bytecode
            .locals
            .iter()
            .map(|local| {
                if let Some(value) = globals.get(&local.name) {
                    Slot::Value(value.clone())
                } else if local.hoisted {
                    Slot::Value(Value::Undefined)
                } else {
                    Slot::Uninitialized
                }
            })
            .collect()
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
                Op::NewArray(count) => self.new_array(count)?,
                Op::NewObject(count) => self.new_object(count)?,
                Op::EnumerateKeys => self.enumerate_keys()?,
                Op::GetProp => self.get_prop()?,
                Op::SetProp => self.set_prop()?,
                Op::DeleteProp => self.delete_prop()?,
                Op::Call(argc) => self.call(argc)?,
                Op::CallMethod(argc) => self.call_method(argc)?,
                Op::New(argc) => self.construct(argc)?,
                Op::NewFunction {
                    name,
                    params,
                    body,
                    bytecode,
                    constructable,
                } => {
                    let env = self.current_env();
                    self.stack
                        .push(Value::Function(Function::new_user_with_bytecode(
                            name,
                            params,
                            body,
                            env,
                            Some(bytecode),
                            constructable,
                        )));
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
                Op::EnterTry { catch, finally } => self.enter_try(catch, finally),
                Op::ExitTry => self.exit_try(),
                Op::EndFinally => {
                    if let Some(value) = self.end_finally()? {
                        return Ok(value);
                    }
                }
                Op::Return => {
                    let value = self.stack.pop().unwrap_or(Value::Undefined);
                    if let Some(value) = self.return_value(value)? {
                        return Ok(value);
                    }
                }
                Op::Throw => {
                    let value = self.pop()?;
                    self.throw_value(value)?;
                }
            }
        }
    }

    fn new_array(&mut self, count: usize) -> Result<(), RuntimeError> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.pop()?);
        }
        values.reverse();
        self.stack.push(Value::Array(ArrayRef::new(values)));
        Ok(())
    }

    fn new_object(&mut self, count: usize) -> Result<(), RuntimeError> {
        let mut values = HashMap::new();
        for _ in 0..count {
            let value = self.pop()?;
            let key = to_property_key(self.pop()?)?;
            values.insert(key, value);
        }
        self.stack.push(Value::Object(ObjectRef::with_prototype(
            values,
            object_prototype(&self.globals),
        )));
        Ok(())
    }

    fn get_prop(&mut self) -> Result<(), RuntimeError> {
        let key = to_property_key(self.pop()?)?;
        let object = self.pop()?;
        self.stack.push(get_property(object, &key, &self.globals)?);
        Ok(())
    }

    fn set_prop(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let key = to_property_key(self.pop()?)?;
        let object = self.pop()?;
        set_property(object, key, value.clone())?;
        self.stack.push(value);
        Ok(())
    }

    fn delete_prop(&mut self) -> Result<(), RuntimeError> {
        let key = to_property_key(self.pop()?)?;
        let object = self.pop()?;
        self.stack.push(delete_property(object, &key)?);
        Ok(())
    }

    fn call(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        let this_value = self
            .globals
            .get(GLOBAL_THIS_BINDING)
            .cloned()
            .unwrap_or(Value::Undefined);
        let result = call_function(callee, this_value, arguments, &mut self.globals, false)?;
        self.stack.push(result);
        Ok(())
    }

    fn call_method(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let key = to_property_key(self.pop()?)?;
        let this_value = self.pop()?;
        let callee = get_property(this_value.clone(), &key, &self.globals)?;
        let result = call_function(callee, this_value, arguments, &mut self.globals, false)?;
        self.stack.push(result);
        Ok(())
    }

    fn construct(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        let Value::Function(function) = &callee else {
            return Err(RuntimeError {
                message: "value is not a constructor".to_owned(),
            });
        };
        if !function.constructable {
            return Err(RuntimeError {
                message: "value is not a constructor".to_owned(),
            });
        }
        let prototype = constructor_prototype(&callee);
        let this_value = Value::Object(ObjectRef::with_prototype(HashMap::new(), prototype));
        let result = call_function(
            callee,
            this_value.clone(),
            arguments,
            &mut self.globals,
            true,
        )?;
        match result {
            Value::Array(_) | Value::Function(_) | Value::Object(_) => self.stack.push(result),
            _ => self.stack.push(this_value),
        }
        Ok(())
    }

    fn pop_arguments(&mut self, argc: usize) -> Result<Vec<Value>, RuntimeError> {
        let mut arguments = Vec::with_capacity(argc);
        for _ in 0..argc {
            arguments.push(self.pop()?);
        }
        arguments.reverse();
        Ok(arguments)
    }

    pub(super) fn current_env(&self) -> HashMap<String, Value> {
        let mut env = self.globals.clone();
        for (index, local) in self.locals.iter().enumerate() {
            if let Slot::Value(value) = local {
                env.insert(self.bytecode.locals[index].name.clone(), value.clone());
            }
        }
        env
    }

    pub(super) fn pop(&mut self) -> Result<Value, RuntimeError> {
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
