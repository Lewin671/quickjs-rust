use std::collections::HashMap;

use qjs_ast::ObjectPropertyKind;

use crate::{
    ArrayRef, CATCH_CAPTURE_PREFIX, CompiledFunctionInit, Function, GLOBAL_THIS_BINDING, ObjectRef,
    Property, RUNTIME_INTRINSIC_NAMES, RuntimeError, Value, call_function, constructor_prototype,
    initialize_builtins, is_truthy, object_prototype, operations, to_property_key_with_env,
};

use super::ir::{Bytecode, Op};
use super::util::{stack_underflow, typeof_value};
use super::vm_call::{insert_scope_call_bindings, user_bytecode_function};
use super::vm_name_ops::NameReference;
use super::vm_props::get_property;
use super::vm_try::TryFrame;

pub(super) type Slot = Option<Value>;
struct VmCallEnv {
    env: HashMap<String, Value>,
    binding_names: Option<Vec<String>>,
}

pub(super) fn property_base_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: cannot access property of null or undefined".to_owned(),
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
    pub(super) pending_jump: Option<usize>,
    pub(super) with_stack: Vec<Value>,
    pub(super) name_references: Vec<NameReference>,
    pub(super) binding_overrides: HashMap<String, Value>,
    pub(super) sync_var_to_global_object: bool,
}

impl<'a> Vm<'a> {
    pub(super) fn new(bytecode: &'a Bytecode) -> Self {
        let mut globals = HashMap::new();
        let global_this = Value::Object(ObjectRef::new(HashMap::new()));
        globals.insert("this".to_owned(), global_this.clone());
        globals.insert(GLOBAL_THIS_BINDING.to_owned(), global_this.clone());
        globals.insert("undefined".to_owned(), Value::Undefined);
        initialize_builtins(&mut globals, &global_this);
        Self::new_with_globals(bytecode, globals, true)
    }

