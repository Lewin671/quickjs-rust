//! Binding stress hooks never retain or mutate escaped storage. Force mode
//! exercises indexed cell routes through the existing VM; production tiers
//! and their admission remain unchanged when the debug mode is off.
use super::vm::Vm;
use crate::binding::{BindingKind, ScopeId};

impl Vm<'_, '_> {
    #[inline]
    pub(super) fn force_binding_exposure(&mut self, id: ScopeId) {
        if !self.runtime.binding_debug.forcing() {
            return;
        }
        let owner = self.bytecode.clone();
        let mut current = Some(id);
        while let Some(id) = current {
            let descriptor = &owner.binding_plan.scopes[id.0];
            for binding in &descriptor.declarations {
                let meta = &owner.binding_plan.bindings[binding.0];
                let slot = meta.slot.0;
                let local = &owner.locals[slot];
                if meta.kind == BindingKind::Received
                    || local.compiler_temporary
                    || local.sloppy_global_fallback
                    || self.env.has_global_binding_root() && local.hoisted && !local.parameter
                {
                    continue;
                }
                self.ensure_upvalue_for_parent_slot(slot);
            }
            current = descriptor.parent;
        }
        self.binding_scope = Some(id);
        self.slow_flags |= super::vm::SLOW_SCOPES;
    }

    #[inline]
    pub(super) fn verify_binding_authority(&self) {
        if !self.runtime.binding_debug.verifying() {
            return;
        }
        for (slot, local) in self.bytecode.locals.iter().enumerate() {
            let Some(cell) = self.local_upvalue_cell(slot) else {
                continue;
            };
            assert!(
                !local.compiler_temporary,
                "temporary has a binding cell: {slot}"
            );
            assert!(
                self.locals[slot].is_none(),
                "cell retains a slot copy: {} ({slot})",
                local.name
            );
            assert!(
                !self.slot_is_authoritative(slot),
                "cell and slot both authoritative: {}",
                local.name
            );
            if let Some(index) = self.bytecode.binding_plan.received_indices[slot]
                && let Some(source) = self.upvalues.get(index).or_else(|| {
                    self.direct_readonly_upvalue_owner
                        .as_ref()?
                        .upvalues
                        .get(index)
                })
            {
                assert!(
                    source.ptr_eq(cell),
                    "received order changed: {}",
                    local.name
                );
            }
            if local.is_received_upvalue()
                || cell.metadata().kind == BindingKind::ObjectStorage
                // Realm object-cache eligibility deliberately excludes lexical
                // cells. Verification instead checks canonical cell identity:
                // a free route's placeholder metadata cannot redefine const.
                || self.env.realm_binding_cell(&local.name)
                    .is_some_and(|canonical| canonical.ptr_eq(cell))
                || self.env.has_module_import(&local.name)
                || self.env.entry_scope().is_some()
                || local.name == "this"
            {
                continue;
            }
            if let Some(meta) = self.bytecode.binding_meta(slot) {
                assert_eq!(cell.metadata().name, meta.name, "metadata name: {slot}");
                assert_eq!(
                    cell.metadata().strict_write,
                    meta.strict_write,
                    "metadata policy: {slot}"
                );
            }
        }
    }

    /// Completed private vectors are not observable. Poison is retained in the
    /// existing recycler until take clears it; live cells/leases are untouched.
    pub(super) fn recycle_binding_slots(&mut self) {
        let poison = self.runtime.binding_debug.poisoning();
        let slots = std::mem::take(&mut self.current.locals);
        self.bytecode.recycle_local_slots_with_poison(slots, poison);
    }
}

