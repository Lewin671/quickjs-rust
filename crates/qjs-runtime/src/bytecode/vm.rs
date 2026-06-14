use super::ir::{Bytecode, Op};
use super::util::{stack_underflow, typeof_value};
use super::vm_call::{insert_scope_call_bindings, user_bytecode_function};
use super::vm_generator::CaptureWriteback;
use super::vm_iter::DelegateStep;
use super::vm_props::{array_index_from_number, get_property_key, set_property_key};
use super::vm_result::{Completion, FunctionBytecodeResult, ResumeMode};
use super::vm_try::TryFrame;
use crate::{
    Function, GLOBAL_THIS_BINDING, HOME_OBJECT_BINDING, NEW_TARGET_BINDING, ObjectRef, PropertyKey,
    RuntimeError, SUPER_CONSTRUCTOR_BINDING, Value,
    array::array_like_values_with_env,
    call_function, construct_function,
    function::{CallEnv, CompiledUserFunction, Realm},
    initialize_builtins, is_truthy, to_js_string_with_env, to_property_key_value,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
pub(super) type Slot = Option<Value>;
struct VmCallEnv {
    env: CallEnv,
    binding_names: Option<Vec<String>>,
    /// Injected caller bindings; only changed values write back.
    injected: HashMap<String, Value>,
}
pub(super) fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode)?;
    let value = vm.run()?;
    vm.drain_promise_jobs()?;
    Ok(value)
}
pub(super) fn eval_function_bytecode(
    bytecode: &Bytecode,
    env: CallEnv,
    captured_env: Rc<RefCell<HashMap<String, Value>>>,
    capture_writeback: Option<CaptureWriteback>,
) -> FunctionBytecodeResult<'_> {
    let mut vm = Vm::new_with_globals_and_captures(bytecode, env, captured_env);
    vm.capture_writeback = capture_writeback;
    let value = vm.run();
    FunctionBytecodeResult {
        value,
        bytecode,
        env: vm.frame_call_env(),
        locals: vm.locals,
        sloppy_global_names: vm.sloppy_global_names,
    }
}
pub(super) struct Vm<'a> {
    pub(super) bytecode: &'a Bytecode,
    pub(super) ip: usize,
    pub(super) stack: Vec<Value>,
    pub(super) locals: Vec<Slot>,
    /// Shared realm plus this frame's internal/caller-scope bindings.
    pub(super) env: CallEnv,
    pub(super) realm: Realm,
    /// Dynamic-import host copied into every `CallEnv` this VM creates.
    pub(super) module_host: Option<crate::module::ModuleHostRef>,
    pub(super) captured_env: Rc<RefCell<HashMap<String, Value>>>,
    pub(super) capture_writeback: Option<CaptureWriteback>,
    pub(super) sloppy_global_names: Vec<String>,
    pub(super) try_stack: Vec<TryFrame>,
    pub(super) pending_throw: Option<Value>,
    pub(super) pending_return: Option<Value>,
    /// Staged resume for a generator body suspended inside `yield*`.
    pub(super) resume_mode: Option<ResumeMode>,
    /// Cached realm Array.prototype for the `a[i] = x` fast path.
    pub(super) array_prototype_cache: Option<ObjectRef>,
    /// Makes generators run parameter prologues before first suspension.
    pub(super) stop_at_prologue: bool,
    /// Enclosing `with` object-environment records, innermost last.
    pub(super) with_stack: Vec<Value>,
}
impl<'a> Vm<'a> {
    pub(super) fn new(bytecode: &'a Bytecode) -> Result<Self, RuntimeError> {
        let mut globals = HashMap::new();
        let global_this = Value::Object(ObjectRef::new(HashMap::new()));
        globals.insert("this".to_owned(), global_this.clone());
        globals.insert(GLOBAL_THIS_BINDING.to_owned(), global_this.clone());
        globals.insert("undefined".to_owned(), Value::Undefined);
        // The realm cell is live before builtin installation: every `install_*`
        // runs against a `CallEnv` over it and writes intrinsics straight to the
        // shared cell (`insert_realm`), so no install-vs-runtime signature split
        // is needed.
        let realm: Realm = Rc::new(RefCell::new(globals));
        let mut env = CallEnv::new(Rc::clone(&realm));
        initialize_builtins(&mut env, &global_this);
        {
            let mut globals = realm.borrow_mut();
            Self::initialize_script_global_bindings(bytecode, &mut globals)?;
        }
        // The script frame captures nothing: its `var`/function bindings live
        // in the shared realm, so closures read them through the realm cell
        // instead of a creation-time snapshot (which would freeze hoisted
        // bindings at `undefined`).
        let captured_env = Rc::new(RefCell::new(HashMap::new()));
        Ok(Self::new_with_globals_and_captures(
            bytecode,
            env,
            captured_env,
        ))
    }

