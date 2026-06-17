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

use super::CaptureWriteback;
use super::ir::{Bytecode, Op};
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
        let mut referenced_names = function_bytecode.closure_referenced_global_names();
        referenced_names.extend(function_bytecode.closure_written_binding_names());
        referenced_names.sort();
        referenced_names.dedup();
        for name in referenced_names {
            if !binding_is_declared_local(function_bytecode, &name) {
                self.insert_referenced_binding(&mut env, &name);
            }
        }
        for name in function_bytecode.local_names() {
            if !function_local_names.iter().any(|local| local == name) {
                self.insert_referenced_binding(&mut env, name);
            }
        }
        env
    }

    fn insert_referenced_binding(&self, env: &mut HashMap<String, Value>, name: &str) {
        if self.in_parameter_prologue()
            && self
                .bytecode
                .local_slot(name)
                .is_some_and(|slot| self.bytecode.local_is_body_hoist_only(slot))
        {
            return;
        }
        let value = self
            .current_local_binding(name)
            .cloned()
            .or_else(|| self.env.locals().get(name).cloned());
        if let Some(value) = value {
            env.insert(name.to_owned(), value);
        }
    }

    pub(super) fn in_parameter_prologue(&self) -> bool {
        if self.bytecode.global_scope {
            return false;
        }
        if !self.bytecode.locals.iter().any(|local| {
            local.name.starts_with("\0\0param_argument_") || local.name == "\0\0rest_argument"
        }) {
            return false;
        }
        !self.bytecode.code[..self.ip]
            .iter()
            .any(|op| matches!(op, super::ir::Op::FunctionPrologueEnd))
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

    pub(super) fn capture_writeback_for_bytecode(
        &self,
        bytecode: &Bytecode,
        function_local_names: &[String],
        lexical_captures: &[(String, usize)],
    ) -> Option<CaptureWriteback> {
        let mut names = Vec::new();
        let mut aliases = Vec::new();
        for (storage_name, slot) in lexical_captures {
            if bytecode.writes_binding(storage_name)
                && let Some(target_name) = self.bytecode.local_name_at(*slot)
            {
                self.push_capture_writeback_alias(&mut aliases, storage_name, target_name);
            }
        }
        self.push_capture_writeback_write_names(bytecode, function_local_names, &mut names);
        let parent_names = names
            .iter()
            .filter(|name| {
                self.bytecode.local_slot(name).is_none_or(|slot| {
                    !(self.bytecode.local_is_body_hoist_only(slot)
                        || self.bytecode.local_is_parameter(slot))
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        let parent = self
            .capture_writeback
            .as_ref()
            .and_then(|writeback| filtered_parent_writeback(writeback, &parent_names));
        (!names.is_empty() || !aliases.is_empty()).then(|| CaptureWriteback {
            target: self.captured_env.clone(),
            names,
            aliases,
            parent,
        })
    }

    fn push_capture_writeback_write_names(
        &self,
        bytecode: &Bytecode,
        function_local_names: &[String],
        names: &mut Vec<String>,
    ) {
        for op in &bytecode.code {
            match op {
                Op::StoreGlobalStrict(name)
                | Op::StoreGlobalSloppy(name)
                | Op::StoreLocalOrGlobalSloppy { name, .. }
                | Op::StoreIdentWith {
                    name, slot: None, ..
                }
                | Op::StoreResolvedIdentWith {
                    name, slot: None, ..
                } => {
                    self.push_capture_writeback_name(names, name);
                }
                Op::StoreLocal(slot)
                | Op::AssignLocal(slot)
                | Op::StoreIdentWith {
                    slot: Some(slot), ..
                }
                | Op::StoreResolvedIdentWith {
                    slot: Some(slot), ..
                } => {
                    if let Some(local) = bytecode.locals.get(*slot)
                        && local.from_env
                        && !function_local_names.iter().any(|name| name == &local.name)
                    {
                        self.push_capture_writeback_name(names, &local.name);
                    }
                }
                _ => {}
            }
        }
        for name in bytecode.closure_written_binding_names() {
            if !binding_is_declared_local(bytecode, &name) {
                self.push_capture_writeback_name(names, &name);
            }
        }
    }

    fn push_capture_writeback_name(&self, names: &mut Vec<String>, name: &str) {
        if crate::function::is_internal_binding_name(name) {
            return;
        }
        if self.current_local_binding(name).is_none()
            && self.bytecode.local_slot(name).is_none()
            && !self.env.locals().contains_key(name)
        {
            return;
        }
        if !names.iter().any(|existing| existing == name) {
            names.push(name.to_owned());
        }
    }

    fn push_capture_writeback_alias(
        &self,
        aliases: &mut Vec<(String, String)>,
        source_name: &str,
        target_name: &str,
    ) {
        if crate::function::is_internal_binding_name(target_name) {
            return;
        }
        if self.current_local_binding(target_name).is_none()
            && self.bytecode.local_slot(target_name).is_none()
            && !self.env.locals().contains_key(target_name)
        {
            return;
        }
        if !aliases
            .iter()
            .any(|(source, target)| source == source_name && target == target_name)
        {
            aliases.push((source_name.to_owned(), target_name.to_owned()));
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

    pub(super) fn refresh_live_locals_from_captured_env(&mut self) {
        let captured_env = self.captured_env.borrow();
        for (name, value) in captured_env.iter() {
            if let Some(index) = self.bytecode.local_slot(name)
                && self.bytecode.local_is_from_env(index)
                && !self.bytecode.local_is_parameter(index)
                && !self.bytecode.local_is_body_hoist_only(index)
                && let Some(local @ Some(_)) = self.locals.get_mut(index)
            {
                *local = Some(value.clone());
            }
            if let Some(binding) = self.env.get_local_mut(name) {
                *binding = value.clone();
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

fn binding_is_declared_local(bytecode: &Bytecode, name: &str) -> bool {
    bytecode.local_slot(name).is_some_and(|slot| {
        bytecode.local_is_body_hoist_only(slot) || bytecode.local_is_parameter(slot)
    })
}

fn filtered_parent_writeback(
    writeback: &CaptureWriteback,
    written_names: &[String],
) -> Option<Box<CaptureWriteback>> {
    let names = writeback
        .names
        .iter()
        .filter(|name| written_names.iter().any(|written| written == *name))
        .cloned()
        .collect::<Vec<_>>();
    let aliases = writeback
        .aliases
        .iter()
        .filter(|(source, target)| {
            written_names
                .iter()
                .any(|written| written == source || written == target)
        })
        .cloned()
        .collect::<Vec<_>>();
    let parent = writeback
        .parent
        .as_deref()
        .and_then(|parent| filtered_parent_writeback(parent, written_names));
    (!names.is_empty() || !aliases.is_empty() || parent.is_some()).then(|| {
        Box::new(CaptureWriteback {
            target: writeback.target.clone(),
            names,
            aliases,
            parent,
        })
    })
}
