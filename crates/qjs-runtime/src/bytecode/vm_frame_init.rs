//! Frame construction: turning a compiled body plus a caller environment into
//! a runnable [`FrameState`].
//!
//! This is deliberately separate from the interpreter loop. Building an
//! activation and running one are different responsibilities with different
//! costs; keeping construction in its own module lets it be reviewed without
//! reading the dispatch match.

use super::DirectCallSlots;
use super::frame_program::FrameBytecode;
use super::ir::Bytecode;
use super::operand_stack::OperandStack;
use super::vm::{ColdFrame, FrameState, Slot, Vm};
use crate::{
    GLOBAL_THIS_BINDING, ObjectRef, RuntimeError, Value,
    function::{BindingCell, CallEnv, Realm},
    initialize_builtins,
};
use std::collections::HashMap;
use std::rc::Rc;

impl<'a> Vm<'a> {
    pub(super) fn new(bytecode: &'a Bytecode) -> Result<Self, RuntimeError> {
        let mut globals = HashMap::new();
        let atoms = crate::property::atom::AtomDomain::new();
        let global_this = Value::Object(ObjectRef::new(HashMap::new(), &atoms));
        globals.insert(GLOBAL_THIS_BINDING.to_owned(), global_this.clone());
        globals.insert("undefined".to_owned(), Value::Undefined);
        // The realm cell is live before builtin installation: every `install_*`
        // runs against a `CallEnv` over it and writes intrinsics straight to the
        // shared cell (`insert_realm`), so no install-vs-runtime signature split
        // is needed.
        let realm: Realm = crate::function::new_realm_sharing_atoms(globals, atoms);
        let mut env = CallEnv::new(Rc::clone(&realm));
        initialize_builtins(&mut env, &global_this);
        Self::initialize_script_global_bindings(bytecode, &realm)?;
        realm.refresh_dynamic_function_realm_global();
        Ok(Self::new_with_globals(bytecode, env))
    }

    pub(super) fn new_with_globals(bytecode: &'a Bytecode, env: CallEnv) -> Self {
        Self::new_with_globals_and_with_stack(bytecode, env, Vec::new())
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
        upvalues: Vec<BindingCell>,
        with_stack: Vec<Value>,
    ) -> Self {
        Self::new_with_globals_upvalues_with_stack_and_direct_call_slots(
            bytecode, env, upvalues, with_stack, None,
        )
    }

