//! The environment a call runs in, and where that environment came from.
//!
//! A call hands its callee a `CallEnv`. Which one depends on what the callee
//! can observe: a user bytecode function gets its own fresh frame, code that
//! resolves names through the realm gets an empty realm frame, and only a
//! callee that can actually see the caller's dynamic name view gets that view
//! materialized. Recording which of those a call produced -- its provenance --
//! is what lets the call path skip bookkeeping that is absent by construction.

use crate::function::CallEnv;
use crate::value::Value;

use super::vm::Vm;
use super::vm_call::user_bytecode_function;

/// Where a call's [`CallEnv`] came from, and therefore what it can contain.
///
/// The generic call path scrubs direct-eval markers on the way in and out and
/// snapshots the marked dynamic realm around the call. Both are only
/// meaningful for an environment that can actually carry them, and most calls
/// hand over an environment that provably cannot. Recording the provenance is
/// what lets those cases skip work that is absent by construction rather than
/// by an optimistic guess about the callee.
///
/// It is also the fact the frame-stack migration needs: whether a callee can
/// run on the caller's own VM depends on which of these its environment is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum VmCallEnvOrigin {
    /// A fresh function frame for a user bytecode callee. Its frame and deopt
    /// bindings start empty, so it holds no direct-eval marker.
    FreshFunctionFrame,
    /// A fresh empty frame over the shared realm, for a callee that resolves
    /// names through the realm rather than through the caller. Also empty, and
    /// so also marker-free.
    RealmOnly,
    /// The caller's own dynamic name view, materialized because the callee can
    /// observe it. It carries whatever the caller carries, markers included.
    InheritedDynamicFrame,
    /// A direct `eval` environment, which deliberately carries the markers.
    DirectEval,
}

impl VmCallEnvOrigin {
    /// True when the environment provably holds no direct-eval marker.
    ///
    /// `CallEnv::remove` reaches only frame bindings and deopt bindings. Both
    /// start empty in a freshly built frame, and nothing has run against that
    /// frame yet at the point the call path scrubs it, so the scrub cannot
    /// find anything to remove.
    pub(super) fn is_marker_free(self) -> bool {
        matches!(self, Self::FreshFunctionFrame | Self::RealmOnly)
    }
}

pub(super) struct VmCallEnv {
    pub(super) env: CallEnv,
    pub(super) origin: VmCallEnvOrigin,
}

impl<'a> Vm<'a> {
    /// A shared-realm `CallEnv` with empty frame locals.
    pub(super) fn realm_env(&self) -> CallEnv {
        self.attach_host(self.env.empty_frame())
    }

    pub(super) fn current_env(&self) -> CallEnv {
        self.frame_call_env()
    }

    pub(super) fn call_env(&self, callee: &Value) -> VmCallEnv {
        if user_bytecode_function(callee).is_some() {
            let env = self.attach_host(self.env.new_function_frame());
            return VmCallEnv {
                env,
                origin: VmCallEnvOrigin::FreshFunctionFrame,
            };
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
        VmCallEnv {
            env: self.callee_env(),
            origin: self.callee_env_origin(),
        }
    }

    /// The provenance matching [`Self::callee_env`]. Kept beside it so the two
    /// cannot answer differently.
    fn callee_env_origin(&self) -> VmCallEnvOrigin {
        if self.bytecode.has_dynamic_scope_tree()
            || !self.with_stack().is_empty()
            || self.env.deopt_bindings().is_some()
            || self.bytecode.creates_closures() && self.env.has_realm_override_context()
        {
            return VmCallEnvOrigin::InheritedDynamicFrame;
        }
        VmCallEnvOrigin::RealmOnly
    }

    /// Services for native calls, getters, setters, traps and coercion hooks.
    /// Static closures use shared cells, including writes through callbacks;
    /// only a frame with dynamic lookup requirements exports a named view.
    /// Natives still own their environment and may mutate the realm/cells.
    pub(super) fn callee_env(&self) -> CallEnv {
        if self.callee_env_origin() == VmCallEnvOrigin::InheritedDynamicFrame {
            return self.current_env();
        }
        self.realm_env()
    }

    pub(super) fn apply_call_env(&mut self, env: VmCallEnv) {
        self.apply_env(env.env);
        self.refresh_realm_backed_locals_from_realm();
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
        assert_eq!(vm.callee_env_origin(), VmCallEnvOrigin::RealmOnly);
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
            assert_eq!(
                vm.callee_env_origin(),
                VmCallEnvOrigin::InheritedDynamicFrame,
                "{source}"
            );
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
            vm.env.insert(
                "__quickjsRustDynamicFunctionRealm".to_owned(),
                marker.clone(),
            );
            assert_eq!(
                vm.callee_env_origin(),
                VmCallEnvOrigin::InheritedDynamicFrame
            );
            assert_eq!(
                vm.callee_env()
                    .get_local("__quickjsRustDynamicFunctionRealm"),
                Some(marker)
            );
        }
    }
}