    pub(super) fn new_with_globals_and_captures(
        bytecode: &'a Bytecode,
        env: CallEnv,
        captured_env: Rc<RefCell<HashMap<String, Value>>>,
    ) -> Self {
        let realm = env.realm_rc();
        let module_host = env.module_host();
        let locals = Self::initial_slots(bytecode, &env);
        Self {
            bytecode,
            ip: 0,
            stack: Vec::with_capacity(64),
            locals,
            env,
            realm,
            module_host,
            captured_env,
            capture_writeback: None,
            sloppy_global_names: Vec::new(),
            try_stack: Vec::new(),
            pending_throw: None,
            pending_return: None,
            resume_mode: None,
            stop_at_prologue: false,
            array_prototype_cache: None,
            with_stack: Vec::new(),
        }
    }

    /// Builds a `CallEnv` over the shared realm with this frame's live slots.
    pub(super) fn frame_call_env(&self) -> CallEnv {
        let mut locals = self.env.snapshot_locals();
        for (index, slot) in self.locals.iter().enumerate() {
            if let Some(value) = slot {
                locals.insert(self.bytecode.locals[index].name.clone(), value.clone());
            }
        }
        let mut env = self.attach_host(CallEnv::with_locals(self.realm_rc(), locals));
        env.set_private_environment(self.current_private_environment());
        env
    }

    /// A clone of the shared realm `Rc`.
    pub(super) fn realm_rc(&self) -> Realm {
        Rc::clone(&self.realm)
    }

    /// A shared-realm `CallEnv` with empty frame locals.
    pub(super) fn realm_env(&self) -> CallEnv {
        self.attach_host(CallEnv::new(self.realm_rc()))
    }

