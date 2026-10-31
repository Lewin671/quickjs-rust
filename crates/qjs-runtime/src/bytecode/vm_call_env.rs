//! Temporary owned service entry helpers until the borrowed Context cut.
//! Direct-eval facts are typed site data. This module has no marker provenance,
//! scrub protocol or separate call-environment wrapper.

use crate::function::CallEnv;
use crate::value::Value;

use super::vm::Vm;
use super::vm_call::user_bytecode_function;

impl<'a> Vm<'a> {
    /// A shared-realm `CallEnv` with empty frame locals.
    pub(super) fn realm_env(&self) -> CallEnv {
        self.attach_host(self.env.empty_frame())
    }

    pub(super) fn current_env(&self) -> CallEnv {
        self.frame_call_env()
    }

    pub(super) fn call_env(&self, callee: &Value) -> CallEnv {
        if user_bytecode_function(callee).is_some() {
            return self.attach_host(self.env.new_function_frame());
        }
        // A native builtin has no closure over the caller's frame: it resolves
        // names through the realm. Snapshotting every caller slot into a
        // name-keyed compatibility frame -- and writing the whole thing back
        // afterwards -- is therefore unobservable work for most calls, and it
        // dominated builtin-heavy workloads.
        //
        // Eval/with anywhere in nested code, retained with/deopt scopes and
        // marked-realm closure contexts keep the legacy view. Static
        // callbacks observe captured bindings through shared cells; exporting
        // their caller's names adds no authority those callbacks need.
        self.callee_env()
    }

    fn needs_scope_view(&self) -> bool {
        self.bytecode.has_dynamic_scope_tree()
            || !self.with_stack().is_empty()
            || self.bytecode.creates_closures() && self.env.has_realm_override_context()
    }

    /// Services for native calls, getters, setters, traps and coercion hooks.
    /// Static closures use shared cells, including writes through callbacks;
    /// only a frame with dynamic lookup requirements exports a named view.
    /// Natives still own their environment and may mutate the realm/cells.
    pub(super) fn callee_env(&self) -> CallEnv {
        if self.needs_scope_view() {
            return self.current_env();
        }
        self.realm_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler::compile_script;

    #[test]
    fn static_callbacks_need_cells_without_a_caller_name_view() {
        let script = qjs_parser::parse_script(
            "let value = 1; const callback = function () { value++; }; [1].forEach(callback);",
        )
        .unwrap();
        let code = compile_script(&script).unwrap();
        assert!(code.creates_closures());
        let vm = Vm::new(&code).unwrap();
        assert!(!vm.needs_scope_view());
        assert!(!vm.callee_env().has_writable_frame_state());
    }

    #[test]
    fn nested_dynamic_callbacks_keep_the_caller_name_view() {
        for source in [
            "let value = 1; const callback = function () { return eval('value'); };",
            "let value = 1; class C { method() { return eval('value'); } }",
            "let value = 1; with ({}) { value++; }",
        ] {
            let script = qjs_parser::parse_script(source).unwrap();
            let code = compile_script(&script).unwrap();
            let vm = Vm::new(&code).unwrap();
            assert!(vm.needs_scope_view(), "{source}");
        }
    }

    #[test]
    fn static_closures_preserve_marked_realm_views_even_when_disabled() {
        let domain = crate::property::atom::AtomDomain::new();
        let script =
            qjs_parser::parse_script("const callback = function () { return 1; };").unwrap();
        let code = compile_script(&script).unwrap();
        let mut vm = Vm::new(&code).unwrap();
        for marker in [
            Value::Object(crate::ObjectRef::new(
                std::collections::HashMap::new(),
                &domain,
            )),
            Value::Boolean(false),
        ] {
            vm.env.set_global_selection(marker.clone());
            assert!(vm.needs_scope_view());
            assert_eq!(
                vm.callee_env().explicit_global_selection_value(),
                Some(match marker {
                    Value::Object(_) => marker,
                    _ => Value::Undefined,
                })
            );
        }
    }
}