    pub(super) fn new_with_globals_upvalues_with_stack_and_direct_call_slots(
        bytecode: &'a Bytecode,
        env: CallEnv,
        upvalues: Vec<BindingCell>,
        with_stack: Vec<Value>,
        direct_call_slots: Option<DirectCallSlots<'_>>,
    ) -> Self {
        Self::with_frame_bytecode(
            FrameBytecode::Borrowed(bytecode),
            env,
            upvalues,
            with_stack,
            direct_call_slots,
            None,
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
        upvalues: Vec<BindingCell>,
        with_stack: Vec<Value>,
        direct_call_slots: Option<DirectCallSlots<'_>>,
        call_inputs: Option<crate::function::CallInputs<'_>>,
    ) -> Self {
        crate::diagnostics::count!(nested_vm_constructions);
        let bytecode: &Bytecode = &handle;
        if env.owner_kind().is_none() {
            let saved = env.execution_state();
            let kind = if bytecode.binding_plan.scopes[0].kind == crate::binding::ScopeKind::Module
            {
                crate::context::OwnerKind::Module
            } else {
                crate::context::OwnerKind::Script
            };
            let receiver = if kind == crate::context::OwnerKind::Module {
                Value::Undefined
            } else {
                env.selected_global_this()
                    .or_else(|| env.get("this"))
                    .unwrap_or(Value::Undefined)
            };
            env.set_call_owner(crate::context::CallOwner::root(receiver, kind));
            if saved.0 == crate::context::ExecutionMode::Eval {
                env.set_execution_state(saved.0, saved.1);
            }
        }
        let (mode, _) = env.execution_state();
        if mode != crate::context::ExecutionMode::ClassKey {
            env.set_execution_state(mode, bytecode.is_strict());
        }
        if bytecode.global_scope {
            env.select_script_bindings();
        }
        // `QJS_CF_TRACE=1` names the body behind every general-path frame, so
        // a histogram of these lines ranks the callees no compact tier runs.
        #[cfg(feature = "perf-counters")]
        if std::env::var_os("QJS_CF_TRACE").is_some() {
            eprintln!(
                "CFVM params=({}) len={}",
                bytecode.parameter_names().join(","),
                bytecode.code.len()
            );
            if std::env::var_os("QJS_CF_TRACE").is_some_and(|v| v == "3") {
                eprintln!("CFVMCODE {:?}", bytecode.code);
            }
        }
        if bytecode.binding_plan.has_realm_lexicals {
            for (slot, &lexical) in bytecode.binding_plan.realm_lexical_slots.iter().enumerate() {
                if lexical && let Some(meta) = bytecode.binding_meta(slot) {
                    env.declare_global_lexical(meta);
                }
            }
        }
        let realm = env.realm_rc();
        let module_host = env.module_host();
        let array_literal_prototype_override = env
            .has_dynamic_function_realm_global()
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
        if let Some(inputs) = call_inputs {
            Self::seed_call_inputs(bytecode.parameter_slots(), &mut locals, inputs.arguments);
            if let Some(slot) = bytecode.binding_plan.parameters.rest_slot {
                let array = crate::ArrayRef::new(
                    env.realm(),
                    inputs
                        .arguments
                        .iter()
                        .skip(inputs.function.params.positional.len())
                        .cloned()
                        .collect(),
                );
                if env.has_dynamic_function_realm_global()
                    && let Some(prototype) = crate::array_prototype(&env)
                {
                    array
                        .set_prototype_slot(Some(crate::Prototype::Object(prototype)))
                        .expect("fresh rest array accepts its realm prototype");
                }
                locals[slot] = Some(Value::Array(array));
            }
        }
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
        if let Some(inputs) = call_inputs
            && let Some(slot) = bytecode.binding_plan.arguments_slot
        {
            let value = crate::function::arguments_object(
                inputs.function,
                inputs.arguments,
                &env,
                &local_upvalues,
            );
            let cell = local_upvalues[slot].as_ref().expect("arguments cell route");
            cell.assign(value.clone(), false)
                .expect("implicit arguments is mutable");
            locals[slot] = Some(value);
            env.insert_frame_cell("arguments".to_owned(), cell.clone());
        }
        if bytecode.has_dynamic_scope_tree() {
            for meta in bytecode
                .binding_plan
                .bindings
                .iter()
                .filter(|meta| meta.kind == crate::binding::BindingKind::Arguments)
            {
                if let Some(cell) = local_upvalues.get(meta.slot.0).and_then(Option::as_ref) {
                    env.insert_frame_cell("arguments".to_owned(), cell.clone());
                }
            }
        }
        let authoritative_slots =
            Self::initial_authoritative_slots(bytecode, &local_upvalues, &env)
                & !direct_readonly_upvalue_slots;
        let realm_binding_slots = direct_realm_binding_slots
            .unwrap_or_else(|| Self::initial_realm_binding_slots(bytecode, &local_upvalues, &env));
        // Ordinary function creation captures these runtime-only contexts.
        // The data-only variant keeps object/array SRA available while leaving
        // function literals materialized in a frame that needs those captures.
        let virtual_function_context_safe = env.scope_cursor().is_none()
            && !bytecode.has_dynamic_scope_tree()
            && with_stack.is_empty();
        // Keep cold virtual candidates allocation-free. Their first
        // initializer grows this bank only as far as the candidate needs.
        let virtual_values = Vec::new();
        let stack = OperandStack::new(bytecode);
        let cold =
            (!with_stack.is_empty() || array_literal_prototype_override.is_some()).then(|| {
                Box::new(ColdFrame {
                    with_stack,
                    array_literal_prototype_override,
                    ..ColdFrame::default()
                })
            });
        Self {
            current: FrameState {
                scope_instances: Default::default(),
                bytecode: handle,
                ip: 0,
                declined_typed_loop_programs: 0,
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
                stop_at_prologue: false,
                array_prototype_cache: None,
                object_prototype_cache: None,
                cold,
                direct_eval_with_stack: false,
                persist_global_lexicals: true,
            },
            callers: Vec::new(),
            pending_frame_entry: None,
        }
    }

    pub(super) fn seed_direct_call_slots(
        _bytecode: &Bytecode,
        locals: &mut [Slot],
        direct_call_slots: DirectCallSlots<'_>,
    ) -> Option<Value> {
        let direct_this = direct_call_slots.this_value;
        Self::seed_call_inputs(
            direct_call_slots.parameter_slots,
            locals,
            direct_call_slots.arguments,
        );
        direct_this
    }

    fn seed_call_inputs(parameter_slots: &[usize], locals: &mut [Slot], arguments: &[Value]) {
        for (index, &slot) in parameter_slots.iter().enumerate() {
            let value = arguments.get(index).cloned().unwrap_or(Value::Undefined);
            locals[slot] = Some(value);
        }
    }

    pub(super) fn initial_direct_call_slots(bytecode: &Bytecode) -> Vec<Slot> {
        // Fill, then seed only the hoisted slots. Testing every local's flag
        // made frame setup scale with the local count.
        let mut locals = bytecode.take_local_slots(bytecode.locals.len());
        for &slot in bytecode.hoisted_slots() {
            locals[slot as usize] = Some(Value::Undefined);
        }
        locals
    }

    /// Retains typed scope ownership; no slot/name/value projection is made.
    pub(super) fn frame_call_env(&self) -> CallEnv {
        let mut env = self.attach_host(self.env.scope_service_view());
        env.set_scope_cursor(self.prepare_scope_cursor());
        env.set_private_environment(self.current_private_environment());
        env
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

    use crate::function::{DynamicBindings, new_realm};

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
        let captured = BindingCell::new(Value::Number(42.0));

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

    #[test]
    fn top_level_loop_compiler_temporaries_stay_frame_private() {
        let script = qjs_parser::parse_script(
            "function addOne(value) { return value + 1; } var limit = 4, checksum = 0; for (var index = 0; index < limit; index++) checksum += addOne(index);",
        )
        .expect("source should parse");
        let bytecode =
            crate::bytecode::compiler::compile_script(&script).expect("source should compile");
        assert!(bytecode.locals.iter().any(|local| local.compiler_temporary));
        assert!(
            bytecode
                .hoisted_local_names()
                .all(|name| !name.starts_with("\0\0"))
        );
        let vm = Vm::new(&bytecode).expect("top-level VM should initialize");
        for (slot, local) in bytecode.locals.iter().enumerate() {
            if !local.compiler_temporary {
                continue;
            }
            assert!(vm.slot_is_authoritative(slot));
            assert!(vm.local_upvalues.get(slot).is_none_or(Option::is_none));
            assert_eq!(vm.locals[slot], Some(Value::Undefined));
        }
    }
}