    pub(super) fn coerce_property_key(
        &mut self,
        value: Value,
    ) -> Result<PropertyKey, RuntimeError> {
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
            value => {
                let mut key_env = self.current_env();
                to_property_key_value(value, &mut key_env)
            }
        }
    }

    pub(super) fn run(&mut self) -> Result<Value, RuntimeError> {
        if let Completion::Return(value) = self.run_completion()? {
            return Ok(value);
        }
        Err(RuntimeError {
            thrown: None,
            message: "yield evaluated outside a generator body".to_owned(),
        })
    }

    /// Runs the bytecode loop until it returns or yields. Generator bodies
    /// re-enter this loop on each resume; ordinary functions and scripts run it
    /// once and only ever observe `Completion::Return`.
    pub(super) fn run_completion(&mut self) -> Result<Completion, RuntimeError> {
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
                Op::LoadNewTarget => self.stack.push(self.load_new_target()),
                Op::StoreLocal(slot) => {
                    let value = self.pop()?;
                    let result = self.store_local(slot, value);
                    self.handle_runtime_result(result)?;
                }
                Op::AssignLocal(slot) => {
                    let value = self.pop()?;
                    let result = self.assign_local(slot, value);
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
                    let value = self.env.get(&name).unwrap_or(Value::Undefined);
                    self.stack.push(Value::String(typeof_value(value)));
                }
                op @ (Op::EnterWith
                | Op::ExitWith
                | Op::LoadIdentWith { .. }
                | Op::ResolveIdentWith { .. }
                | Op::StoreIdentWith { .. }
                | Op::StoreResolvedIdentWith { .. }
                | Op::TypeofIdentWith { .. }) => {
                    self.run_with_op(op)?;
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
                Op::NewObjectLiteral => self.new_object_literal(),
                Op::DefineObjectProperty(meta) => self.define_object_property(meta)?,
                Op::CopyObjectSpread => self.copy_object_spread()?,
                Op::EnumerateKeys => self.enumerate_keys()?,
                Op::GetIterator => self.get_iterator()?,
                Op::GetAsyncIterator => self.get_async_iterator()?,
                Op::AsyncIteratorComplete { done_slot } => {
                    self.async_iterator_complete(done_slot)?
                }
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
                Op::GetPrivate(name) => {
                    let result = self.get_private(&name);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::SetPrivate(name) => {
                    let result = self.set_private(&name);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::PrivateIn(name) => {
                    let result = self.private_in(&name);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::DeleteProp { is_strict } => {
                    let result = self.delete_prop(is_strict);
                    self.handle_runtime_result(result)?;
                }
                Op::Call(argc) => self.call(argc)?,
                Op::CallDirectEval(argc) => self.call_direct_eval(argc)?,
                Op::CallMethod(argc) => self.call_method(argc)?,
                Op::CallSpread => self.call_spread()?,
                Op::CallDirectEvalSpread => self.call_direct_eval_spread()?,
                Op::CallMethodSpread => self.call_method_spread()?,
                Op::IteratorClose { swallow } => self.iterator_close(swallow)?,
                Op::New(argc) => self.construct(argc)?,
                Op::NewSpread => self.construct_spread()?,
                Op::NewFunction {
                    name,
                    has_name_binding,
                    params,
                    local_names,
                    lexical_captures,
                    bytecode,
                    constructable,
                    is_strict,
                    lexical_this,
                    lexical_arguments,
                    is_generator,
                    is_async,
                } => {
                    let mut env = self.function_capture_env(&bytecode, &local_names);
                    self.insert_lexical_captures(&mut env, &lexical_captures);
                    let (home_object, super_constructor) = if lexical_this {
                        let home_object = self.env.get(HOME_OBJECT_BINDING);
                        let super_constructor = self.env.get(SUPER_CONSTRUCTOR_BINDING);
                        if self.load_global("this").is_err() {
                            env.insert(
                                "this".to_owned(),
                                Value::Function(Function::uninitialized_lexical_marker()),
                            );
                        }
                        if let Some(new_target) = self.env.get(NEW_TARGET_BINDING) {
                            env.insert(NEW_TARGET_BINDING.to_owned(), new_target);
                        }
                        (home_object, super_constructor)
                    } else {
                        (None, None)
                    };
                    self.refresh_captured_env(&env);
                    let captured_env = if self.in_parameter_prologue() {
                        Rc::new(RefCell::new(env.clone()))
                    } else {
                        self.captured_env.clone()
                    };
                    let function = Function::new_user_compiled(CompiledUserFunction {
                        name,
                        has_name_binding,
                        params,
                        env,
                        bytecode,
                        local_names,
                        constructable,
                        is_strict,
                        lexical_this,
                        lexical_arguments,
                        is_generator,
                        is_async,
                        is_class_constructor: false,
                        is_derived_constructor: false,
                        is_field_initializer: false,
                        home_object,
                        super_constructor,
                        captured_env,
                        capture_writeback: self.capture_writeback.clone(),
                    });
                    self.capture_private_environment(&function);
                    if is_generator && is_async {
                        crate::async_generator::wire_async_generator_function_intrinsics(
                            &function,
                            &self.realm_env(),
                        );
                    } else if is_generator {
                        self.wire_generator_function_intrinsics(&function);
                    } else if is_async {
                        self.wire_async_function_intrinsics(&function);
                    }
                    self.stack.push(Value::Function(function));
                }
                Op::NewClass {
                    name,
                    constructor,
                    elements,
                    private_elements,
                    computed_keys,
                    has_heritage,
                } => {
                    let result = self.new_class(
                        name.as_deref(),
                        &constructor,
                        &elements,
                        &private_elements,
                        &computed_keys,
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
                Op::RequireSuperThis => {
                    let result = self.require_super_this();
                    self.handle_runtime_result(result)?;
                }
                Op::SuperGetComputed => {
                    let key_value = self.pop()?;
                    let key = self.coerce_property_key(key_value)?;
                    let result = self.super_get(&key);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::SuperSet { key, is_strict } => {
                    let result = self.super_set(&PropertyKey::String(key), is_strict);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::SuperSetComputed { is_strict } => {
                    let value = self.pop()?;
                    let key_value = self.pop()?;
                    let key = self.coerce_property_key(key_value)?;
                    let result = self.super_set_value(key, value, is_strict);
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
                Op::ToPropertyKey => {
                    let value = self.pop()?;
                    let key = self.coerce_property_key(value)?;
                    self.stack.push(key.into_value());
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
                        return Ok(Completion::Return(value));
                    }
                }
                Op::Return => {
                    let value = self.stack.pop().unwrap_or(Value::Undefined);
                    if let Some(value) = self.return_value(value)? {
                        return Ok(Completion::Return(value));
                    }
                }
                Op::Throw => {
                    let value = self.pop()?;
                    self.throw_value(value)?;
                }
                Op::FunctionPrologueEnd => {
                    if self.stop_at_prologue {
                        self.stop_at_prologue = false;
                        return Ok(Completion::PrologueEnd);
                    }
                }
                Op::Yield => {
                    let value = self.pop()?;
                    return Ok(Completion::Yield(value));
                }
                Op::Await => {
                    let value = self.pop()?;
                    return Ok(Completion::Await(value));
                }
                Op::YieldDelegate {
                    iterator_slot,
                    next_slot,
                    async_delegate,
                } => match self.yield_delegate(iterator_slot, next_slot, async_delegate)? {
                    DelegateStep::Suspend(value) if async_delegate => {
                        return Ok(Completion::YieldDelegateAsync(value));
                    }
                    DelegateStep::Suspend(value) => return Ok(Completion::YieldDelegate(value)),
                    DelegateStep::Await(value) => return Ok(Completion::YieldDelegateAwait(value)),
                    DelegateStep::AwaitReturn(value) => {
                        return Ok(Completion::YieldDelegateAwaitReturn(value));
                    }
                    DelegateStep::AwaitReturnValue(value) => {
                        return Ok(Completion::YieldDelegateAwaitReturnValue(value));
                    }
                    DelegateStep::Return(value) => return Ok(Completion::Return(value)),
                    DelegateStep::Continue => {}
                },
                Op::ImportCall { has_options } => self.import_call(has_options)?,
                Op::ImportMeta => {
                    return Err(RuntimeError {
                        thrown: None,
                        message: "SyntaxError: 'import.meta' is only valid in a module".to_owned(),
                    });
                }
            }
        }
    }

    fn get_prop(&mut self) -> Result<(), RuntimeError> {
        let key_value = self.pop()?;
        let key = self.coerce_property_key(key_value)?;
        let object = self.pop()?;
        let value = if let Some(value) = self.try_direct_get(&object, &key) {
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
        // Fast path: writing a real array index to a plain array with the
        // default prototype, no own descriptor at that index, and no exotic
        // inherited index accessor. This is the dominant pattern in tight
        // `a[i] = x` append loops, so it skips the string-key allocation and
        // the per-write prototype-chain setter probe taken by the generic path.
        if let Value::Number(number) = &key_value
            && let Some(index) = array_index_from_number(*number)
            && let Some(Value::Array(elements)) = self.stack.last()
            && elements.uses_default_prototype()
            && elements.dense_index_store_eligible(index)
        {
            let elements = elements.clone();
            // A plain array with the default prototype takes the dense-store fast
            // path when the index has no own special descriptor and the realm's
            // Array.prototype carries no own indexed property that an OrdinarySet
            // would have to honor. Both checks are O(1), so a tight `a[i] = x`
            // loop avoids the string-key allocation and prototype walk of the
            // generic path.
            if !self.array_prototype_has_index_property().unwrap_or(true) {
                self.pop()?;
                elements.set(index, value.clone());
                self.stack.push(value);
                return Ok(());
            }
        }
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
        let mut env = self.current_env();
        let wrote_data = set_property_key(object, key.clone(), value.clone(), &mut env)?;
        self.apply_env(env);
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
            self.invalidate_array_prototype_cache(&key);
            self.realm.borrow_mut().insert(key, value.clone());
        }
        self.stack.push(value);
        Ok(())
    }

    fn call(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        self.call_callee(callee, Value::Undefined, arguments)
    }

    fn call_direct_eval(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        self.call_callee_with_direct_eval(callee, Value::Undefined, arguments)
    }

    fn call_callee(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: Vec<Value>,
    ) -> Result<(), RuntimeError> {
        self.call_callee_with_marker(callee, this_value, arguments, false)
    }

    fn call_callee_with_direct_eval(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: Vec<Value>,
    ) -> Result<(), RuntimeError> {
        self.call_callee_with_marker(callee, this_value, arguments, true)
    }

    fn call_callee_with_marker(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: Vec<Value>,
        direct_eval: bool,
    ) -> Result<(), RuntimeError> {
        if let Some(result) = super::vm_call::try_fast_global_native_call(
            &callee,
            &this_value,
            &arguments,
            &self.realm_env(),
        ) {
            if let Some(value) = self.handle_runtime_result(result)? {
                self.stack.push(value);
            }
            return Ok(());
        }
        let mut env = self.call_env(&callee);
        if direct_eval {
            env.env
                .insert(crate::DIRECT_EVAL_BINDING.to_owned(), Value::Boolean(true));
        }
        let result = call_function(callee, this_value, arguments, &mut env.env, false);
        env.env.remove(crate::DIRECT_EVAL_BINDING);
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

    fn call_direct_eval_spread(&mut self) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("direct eval spread")?;
        let callee = self.pop()?;
        self.call_callee_with_direct_eval(callee, Value::Undefined, arguments)
    }

    fn call_method(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        // Method resolution errors (e.g. reading a property of undefined) are
        // catchable runtime errors, not VM faults.
        let resolved = self.pop_method_callee();
        let Some((callee, this_value)) = self.handle_runtime_result(resolved)? else {
            return Ok(());
        };
        self.call_callee(callee, this_value, arguments)
    }

    fn call_method_spread(&mut self) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("method call spread")?;
        let resolved = self.pop_method_callee();
        let Some((callee, this_value)) = self.handle_runtime_result(resolved)? else {
            return Ok(());
        };
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
        let callee = if let Some(callee) = self.try_direct_get(&this_value, &key) {
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

    pub(super) fn current_env(&self) -> CallEnv {
        self.frame_call_env()
    }

    fn call_env(&self, callee: &Value) -> VmCallEnv {
        if let Some(function) = user_bytecode_function(callee) {
            let mut locals = HashMap::new();
            let mut binding_names = Vec::new();
            if let Some(bytecode) = &function.bytecode {
                self.insert_referenced_call_bindings(
                    &mut locals,
                    &mut binding_names,
                    bytecode,
                    &function.local_names,
                );
                if function.lexical_this && bytecode.contains_super_call() {
                    self.insert_lexical_super_call_this(&mut locals, &mut binding_names);
                }
                if bytecode.requires_scope_call_bindings() {
                    insert_scope_call_bindings(
                        &mut locals,
                        &mut binding_names,
                        self.bytecode,
                        &self.locals,
                        &function.local_names,
                    );
                }
            }
            return VmCallEnv {
                injected: locals.clone(),
                env: self.attach_host(CallEnv::with_locals(self.realm_rc(), locals)),
                binding_names: Some(binding_names),
            };
        }
        if let Some((env, injected, binding_names)) =
            super::vm_call::call_forwarding_native_env(callee, self.current_env())
        {
            return VmCallEnv {
                injected,
                env,
                binding_names: Some(binding_names),
            };
        }
        VmCallEnv {
            env: self.current_env(),
            binding_names: None,
            injected: HashMap::new(),
        }
    }

    fn insert_referenced_call_bindings(
        &self,
        locals: &mut HashMap<String, Value>,
        binding_names: &mut Vec<String>,
        function_bytecode: &Bytecode,
        function_local_names: &[String],
    ) {
        for name in function_bytecode.global_names() {
            self.insert_call_binding(locals, binding_names, name);
        }
        for name in function_bytecode.sloppy_global_assignment_names() {
            insert_missing_binding_name(binding_names, name);
        }
        for name in function_bytecode.local_names() {
            if function_local_names
                .binary_search_by(|local| local.as_str().cmp(name))
                .is_err()
            {
                self.insert_call_binding(locals, binding_names, name);
            }
        }
    }
    fn insert_call_binding(
        &self,
        locals: &mut HashMap<String, Value>,
        binding_names: &mut Vec<String>,
        name: &str,
    ) {
        if crate::function::is_internal_binding_name(name) {
            return;
        }
        let value = self
            .current_local_binding(name)
            .cloned()
            .or_else(|| self.env.locals().get(name).cloned());
        if let Some(value) = value {
            locals.insert(name.to_owned(), value);
            if !binding_names.iter().any(|existing| existing == name) {
                binding_names.push(name.to_owned());
            }
        }
    }

    fn insert_lexical_super_call_this(
        &self,
        locals: &mut HashMap<String, Value>,
        binding_names: &mut Vec<String>,
    ) {
        let value = self
            .current_local_binding("this")
            .cloned()
            .or_else(|| self.env.locals().get("this").cloned())
            .unwrap_or_else(|| Value::Function(Function::uninitialized_lexical_marker()));
        locals.insert("this".to_owned(), value);
        insert_missing_binding_name(binding_names, "this");
    }

    fn apply_call_env(&mut self, env: VmCallEnv) {
        if let Some(binding_names) = env.binding_names {
            self.apply_selected_env(env.env, &binding_names, &env.injected);
        } else {
            self.apply_env(env.env);
        }
        if !self.bytecode.global_scope {
            return;
        }
        let captured = self.captured_env.borrow();
        if captured.is_empty() {
            return;
        }
        for (name, value) in captured.iter() {
            if let Some(index) = self.bytecode.local_slot(name)
                && let Some(slot) = self.locals.get_mut(index)
                && slot.is_some()
            {
                *slot = Some(value.clone());
            }
        }
    }

    pub(super) fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(stack_underflow)
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

fn insert_missing_binding_name(binding_names: &mut Vec<String>, name: &str) {
    if !binding_names.iter().any(|existing| existing == name) {
        binding_names.push(name.to_owned());
    }
}
