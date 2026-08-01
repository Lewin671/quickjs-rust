//! Frame construction: turning a compiled body plus a caller environment into
//! a runnable [`FrameState`].
//!
//! This is deliberately separate from the interpreter loop. Building an
//! activation and running one are different responsibilities with different
//! costs, and construction is what the call-frame migration rewrites; keeping
//! it in its own module lets that work be reviewed without reading the
//! dispatch match.

use super::DirectCallSlots;
use super::frame_program::FrameBytecode;
use super::ir::Bytecode;
use super::operand_stack::OperandStack;
use super::vm::{FrameState, Slot, Vm};
use crate::{
    GLOBAL_THIS_BINDING, ObjectRef, RuntimeError, Value,
    function::{CallEnv, DynamicBindings, Realm, Upvalue, new_realm},
    initialize_builtins,
};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

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

    pub(super) fn persist_global_lexical_bindings(&mut self) {
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

    pub(super) fn new_with_globals_upvalues_with_stack_and_direct_call_slots(
        bytecode: &'a Bytecode,
        env: CallEnv,
        upvalues: Vec<Upvalue>,
        with_stack: Vec<Value>,
        direct_call_slots: Option<DirectCallSlots<'_>>,
    ) -> Self {
        Self::with_frame_bytecode(
            FrameBytecode::Borrowed(bytecode),
            env,
            upvalues,
            with_stack,
            direct_call_slots,
        )
    }

    /// Builds a frame over a bytecode handle the frame itself owns.
    ///
    /// A callee's bytecode is an `Rc` held by a `Function` on its caller's
    /// operand stack, not by anything with the VM's root lifetime, so a routed
    /// call must be able to build a frame from the handle rather than from a
    /// borrow.
    pub(super) fn with_frame_bytecode(
        handle: FrameBytecode<'a>,
        mut env: CallEnv,
        upvalues: Vec<Upvalue>,
        with_stack: Vec<Value>,
        direct_call_slots: Option<DirectCallSlots<'_>>,
    ) -> Self {
        crate::diagnostics::count!(nested_vm_constructions);
        let bytecode: &Bytecode = &handle;
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
        let direct_upvalue_source = direct_call_slots
            .as_ref()
            .map(|direct_call_slots| direct_call_slots.upvalues);
        let direct_upvalues = direct_upvalue_source.map(|source| source.as_slice());
        let direct_realm_upvalue_slots = direct_call_slots
            .as_ref()
            .map_or(0, |direct_call_slots| direct_call_slots.realm_upvalue_slots);
        let (direct_readonly_upvalue_owner, direct_readonly_upvalue_slots) = direct_upvalue_source
            .and_then(|source| {
                let slots = bytecode.direct_readonly_received_upvalue_slots()?;
                let owner = source.function()?;
                (!env.has_module_imports()
                    && owner.upvalues.len() == bytecode.received_upvalue_slots().len()
                    && source.as_slice().len() == owner.upvalues.len())
                .then(|| (owner.clone(), slots))
            })
            .map_or((None, 0), |(owner, slots)| (Some(owner), slots));
        let direct_this = direct_call_slots.and_then(|direct_call_slots| {
            Self::seed_direct_call_slots(bytecode, &mut locals, direct_call_slots)
        });
        let (local_upvalues, direct_realm_binding_slots) =
            if direct_readonly_upvalue_owner.is_some() {
                (
                    Vec::new(),
                    Some(direct_realm_upvalue_slots & direct_readonly_upvalue_slots),
                )
            } else if is_direct_call {
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
            Self::initial_authoritative_slots(bytecode, &local_upvalues, &env)
                & !direct_readonly_upvalue_slots;
        let realm_binding_slots = direct_realm_binding_slots
            .unwrap_or_else(|| Self::initial_realm_binding_slots(bytecode, &local_upvalues, &env));
        // Ordinary function creation captures these runtime-only contexts.
        // The data-only variant keeps object/array SRA available while leaving
        // function literals materialized in a frame that needs those captures.
        let virtual_function_context_safe = env.deopt_bindings().is_none()
            && env.immutable_function_name().is_none()
            && with_stack.is_empty();
        // Keep cold virtual candidates allocation-free. Their first
        // initializer grows this bank only as far as the candidate needs.
        let virtual_values = Vec::new();
        let stack = OperandStack::new(bytecode);
        Self {
            current: FrameState {
                bytecode: handle,
                ip: 0,
                declined_numeric_loop_plans: 0,
                declined_typed_loop_programs: 0,
                numeric_mutation_loop_plans: None,
                virtual_function_context_safe,
                virtual_values,
                stack,
                locals,
                local_upvalues,
                direct_readonly_upvalue_owner,
                direct_readonly_upvalue_slots,
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
            callers: Vec::new(),
            pending_frame_entry: None,
        }
    }

    pub(super) fn seed_direct_call_slots(
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

    pub(super) fn initial_direct_call_slots(bytecode: &Bytecode) -> Vec<Slot> {
        // Fill, then seed only the hoisted slots. Testing every local's flag
        // made frame setup scale with the local count.
        let mut locals = vec![None; bytecode.locals.len()];
        for &slot in bytecode.hoisted_slots() {
            locals[slot as usize] = Some(Value::Undefined);
        }
        locals
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
        for index in 0..self.locals.len() {
            let Some(upvalue) = self.local_upvalue_cell(index) else {
                continue;
            };
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
            if let Some(upvalue) = self.local_upvalue_cell(slot) {
                bindings.overlay_cell(&local.name, upvalue);
            }
        }
        Some(bindings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bytecode::{DirectCallUpvalues, ir::Local},
        eval,
    };

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
    fn direct_readonly_captured_frame_shares_function_upvalues() {
        let Value::Function(function) =
            eval("(function () { let captured = 42; return function () { return captured; }; })()")
                .expect("source should evaluate")
        else {
            panic!("expected returned function");
        };
        let bytecode = function
            .bytecode
            .as_ref()
            .expect("function should have bytecode");
        let slot = *bytecode
            .received_upvalue_slots()
            .first()
            .expect("function should receive captured cell");
        let mut vm = Vm::new_with_globals_upvalues_with_stack_and_direct_call_slots(
            bytecode,
            empty_env(),
            Vec::new(),
            Vec::new(),
            Some(DirectCallSlots {
                this_value: None,
                parameter_slots: bytecode.parameter_slots(),
                arguments: &[],
                upvalues: DirectCallUpvalues::Function(&function),
                realm_upvalue_slots: function.realm_upvalue_slots,
            }),
        );

        assert!(vm.local_upvalues.is_empty());
        assert_eq!(vm.direct_readonly_upvalue_slots, 1_u128 << slot);
        assert!(
            vm.direct_readonly_upvalue_owner
                .as_ref()
                .is_some_and(|owner| owner.upvalues[0].ptr_eq(&function.upvalues[0]))
        );
        assert!(!vm.slot_is_authoritative(slot));
        assert!(matches!(
            vm.load_local(slot),
            Ok(Value::Number(value)) if value == 42.0
        ));
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
