use super::util::{stack_underflow, typeof_value};
use super::vm_call::user_bytecode_function;
use super::vm_iter::DelegateStep;
use super::vm_props::{
    array_index_from_number, array_index_from_string, get_property, get_property_key,
};
use super::vm_result::{Completion, FunctionBytecodeResult, ResumeMode};
use super::vm_set::set_property_key;
use super::vm_try::TryFrame;
use super::{
    DirectCallSlots,
    ir::{Bytecode, NamedPropertyCache, Op, decode_index_receiver},
};
use crate::{
    Function, GLOBAL_THIS_BINDING, HOME_OBJECT_BINDING, ObjectRef, PropertyKey, RuntimeError,
    SUPER_CONSTRUCTOR_BINDING, Value, construct_function,
    function::{CallEnv, CompiledUserFunction, DynamicBindings, Realm, Upvalue, new_realm},
    initialize_builtins, is_truthy,
    property::try_to_property_key_without_coercion,
    to_js_string_with_env, to_property_key_value,
    value::OwnDataPropertyWrite,
};
use std::{
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut},
    rc::Rc,
};
pub(super) type Slot = Option<Value>;

pub(super) struct OperandStack<'a> {
    bytecode: &'a Bytecode,
    values: Vec<Value>,
}

impl<'a> OperandStack<'a> {
    fn new(bytecode: &'a Bytecode) -> Self {
        Self {
            bytecode,
            values: bytecode.take_operand_stack(),
        }
    }

    pub(super) fn take(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.values)
    }

    pub(super) fn replace(&mut self, values: Vec<Value>) {
        let previous = std::mem::replace(&mut self.values, values);
        self.bytecode.recycle_operand_stack(previous);
    }
}

impl Deref for OperandStack<'_> {
    type Target = Vec<Value>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl DerefMut for OperandStack<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl Drop for OperandStack<'_> {
    fn drop(&mut self) {
        self.bytecode
            .recycle_operand_stack(std::mem::take(&mut self.values));
    }
}

pub(super) struct VmCallEnv {
    pub(super) env: CallEnv,
}
pub(super) fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode)?;
    let value = vm.run()?;
    vm.persist_global_lexical_bindings();
    vm.drain_promise_jobs()?;
    Ok(value)
}
pub(super) fn eval_function_bytecode<'a>(
    bytecode: &'a Bytecode,
    env: CallEnv,
    upvalues: Vec<Upvalue>,
    with_stack: Vec<Value>,
    persist_global_lexicals: bool,
    direct_call_slots: Option<DirectCallSlots<'_>>,
) -> FunctionBytecodeResult<'a> {
    let direct_eval_with_stack = !env.direct_eval_with_stack().is_empty();
    let mut vm = Vm::new_with_globals_upvalues_with_stack_and_direct_call_slots(
        bytecode,
        env,
        upvalues,
        with_stack,
        direct_call_slots,
    );
    vm.persist_global_lexicals = persist_global_lexicals;
    // Ordinary functions created inside `with` are compiled with explicit
    // with-aware ops for free names; their own slot-indexed locals remain
    // closer than the retained object environment. Only direct-eval bytecode
    // needs generic load/store ops redirected through the caller's with stack.
    vm.direct_eval_with_stack = direct_eval_with_stack;
    let value = vm.run();
    let frame = vm.into_frame();
    FunctionBytecodeResult {
        value,
        bytecode,
        env: frame.env,
        locals: frame.locals,
        local_upvalues: frame.local_upvalues,
        sloppy_global_names: frame.sloppy_global_names,
    }
}

pub(super) struct FrameState<'a> {
    pub(super) bytecode: &'a Bytecode,
    pub(super) execution_code: &'a [Op],
    pub(super) ip: usize,
    pub(super) control_loop_plans: Vec<super::vm_control_loop::ControlLoopPlan>,
    pub(super) numeric_loop_plans: Vec<super::vm_numeric_loop::NumericLoopPlan>,
    pub(super) numeric_mutation_loop_plans:
        Vec<super::vm_numeric_mutation_loop::NumericMutationLoopPlan>,
    pub(super) virtual_values: Vec<Value>,
    pub(super) stack: OperandStack<'a>,
    pub(super) locals: Vec<Slot>,
    pub(super) local_upvalues: Vec<Option<Upvalue>>,
    /// Inline per-slot cache for frames where indexed storage is the sole
    /// binding authority. The common first 128 slots require no allocation;
    /// larger slot indices conservatively use the full binding path.
    pub(super) authoritative_slots: u128,
    /// Inline per-slot cache for locals backed by this realm's shared binding
    /// cells. This turns ordinary global-var reads into a direct cell load;
    /// the cell's uninitialized marker still deoptimizes deleted/accessor
    /// globals through the observable global-object path.
    pub(super) realm_binding_slots: u128,
    pub(super) upvalues: Vec<Upvalue>,
    /// Shared realm plus this frame's internal/caller-scope bindings.
    pub(super) env: CallEnv,
    /// Ordinary leaf calls can keep their receiver here instead of
    /// materializing a name-keyed frame binding. Functions that compile an
    /// own `this` local store it in `locals` and leave this empty.
    pub(super) direct_this: Option<Value>,
    pub(super) realm: Realm,
    /// Dynamic-import host copied into every `CallEnv` this VM creates.
    pub(super) module_host: Option<crate::module::ModuleHostRef>,
    /// Test262 `$262.agent` context stamped onto every `CallEnv` this VM builds
    /// (via `attach_host`), so native `Atomics`/`$262.agent` hooks reach it.
    #[cfg(feature = "agents")]
    pub(super) agent_context: Option<crate::agent::AgentContextRef>,
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
    /// Explicit prototype required only for arrays created in a synthetic
    /// cross-realm VM. Precomputed once so ordinary `[]` stays on a cheap
    /// `None` branch instead of consulting realm metadata per allocation.
    pub(super) array_literal_prototype_override: Option<ObjectRef>,
    /// Cached intrinsic Object.prototype for object-literal construction.
    /// Mutable `Object` global rebinding does not invalidate the realm slot.
    pub(super) object_prototype_cache: Option<ObjectRef>,
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
    /// Only a fresh ordinary script VM may batch realm-global loop writes.
    /// Eval, module, dynamic-function, and cross-realm entry points construct
    /// their frames through `new_with_globals*` and leave this disabled.
    pub(super) transactional_realm_globals: bool,
    /// Dynamic source evaluation can replace global descriptors and binding
    /// identities outside the current bytecode stream. Once observed, guarded
    /// realm-global loop batching stays disabled for the rest of this frame.
    pub(super) dynamic_code_executed: bool,
}

