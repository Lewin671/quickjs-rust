//! Closure capture-environment construction for the VM.
//!
//! When `Op::NewFunction`/`NewClass` creates a closure, it snapshots the
//! current frame's referenced bindings (plus the realm intrinsics needed to
//! resolve the new function's prototype chain) into the closure's capture
//! environment. [`Vm::refresh_captured_env`] writes back updates so a closure
//! created earlier in the same frame observes later mutations to the names it
//! captured.

use std::collections::HashMap;

use crate::{CallEnv, Function, Value};

use super::ir::Bytecode;
use super::vm::Vm;

impl Vm<'_> {
    pub(super) fn function_capture_env(
        &self,
        function_bytecode: &Bytecode,
        function_local_names: &[String],
    ) -> HashMap<String, Value> {
        let mut env = HashMap::with_capacity(function_bytecode.locals.len());
        // The created function's `env` field is consulted at construction time to
        // resolve its `.prototype`'s `[[Prototype]]` (`object_prototype(env)`),
        // so seed it with the realm intrinsics. This runs only at closure/class
        // creation (`Op::NewFunction`/`NewClass`), never on the leaf-call path,
        // so the per-call clone the migration removed stays gone.
        {
            let realm = self.realm.borrow();
            for name in crate::RUNTIME_INTRINSIC_NAMES {
                if let Some(value) = realm.get(*name) {
                    env.insert((*name).to_owned(), value.clone());
                }
            }
        }
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

    fn insert_referenced_binding(&self, env: &mut HashMap<String, Value>, name: &str) {
        if let Some(value) = self.current_local_binding(name) {
            env.insert(name.to_owned(), value.clone());
        }
    }

    pub(super) fn insert_lexical_captures(
        &self,
        env: &mut HashMap<String, Value>,
        captures: &[(String, usize)],
    ) {
        for (name, slot) in captures {
            if env.contains_key(name) {
                continue;
            }
            let value = self
                .locals
                .get(*slot)
                .and_then(Option::as_ref)
                .cloned()
                .unwrap_or_else(|| Value::Function(Function::uninitialized_lexical_marker()));
            env.insert(name.clone(), value);
        }
    }

    pub(super) fn refresh_captured_env(&self, env: &HashMap<String, Value>) {
        let mut captured_env = self.captured_env.borrow_mut();
        for (name, value) in env {
            captured_env.insert(name.clone(), value.clone());
        }
    }

    pub(super) fn refresh_locals_from_captured_env(&mut self) {
        let captured_env = self.captured_env.borrow();
        for (name, value) in captured_env.iter() {
            if let Some(index) = self.bytecode.local_slot(name)
                && let Some(local) = self.locals.get_mut(index)
            {
                *local = Some(value.clone());
            }
        }
    }

    pub(super) fn refresh_call_env_from_captured_env(&self, env: &mut CallEnv) {
        let captured_env = self.captured_env.borrow();
        for (name, value) in captured_env.iter() {
            if let Some(binding) = env.get_local_mut(name) {
                *binding = value.clone();
            }
        }
    }

    pub(super) fn current_local_binding(&self, name: &str) -> Option<&Value> {
        self.bytecode
            .local_slot(name)
            .and_then(|index| self.locals.get(index))
            .and_then(Option::as_ref)
    }
}
