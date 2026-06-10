use std::{cell::RefCell, collections::HashMap, rc::Rc};

use qjs_ast::ObjectPropertyKind;

use crate::{
    ArrayRef, Function, GLOBAL_THIS_BINDING, NativeFunction, ObjectRef, Property, PropertyKey,
    RUNTIME_INTRINSIC_NAMES, RuntimeError, Value,
    array::{array_like_values_with_env, iterable_values_with_env},
    call_function, construct_function,
    function::CompiledUserFunction,
    initialize_builtins, is_truthy, object, object_prototype, promise, symbol,
    to_js_string_with_env, to_property_key_value,
};

use super::ir::{ArrayElementKind, Bytecode, Op};
use super::util::{stack_underflow, typeof_value};
use super::vm_call::{insert_scope_call_bindings, user_bytecode_function};
use super::vm_props::{
    delete_property_key, get_property_key, property_set_uses_setter, set_property_key,
};
use super::vm_result::FunctionBytecodeResult;
use super::vm_try::TryFrame;

pub(super) type Slot = Option<Value>;

struct VmCallEnv {
    env: HashMap<String, Value>,
    binding_names: Option<Vec<String>>,
}

pub(super) fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode);
    let value = vm.run()?;
    vm.drain_promise_jobs()?;
    Ok(value)
}

pub(super) fn eval_function_bytecode(
    bytecode: &Bytecode,
    env: HashMap<String, Value>,
    captured_env: Rc<RefCell<HashMap<String, Value>>>,
) -> FunctionBytecodeResult<'_> {
    let mut vm = Vm::new_with_globals_and_captures(bytecode, env, captured_env);
    let value = vm.run();
    FunctionBytecodeResult {
        value,
        bytecode,
        globals: vm.globals,
        locals: vm.locals,
        sloppy_global_names: vm.sloppy_global_names,
    }
}