pub(super) struct Vm<'a> {
    pub(super) current: FrameState<'a>,
}

impl<'a> Vm<'a> {
    pub(super) fn into_frame(self) -> FrameState<'a> {
        self.current
    }
}

impl<'a> Deref for Vm<'a> {
    type Target = FrameState<'a>;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}

impl DerefMut for Vm<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.current
    }
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
        let realm: Realm = new_realm(globals);
        let mut env = CallEnv::new(Rc::clone(&realm));
        initialize_builtins(&mut env, &global_this);
        Self::initialize_script_global_bindings(bytecode, &realm)?;
        realm.refresh_dynamic_function_realm_global();
        let mut vm = Self::new_with_globals(bytecode, env);
        vm.transactional_realm_globals = true;
        Ok(vm)
    }

    pub(super) fn new_with_globals(bytecode: &'a Bytecode, env: CallEnv) -> Self {
        Self::new_with_globals_and_with_stack(bytecode, env, Vec::new())
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

    pub(super) fn new_with_globals_and_with_stack(
        bytecode: &'a Bytecode,
        env: CallEnv,
        with_stack: Vec<Value>,
    ) -> Self {
        Self::new_with_globals_upvalues_and_with_stack(bytecode, env, Vec::new(), with_stack)
    }

    pub(super) fn new_with_globals_upvalues_and_with_stack(
        bytecode: &'a Bytecode,
        env: CallEnv,
        upvalues: Vec<Upvalue>,
        with_stack: Vec<Value>,
    ) -> Self {
        Self::new_with_globals_upvalues_with_stack_and_direct_call_slots(
            bytecode, env, upvalues, with_stack, None,
        )
    }

    fn new_with_globals_upvalues_with_stack_and_direct_call_slots(
        bytecode: &'a Bytecode,
        mut env: CallEnv,
        upvalues: Vec<Upvalue>,
        with_stack: Vec<Value>,
        direct_call_slots: Option<DirectCallSlots<'_>>,
    ) -> Self {
        if (bytecode.contains_direct_eval() || bytecode.contains_with())
            && env.deopt_bindings().is_none()
        {
            env.set_deopt_bindings(DynamicBindings::new());
        }
        let realm = env.realm_rc();
        let module_host = env.module_host();
        let array_literal_prototype_override = env
            .dynamic_function_realm_global()
            .is_some()
            .then(|| crate::array_prototype(&env))
            .flatten();
        #[cfg(feature = "agents")]
        let agent_context = env.agent_context();
        let is_direct_call = direct_call_slots.is_some();
        let mut locals = if is_direct_call {
            Self::initial_direct_call_slots(bytecode)
        } else {
            Self::initial_slots(bytecode, &env)
        };
        let direct_upvalues = direct_call_slots
            .as_ref()
            .map(|direct_call_slots| direct_call_slots.upvalues);
        let direct_realm_upvalue_slots = direct_call_slots
            .as_ref()
            .map_or(0, |direct_call_slots| direct_call_slots.realm_upvalue_slots);
        let direct_this = direct_call_slots.and_then(|direct_call_slots| {
            Self::seed_direct_call_slots(bytecode, &mut locals, direct_call_slots)
        });
        let (local_upvalues, direct_realm_binding_slots) = if is_direct_call {
            Self::initial_direct_local_upvalues(
                bytecode,
                direct_upvalues.unwrap_or(&upvalues),
                direct_realm_upvalue_slots,
                &env,
            )
        } else {
            (
                Self::initial_local_upvalues(bytecode, &locals, &upvalues, &env),
                None,
            )
        };
        let authoritative_slots =
            Self::initial_authoritative_slots(bytecode, &local_upvalues, &env);
        let realm_binding_slots = direct_realm_binding_slots
            .unwrap_or_else(|| Self::initial_realm_binding_slots(bytecode, &local_upvalues, &env));
        let numeric_loop_plans = bytecode
            .numeric_loop_plans
            .get_or_init(|| super::vm_numeric_loop::NumericLoopPlan::compile_all(bytecode))
            .clone();
        let control_loop_plans = bytecode
            .control_loop_plans
            .get_or_init(|| super::vm_control_loop::ControlLoopPlan::compile_all(bytecode))
            .clone();
        let numeric_mutation_loop_plans = bytecode
            .numeric_mutation_loop_plans
            .get_or_init(|| {
                super::vm_numeric_mutation_loop::NumericMutationLoopPlan::compile_all(bytecode)
            })
            .clone();
        let virtual_object_program = bytecode
            .virtual_object_program
            .get_or_init(|| super::virtual_object::lower(bytecode));
        // Ordinary function creation captures these runtime-only contexts.
        // The data-only variant keeps object/array SRA available while leaving
        // function literals materialized in a frame that needs those captures.
        let virtual_function_context_safe = env.deopt_bindings().is_none()
            && env.immutable_function_name().is_none()
            && with_stack.is_empty();
        let execution_code = virtual_object_program.code_for_frame(
            &bytecode.code,
            authoritative_slots,
            virtual_function_context_safe,
        );
        // Keep cold virtual candidates allocation-free. Their first
        // initializer grows this bank only as far as the candidate needs.
        let virtual_values = Vec::new();
        Self {
            current: FrameState {
                bytecode,
                execution_code,
                ip: 0,
                control_loop_plans,
                numeric_loop_plans,
                numeric_mutation_loop_plans,
                virtual_values,
                stack: OperandStack::new(bytecode),
                locals,
                local_upvalues,
                authoritative_slots,
                realm_binding_slots,
                upvalues,
                env,
                direct_this,
                realm,
                module_host,
                #[cfg(feature = "agents")]
                agent_context,
                sloppy_global_names: Vec::new(),
                try_stack: Vec::new(),
                pending_throw: None,
                pending_return: None,
                pending_jump: None,
                resume_mode: None,
                stop_at_prologue: false,
                array_prototype_cache: None,
                array_literal_prototype_override,
                object_prototype_cache: None,
                with_stack,
                direct_eval_with_stack: false,
                disposable_scopes: Vec::new(),
                persist_global_lexicals: true,
                transactional_realm_globals: false,
                dynamic_code_executed: false,
            },
        }
    }

    fn seed_direct_call_slots(
        bytecode: &Bytecode,
        locals: &mut [Slot],
        direct_call_slots: DirectCallSlots<'_>,
    ) -> Option<Value> {
        let direct_this = if let Some(this_value) = direct_call_slots.this_value {
            if let Some(slot) = bytecode.local_slot("this") {
                locals[slot] = Some(this_value);
                None
            } else {
                Some(this_value)
            }
        } else {
            None
        };
        for (index, &slot) in direct_call_slots.parameter_slots.iter().enumerate() {
            let value = direct_call_slots
                .arguments
                .get(index)
                .cloned()
                .unwrap_or(Value::Undefined);
            locals[slot] = Some(value);
        }
        direct_this
    }

    fn initial_direct_call_slots(bytecode: &Bytecode) -> Vec<Slot> {
        bytecode
            .locals
            .iter()
            .map(|local| local.hoisted.then_some(Value::Undefined))
            .collect()
    }

    fn initial_direct_local_upvalues(
        bytecode: &Bytecode,
        upvalues: &[Upvalue],
        received_realm_binding_slots: u128,
        env: &CallEnv,
    ) -> (Vec<Option<Upvalue>>, Option<u128>) {
        // Most direct leaf calls have no captured, module, or sloppy-global
        // cells. An empty vector represents the all-None state for those
        // frames and avoids allocating one pointer-sized entry per local on
        // every call. Direct-call eligibility excludes operations that can
        // create cells later (closures, eval, and with).
        if !bytecode.has_direct_local_upvalue_routes() && !env.has_module_imports() {
            return (Vec::new(), Some(0));
        }
        let direct_eval_frame = matches!(
            env.get_local(crate::DIRECT_EVAL_BINDING),
            Some(Value::Boolean(true))
        );
        let mut next_received = 0;
        let has_module_imports = env.has_module_imports();
        let mut realm_binding_slots = 0_u128;
        let mut local_upvalues = Vec::with_capacity(bytecode.locals.len());
        for (slot, local) in bytecode.locals.iter().enumerate() {
            if local.compiler_temporary {
                local_upvalues.push(None);
                continue;
            }
            if has_module_imports && let Some(upvalue) = env.module_import_cell(&local.name) {
                if local.is_received_upvalue() {
                    next_received += 1;
                }
                local_upvalues.push(Some(upvalue));
                continue;
            }
            if local.sloppy_global_fallback {
                if direct_eval_frame && let Some(upvalue) = env.local_binding_cell(&local.name) {
                    local_upvalues.push(Some(upvalue));
                    continue;
                }
                let upvalue = env.realm_binding_cell(&local.name);
                if upvalue.is_some() && slot < u128::BITS as usize {
                    realm_binding_slots |= 1_u128 << slot;
                }
                local_upvalues.push(upvalue);
                continue;
            }
            if local.is_received_upvalue() {
                let upvalue = upvalues.get(next_received).cloned();
                next_received += 1;
                if slot < u128::BITS as usize
                    && received_realm_binding_slots & (1_u128 << slot) != 0
                {
                    realm_binding_slots |= 1_u128 << slot;
                }
                local_upvalues.push(upvalue);
                continue;
            }
            local_upvalues.push(None);
        }
        (local_upvalues, Some(realm_binding_slots))
    }

    /// Builds a `CallEnv` over the shared realm with this frame's live slots.
    pub(super) fn frame_call_env(&self) -> CallEnv {
        let deopt_bindings = self.frame_deopt_bindings();
        let mut env = self.attach_host(self.env.fork_current_frame_values());
        for index in 0..self.locals.len() {
            if self.bytecode.local_is_compiler_temporary(index)
                || self.bytecode.local_is_sloppy_global_fallback(index)
                || (self.bytecode.is_global_scope()
                    && self.bytecode.local_is_body_hoist_only(index)
                    && !self.bytecode.local_is_compiler_temporary(index))
            {
                continue;
            }
            if let Some(value) = self.local_slot_value(index) {
                let name = self.bytecode.locals[index].name.clone();
                // Slots are emitted in lexical declaration order. Inserting every
                // active slot under its source name therefore makes an inner
                // shadowing binding replace the outer entry, while an exited
                // block's cleared slot never wins. Slot identity, not a mangled
                // name, remains authoritative for ordinary bytecode access.
                env.insert(name, value);
            }
        }
        for (index, upvalue) in self.local_upvalues.iter().enumerate() {
            let Some(upvalue) = upvalue else { continue };
            if self.bytecode.local_is_compiler_temporary(index)
                || self.bytecode.local_is_sloppy_global_fallback(index)
                || (self.bytecode.is_global_scope()
                    && self.bytecode.local_is_body_hoist_only(index)
                    && !self.bytecode.local_is_compiler_temporary(index))
            {
                continue;
            }
            if self.locals.get(index).is_some_and(Option::is_some)
                || self.bytecode.locals[index].is_received_upvalue()
            {
                env.insert_frame_cell(self.bytecode.locals[index].name.clone(), upvalue.clone());
            }
        }
        for (index, slot) in self.locals.iter().enumerate() {
            if slot.is_some() && self.bytecode.locals[index].catch_binding {
                env.mark_catch_binding(self.bytecode.locals[index].name.clone());
            }
        }
        env.clear_direct_eval_var_conflicts();
        let in_parameter_prologue = self.in_parameter_prologue();
        for (index, local) in self.bytecode.locals.iter().enumerate() {
            if self.bytecode.local_is_compiler_temporary(index) {
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
        if let Some(bindings) = deopt_bindings {
            env.set_deopt_bindings(bindings);
        }
        env
    }

    pub(super) fn frame_deopt_bindings(&self) -> Option<DynamicBindings> {
        let bindings = self.env.deopt_bindings()?.clone();
        for (slot, local) in self.bytecode.locals.iter().enumerate() {
            if self.bytecode.local_is_compiler_temporary(slot)
                || local.sloppy_global_fallback
                || (self.bytecode.is_global_scope()
                    && self.bytecode.local_is_body_hoist_only(slot)
                    && !self.bytecode.local_is_compiler_temporary(slot))
            {
                continue;
            }
            if self.locals.get(slot).is_none_or(Option::is_none)
                && !(self.in_parameter_prologue() && local.from_env)
            {
                continue;
            }
            if let Some(upvalue) = self.local_upvalues.get(slot).and_then(Option::as_ref) {
                bindings.insert_cell(local.name.clone(), upvalue.clone());
            }
        }
        Some(bindings)
    }

    /// A shared-realm `CallEnv` with empty frame locals.
    pub(super) fn realm_env(&self) -> CallEnv {
        self.attach_host(self.env.empty_frame())
    }

    pub(super) fn coerce_property_key(
        &mut self,
        value: Value,
    ) -> Result<PropertyKey, RuntimeError> {
        let value = match try_to_property_key_without_coercion(value) {
            Ok(key) => return Ok(key),
            Err(value) => value,
        };
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
            // Copy the shared bytecode reference out of the VM so the current
            // instruction can stay borrowed while its handler mutates VM state.
            let bytecode = self.bytecode;
            let execution_code = self.execution_code;
            let op = execution_code.get(self.ip).ok_or_else(|| RuntimeError {
                thrown: None,
                message: "bytecode instruction pointer out of bounds".to_owned(),
            })?;
            self.ip += 1;
            match op {
                Op::LoadConst(index) => {
                    let value =
                        bytecode
                            .constants
                            .get(*index)
                            .cloned()
                            .ok_or_else(|| RuntimeError {
                                thrown: None,
                                message: "bytecode constant index out of bounds".to_owned(),
                            })?;
                    self.stack.push(value);
                }
                Op::LoadLocal(slot) => {
                    let result =
                        if self.direct_eval_with_stack && self.bytecode.local_is_from_env(*slot) {
                            let name = self.bytecode.locals[*slot].name.clone();
                            self.load_ident_with(&name, Some(*slot), self.bytecode.is_strict())
                        } else {
                            self.load_local(*slot)
                        };
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::LoadLocalOrUndefined(slot) => {
                    let value = self.load_local_or_undefined(*slot)?;
                    self.stack.push(value);
                }
                Op::LoadNewTarget => {
                    let value = self.load_new_target();
                    self.stack.push(value);
                }
                op @ (Op::AppendStringLiteralLocal { .. }
                | Op::AppendStringLiteralGlobal { .. }) => self.run_string_append_op(op.clone())?,
                Op::StoreLocal(slot) => {
                    let value = self.pop()?;
                    let result = self.store_local(*slot, value);
                    self.handle_runtime_result(result)?;
                }
                Op::AssignLocal(slot) => {
                    let value = self.pop()?;
                    let result = if self.direct_eval_with_stack
                        && self.bytecode.local_is_from_env(*slot)
                    {
                        let name = self.bytecode.locals[*slot].name.clone();
                        self.store_ident_with(&name, Some(*slot), self.bytecode.is_strict(), value)
                    } else {
                        self.assign_local(*slot, value)
                    };
                    self.handle_runtime_result(result)?;
                }
                Op::ClearLocal(slot) => self.clear_local(*slot)?,
                Op::DefineGlobalVar(name) => {
                    let value = self.pop()?;
                    let result = self.define_global_var(name.clone(), value);
                    self.handle_runtime_result(result)?;
                }
                Op::LoadGlobal(name) => {
                    let result = if self.direct_eval_with_stack {
                        self.load_ident_with(name, None, self.bytecode.is_strict())
                    } else {
                        self.load_global(name)
                    };
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::StoreGlobalStrict(name) => {
                    let value = self.pop()?;
                    let result = if self.direct_eval_with_stack {
                        self.store_ident_with(name, None, true, value)
                    } else {
                        self.store_global_strict(name, value)
                    };
                    self.handle_runtime_result(result)?;
                }
                Op::StoreGlobalSloppy { slot, name } => {
                    let value = self.pop()?;
                    let result = if self.direct_eval_with_stack {
                        self.store_ident_with(name, None, false, value)
                    } else {
                        self.store_global_sloppy_at_slot(*slot, name, value)
                    };
                    self.handle_runtime_result(result)?;
                }
                Op::StoreLocalOrGlobalSloppy { slot, name } => {
                    let value = self.pop()?;
                    let result = self.store_local_or_global_sloppy(*slot, name, value);
                    self.handle_runtime_result(result)?;
                }
                Op::TypeofGlobal(name) => {
                    let result: Result<Value, RuntimeError> = (|| {
                        if self.direct_eval_with_stack {
                            return self.typeof_ident_with(name, None);
                        }
                        let value = if let Some(value) = self.env.module_import_value(name) {
                            if value.is_uninitialized_lexical_marker() {
                                return Err(RuntimeError {
                                    thrown: None,
                                    message: format!(
                                        "ReferenceError: undefined identifier `{name}`"
                                    ),
                                });
                            }
                            value
                        } else if let Some(value) = self.env.get(name) {
                            value
                        } else {
                            // A bare global name may resolve to a property on
                            // globalThis added via assignment or
                            // defineProperty; reading it invokes any getter.
                            // typeof yields "undefined" only when the reference
                            // is genuinely unresolvable.
                            self.global_this_own_value(name)?
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
                    self.run_with_op(op.clone())?;
                }
                Op::Pop => {
                    self.pop()?;
                }
                Op::Dup => {
                    let value = self.stack.last().cloned().ok_or_else(stack_underflow)?;
                    self.stack.push(value);
                }
                Op::NewArray { elements } => self.new_array(elements)?,
                Op::NewTemplateObject { site, cooked, raw } => {
                    self.new_template_object(*site, cooked, raw)
                }
                Op::NewObjectLiteral => self.new_object_literal(),
                Op::NewObjectDataLiteral { shape } => {
                    self.new_object_data_literal(shape.clone())?
                }
                op @ (Op::InitVirtualObject { .. }
                | Op::InitVirtualConstants { .. }
                | Op::LoadVirtualValue { .. }
                | Op::StoreVirtualValue { .. }
                | Op::LoadVirtualLength { .. }
                | Op::GuardVirtualObject
                | Op::LoadVirtualBinary { .. }
                | Op::BinaryAssignLocals { .. }
                | Op::IncrementLocal { .. }
                | Op::CopyLocal { .. }
                | Op::CompareLocalsJumpFalse { .. }
                | Op::InitVirtualFunction { .. }
                | Op::CallVirtualFunction { .. }) => self.run_virtual_object_op(op)?,
                op @ (Op::EnterDisposableScope
                | Op::RegisterDisposable
                | Op::RegisterAsyncDisposable
                | Op::DisposeScope { .. }) => {
                    self.run_disposal_op(op)?;
                }
                Op::SetComputedFunctionName(kind) => self.set_computed_function_name(*kind)?,
                Op::DefineObjectProperty(meta) => self.define_object_property(*meta)?,
                Op::CopyObjectSpread => self.copy_object_spread()?,
                Op::EnumerateKeys => self.enumerate_keys()?,
                Op::ForInKeyIsEnumerable => self.for_in_key_is_enumerable()?,
                Op::GetPropNamed { key, cache } => {
                    let result = self.get_named_prop(key, cache);
                    self.handle_runtime_result(result)?;
                }
                Op::GetPropIndex(index) => {
                    let result = self.get_index_prop(*index);
                    self.handle_runtime_result(result)?;
                }
                Op::GetIterator => self.get_iterator()?,
                Op::GetAsyncIterator => self.get_async_iterator()?,
                Op::AsyncIteratorComplete { done_slot } => {
                    self.async_iterator_complete(*done_slot)?
                }
                Op::IteratorStep { done_slot } => self.iterator_step(*done_slot)?,
                Op::IteratorRest { done_slot } => self.iterator_rest(*done_slot)?,
                Op::ObjectRestExcluding { excluded } => self.object_rest_excluding(excluded)?,
                Op::RequireObjectCoercible => self.require_object_coercible()?,
                Op::GetProp => {
                    let result = self.get_prop();
                    self.handle_runtime_result(result)?;
                }
                Op::SetProp { is_strict } => {
                    let result = self.set_prop(*is_strict);
                    self.handle_runtime_result(result)?;
                }
                Op::SetPropIndex { index, is_strict } => {
                    let result = self.set_index_prop(*index, *is_strict);
                    self.handle_runtime_result(result)?;
                }
                Op::SetPropNamed { key, is_strict } => {
                    let result = self.set_named_prop(key, *is_strict);
                    self.handle_runtime_result(result)?;
                }
                Op::GetPrivate(name) => {
                    let result = self.get_private(name);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::SetPrivate(name) => {
                    let result = self.set_private(name);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::PrivateIn(name) => {
                    let result = self.private_in(name);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::DeleteProp { is_strict } => {
                    let result = self.delete_prop(*is_strict);
                    self.handle_runtime_result(result)?;
                }
                Op::DeleteIdent(name) => {
                    let result = self.delete_ident(name);
                    self.stack.push(Value::Boolean(result));
                }
                Op::RequireCallable => {
                    let result = self.require_callable();
                    self.handle_runtime_result(result)?;
                }
                Op::Call(argc) => self.call(*argc)?,
                Op::CallDirectEval { argc, is_strict } => {
                    self.call_direct_eval(*argc, *is_strict)?
                }
                Op::CallSpread => self.call_spread()?,
                Op::CallDirectEvalSpread { is_strict } => {
                    self.call_direct_eval_spread(*is_strict)?
                }
                Op::IteratorClose { swallow } => self.iterator_close(*swallow)?,
                Op::New(argc) => self.construct(*argc)?,
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
                    let (home_object, super_constructor) = if *lexical_this {
                        let home_object = self.env.get_local(HOME_OBJECT_BINDING);
                        let mut super_constructor = self.env.get(SUPER_CONSTRUCTOR_BINDING);
                        if self.load_global("this").is_err() && super_constructor.is_none() {
                            super_constructor = Some(Value::Undefined);
                        }
                        (home_object, super_constructor)
                    } else {
                        (None, None)
                    };
                    let upvalues = self.captured_upvalues_for_function(bytecode, lexical_captures);
                    let immutable_env_binding =
                        self.captured_immutable_function_name(bytecode, local_names);
                    let immutable_env_value = immutable_env_binding
                        .as_deref()
                        .and_then(|name| self.env.get(name))
                        .map(Upvalue::new);
                    let lexical_new_target = if *lexical_this {
                        self.env.get(crate::NEW_TARGET_BINDING).map(Upvalue::new)
                    } else {
                        None
                    };
                    let deopt_bindings = self.frame_deopt_bindings();
                    let function = Function::new_user_compiled(CompiledUserFunction {
                        name: name.clone(),
                        has_name_binding: *has_name_binding,
                        immutable_name_binding: *immutable_name_binding,
                        immutable_env_binding,
                        immutable_env_value,
                        params: Rc::clone(params),
                        realm: Rc::clone(&self.realm),
                        module_host: self.module_host.clone(),
                        module_imports: self.env.module_imports(),
                        bytecode: Rc::clone(bytecode),
                        source_text: source_text.clone(),
                        local_names: Rc::clone(local_names),
                        constructable: *constructable,
                        is_strict: *is_strict,
                        lexical_this: *lexical_this,
                        lexical_arguments: *lexical_arguments,
                        lexical_new_target,
                        is_generator: *is_generator,
                        is_async: *is_async,
                        is_class_constructor: false,
                        is_derived_constructor: false,
                        is_field_initializer: *lexical_this
                            && matches!(
                                self.env.get(crate::FIELD_INITIALIZER_EVAL_BINDING),
                                Some(Value::Boolean(true))
                            ),
                        home_object,
                        super_constructor,
                        deopt_bindings,
                        with_stack: self.with_stack.clone(),
                        upvalues,
                    });
                    self.capture_private_environment(&function);
                    if *is_generator && *is_async {
                        crate::async_generator::wire_async_generator_function_intrinsics(
                            &function,
                            &self.realm_env(),
                        );
                    } else if *is_generator {
                        self.wire_generator_function_intrinsics(&function);
                    } else if *is_async {
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
                        constructor,
                        elements,
                        private_elements,
                        computed_keys,
                        *has_heritage,
                    );
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::SuperGet { key } => {
                    let result = self.super_get(&PropertyKey::String(key.clone()));
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
                    let result = self.super_set(&PropertyKey::String(key.clone()), *is_strict);
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
                        let result = self.super_set_value_from(
                            lookup_base,
                            receiver,
                            key,
                            value,
                            *is_strict,
                        );
                        if let Some(value) = self.handle_runtime_result(result)? {
                            self.stack.push(value);
                        }
                    }
                }
                Op::SuperMethod { key } => {
                    let result = self.super_method(PropertyKey::String(key.clone()));
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
                Op::CallResolved(argc) => self.call_resolved(*argc)?,
                Op::CallResolvedSpread => self.call_resolved_spread()?,
                Op::SuperCall(argc) => {
                    let arguments = self.pop_arguments(*argc)?;
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
                Op::ToPropertyKeyForAccess => {
                    let value = self.pop()?;
                    if matches!(&value, Value::Number(number) if array_index_from_number(*number).is_some())
                    {
                        self.stack.push(value);
                    } else {
                        let key = self.coerce_property_key(value)?;
                        self.stack.push(key.into_value());
                    }
                }
                Op::ToNumeric => {
                    let result = self.eval_to_numeric();
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::Unary(op) => {
                    let result = self.eval_unary(*op);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::Update(op) => {
                    let result = self.eval_update(*op);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::Binary(op) => {
                    let result = self.eval_binary(*op);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                Op::Jump(target) => {
                    let backedge = self.ip - 1;
                    self.jump_with_loop_plans(*target, backedge);
                }
                Op::AbruptJump(target) => {
                    self.abrupt_jump(*target)?;
                }
                Op::FreshIterationScope(slots) => self.fresh_iteration_scope(slots),
                Op::JumpIfFalse(target) => {
                    if !is_truthy(self.stack.last().ok_or_else(stack_underflow)?) {
                        self.ip = *target;
                    }
                }
                Op::JumpIfTrue(target) => {
                    if is_truthy(self.stack.last().ok_or_else(stack_underflow)?) {
                        self.ip = *target;
                    }
                }
                Op::JumpIfNotNullish(target) => {
                    if !matches!(self.stack.last(), Some(Value::Null | Value::Undefined)) {
                        self.ip = *target;
                    }
                }
                Op::EnterTry {
                    catch,
                    finally,
                    catch_scope,
                    cleanup_slots,
                } => self.enter_try(*catch, *finally, catch_scope.clone(), cleanup_slots.clone()),
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
                    self.enter_body_deopt_scope();
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
                } => match self.yield_delegate(*iterator_slot, *next_slot, *async_delegate)? {
                    DelegateStep::Suspend(value) if *async_delegate => {
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
                Op::ImportCall { has_options } => self.import_call(*has_options)?,
                Op::ImportMeta => {
                    let Some(host) = self.current.module_host.as_ref() else {
                        return Err(RuntimeError {
                            thrown: None,
                            message: "SyntaxError: 'import.meta' is only valid in a module"
                                .to_owned(),
                        });
                    };
                    let import_meta = host.borrow_mut().import_meta();
                    self.current.stack.push(Value::Object(import_meta));
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
        } else if let PropertyKey::String(key) = &key
            && let Some(result) = self.try_direct_leaf_getter(&object, key)
        {
            result?
        } else {
            let mut env = self.current_env();
            let value = get_property_key(object, &key, &mut env)?;
            self.apply_env(env);
            value
        };
        self.stack.push(value);
        Ok(())
    }

    fn get_named_prop(
        &mut self,
        key: &str,
        cache: &NamedPropertyCache,
    ) -> Result<(), RuntimeError> {
        let object = if let Some(slot) = cache.local_slot() {
            let direct_eval_lookup =
                self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot);
            if self.slot_is_authoritative(slot)
                && !direct_eval_lookup
                && let Some(Some(object)) = self.locals.get(slot)
                && !matches!(
                    object,
                    Value::Function(function) if function.is_uninitialized_lexical_marker()
                )
                && let Some(value) = self.try_cached_get_string(object, key, cache)
            {
                self.stack.push(value);
                return Ok(());
            }
            self.load_local(slot)?
        } else {
            self.pop()?
        };
        if matches!(object, Value::Null | Value::Undefined) {
            let object_name = if matches!(object, Value::Null) {
                "null"
            } else {
                "undefined"
            };
            return Err(RuntimeError {
                thrown: None,
                message: format!(
                    "TypeError: Cannot read properties of {object_name} (reading '{key}')"
                ),
            });
        }
        let value = if let Some(value) = self.try_cached_get_string(&object, key, cache) {
            value
        } else if let Some(result) = self.try_direct_leaf_getter(&object, key) {
            result?
        } else {
            let mut env = self.current_env();
            let value = get_property(object, key, &mut env)?;
            self.apply_env(env);
            value
        };
        self.stack.push(value);
        Ok(())
    }

    fn get_index_prop(&mut self, encoded_index: usize) -> Result<(), RuntimeError> {
        let (index, local_slot) = decode_index_receiver(encoded_index);
        let object = if let Some(slot) = local_slot {
            let direct_eval_lookup =
                self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot);
            if self.slot_is_authoritative(slot)
                && !direct_eval_lookup
                && let Some(Some(object)) = self.locals.get(slot)
            {
                let value = match object {
                    Value::Array(elements) => elements.direct_dense_index_value(index),
                    Value::Object(object) if crate::typed_array::is_typed_array_object(object) => {
                        Some(crate::typed_array::integer_indexed_value(object, index))
                    }
                    _ => None,
                };
                if let Some(value) = value {
                    self.stack.push(value);
                    return Ok(());
                }
            }
            self.load_local(slot)?
        } else {
            self.pop()?
        };
        if matches!(object, Value::Null | Value::Undefined) {
            let object_name = if matches!(object, Value::Null) {
                "null"
            } else {
                "undefined"
            };
            return Err(RuntimeError {
                thrown: None,
                message: format!(
                    "TypeError: Cannot read properties of {object_name} (reading '{index}')"
                ),
            });
        }
        if let Value::Array(elements) = &object
            && let Some(value) = elements.direct_dense_index_value(index)
        {
            self.stack.push(value);
            return Ok(());
        }
        if let Value::Object(object) = &object
            && crate::typed_array::is_typed_array_object(object)
        {
            let value = crate::typed_array::integer_indexed_value(object, index);
            self.stack.push(value);
            return Ok(());
        }

        let key = PropertyKey::String(index.to_string());
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
        // `a[i] = x` append loops and computed compound updates. The latter
        // arrive as canonical string keys because the compiler must perform
        // observable `ToPropertyKey` exactly once before the read/write pair.
        let array_index = match &key_value {
            Value::Number(number) => array_index_from_number(*number),
            Value::String(key) => array_index_from_string(key),
            _ => None,
        };
        if let Some(index) = array_index
            && let Some(Value::Array(elements)) = self.stack.last()
            && elements.dense_index_store_eligible(index)
        {
            let elements = elements.clone();
            // A plain array with the default prototype takes the dense-store fast
            // path when the index has no own special descriptor and the realm's
            // Array.prototype carries no own indexed property that an OrdinarySet
            // would have to honor. Both checks are O(1), so a tight `a[i] = x`
            // loop avoids the string-key allocation and prototype walk of the
            // generic path.
            if self.array_uses_realm_prototype(&elements)
                && !self.array_prototype_has_index_property().unwrap_or(true)
            {
                self.pop()?;
                elements.set(index, value.clone());
                self.stack.push(value);
                return Ok(());
            }
        }
        if let Value::Number(number) = &key_value
            && let Some(index) = array_index_from_number(*number)
            && let Some(Value::Object(object)) = self.stack.last()
            && crate::typed_array::is_typed_array_object(object)
        {
            let object = object.clone();
            self.set_typed_array_index(&object, index, &value)?;
            self.pop()?;
            self.stack.push(value);
            return Ok(());
        }
        let key = self.coerce_property_key(key_value)?;
        let object = self.pop()?;
        self.set_property_value(object, key, value, is_strict)
    }

    fn set_index_prop(&mut self, index: usize, is_strict: bool) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let object = self.pop()?;

        if let Value::Array(elements) = &object
            && elements.dense_index_store_eligible(index)
        {
            let elements = elements.clone();
            // Mirror `set_prop`'s dense-array eligibility exactly. Custom own
            // descriptors, custom prototypes, and indexed Array.prototype
            // properties must retain the full OrdinarySet path below.
            if self.array_uses_realm_prototype(&elements)
                && !self.array_prototype_has_index_property().unwrap_or(true)
            {
                elements.set(index, value.clone());
                self.stack.push(value);
                return Ok(());
            }
        }

        if let Value::Object(typed_array) = &object
            && crate::typed_array::is_typed_array_object(typed_array)
        {
            let typed_array = typed_array.clone();
            self.set_typed_array_index(&typed_array, index, &value)?;
            self.stack.push(value);
            return Ok(());
        }

        self.set_property_value(
            object,
            PropertyKey::String(index.to_string()),
            value,
            is_strict,
        )
    }

    /// Shared IntegerIndexedElementSet path for already-classified numeric
    /// indices. Both computed numeric keys and numeric-literal bytecode call
    /// this helper so primitive conversion, detached-buffer handling, and
    /// object coercion cannot drift between the two fast paths.
    fn set_typed_array_index(
        &mut self,
        object: &ObjectRef,
        index: usize,
        value: &Value,
    ) -> Result<(), RuntimeError> {
        if crate::typed_array::try_set_integer_indexed_primitive_element(object, index, value) {
            return Ok(());
        }

        let mut env = self.current_env();
        crate::typed_array::set_integer_indexed_element(object, index, value.clone(), &mut env)?;
        self.apply_env(env);
        Ok(())
    }

    fn set_named_prop(&mut self, key: &Rc<str>, is_strict: bool) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let object = self.pop()?;
        if !self.is_global_object(&object)
            && let Value::Object(object_ref) = &object
            && !crate::symbol::is_symbol_primitive(object_ref)
        {
            match object_ref.write_existing_own_data_property(key, &value) {
                OwnDataPropertyWrite::Written => {
                    self.stack.push(value);
                    return Ok(());
                }
                OwnDataPropertyWrite::ReadOnly => {
                    if is_strict {
                        return Err(RuntimeError {
                            thrown: None,
                            message: "TypeError: cannot set property".to_owned(),
                        });
                    }
                    self.stack.push(value);
                    return Ok(());
                }
                OwnDataPropertyWrite::NeedsSlowPath => {
                    if self.try_create_ordinary_own_data_property(
                        object_ref,
                        Rc::clone(key),
                        &value,
                    ) {
                        self.stack.push(value);
                        return Ok(());
                    }
                }
            }
        }
        self.set_property_value(
            object,
            PropertyKey::String(key.to_string()),
            value,
            is_strict,
        )
    }

    fn set_property_value(
        &mut self,
        object: Value,
        key: PropertyKey,
        value: Value,
        is_strict: bool,
    ) -> Result<(), RuntimeError> {
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
        if !updates_global_binding
            && let (Value::Object(object), PropertyKey::String(key)) = (&object, &key)
        {
            match object.write_existing_own_data_property(key, &value) {
                OwnDataPropertyWrite::Written => {
                    self.stack.push(value);
                    return Ok(());
                }
                OwnDataPropertyWrite::ReadOnly => {
                    if is_strict {
                        return Err(RuntimeError {
                            thrown: None,
                            message: "TypeError: cannot set property".to_owned(),
                        });
                    }
                    self.stack.push(value);
                    return Ok(());
                }
                OwnDataPropertyWrite::NeedsSlowPath => {
                    if self.try_create_ordinary_own_data_property(
                        object,
                        Rc::from(key.as_str()),
                        &value,
                    ) {
                        self.stack.push(value);
                        return Ok(());
                    }
                }
            }
        }
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
            self.env.insert_realm(key, value.clone());
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
        if let [Value::Number(date_value)] = arguments.as_slice()
            && matches!(
                &callee,
                Value::Function(function) if function.native_kind() == Some(crate::NativeFunction::Date)
            )
        {
            let mut env = self.realm_env();
            let result =
                crate::date::fast_construct_date_from_number(callee.clone(), *date_value, &mut env);
            if let Some(result) = self.handle_call_result(result)? {
                self.stack.push(result);
            }
            return Ok(());
        }
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
        if user_bytecode_function(callee).is_some() {
            let env = self.attach_host(self.env.new_function_frame());
            return VmCallEnv { env };
        }
        VmCallEnv {
            env: self.current_env(),
        }
    }

    pub(super) fn apply_call_env(&mut self, env: VmCallEnv) {
        self.apply_env(env.env);
        self.refresh_realm_backed_locals_from_realm();
    }

    fn refresh_realm_backed_locals_from_realm(&mut self) {
        for index in 0..self.locals.len() {
            if !self.bytecode.local_is_sloppy_global_fallback(index) {
                continue;
            }
            let Some(name) = self.current.bytecode.local_name_at(index) else {
                continue;
            };
            if !self
                .sloppy_global_names
                .iter()
                .any(|candidate| candidate == name)
            {
                continue;
            }
            let value = if let Some(value) = self.realm.get_value(name) {
                value
            } else if let Some(property) = self.global_this_own_property(name)
                && !property.is_accessor()
            {
                property.value
            } else {
                continue;
            };
            self.current.locals[index] = Some(value.clone());
            if let Some(binding) = self.current.env.module_live_binding_cell(name) {
                binding.set(value);
            }
        }
    }

    pub(super) fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(stack_underflow)
    }

    /// Performs one bytecode jump while preserving the shared counted-loop
    /// accelerators attached to ordinary backward edges.
    pub(super) fn jump_with_loop_plans(&mut self, target: usize, backedge: usize) {
        if target >= backedge
            || (!super::vm_numeric_mutation_loop::try_run_numeric_mutation_loop(
                self, target, backedge,
            ) && !super::vm_numeric_loop::try_run_numeric_loop(self, target, backedge)
                && !super::vm_control_loop::try_run_control_loop(self, target, backedge))
        {
            self.ip = target;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ir::Local;

    fn local(name: &str, from_env: bool) -> Local {
        Local {
            name: name.to_owned(),
            compiler_temporary: false,
            hoisted: false,
            hoisted_function: false,
            parameter: false,
            catch_binding: false,
            mutable: true,
            from_env,
            sloppy_global_fallback: false,
        }
    }

    fn empty_env() -> CallEnv {
        CallEnv::new(new_realm(HashMap::new()))
    }

    #[test]
    fn direct_cell_free_frame_uses_empty_upvalue_storage() {
        let bytecode = Bytecode::new(Vec::new(), vec![local("value", false)], Vec::new());
        let env = empty_env();

        let (local_upvalues, realm_binding_slots) =
            Vm::initial_direct_local_upvalues(&bytecode, &[], 0, &env);

        assert!(local_upvalues.is_empty());
        assert_eq!(realm_binding_slots, Some(0));
        assert_eq!(
            Vm::initial_authoritative_slots(&bytecode, &local_upvalues, &env),
            1
        );
    }

    #[test]
    fn direct_captured_frame_keeps_received_upvalue_storage() {
        let bytecode = Bytecode::new(Vec::new(), vec![local("captured", true)], Vec::new());
        let env = empty_env();
        let captured = Upvalue::new(Value::Number(42.0));

        let (local_upvalues, realm_binding_slots) =
            Vm::initial_direct_local_upvalues(&bytecode, std::slice::from_ref(&captured), 0, &env);

        assert_eq!(local_upvalues.len(), 1);
        assert_eq!(realm_binding_slots, Some(0));
        assert!(
            local_upvalues[0]
                .as_ref()
                .is_some_and(|upvalue| upvalue.ptr_eq(&captured))
        );
    }

    #[test]
    fn direct_captured_frame_reuses_preclassified_realm_cell_slot() {
        let bytecode = Bytecode::new(Vec::new(), vec![local("captured", true)], Vec::new());
        let realm = new_realm(HashMap::from([(
            "captured".to_owned(),
            Value::Number(42.0),
        )]));
        let env = CallEnv::new(realm);
        let captured = env.realm_binding_cell("captured").unwrap();

        let (local_upvalues, realm_binding_slots) =
            Vm::initial_direct_local_upvalues(&bytecode, std::slice::from_ref(&captured), 1, &env);

        assert_eq!(realm_binding_slots, Some(1));
        assert!(
            local_upvalues[0]
                .as_ref()
                .is_some_and(|upvalue| upvalue.ptr_eq(&captured))
        );
    }

    #[test]
    fn direct_module_frame_keeps_import_cell_storage() {
        let bytecode = Bytecode::new(Vec::new(), vec![local("imported", false)], Vec::new());
        let mut env = empty_env();
        let exports = DynamicBindings::new();
        exports.insert("exported".to_owned(), Value::Number(7.0));
        env.set_module_import(
            "imported".to_owned(),
            exports.clone(),
            "exported".to_owned(),
        );

        let (local_upvalues, realm_binding_slots) =
            Vm::initial_direct_local_upvalues(&bytecode, &[], 0, &env);

        assert_eq!(local_upvalues.len(), 1);
        assert_eq!(realm_binding_slots, Some(0));
        assert!(
            local_upvalues[0]
                .as_ref()
                .zip(exports.cell("exported").as_ref())
                .is_some_and(|(local, exported)| local.ptr_eq(exported))
        );
    }
}