/// Admission/counter unit tests explicitly exercise slot execution. Semantic
/// runtime/fixture tests keep the captured stress policy, including forcing.
#[cfg(test)]
pub(crate) fn eval_with_slots_for_test(source: &str) -> Result<crate::Value, crate::RuntimeError> {
    let script = qjs_parser::parse_script(source).map_err(|error| crate::RuntimeError {
        thrown: None,
        message: error.message,
    })?;
    let bytecode = super::compiler::compile_script(&script).map_err(|error| error.error)?;
    let (mut runtime, activation) = crate::context::HostRoot::script().into_parts();
    runtime.binding_debug.force = false;
    let mut vm = Vm::new(&bytecode, &mut runtime, activation)?;
    let value = vm.run()?;
    vm.drain_promise_jobs()?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    fn eval_with_policy(source: &str, force: bool) -> Result<Value, crate::RuntimeError> {
        let script = qjs_parser::parse_script(source).unwrap();
        let bytecode = super::super::compiler::compile_script(&script).unwrap();
        let (mut runtime, activation) = crate::context::HostRoot::script().into_parts();
        runtime.binding_debug = crate::binding::BindingDebug {
            verify: true,
            force,
            poison: true,
            poison_activations: true,
        };
        let mut vm = Vm::new(&bytecode, &mut runtime, activation)?;
        let value = vm.run()?;
        vm.drain_promise_jobs()?;
        Ok(value)
    }

    #[test]
    fn forced_exposure_keeps_nested_loop_bindings_and_rest_redeclarations() {
        for (source, expected) in [
            (
                "var p=function(){var t=0;for(let j=0;j<2;j++){var n=0;while(n++<2){if(this.q!==0.5)t+=j;}}return t;};p.call({q:1});",
                Value::Number(2.0),
            ),
            (
                "function f(c,...a){eval('var b=8;');var a=(b+a);return +a;}f('s');",
                Value::Number(8.0),
            ),
        ] {
            assert_eq!(eval_with_policy(source, true), Ok(expected), "{source}");
        }
    }

    #[test]
    fn verifier_uses_canonical_global_lexical_metadata_for_free_routes() {
        for source in [
            "const y=1;var f=eval('(function(){return y--;})');var err;try{f();}catch(e){err=e.name;}err+':'+y;",
            "const z=1;function f(){{let z=2;}return z--;}var err;try{f();}catch(e){err=e.name;}err+':'+z;",
        ] {
            assert_eq!(
                eval_with_policy(source, false),
                Ok(Value::String("TypeError:1".into())),
                "{source}"
            );
        }
    }

    #[test]
    fn forced_cells_and_recycled_poison_preserve_escaping_identity() {
        let Value::Function(function) =
            crate::eval("function f() { let value = 4; return () => value; } f;").unwrap()
        else {
            panic!("function");
        };
        let bytecode = function.bytecode.as_ref().unwrap();
        let slot = bytecode.local_slot("value").unwrap();
        let mut retained = Vec::new();
        for force in [false, true, false, true] {
            let activation = crate::function::enter_indexed_activation(&function);
            let mut runtime = crate::context::RuntimeServices::for_realm(activation.realm());
            runtime.binding_debug = crate::binding::BindingDebug {
                verify: true,
                force,
                poison: true,
                poison_activations: true,
            };
            let mut vm = Vm::new_with_globals(bytecode, &mut runtime, activation);
            vm.stop_at_prologue = true;
            assert!(matches!(
                vm.run_completion().unwrap(),
                super::super::vm_result::Completion::PrologueEnd
            ));
            vm.stop_at_prologue = false;
            let body_scope = vm.bytecode.binding_plan.pc_scopes[vm.ip];
            vm.force_binding_exposure(body_scope);
            assert_eq!(vm.local_upvalue_cell(slot).is_some(), force);
            let Value::Function(read) = vm.run().unwrap() else {
                panic!("reader");
            };
            let cell = vm.local_upvalue_cell(slot).unwrap().clone();
            assert!(cell.ptr_eq(&read.upvalues[0]));
            vm.verify_binding_authority();
            vm.recycle_binding_slots();
            retained.push(cell);
        }
        for cell in retained {
            assert_eq!(cell.read(), Ok(Value::Number(4.0)));
        }
    }
}