pub(super) struct Vm<'a> {
    pub(super) bytecode: &'a Bytecode,
    pub(super) ip: usize,
    pub(super) stack: Vec<Value>,
    pub(super) locals: Vec<Slot>,
    pub(super) globals: HashMap<String, Value>,
    pub(super) captured_env: Rc<RefCell<HashMap<String, Value>>>,
    pub(super) sloppy_global_names: Vec<String>,
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
        Self::initialize_script_global_bindings(bytecode, &mut globals);
        let captured_env = Rc::new(RefCell::new(globals.clone()));
        Self::new_with_globals_and_captures(bytecode, globals, captured_env)
    }

    fn new_with_globals_and_captures(
        bytecode: &'a Bytecode,
        globals: HashMap<String, Value>,
        captured_env: Rc<RefCell<HashMap<String, Value>>>,
    ) -> Self {
        Self {
            bytecode,
            ip: 0,
            stack: Vec::with_capacity(64),
            locals: Self::initial_slots(bytecode, &globals),
            globals,
            captured_env,
            sloppy_global_names: Vec::new(),
            try_stack: Vec::new(),
            pending_throw: None,
            pending_return: None,
        }
    }

    fn coerce_property_key(&mut self, value: Value) -> Result<PropertyKey, RuntimeError> {
        match value {
            Value::Object(_)
            | Value::Function(_)
            | Value::Array(_)
            | Value::Map(_)
            | Value::Set(_) => {
                let mut key_env = self.current_env();
                let key = to_property_key_value(value, &mut key_env)?;
                self.apply_env(key_env);
                Ok(key)
            }
            value => to_property_key_value(value, &mut self.globals),
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
                Op::LoadLocalOrUndefined(slot) => {
                    self.stack.push(self.load_local_or_undefined(slot)?)
                }
                Op::StoreLocal(slot) => {
                    let value = self.pop()?;
                    let result = self.store_local(slot, value);
                    self.handle_runtime_result(result)?;
                }
                Op::ClearLocal(slot) => self.clear_local(slot)?,
                Op::DefineGlobalVar(name) => {
                    let value = self.pop()?;
                    let result = self.define_global_var(name, value);
                    self.handle_runtime_result(result)?;
                }
                Op::LoadGlobal(name) => {
                    let result = self.load_global(&name);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::StoreGlobalStrict(name) => {
                    let value = self.pop()?;
                    let result = self.store_global_strict(name, value);
                    self.handle_runtime_result(result)?;
                }
                Op::StoreLocalOrGlobalSloppy { slot, name } => {
                    let value = self.pop()?;
                    let result = self.store_local_or_global_sloppy(slot, name, value);
                    self.handle_runtime_result(result)?;
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
                Op::NewArray { elements } => self.new_array(&elements)?,
                Op::NewTemplateObject { cooked, raw } => self.new_template_object(&cooked, &raw),
                Op::NewObject(kinds) => self.new_object(&kinds)?,
                Op::EnumerateKeys => self.enumerate_keys()?,
                Op::GetIterator => self.get_iterator()?,
                Op::IteratorStep { done_slot } => self.iterator_step(done_slot)?,
                Op::IteratorRest { done_slot } => self.iterator_rest(done_slot)?,
                Op::ObjectRestExcluding { excluded } => self.object_rest_excluding(&excluded)?,
                Op::RequireObjectCoercible => self.require_object_coercible()?,
                Op::GetProp => {
                    let result = self.get_prop();
                    self.handle_runtime_result(result)?;
                }
                Op::SetProp { is_strict } => {
                    let result = self.set_prop(is_strict);
                    self.handle_runtime_result(result)?;
                }
                Op::DeleteProp => self.delete_prop()?,
                Op::Call(argc) => self.call(argc)?,
                Op::CallMethod(argc) => self.call_method(argc)?,
                Op::CallSpread => self.call_spread()?,
                Op::CallMethodSpread => self.call_method_spread()?,
                Op::IteratorClose { swallow } => self.iterator_close(swallow)?,
                Op::New(argc) => self.construct(argc)?,
                Op::NewSpread => self.construct_spread()?,
                Op::NewFunction {
                    name,
                    params,
                    local_names,
                    bytecode,
                    constructable,
                    is_strict,
                    lexical_this,
                    lexical_arguments,
                } => {
                    let env = self.function_capture_env(&bytecode, &local_names);
                    self.refresh_captured_env(&env);
                    self.stack.push(Value::Function(Function::new_user_compiled(
                        CompiledUserFunction {
                            name,
                            params,
                            env,
                            bytecode,
                            local_names,
                            constructable,
                            is_strict,
                            lexical_this,
                            lexical_arguments,
                            is_class_constructor: false,
                            is_derived_constructor: false,
                            home_object: None,
                            super_constructor: None,
                            captured_env: self.captured_env.clone(),
                        },
                    )));
                }
                Op::NewClass {
                    name,
                    constructor,
                    methods,
                    computed_key_count,
                    has_heritage,
                } => {
                    let result = self.new_class(
                        name.as_deref(),
                        &constructor,
                        &methods,
                        computed_key_count,
                        has_heritage,
                    );
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::SuperGet { key } => {
                    let result = self.super_get(&PropertyKey::String(key));
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::SuperGetComputed => {
                    let key_value = self.pop()?;
                    let key = self.coerce_property_key(key_value)?;
                    let result = self.super_get(&key);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::SuperMethod { key } => {
                    let result = self.super_method(PropertyKey::String(key));
                    self.handle_runtime_result(result)?;
                }
                Op::SuperMethodComputed => {
                    let key_value = self.pop()?;
                    let key = self.coerce_property_key(key_value)?;
                    let result = self.super_method(key);
                    self.handle_runtime_result(result)?;
                }
                Op::CallResolved(argc) => self.call_resolved(argc)?,
                Op::CallResolvedSpread => self.call_resolved_spread()?,
                Op::SuperCall(argc) => {
                    let arguments = self.pop_arguments(argc)?;
                    self.super_call(arguments)?;
                }
                Op::SuperCallSpread => {
                    let arguments = self.pop_argument_array("super call spread")?;
                    self.super_call(arguments)?;
                }
                Op::Typeof => {
                    let value = self.pop()?;
                    self.stack.push(Value::String(typeof_value(value)));
                }
                Op::ToString => {
                    let value = self.pop()?;
                    let mut env = self.current_env();
                    let result = to_js_string_with_env(value, &mut env);
                    self.apply_env(env);
                    self.stack.push(Value::String(result?));
                }
                Op::ToNumeric => {
                    let result = self.eval_to_numeric();
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::Unary(op) => {
                    let result = self.eval_unary(op);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::Update(op) => {
                    let result = self.eval_update(op);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::Binary(op) => {
                    let result = self.eval_binary(op);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
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
            }
        }
    }

    pub(super) fn function_capture_env(
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

    pub(super) fn refresh_captured_env(&self, env: &HashMap<String, Value>) {
        let mut captured_env = self.captured_env.borrow_mut();
        for (name, value) in env {
            captured_env.insert(name.clone(), value.clone());
        }
    }

    fn current_local_binding(&self, name: &str) -> Option<&Value> {
        self.bytecode
            .local_slot(name)
            .and_then(|index| self.locals.get(index))
            .and_then(Option::as_ref)
    }

    fn new_array(&mut self, elements: &[ArrayElementKind]) -> Result<(), RuntimeError> {
        let value_count = elements
            .iter()
            .filter(|element| !matches!(element, ArrayElementKind::Elision))
            .count();
        let mut element_values = Vec::with_capacity(value_count);
        for _ in 0..value_count {
            element_values.push(self.pop()?);
        }
        element_values.reverse();

        let mut values = Vec::new();
        let mut holes = Vec::new();
        let mut next_value = element_values.into_iter();
        for element in elements {
            match element {
                ArrayElementKind::Expr => {
                    values.push(next_value.next().ok_or_else(stack_underflow)?);
                }
                ArrayElementKind::Elision => {
                    holes.push(values.len());
                    values.push(Value::Undefined);
                }
                ArrayElementKind::Spread => {
                    let value = next_value.next().ok_or_else(stack_underflow)?;
                    let mut env = self.current_env();
                    let spread_values = iterable_values_with_env(value, "array spread", &mut env)?;
                    self.apply_env(env);
                    values.extend(spread_values);
                }
            }
        }
        self.stack
            .push(Value::Array(ArrayRef::new_sparse(values, holes)));
        Ok(())
    }

    fn new_template_object(&mut self, cooked: &[String], raw: &[String]) {
        let cooked_values = cooked
            .iter()
            .cloned()
            .map(Value::String)
            .collect::<Vec<_>>();
        let raw_values = raw.iter().cloned().map(Value::String).collect::<Vec<_>>();
        let cooked_array = ArrayRef::new(cooked_values);
        let raw_array = ArrayRef::new(raw_values);
        raw_array.freeze();
        cooked_array.define_property(
            "raw".to_owned(),
            Property::fixed_non_enumerable(Value::Array(raw_array)),
        );
        cooked_array.freeze();
        self.stack.push(Value::Array(cooked_array));
    }

    fn new_object(&mut self, kinds: &[ObjectPropertyKind]) -> Result<(), RuntimeError> {
        let object = ObjectRef::with_prototype(HashMap::new(), object_prototype(&self.globals));
        let mut entries = Vec::with_capacity(kinds.len());
        for kind in kinds.iter().rev() {
            let value = self.pop()?;
            let key = to_property_key_value(self.pop()?, &mut self.globals)?;
            let descriptor = match kind {
                ObjectPropertyKind::Data => Property::enumerable(value),
                ObjectPropertyKind::Getter => Property::accessor(Some(value), None, true, true),
                ObjectPropertyKind::Setter => Property::accessor(None, Some(value), true, true),
            };
            entries.push((key, descriptor));
        }
        for (key, mut descriptor) in entries.into_iter().rev() {
            if descriptor.is_accessor()
                && let Some(existing) = match &key {
                    crate::PropertyKey::String(key) => object.own_property(key),
                    crate::PropertyKey::Symbol(symbol) => object.own_symbol_property(symbol),
                }
                && existing.is_accessor()
            {
                descriptor.get = descriptor.get.or(existing.get);
                descriptor.set = descriptor.set.or(existing.set);
            }
            let success = object::define_property_on_value_key(
                Value::Object(object.clone()),
                key,
                descriptor,
            )?;
            if !success {
                return Err(RuntimeError {
                    thrown: None,
                    message: "object literal property definition failed".to_owned(),
                });
            }
        }
        self.stack.push(Value::Object(object));
        Ok(())
    }

    fn get_prop(&mut self) -> Result<(), RuntimeError> {
        let key_value = self.pop()?;
        let key = self.coerce_property_key(key_value)?;
        let object = self.pop()?;
        let value = if let Some(value) = direct_get_property_key(&object, &key) {
            value
        } else {
            let mut env = self.current_env();
            let value = get_property_key(object, &key, &mut env)?;
            self.apply_env(env);
            value
        };
        self.stack.push(value);
        Ok(())
    }

    fn set_prop(&mut self, is_strict: bool) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let key_value = self.pop()?;
        let key = self.coerce_property_key(key_value)?;
        let object = self.pop()?;
        if self.symbol_primitive_set_fails(&object, &key) {
            if is_strict {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: cannot assign property on Symbol primitive".to_owned(),
                });
            }
            self.stack.push(value);
            return Ok(());
        }
        let updates_global_binding = self.is_global_object(&object);
        let wrote_data = if property_set_uses_setter(&object, &key, &self.globals) {
            let mut env = self.current_env();
            let wrote_data = set_property_key(object, key.clone(), value.clone(), &mut env)?;
            self.apply_env(env);
            wrote_data
        } else {
            set_property_key(object, key.clone(), value.clone(), &mut self.globals)?
        };
        if !wrote_data && is_strict {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot set property".to_owned(),
            });
        }
        if updates_global_binding
            && wrote_data
            && let crate::PropertyKey::String(key) = key
        {
            self.globals.insert(key, value.clone());
        }
        self.stack.push(value);
        Ok(())
    }

    fn symbol_primitive_set_fails(&self, object: &Value, key: &crate::PropertyKey) -> bool {
        matches!(object, Value::Object(object) if symbol::is_symbol_primitive(object))
            && !property_set_uses_setter(object, key, &self.globals)
    }

    fn is_global_object(&self, value: &Value) -> bool {
        let Value::Object(object) = value else {
            return false;
        };
        matches!(
            self.globals.get(GLOBAL_THIS_BINDING),
            Some(Value::Object(global_object)) if object.ptr_eq(global_object)
        )
    }

    fn delete_prop(&mut self) -> Result<(), RuntimeError> {
        let key_value = self.pop()?;
        let key = self.coerce_property_key(key_value)?;
        let object = self.pop()?;
        self.stack
            .push(delete_property_key(object, &key, &mut self.globals)?);
        Ok(())
    }

    fn call(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        self.call_callee(callee, Value::Undefined, arguments)
    }

    fn call_callee(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: Vec<Value>,
    ) -> Result<(), RuntimeError> {
        if let Some(result) = self.try_fast_global_native_call(&callee, &arguments)? {
            if let Some(value) = result {
                self.stack.push(value);
            }
            return Ok(());
        }
        let mut env = self.call_env(&callee);
        let result = call_function(callee, this_value, arguments, &mut env.env, false);
        self.apply_call_env(env);
        if let Some(result) = self.handle_call_result(result)? {
            self.stack.push(result);
        }
        Ok(())
    }

    fn call_spread(&mut self) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("function call spread")?;
        let callee = self.pop()?;
        self.call_callee(callee, Value::Undefined, arguments)
    }

    fn try_fast_global_native_call(
        &mut self,
        callee: &Value,
        arguments: &[Value],
    ) -> Result<Option<Option<Value>>, RuntimeError> {
        let Value::Function(function) = callee else {
            return Ok(None);
        };
        let Some(native) = function.native else {
            return Ok(None);
        };
        let result = match native {
            NativeFunction::DecodeUri | NativeFunction::DecodeUriComponent => {
                let source = match arguments.first().cloned().unwrap_or(Value::Undefined) {
                    Value::String(source) => source,
                    Value::Undefined => "undefined".to_owned(),
                    _ => return Ok(None),
                };
                let result = match native {
                    NativeFunction::DecodeUri => crate::global::decode_uri_string(&source),
                    NativeFunction::DecodeUriComponent => {
                        crate::global::decode_uri_component_string(&source)
                    }
                    _ => unreachable!("URI native matched above"),
                };
                result.map(Value::String)
            }
            NativeFunction::StringFromCharCode => {
                if !arguments
                    .iter()
                    .all(|value| matches!(value, Value::Number(_)))
                {
                    return Ok(None);
                }
                Ok(Value::String(fast_string_from_char_code_numbers(arguments)))
            }
            _ => return Ok(None),
        };
        Ok(Some(self.handle_runtime_result(result)?))
    }

    fn call_method(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let (callee, this_value) = self.pop_method_callee()?;
        self.call_callee(callee, this_value, arguments)
    }

    fn call_method_spread(&mut self) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("method call spread")?;
        let (callee, this_value) = self.pop_method_callee()?;
        self.call_callee(callee, this_value, arguments)
    }

    /// Calls a pre-resolved callee whose receiver and callee are already on the
    /// stack as `[receiver, callee, args...]`.
    fn call_resolved(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        let this_value = self.pop()?;
        self.call_callee(callee, this_value, arguments)
    }

    fn call_resolved_spread(&mut self) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("super method call spread")?;
        let callee = self.pop()?;
        let this_value = self.pop()?;
        self.call_callee(callee, this_value, arguments)
    }

    fn pop_method_callee(&mut self) -> Result<(Value, Value), RuntimeError> {
        let key_value = self.pop()?;
        let key = self.coerce_property_key(key_value)?;
        let this_value = self.pop()?;
        let callee = if let Some(callee) = direct_function_data_property(&this_value, &key) {
            callee
        } else {
            let mut getter_env = self.current_env();
            let callee = get_property_key(this_value.clone(), &key, &mut getter_env)?;
            self.apply_env(getter_env);
            callee
        };
        Ok((callee, this_value))
    }

    fn construct(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        self.construct_callee(callee, arguments)
    }

    fn construct_spread(&mut self) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("constructor spread")?;
        let callee = self.pop()?;
        self.construct_callee(callee, arguments)
    }

    fn construct_callee(
        &mut self,
        callee: Value,
        arguments: Vec<Value>,
    ) -> Result<(), RuntimeError> {
        let mut env = self.call_env(&callee);
        let result = construct_function(callee.clone(), callee, arguments, &mut env.env);
        self.apply_call_env(env);
        if let Some(result) = self.handle_call_result(result)? {
            self.stack.push(result);
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

    fn pop_argument_array(&mut self, context: &str) -> Result<Vec<Value>, RuntimeError> {
        let value = self.pop()?;
        let mut env = self.current_env();
        let arguments = array_like_values_with_env(value, context, &mut env)?;
        self.apply_env(env);
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
                if bytecode.requires_scope_call_bindings() {
                    insert_scope_call_bindings(
                        &mut env,
                        &mut binding_names,
                        self.bytecode,
                        &self.locals,
                        &self.globals,
                        &function.local_names,
                    );
                }
            }
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
        for name in function_bytecode.sloppy_global_assignment_names() {
            insert_missing_binding_name(binding_names, name);
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
        if crate::function::is_internal_binding_name(name) {
            return;
        }
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
            if self.locals[index].is_some()
                && let Some(value) = env.get(&local.name)
            {
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

    fn drain_promise_jobs(&mut self) -> Result<(), RuntimeError> {
        let mut env = self.current_env();
        promise::drain_promise_jobs(&mut env)?;
        self.apply_env(env);
        Ok(())
    }

    pub(super) fn record_sloppy_global_name(&mut self, name: &str) {
        if !self
            .sloppy_global_names
            .iter()
            .any(|existing| existing == name)
        {
            self.sloppy_global_names.push(name.to_owned());
        }
    }
}

fn fast_string_from_char_code_numbers(arguments: &[Value]) -> String {
    let code_units: Vec<u16> = arguments
        .iter()
        .map(|value| match value {
            Value::Number(number) if number.is_finite() && *number != 0.0 => {
                number.trunc().rem_euclid(65_536.0) as u16
            }
            Value::Number(_) => 0,
            _ => unreachable!("fast path only accepts numeric arguments"),
        })
        .collect();
    crate::string::string_from_code_units(&code_units)
}

fn direct_function_data_property(this_value: &Value, key: &PropertyKey) -> Option<Value> {
    let (Value::Function(function), PropertyKey::String(name)) = (this_value, key) else {
        return None;
    };
    function
        .properties
        .borrow()
        .get(name)
        .filter(|property| !property.accessor)
        .map(|property| property.value.clone())
}

fn direct_get_property_key(value: &Value, key: &PropertyKey) -> Option<Value> {
    let PropertyKey::String(name) = key else {
        return None;
    };
    match value {
        Value::Array(array) if name == "length" => Some(Value::Number(array.len() as f64)),
        Value::Array(array) => name
            .parse::<usize>()
            .ok()
            .and_then(|index| array.get(index)),
        Value::Function(_) => direct_function_data_property(value, key),
        _ => None,
    }
}

fn insert_missing_binding_name(binding_names: &mut Vec<String>, name: &str) {
    if !binding_names.iter().any(|existing| existing == name) {
        binding_names.push(name.to_owned());
    }
}
