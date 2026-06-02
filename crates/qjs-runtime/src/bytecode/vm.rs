use std::collections::HashMap;

use crate::{
    ArrayRef, Function, GLOBAL_THIS_BINDING, ObjectRef, RUNTIME_INTRINSIC_NAMES, RuntimeError,
    Value, call_function, constructor_prototype, error, initialize_builtins, is_truthy,
    object_prototype, operations, to_property_key,
};

use super::ir::{Bytecode, Op};
use super::util::{stack_underflow, typeof_value};
use super::vm_call::{insert_scope_call_bindings, native_error_message, user_bytecode_function};
use super::vm_props::{delete_property, get_property, set_property};
use super::vm_result::FunctionBytecodeResult;
use super::vm_try::TryFrame;

pub(super) type Slot = Option<Value>;

struct VmCallEnv {
    env: HashMap<String, Value>,
    binding_names: Option<Vec<String>>,
}

pub(super) fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode);
    vm.run()
}

pub(super) fn eval_function_bytecode(
    bytecode: &Bytecode,
    env: HashMap<String, Value>,
) -> FunctionBytecodeResult<'_> {
    let mut vm = Vm::new_with_globals(bytecode, env);
    let value = vm.run();
    FunctionBytecodeResult {
        value,
        bytecode,
        globals: vm.globals,
        locals: vm.locals,
    }
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
                    Some(value.clone())
                } else if local.hoisted {
                    Some(Value::Undefined)
                } else {
                    None
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
                    thrown: None,
                    message: "bytecode instruction pointer out of bounds".to_owned(),
                })?;
            self.ip += 1;
            match op {
                Op::LoadConst(index) => {
                    self.stack
                        .push(self.bytecode.constants.get(index).cloned().ok_or_else(|| {
                            RuntimeError {
                                thrown: None,
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
                            thrown: None,
                            message: format!("ReferenceError: undefined identifier `{name}`"),
                        })?;
                    self.stack.push(value);
                }
                Op::TypeofGlobal(name) => {
                    let value = self.globals.get(&name).cloned().unwrap_or(Value::Undefined);
                    self.stack.push(Value::String(typeof_value(value)));
                }
                Op::Pop => {
                    self.pop()?;
                }
                Op::Dup => {
                    let value = self.stack.last().cloned().ok_or_else(stack_underflow)?;
                    self.stack.push(value);
                }
                Op::NewArray { count, holes } => self.new_array(count, holes)?,
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
                    local_names,
                    bytecode,
                    constructable,
                    is_strict,
                } => {
                    let env = self.function_capture_env(&bytecode, &local_names);
                    self.stack.push(Value::Function(Function::new_user_compiled(
                        name,
                        params,
                        env,
                        bytecode,
                        local_names,
                        constructable,
                        is_strict,
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

    fn function_capture_env(
        &self,
        function_bytecode: &Bytecode,
        function_local_names: &[String],
    ) -> HashMap<String, Value> {
        let mut env =
            HashMap::with_capacity(RUNTIME_INTRINSIC_NAMES.len() + function_bytecode.locals.len());
        self.insert_runtime_intrinsics(&mut env);
        for name in function_bytecode.global_names() {
            self.insert_referenced_binding(&mut env, name);
        }
        for name in function_bytecode.local_names() {
            if function_local_names
                .binary_search_by(|local| local.as_str().cmp(name))
                .is_err()
            {
                self.insert_referenced_binding(&mut env, name);
            }
        }
        env
    }

    fn insert_runtime_intrinsics(&self, env: &mut HashMap<String, Value>) {
        for name in RUNTIME_INTRINSIC_NAMES {
            if let Some(value) = self.globals.get(*name) {
                env.insert((*name).to_owned(), value.clone());
            }
        }
    }

    fn insert_referenced_binding(&self, env: &mut HashMap<String, Value>, name: &str) {
        if let Some(value) = self.current_local_binding(name) {
            env.insert(name.to_owned(), value.clone());
        } else if let Some(value) = self.globals.get(name) {
            env.insert(name.to_owned(), value.clone());
        }
    }

    fn current_local_binding(&self, name: &str) -> Option<&Value> {
        self.bytecode
            .local_slot(name)
            .and_then(|index| self.locals.get(index))
            .and_then(Option::as_ref)
    }

    fn new_array(&mut self, count: usize, holes: Vec<usize>) -> Result<(), RuntimeError> {
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            values.push(self.pop()?);
        }
        values.reverse();
        self.stack
            .push(Value::Array(ArrayRef::new_sparse(values, holes)));
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
        self.stack
            .push(get_property(object, &key, &mut self.globals)?);
        Ok(())
    }

    fn set_prop(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let key = to_property_key(self.pop()?)?;
        let object = self.pop()?;
        set_property(object, key, value.clone(), &mut self.globals)?;
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
        let mut env = self.call_env(&callee);
        let result = call_function(callee, Value::Undefined, arguments, &mut env.env, false);
        self.apply_call_env(env);
        if let Some(result) = self.handle_call_result(result)? {
            self.stack.push(result);
        }
        Ok(())
    }

    fn call_method(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let key = to_property_key(self.pop()?)?;
        let this_value = self.pop()?;
        let callee = get_property(this_value.clone(), &key, &mut self.globals)?;
        let mut env = self.call_env(&callee);
        let result = call_function(callee, this_value, arguments, &mut env.env, false);
        self.apply_call_env(env);
        if let Some(result) = self.handle_call_result(result)? {
            self.stack.push(result);
        }
        Ok(())
    }

    fn construct(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        let Value::Function(function) = &callee else {
            return Err(RuntimeError {
                thrown: None,
                message: "value is not a constructor".to_owned(),
            });
        };
        if !function.constructable {
            return Err(RuntimeError {
                thrown: None,
                message: "value is not a constructor".to_owned(),
            });
        }
        let prototype = constructor_prototype(&callee, &self.globals);
        let this_value = Value::Object(ObjectRef::with_prototype(HashMap::new(), prototype));
        let mut env = self.call_env(&callee);
        let result = call_function(callee, this_value.clone(), arguments, &mut env.env, true);
        self.apply_call_env(env);
        if let Some(result) = self.handle_call_result(result)? {
            match result {
                Value::Array(_) | Value::Function(_) | Value::Object(_) => self.stack.push(result),
                _ => self.stack.push(this_value),
            }
        }
        Ok(())
    }

    fn handle_call_result(
        &mut self,
        result: Result<Value, RuntimeError>,
    ) -> Result<Option<Value>, RuntimeError> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error) if self.should_rethrow_js_error(&error) => {
                let value = error.thrown.as_deref().cloned().unwrap_or_else(|| {
                    Value::String(
                        error
                            .message
                            .trim_start_matches("throw statement executed: ")
                            .to_owned(),
                    )
                });
                self.throw_value(value)?;
                Ok(None)
            }
            Err(error) if self.should_throw_native_error(&error) => {
                let value = self.native_error_value(&error.message)?;
                self.throw_value(value)?;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn should_rethrow_js_error(&self, error: &RuntimeError) -> bool {
        !self.try_stack.is_empty()
            && (error.thrown.is_some() || error.message.starts_with("throw statement executed: "))
    }

    fn should_throw_native_error(&self, error: &RuntimeError) -> bool {
        !self.try_stack.is_empty() && !error.message.starts_with("throw statement executed:")
    }

    fn native_error_value(&self, message: &str) -> Result<Value, RuntimeError> {
        let (constructor_name, message) = native_error_message(message);
        let Value::Function(function) = self
            .native_error_constructor(constructor_name)
            .ok_or_else(|| RuntimeError {
                thrown: None,
                message: format!("{constructor_name} constructor is not available"),
            })?
        else {
            return Err(RuntimeError {
                thrown: None,
                message: format!("{constructor_name} constructor is not callable"),
            });
        };
        error::native_error(
            &function,
            Value::Undefined,
            &[Value::String(message)],
            false,
        )
    }

    fn native_error_constructor(&self, name: &str) -> Option<Value> {
        self.globals.get(name).cloned().or_else(|| {
            let Some(Value::Object(global_this)) = self.globals.get(GLOBAL_THIS_BINDING) else {
                return None;
            };
            global_this.get(name)
        })
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
            if let Some(value) = local {
                env.insert(self.bytecode.locals[index].name.clone(), value.clone());
            }
        }
        env
    }

    fn call_env(&self, callee: &Value) -> VmCallEnv {
        if let Some(function) = user_bytecode_function(callee) {
            let mut env = HashMap::with_capacity(RUNTIME_INTRINSIC_NAMES.len());
            self.insert_runtime_intrinsics(&mut env);
            let mut binding_names = Vec::new();
            if let Some(bytecode) = &function.bytecode {
                self.insert_referenced_call_bindings(
                    &mut env,
                    &mut binding_names,
                    bytecode,
                    &function.local_names,
                );
            }
            insert_scope_call_bindings(
                &mut env,
                &mut binding_names,
                self.bytecode,
                &self.locals,
                &self.globals,
                &function.local_names,
            );
            return VmCallEnv {
                env,
                binding_names: Some(binding_names),
            };
        }
        VmCallEnv {
            env: self.current_env(),
            binding_names: None,
        }
    }

    fn insert_referenced_call_bindings(
        &self,
        env: &mut HashMap<String, Value>,
        binding_names: &mut Vec<String>,
        function_bytecode: &Bytecode,
        function_local_names: &[String],
    ) {
        for name in function_bytecode.global_names() {
            self.insert_call_binding(env, binding_names, name);
        }
        for name in function_bytecode.local_names() {
            if function_local_names
                .binary_search_by(|local| local.as_str().cmp(name))
                .is_err()
            {
                self.insert_call_binding(env, binding_names, name);
            }
        }
    }

    fn insert_call_binding(
        &self,
        env: &mut HashMap<String, Value>,
        binding_names: &mut Vec<String>,
        name: &str,
    ) {
        if let Some(value) = self.current_local_binding(name) {
            env.insert(name.to_owned(), value.clone());
            if !binding_names.iter().any(|existing| existing == name) {
                binding_names.push(name.to_owned());
            }
        } else if let Some(value) = self.globals.get(name) {
            env.insert(name.to_owned(), value.clone());
        }
    }

    fn apply_call_env(&mut self, env: VmCallEnv) {
        if let Some(binding_names) = env.binding_names {
            self.apply_selected_env(env.env, &binding_names);
        } else {
            self.apply_env(env.env);
        }
    }

    fn apply_selected_env(&mut self, env: HashMap<String, Value>, binding_names: &[String]) {
        for name in binding_names {
            let Some(value) = env.get(name) else {
                continue;
            };
            if let Some(index) = self.bytecode.local_slot(name) {
                self.locals[index] = Some(value.clone());
            } else {
                self.globals.insert(name.clone(), value.clone());
            }
        }
    }

    fn apply_env(&mut self, env: HashMap<String, Value>) {
        for (index, local) in self.bytecode.locals.iter().enumerate() {
            if let Some(value) = env.get(&local.name) {
                self.locals[index] = Some(value.clone());
            }
        }
        for (name, value) in env {
            if self.bytecode.local_slot(&name).is_none() {
                self.globals.insert(name, value);
            }
        }
    }

    pub(super) fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(stack_underflow)
    }

    fn load_local(&self, slot: usize) -> Result<Value, RuntimeError> {
        match self.locals.get(slot) {
            Some(Some(value)) => Ok(value.clone()),
            Some(None) => Err(RuntimeError {
                thrown: None,
                message: format!(
                    "ReferenceError: undefined identifier `{}`",
                    self.bytecode.locals[slot].name
                ),
            }),
            None => Err(RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    fn store_local(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        *local = Some(value);
        Ok(())
    }
}
