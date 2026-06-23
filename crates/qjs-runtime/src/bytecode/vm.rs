use super::ir::{Bytecode, Op};
use super::util::{stack_underflow, typeof_value};
use super::vm_call::{insert_scope_call_bindings, user_bytecode_function};
use super::vm_generator::CaptureWriteback;
use super::vm_iter::DelegateStep;
use super::vm_props::{array_index_from_number, get_property_key};
use super::vm_result::{Completion, FunctionBytecodeResult, ResumeMode};
use super::vm_set::set_property_key;
use super::vm_try::TryFrame;
use crate::{
    Function, GLOBAL_THIS_BINDING, HOME_OBJECT_BINDING, NEW_TARGET_BINDING, ObjectRef, PropertyKey,
    RuntimeError, SUPER_CONSTRUCTOR_BINDING, Value, construct_function,
    function::{CallEnv, CompiledUserFunction, Realm, Upvalue},
    initialize_builtins, is_truthy, to_js_string_with_env, to_property_key_value,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
pub(super) type Slot = Option<Value>;
pub(super) struct VmCallEnv {
    pub(super) env: CallEnv,
    pub(super) binding_names: Option<Vec<String>>,
    /// Injected caller bindings; only changed values write back.
    pub(super) injected: HashMap<String, Value>,
}
pub(super) fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode)?;
    let value = vm.run()?;
    vm.persist_global_lexical_bindings();
    vm.drain_promise_jobs()?;
    Ok(value)
}
pub(super) fn eval_function_bytecode(
    bytecode: &Bytecode,
    env: CallEnv,
    captured_env: Rc<RefCell<HashMap<String, Value>>>,
    upvalues: Vec<Upvalue>,
    with_stack: Vec<Value>,
    capture_writeback: Option<CaptureWriteback>,
    persist_global_lexicals: bool,
) -> FunctionBytecodeResult<'_> {
    let direct_eval_with_stack = !env.direct_eval_with_stack().is_empty();
    let mut vm = Vm::new_with_globals_captures_upvalues_and_with_stack(
        bytecode,
        env,
        captured_env,
        upvalues,
        with_stack,
    );
    vm.capture_writeback = capture_writeback;
    vm.persist_global_lexicals = persist_global_lexicals;
    vm.direct_eval_with_stack = direct_eval_with_stack;
    let value = vm.run();
    // A frame that neither creates closures nor holds any captured (`from_env`)
    // local never reads or writes the shared captured env beyond the realm
    // intrinsics seeded into it, so the per-call refresh and the result clone —
    // each O(captured-env size), ~48 intrinsic entries — are pure overhead on
    // the hot leaf-call path. `result.captured_env` is only consulted by
    // `FunctionBytecodeResult::binding` as a fallback for captured names, which
    // such a frame never propagates.
    let interacts_with_captures = bytecode.creates_closures() || bytecode.has_from_env_locals();
    let result_captured_env = if interacts_with_captures {
        vm.refresh_live_locals_from_captured_env();
        vm.captured_env.borrow().clone()
    } else {
        HashMap::new()
    };
    FunctionBytecodeResult {
        value,
        bytecode,
        env: vm.env,
        locals: vm.locals,
        captured_env: result_captured_env,
        sloppy_global_names: vm.sloppy_global_names,
    }
}
pub(super) struct Vm<'a> {
    pub(super) bytecode: &'a Bytecode,
    pub(super) ip: usize,
    pub(super) stack: Vec<Value>,
    pub(super) locals: Vec<Slot>,
    pub(super) local_upvalues: Vec<Option<Upvalue>>,
    pub(super) upvalues: Vec<Upvalue>,
    /// Shared realm plus this frame's internal/caller-scope bindings.
    pub(super) env: CallEnv,
    pub(super) realm: Realm,
    /// Dynamic-import host copied into every `CallEnv` this VM creates.
    pub(super) module_host: Option<crate::module::ModuleHostRef>,
    pub(super) captured_env: Rc<RefCell<HashMap<String, Value>>>,
    pub(super) captured_env_stack: Vec<Rc<RefCell<HashMap<String, Value>>>>,
    /// Per-iteration loop-variable names introduced by `fresh_iteration_scope`,
    /// aligned with `captured_env_stack`. These bindings are fresh each
    /// iteration (and may shadow an outer binding of the same name), so a write
    /// to one must never propagate to an enclosing captured env — see
    /// `propagate_captured_write_to_parents`.
    pub(super) captured_env_iteration_names: Vec<Vec<String>>,
    pub(super) parameter_captured_envs: Vec<Rc<RefCell<HashMap<String, Value>>>>,
    pub(super) capture_writeback: Option<CaptureWriteback>,
    pub(super) sloppy_global_names: Vec<String>,
    pub(super) try_stack: Vec<TryFrame>,
    pub(super) pending_throw: Option<Value>,
    pub(super) pending_return: Option<Value>,
    /// Target IP for a break/continue routed through a finally block.
    pub(super) pending_jump: Option<usize>,
    /// Staged resume for a generator body suspended inside `yield*`.
    pub(super) resume_mode: Option<ResumeMode>,
    /// Cached realm Array.prototype for the `a[i] = x` fast path.
    pub(super) array_prototype_cache: Option<ObjectRef>,
    /// Makes generators run parameter prologues before first suspension.
    pub(super) stop_at_prologue: bool,
    /// Enclosing `with` object-environment records, innermost last.
    pub(super) with_stack: Vec<Value>,
    /// True only for direct eval VMs that inherited an active with-chain from
    /// their caller. Ordinary functions created inside `with` also retain the
    /// chain, but their own local/global opcodes must not be dynamically
    /// re-resolved through it.
    pub(super) direct_eval_with_stack: bool,
    /// Active `using` disposal scopes (innermost last); each block's resources,
    /// disposed LIFO when the scope exits via the block's implicit finally.
    pub(super) disposable_scopes: Vec<Vec<super::vm_dispose::DisposeResource>>,
    /// Whether global-scope lexical declarations should become persistent
    /// global lexical bindings. Indirect eval uses global-scope bytecode, but
    /// its lexical environment is ephemeral.
    pub(super) persist_global_lexicals: bool,
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
        Self::new_with_globals_captures_and_with_stack(bytecode, env, captured_env, Vec::new())
    }

    fn persist_global_lexical_bindings(&mut self) {
        if !self.bytecode.is_global_scope() {
            return;
        }
        let hoisted = self.bytecode.hoisted_local_names().collect::<HashSet<_>>();
        let global_lexical_names = self.bytecode.global_lexical_names();
        for (slot, local) in self.bytecode.locals.iter().enumerate() {
            if hoisted.contains(local.name.as_str()) {
                continue;
            }
            if !global_lexical_names.iter().any(|name| name == &local.name) {
                continue;
            }
            let Some(_value) = self.local_slot_value(slot) else {
                continue;
            };
            self.env.mark_global_lexical_binding(local.name.clone());
            if !local.mutable {
                self.env.mark_immutable_lexical_binding(local.name.clone());
            }
        }
    }

    pub(super) fn new_with_globals_captures_and_with_stack(
        bytecode: &'a Bytecode,
        env: CallEnv,
        captured_env: Rc<RefCell<HashMap<String, Value>>>,
        with_stack: Vec<Value>,
    ) -> Self {
        Self::new_with_globals_captures_upvalues_and_with_stack(
            bytecode,
            env,
            captured_env,
            Vec::new(),
            with_stack,
        )
    }

    pub(super) fn new_with_globals_captures_upvalues_and_with_stack(
        bytecode: &'a Bytecode,
        env: CallEnv,
        captured_env: Rc<RefCell<HashMap<String, Value>>>,
        upvalues: Vec<Upvalue>,
        with_stack: Vec<Value>,
    ) -> Self {
        let realm = env.realm_rc();
        let module_host = env.module_host();
        let parameter_captured_envs = env.parameter_captured_envs().to_vec();
        let locals = Self::initial_slots(bytecode, &env);
        let local_upvalues = Self::initial_local_upvalues(bytecode, &locals, &upvalues);
        Self {
            bytecode,
            ip: 0,
            stack: Vec::with_capacity(64),
            locals,
            local_upvalues,
            upvalues,
            env,
            realm,
            module_host,
            captured_env,
            captured_env_stack: Vec::new(),
            captured_env_iteration_names: Vec::new(),
            parameter_captured_envs,
            capture_writeback: None,
            sloppy_global_names: Vec::new(),
            try_stack: Vec::new(),
            pending_throw: None,
            pending_return: None,
            pending_jump: None,
            resume_mode: None,
            stop_at_prologue: false,
            array_prototype_cache: None,
            with_stack,
            direct_eval_with_stack: false,
            disposable_scopes: Vec::new(),
            persist_global_lexicals: true,
        }
    }

    /// Builds a `CallEnv` over the shared realm with this frame's live slots.
    pub(super) fn frame_call_env(&self) -> CallEnv {
        let mut locals = self.env.snapshot_locals();
        for index in 0..self.locals.len() {
            if let Some(value) = self.local_slot_value(index) {
                let name = self.bytecode.locals[index].name.clone();
                locals.insert(name.clone(), value);
                // A block lexical that shadows a same-named binding is stored
                // under a mangled name (`\0lexical:w:N`); also expose it under
                // its plain source name so a direct eval (or other dynamic name
                // lookup) at this point resolves `w` to the innermost active
                // block binding. Slots are iterated in scope order, so an inner
                // block's binding overwrites an outer one, and an exited block's
                // cleared slot (None) never wins.
                if let Some(source_name) = unmangle_lexical_storage_name(&name) {
                    if let Some(value) = self.local_slot_value(index) {
                        locals.insert(source_name.to_owned(), value);
                    }
                }
            }
        }
        let mut env = self.attach_host(self.env.with_current_frame_locals(locals));
        for (index, slot) in self.locals.iter().enumerate() {
            if slot.is_some() && self.bytecode.locals[index].catch_binding {
                env.mark_catch_binding(self.bytecode.locals[index].name.clone());
            }
        }
        env.clear_direct_eval_var_conflicts();
        let in_parameter_prologue = self.in_parameter_prologue();
        for (index, local) in self.bytecode.locals.iter().enumerate() {
            if super::vm_bindings::is_compiler_temporary(&local.name) {
                continue;
            }
            if in_parameter_prologue && local.parameter {
                env.mark_direct_eval_var_conflict(local.name.clone());
                continue;
            }
            if local.hoisted {
                continue;
            }
            let active_lexical = self.locals.get(index).is_some_and(Option::is_some);
            if active_lexical {
                env.mark_direct_eval_var_conflict(local.name.clone());
            }
        }
        env.set_private_environment(self.current_private_environment());
        env.set_activation_captured_env(Rc::clone(&self.captured_env));
        if let Some(source) = self.env.captured_binding_source_env() {
            env.set_captured_binding_source_env(Rc::clone(source));
        }
        env.set_parameter_captured_envs(self.parameter_captured_envs.clone());
        env
    }

    /// A shared-realm `CallEnv` with empty frame locals.
    pub(super) fn realm_env(&self) -> CallEnv {
        self.attach_host(self.env.empty_frame())
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
    /// re-enter on each resume; ordinary functions/scripts run it once.
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
                    let result =
                        if self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot) {
                            let name = self.bytecode.locals[slot].name.clone();
                            self.load_ident_with(&name, Some(slot))
                        } else {
                            self.load_local(slot)
                        };
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::LoadLocalOrUndefined(slot) => {
                    self.stack.push(self.load_local_or_undefined(slot)?)
                }
                Op::LoadNewTarget => self.stack.push(self.load_new_target()),
                op @ (Op::AppendStringLiteralLocal { .. }
                | Op::AppendStringLiteralGlobal { .. }) => self.run_string_append_op(op)?,
                Op::StoreLocal(slot) => {
                    let value = self.pop()?;
                    let result = self.store_local(slot, value);
                    self.handle_runtime_result(result)?;
                }
                Op::AssignLocal(slot) => {
                    let value = self.pop()?;
                    let result = if self.direct_eval_with_stack
                        && self.bytecode.local_is_from_env(slot)
                    {
                        let name = self.bytecode.locals[slot].name.clone();
                        self.store_ident_with(&name, Some(slot), self.bytecode.is_strict(), value)
                    } else {
                        self.assign_local(slot, value)
                    };
                    self.handle_runtime_result(result)?;
                }
                Op::ClearLocal(slot) => self.clear_local(slot)?,
                Op::DefineGlobalVar(name) => {
                    let value = self.pop()?;
                    let result = self.define_global_var(name, value);
                    self.handle_runtime_result(result)?;
                }
                Op::LoadGlobal(name) => {
                    let result = if self.direct_eval_with_stack {
                        self.load_ident_with(&name, None)
                    } else {
                        self.load_global(&name)
                    };
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::StoreGlobalStrict(name) => {
                    let value = self.pop()?;
                    let result = if self.direct_eval_with_stack {
                        self.store_ident_with(&name, None, true, value)
                    } else {
                        self.store_global_strict(name, value)
                    };
                    self.handle_runtime_result(result)?;
                }
                Op::StoreGlobalSloppy(name) => {
                    let value = self.pop()?;
                    let result = if self.direct_eval_with_stack {
                        self.store_ident_with(&name, None, false, value)
                    } else {
                        self.store_global_sloppy(name, value)
                    };
                    self.handle_runtime_result(result)?;
                }
                Op::StoreLocalOrGlobalSloppy { slot, name } => {
                    let value = self.pop()?;
                    let result = self.store_local_or_global_sloppy(slot, name, value);
                    self.handle_runtime_result(result)?;
                }
                Op::TypeofGlobal(name) => {
                    let result: Result<Value, RuntimeError> = (|| {
                        if self.direct_eval_with_stack {
                            return self.typeof_ident_with(&name, None);
                        }
                        let value = if let Some(value) = self.env.module_import_value(&name) {
                            if value.is_uninitialized_lexical_marker() {
                                return Err(RuntimeError {
                                    thrown: None,
                                    message: format!(
                                        "ReferenceError: undefined identifier `{name}`"
                                    ),
                                });
                            }
                            value
                        } else if let Some(value) = self.env.get(&name) {
                            value
                        } else {
                            // A bare global name may resolve to a property on
                            // globalThis added via assignment or
                            // defineProperty; reading it invokes any getter.
                            // typeof yields "undefined" only when the reference
                            // is genuinely unresolvable.
                            self.global_this_own_value(&name)?
                                .unwrap_or(Value::Undefined)
                        };
                        let value = if matches!(
                            &value,
                            Value::Function(function) if function.is_uninitialized_lexical_marker()
                        ) {
                            Value::Undefined
                        } else {
                            value
                        };
                        Ok(Value::String(typeof_value(value).into()))
                    })();
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                op @ (Op::EnterWith
                | Op::ExitWith
                | Op::LoadIdentWith { .. }
                | Op::ResolveIdentWith { .. }
                | Op::LoadResolvedIdentWith { .. }
                | Op::StoreIdentWith { .. }
                | Op::StoreResolvedIdentWith { .. }
                | Op::TypeofIdentWith { .. }
                | Op::DeleteIdentWith { .. }) => {
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
                Op::NewTemplateObject { site, cooked, raw } => {
                    self.new_template_object(site, &cooked, &raw)
                }
                Op::NewObjectLiteral => self.new_object_literal(),
                op @ (Op::EnterDisposableScope
                | Op::RegisterDisposable
                | Op::RegisterAsyncDisposable
                | Op::DisposeScope { .. }) => {
                    self.run_disposal_op(&op)?;
                }
                Op::SetComputedFunctionName(kind) => self.set_computed_function_name(kind)?,
                Op::DefineObjectProperty(meta) => self.define_object_property(meta)?,
                Op::CopyObjectSpread => self.copy_object_spread()?,
                Op::EnumerateKeys => self.enumerate_keys()?,
                Op::ForInKeyIsEnumerable => self.for_in_key_is_enumerable()?,
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
                Op::DeleteIdent(name) => {
                    let result = self.delete_ident(&name);
                    self.stack.push(Value::Boolean(result));
                }
                Op::RequireCallable => {
                    let result = self.require_callable();
                    self.handle_runtime_result(result)?;
                }
                Op::Call(argc) => self.call(argc)?,
                Op::CallDirectEval { argc, is_strict } => self.call_direct_eval(argc, is_strict)?,
                Op::CallSpread => self.call_spread()?,
                Op::CallDirectEvalSpread { is_strict } => {
                    self.call_direct_eval_spread(is_strict)?
                }
                Op::IteratorClose { swallow } => self.iterator_close(swallow)?,
                Op::New(argc) => self.construct(argc)?,
                Op::NewSpread => self.construct_spread()?,
                Op::NewFunction {
                    name,
                    has_name_binding,
                    immutable_name_binding,
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
                    source_text,
                } => {
                    let mut env = self.function_capture_env(&bytecode, &local_names);
                    self.insert_lexical_captures(&mut env, &lexical_captures);
                    let capture_writeback = self.capture_writeback_for_bytecode(
                        &bytecode,
                        &local_names,
                        &lexical_captures,
                    );
                    let (home_object, super_constructor) = if lexical_this {
                        let home_object = self.env.get(HOME_OBJECT_BINDING);
                        let mut super_constructor = self.env.get(SUPER_CONSTRUCTOR_BINDING);
                        if self.load_global("this").is_err() {
                            self.captured_env.borrow_mut().insert(
                                "this".to_owned(),
                                Value::Function(Function::uninitialized_lexical_marker()),
                            );
                            if super_constructor.is_none() {
                                super_constructor = Some(Value::Undefined);
                            }
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
                    let in_parameter_prologue = self.in_parameter_prologue();
                    let captured_env = if in_parameter_prologue {
                        Rc::new(RefCell::new(env.clone()))
                    } else {
                        self.captured_env.clone()
                    };
                    if in_parameter_prologue {
                        self.parameter_captured_envs.push(Rc::clone(&captured_env));
                    }
                    let upvalues =
                        self.captured_upvalues_for_function(&bytecode, &lexical_captures);
                    let immutable_env_binding =
                        self.captured_immutable_function_name(&bytecode, &local_names);
                    let function = Function::new_user_compiled(CompiledUserFunction {
                        name,
                        has_name_binding,
                        immutable_name_binding,
                        immutable_env_binding,
                        params: Rc::new(params),
                        env,
                        module_host: self.module_host.clone(),
                        module_imports: self.env.module_imports(),
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
                        with_stack: self.with_stack.clone(),
                        capture_writeback,
                        upvalues,
                    });
                    function.set_source_text(source_text);
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
                Op::SuperReference => {
                    let result = self.super_reference();
                    if let Some((receiver, lookup_base)) = self.handle_runtime_result(result)? {
                        self.stack.push(receiver);
                        self.stack.push(lookup_base);
                    }
                }
                Op::SuperGetComputed => {
                    let key_value = self.pop()?;
                    let key = self.coerce_property_key(key_value);
                    if let Some(key) = self.handle_runtime_result(key)? {
                        let lookup_base = self.pop()?;
                        let receiver = self.pop()?;
                        let result = self.super_get_from(lookup_base, receiver, &key);
                        if let Some(value) = self.handle_runtime_result(result)? {
                            self.stack.push(value);
                        }
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
                    let key = self.coerce_property_key(key_value);
                    if let Some(key) = self.handle_runtime_result(key)? {
                        let lookup_base = self.pop()?;
                        let receiver = self.pop()?;
                        let result =
                            self.super_set_value_from(lookup_base, receiver, key, value, is_strict);
                        if let Some(value) = self.handle_runtime_result(result)? {
                            self.stack.push(value);
                        }
                    }
                }
                Op::SuperMethod { key } => {
                    let result = self.super_method(PropertyKey::String(key));
                    self.handle_runtime_result(result)?;
                }
                Op::SuperMethodComputed => {
                    let key_value = self.pop()?;
                    let key = self.coerce_property_key(key_value);
                    if let Some(key) = self.handle_runtime_result(key)? {
                        let lookup_base = self.pop()?;
                        let receiver = self.pop()?;
                        let result = self.super_method_from(lookup_base, receiver, key);
                        self.handle_runtime_result(result)?;
                    }
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
                    self.stack.push(Value::String(typeof_value(value).into()));
                }
                Op::ToString => {
                    let value = self.pop()?;
                    let mut env = self.current_env();
                    let result = to_js_string_with_env(value, &mut env);
                    self.apply_env(env);
                    // Route a throwing toString/Symbol.toPrimitive through the
                    // try-handler stack so `` `${bad}` `` is catchable, instead
                    // of escaping the VM loop.
                    if let Some(string) = self.handle_runtime_result(result)? {
                        self.stack.push(Value::String(string.into()));
                    }
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
                Op::AbruptJump(target) => {
                    self.abrupt_jump(target)?;
                }
                Op::FreshIterationScope(ref slots) => self.fresh_iteration_scope(slots),
                Op::PushCapturedEnv => self.push_captured_env(),
                Op::PopCapturedEnv => self.pop_captured_env(),
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
                    cleanup_slots,
                } => self.enter_try(catch, finally, catch_scope, cleanup_slots),
                Op::ExitTry => self.exit_try()?,
                Op::EndFinally => {
                    if let Some(value) = self.end_finally()? {
                        return Ok(Completion::Return(value));
                    }
                }
                Op::DiscardPendingAbrupt => {
                    self.pending_throw = None;
                    self.pending_return = None;
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
                Op::ThrowReferenceError(message) => {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!("ReferenceError: {message}"),
                    });
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
                    let Some(host) = self.module_host.as_ref() else {
                        return Err(RuntimeError {
                            thrown: None,
                            message: "SyntaxError: 'import.meta' is only valid in a module"
                                .to_owned(),
                        });
                    };
                    self.stack
                        .push(Value::Object(host.borrow_mut().import_meta()));
                }
            }
        }
    }

    fn get_prop(&mut self) -> Result<(), RuntimeError> {
        let key_value = self.pop()?;
        let object = self.pop()?;
        if matches!(object, Value::Null | Value::Undefined) {
            let object_name = if matches!(object, Value::Null) {
                "null"
            } else {
                "undefined"
            };
            let key_name = match &key_value {
                Value::String(key) => Some(key.to_string()),
                Value::Number(number) => Some(number.to_string()),
                _ => None,
            };
            let message = match key_name {
                Some(key) => {
                    format!("TypeError: Cannot read properties of {object_name} (reading '{key}')")
                }
                None => format!("TypeError: cannot convert {object_name} to object"),
            };
            return Err(RuntimeError {
                thrown: None,
                message,
            });
        }
        if let Value::Number(number) = &key_value
            && let Some(index) = array_index_from_number(*number)
            && let Value::Array(elements) = &object
            && let Some(value) = elements.direct_dense_index_value(index)
        {
            self.stack.push(value);
            return Ok(());
        }
        // Typed-array integer-index read fast path: a non-negative integer index
        // is owned by the exotic [[Get]], so read it directly from the backing
        // buffer without building a string key or re-parsing it.
        if let Value::Number(number) = &key_value
            && let Some(index) = array_index_from_number(*number)
            && let Value::Object(object) = &object
            && crate::typed_array::is_typed_array_object(object)
        {
            let value = crate::typed_array::integer_indexed_value(object, index);
            self.stack.push(value);
            return Ok(());
        }
        let key = self.coerce_property_key(key_value)?;
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

    pub(super) fn pop_arguments(&mut self, argc: usize) -> Result<Vec<Value>, RuntimeError> {
        let mut arguments = Vec::with_capacity(argc);
        for _ in 0..argc {
            arguments.push(self.pop()?);
        }
        arguments.reverse();
        Ok(arguments)
    }

    pub(super) fn pop_argument_array(&mut self, context: &str) -> Result<Vec<Value>, RuntimeError> {
        let value = self.pop()?;
        let mut env = self.current_env();
        let arguments = crate::array::array_like_values_with_env(value, context, &mut env)?;
        self.apply_env(env);
        Ok(arguments)
    }

    pub(super) fn current_env(&self) -> CallEnv {
        self.frame_call_env()
    }

    pub(super) fn call_env(&self, callee: &Value) -> VmCallEnv {
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
            let injected = locals.clone();
            let mut env = self.attach_host(self.env.with_current_frame_locals(locals));
            env.set_activation_captured_env(Rc::clone(&self.captured_env));
            if let Some(source) = self.env.captured_binding_source_env() {
                env.set_captured_binding_source_env(Rc::clone(source));
            }
            env.set_parameter_captured_envs(self.parameter_captured_envs.clone());
            return VmCallEnv {
                injected,
                env,
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
        let mut referenced_names = function_bytecode.referenced_global_names();
        referenced_names.extend(function_bytecode.written_binding_names());
        referenced_names.sort();
        referenced_names.dedup();
        for name in referenced_names {
            let declared_local = function_bytecode.local_slot(&name).is_some_and(|slot| {
                function_bytecode.local_is_body_hoist_only(slot)
                    || function_bytecode.local_is_parameter(slot)
            });
            if !declared_local {
                self.insert_call_binding(locals, binding_names, &name);
            }
        }
        for name in function_bytecode.sloppy_global_assignment_names() {
            insert_missing_binding_name(binding_names, name);
        }
        for name in function_bytecode.local_names() {
            if !function_local_names.iter().any(|local| local == name) {
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

    pub(super) fn apply_call_env(&mut self, env: VmCallEnv) {
        if let Some(binding_names) = env.binding_names {
            self.apply_selected_env(env.env, &binding_names, &env.injected);
        } else {
            self.apply_env(env.env);
        }
        self.refresh_realm_backed_locals_from_realm();
        if !self.bytecode.global_scope {
            return;
        }
        let captured = self.captured_env.borrow();
        if captured.is_empty() {
            return;
        }
        for (name, value) in captured.iter() {
            if matches!(
                value,
                Value::Function(function) if function.is_uninitialized_lexical_marker()
            ) {
                continue;
            }
            if let Some(index) = self.bytecode.local_slot(name) {
                let value = if self.bytecode.global_scope
                    && self.bytecode.local_is_body_hoist_only(index)
                    && !super::vm_bindings::is_compiler_temporary(name)
                {
                    self.global_this_property(name)
                        .unwrap_or_else(|| value.clone())
                } else {
                    value.clone()
                };
                let Some(slot) = self.locals.get_mut(index) else {
                    continue;
                };
                if slot.is_none() && !self.bytecode.local_is_body_hoist_only(index) {
                    continue;
                }
                *slot = Some(value);
            }
        }
    }

    fn refresh_realm_backed_locals_from_realm(&mut self) {
        for index in 0..self.locals.len() {
            if !self.bytecode.local_is_sloppy_global_fallback(index) {
                continue;
            }
            let Some(name) = self.bytecode.local_name_at(index) else {
                continue;
            };
            if !self
                .sloppy_global_names
                .iter()
                .any(|candidate| candidate == name)
            {
                continue;
            }
            let Some(value) = self
                .global_this_property(name)
                .or_else(|| self.realm.borrow().get(name).cloned())
            else {
                continue;
            };
            self.locals[index] = Some(value.clone());
            self.write_through_captured(name, value);
        }
    }

    pub(super) fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(stack_underflow)
    }

    fn captured_immutable_function_name(
        &self,
        bytecode: &Bytecode,
        local_names: &[String],
    ) -> Option<String> {
        let name = self.env.immutable_function_name()?;
        if local_names.iter().any(|local| local == name) {
            return None;
        }
        let references_name = bytecode.local_slot(name).is_some()
            || bytecode.global_names().iter().any(|global| global == name);
        references_name.then(|| name.to_owned())
    }
}

pub(super) fn insert_missing_binding_name(binding_names: &mut Vec<String>, name: &str) {
    if !binding_names.iter().any(|existing| existing == name) {
        binding_names.push(name.to_owned());
    }
}

/// Recovers the source identifier from a mangled block-lexical storage name of
/// the form `\0lexical:<name>:<index>` (see `lexical_storage_name`). Returns
/// `None` for an ordinary, unmangled binding name.
pub(super) fn unmangle_lexical_storage_name(storage_name: &str) -> Option<&str> {
    storage_name
        .strip_prefix("\u{0}lexical:")
        .and_then(|rest| rest.rsplit_once(':'))
        .map(|(name, _index)| name)
}
