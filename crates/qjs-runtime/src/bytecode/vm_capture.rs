//! Closure capture-environment construction for the VM.
//!
//! When `Op::NewFunction`/`NewClass` creates a closure, it snapshots the
//! current frame's referenced bindings (plus the realm intrinsics needed to
//! resolve the new function's prototype chain) into the closure's capture
//! environment. [`Vm::refresh_captured_env`] writes back updates so a closure
//! created earlier in the same frame observes later mutations to the names it
//! captured.

use std::collections::HashMap;

use crate::{CallEnv, Function, Value, function::Upvalue};

use super::CaptureWriteback;
use super::ir::{Bytecode, Op};
use super::vm::Vm;

impl Vm<'_> {
    pub(super) fn captured_upvalues_for_function(
        &mut self,
        function_bytecode: &Bytecode,
        lexical_captures: &[(String, usize)],
    ) -> Vec<Upvalue> {
        function_bytecode
            .locals
            .iter()
            .filter(|local| local.from_env)
            .filter_map(|local| {
                lexical_captures
                    .iter()
                    .find(|(name, _)| name == &local.name)
                    .map(|(_, slot)| self.ensure_upvalue_for_parent_slot(*slot))
            })
            .collect()
    }

    fn ensure_upvalue_for_parent_slot(&mut self, slot: usize) -> Upvalue {
        if let Some(upvalue) = self.local_upvalues.get(slot).and_then(Option::as_ref) {
            return upvalue.clone();
        }
        if self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| local.from_env)
        {
            let index = self.bytecode.locals[..slot]
                .iter()
                .filter(|local| local.from_env)
                .count();
            if let Some(upvalue) = self.upvalues.get(index) {
                return upvalue.clone();
            }
        }
        let value = self
            .locals
            .get(slot)
            .and_then(Option::as_ref)
            .cloned()
            .unwrap_or_else(|| Value::Function(Function::uninitialized_lexical_marker()));
        let upvalue = Upvalue::new(value);
        if let Some(local_upvalue) = self.local_upvalues.get_mut(slot) {
            *local_upvalue = Some(upvalue.clone());
        }
        upvalue
    }

    pub(super) fn function_capture_env(
        &self,
        function_bytecode: &Bytecode,
        function_local_names: &[String],
    ) -> HashMap<String, Value> {
        self.function_capture_env_with_global_names(function_bytecode, function_local_names)
            .0
    }

    pub(super) fn function_capture_env_without_global_names(
        &self,
        function_bytecode: &Bytecode,
        function_local_names: &[String],
    ) -> HashMap<String, Value> {
        let mut env = HashMap::with_capacity(function_bytecode.locals.len());
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

    pub(super) fn function_capture_env_with_global_names(
        &self,
        function_bytecode: &Bytecode,
        function_local_names: &[String],
    ) -> (HashMap<String, Value>, Vec<String>) {
        let mut env = HashMap::with_capacity(function_bytecode.locals.len());
        let mut global_capture_names = Vec::new();
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
                if function_bytecode.writes_binding(&name) {
                    self.insert_referenced_binding(&mut env, &name);
                } else if self.insert_referenced_global_binding(&mut env, &name) {
                    global_capture_names.push(name);
                }
            }
        }
        for name in function_bytecode.local_names() {
            if !function_local_names.iter().any(|local| local == name) {
                self.insert_referenced_binding(&mut env, name);
            }
        }
        (env, global_capture_names)
    }

    fn insert_referenced_global_binding(
        &self,
        env: &mut HashMap<String, Value>,
        name: &str,
    ) -> bool {
        if self.in_parameter_prologue()
            || name == "this"
            || name == "arguments"
            || !self.with_stack.is_empty()
            || self.env.locals().contains_key(&format!(
                "{}{}",
                crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX,
                name
            ))
        {
            self.insert_referenced_binding(env, name);
            return false;
        }
        if self.env.is_immutable_function_name(name)
            && let Some(value) = self.env.locals().get(name).cloned()
        {
            env.insert(name.to_owned(), value);
            return false;
        }
        let local_value = self
            .bytecode
            .local_slot(name)
            .filter(|slot| self.bytecode.local_is_body_hoist_only(*slot))
            .and_then(|_| self.env.locals().get(name).cloned())
            .or_else(|| self.current_local_binding(name).cloned())
            .or_else(|| self.env.locals().get(name).cloned());
        let global_value = self
            .global_this_property(name)
            .or_else(|| self.realm.borrow().get(name).cloned());
        if let Some(value @ Value::Function(_)) = self.captured_env.borrow().get(name).cloned() {
            env.insert(name.to_owned(), value);
            return false;
        }
        if self.bytecode.local_slot(name).is_none()
            && let Some(value) = global_value.clone()
        {
            env.insert(name.to_owned(), value);
            return true;
        }
        if let Some(value) = local_value {
            let is_global_snapshot = global_value.as_ref() == Some(&value);
            env.insert(name.to_owned(), value);
            return is_global_snapshot;
        }
        if let Some(value) = self
            .global_this_property(name)
            .or_else(|| self.realm.borrow().get(name).cloned())
        {
            env.insert(name.to_owned(), value);
            return true;
        }
        self.insert_referenced_binding(env, name);
        false
    }

    fn insert_referenced_binding(&self, env: &mut HashMap<String, Value>, name: &str) {
        if self.env.is_immutable_function_name(name)
            && let Some(value) = self.env.locals().get(name).cloned()
        {
            env.insert(name.to_owned(), value);
            return;
        }
        if self.in_parameter_prologue()
            && self
                .bytecode
                .local_slot(name)
                .is_some_and(|slot| self.bytecode.local_is_body_hoist_only(slot))
        {
            if let Some(value) = self.env.get(name) {
                env.insert(name.to_owned(), value);
            } else {
                env.insert(
                    name.to_owned(),
                    Value::Function(Function::uninitialized_lexical_marker()),
                );
            }
            return;
        }
        if self
            .bytecode
            .local_slot(name)
            .is_some_and(|slot| self.bytecode.local_is_sloppy_global_fallback(slot))
            && let Some(value) = self
                .global_this_property(name)
                .or_else(|| self.realm.borrow().get(name).cloned())
        {
            env.insert(name.to_owned(), value);
            return;
        }
        let value = self
            .bytecode
            .local_slot(name)
            .filter(|slot| self.bytecode.local_is_body_hoist_only(*slot))
            .and_then(|_| self.env.locals().get(name).cloned())
            .or_else(|| self.current_local_binding(name).cloned())
            .or_else(|| self.env.locals().get(name).cloned());
        if let Some(value) = value {
            env.insert(name.to_owned(), value);
        }
    }

    pub(super) fn in_parameter_prologue(&self) -> bool {
        if self.bytecode.global_scope {
            return false;
        }
        if !self.bytecode.locals.iter().any(|local| local.parameter) {
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
        if self.env.is_immutable_function_name(name) {
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
        if self.env.is_immutable_function_name(target_name) {
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
            if super::vm_bindings::is_compiler_temporary(name) {
                continue;
            }
            if self.in_parameter_prologue() && self.env.is_immutable_function_name(name) {
                continue;
            }
            captured_env.insert(name.clone(), value.clone());
        }
    }

    pub(super) fn refresh_locals_from_captured_env(&mut self) {
        let captured_env = self.captured_env.borrow();
        for (name, value) in captured_env.iter() {
            if super::vm_bindings::is_compiler_temporary(name) {
                continue;
            }
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
            if super::vm_bindings::is_compiler_temporary(name) {
                continue;
            }
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

    /// Refreshes this frame's live captured locals from the shared captured env
    /// after a nested call returns, so a write a sibling/forwarded closure made
    /// to a shared binding is observed by a later read in this frame. Unlike
    /// [`Self::refresh_live_locals_from_captured_env`] (run at frame exit) this
    /// runs mid-execution, so it must skip the internal call-frame bindings
    /// (`this`, `arguments`, `new.target`, ...) whose frame-local value is
    /// authoritative and must not be clobbered by a stale captured snapshot.
    pub(super) fn refresh_shared_captured_locals_after_call(&mut self) {
        if self.captured_env.borrow().is_empty() {
            return;
        }
        let captured_env = self.captured_env.borrow();
        for (name, value) in captured_env.iter() {
            if super::vm_bindings::is_compiler_temporary(name)
                || super::vm_bindings::is_call_frame_binding(name)
            {
                continue;
            }
            if self.in_parameter_prologue()
                && self.env.locals().contains_key(&format!(
                    "{}{}",
                    crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX,
                    name
                ))
            {
                continue;
            }
            if let Some(index) = self.bytecode.local_slot(name)
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

pub(super) fn filtered_parent_writeback(
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