    pub(super) fn new_with_globals(
        bytecode: &'a Bytecode,
        globals: HashMap<String, Value>,
        sync_var_to_global_object: bool,
    ) -> Self {
        Self {
            bytecode,
            ip: 0,
            stack: Vec::with_capacity(64),
            locals: Self::initial_slots(bytecode, &globals),
            globals,
            try_stack: Vec::new(),
            pending_throw: None,
            pending_return: None,
            pending_jump: None,
            with_stack: Vec::new(),
            name_references: Vec::new(),
            binding_overrides: HashMap::new(),
            sync_var_to_global_object,
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

    pub(super) fn run(&mut self) -> Result<Value, RuntimeError> {
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
                Op::LoadLocal(slot) => {
                    if let Some(value) = self.handle_runtime_result(self.load_local(slot))? {
                        self.stack.push(value);
                    }
                }
                Op::LoadLocalOrUndefined(slot) => {
                    self.stack.push(self.load_local_or_undefined(slot)?)
                }
                Op::StoreLocal(slot) => {
                    let value = self.pop()?;
                    self.store_local(slot, value)?;
                }
                Op::ClearLocal(slot) => {
                    self.clear_local(slot)?;
                }
                Op::LoadName(name) => {
                    let result = self.load_name(&name);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::StoreName { name, strict } => {
                    let value = self.pop()?;
                    let result = self.store_name(&name, value, strict);
                    self.handle_runtime_result(result)?;
                }
                Op::ResolveName(name) => self.resolve_name(&name),
                Op::LoadGlobal(name) => {
                    let value = self
                        .globals
                        .get(&name)
                        .cloned()
                        .ok_or_else(|| RuntimeError {
                            thrown: None,
                            message: format!("ReferenceError: undefined identifier `{name}`"),
                        });
                    if let Some(value) = self.handle_runtime_result(value)? {
                        self.stack.push(value);
                    }
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
                Op::ForOfValues => self.for_of_values()?,
                Op::NewObject(kinds) => self.new_object(&kinds)?,
                Op::EnumerateKeys => self.enumerate_keys()?,
                Op::CheckObjectCoercible => self.check_object_coercible()?,
                Op::ToPropertyKey => self.coerce_property_key()?,
                Op::GetProp => self.get_prop()?,
                Op::SetProp { strict } => self.set_prop(strict)?,
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
                        CompiledFunctionInit {
                            env,
                            with_stack: self.with_stack.clone(),
                            bytecode,
                            local_names,
                            constructable,
                            is_strict,
                        },
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
                Op::Binary(op) => {
                    let result = self.eval_binary(op);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::Jump(target) => self.ip = target,
                Op::JumpAbrupt(target) => self.jump_abrupt(target)?,
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
                Op::JumpIfNotUndefined(target) => {
                    if !matches!(self.stack.last(), Some(Value::Undefined)) {
                        self.ip = target;
                    }
                }
                Op::IteratorCloseForThrow(iterator_slot) => {
                    self.iterator_close_for_throw(iterator_slot)?;
                }
                Op::EnterWith => self.enter_with()?,
                Op::ExitWith => self.exit_with()?,
                Op::EnterTry {
                    catch,
                    finally,
                    catch_scope,
                } => self.enter_try(catch, finally, catch_scope),
                Op::ExitTry => self.exit_try()?,
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
                Op::ThrowTypeError(message) => self.throw_type_error(message)?,
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
        let marker = catch_capture_marker(name);
        if let Some(value) = self.current_local_binding(&marker)
            && matches!(value, Value::Boolean(true))
        {
            env.insert(marker, value.clone());
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

    fn new_object(&mut self, kinds: &[ObjectPropertyKind]) -> Result<(), RuntimeError> {
        let object = ObjectRef::with_prototype(HashMap::new(), object_prototype(&self.globals));
        for kind in kinds.iter().rev() {
            let value = self.pop()?;
            let mut env = self.current_env();
            let key = to_property_key_with_env(self.pop()?, &mut env)?;
            self.apply_env(env);
            match kind {
                ObjectPropertyKind::Data => {
                    object.define_property(key, Property::enumerable(value))
                }
                ObjectPropertyKind::Getter => {
                    object.define_property(key, Property::accessor(Some(value), None, true, true))
                }
                ObjectPropertyKind::Setter => {
                    object.define_property(key, Property::accessor(None, Some(value), true, true))
                }
            }
        }
        self.stack.push(Value::Object(object));
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
        let key_value = self.pop()?;
        let this_value = self.pop()?;
        if matches!(this_value, Value::Null | Value::Undefined) {
            if self
                .handle_runtime_result::<()>(Err(property_base_error()))?
                .is_none()
            {
                return Ok(());
            }
            return Err(RuntimeError {
                thrown: None,
                message: "property base error did not throw".to_owned(),
            });
        }
        let mut key_env = self.current_env();
        let key_result = to_property_key_with_env(key_value, &mut key_env);
        self.apply_env(key_env);
        let Some(key) = self.handle_runtime_result(key_result)? else {
            return Ok(());
        };
        let mut property_env = self.current_env();
        let callee_result = get_property(this_value.clone(), &key, &mut property_env);
        self.apply_env(property_env);
        let Some(callee) = self.handle_runtime_result(callee_result)? else {
            return Ok(());
        };
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
            return self.handle_construct_error();
        };
        if !function.constructable {
            return self.handle_construct_error();
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

    fn handle_construct_error(&mut self) -> Result<(), RuntimeError> {
        self.handle_call_result(Err(RuntimeError {
            thrown: None,
            message: "TypeError: value is not a constructor".to_owned(),
        }))?;
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

    pub(super) fn apply_env(&mut self, env: HashMap<String, Value>) {
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
}

fn catch_capture_marker(name: &str) -> String {
    format!("{CATCH_CAPTURE_PREFIX}{name}")
}
