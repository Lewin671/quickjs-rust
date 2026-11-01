//! With-aware opcodes use the ordered live scope resolver. Only the active
//! frame's own object regions live on its stack; functions retain ScopeLeases.
//! Explicit references are selected once and consumed without a fallback lookup.
use super::{ir::Op, util::typeof_value, vm::Vm};
use crate::{RuntimeError, Value, binding::ResolvedReference, object::boxed_primitive};

impl Vm<'_, '_> {
    /// Executes a `with`-related opcode: scope push/pop and the with-aware
    /// identifier load/store/typeof. Centralizing the stack interaction here
    /// keeps the main bytecode loop terse.
    pub(super) fn run_with_op(&mut self, op: Op) -> Result<(), RuntimeError> {
        match op {
            Op::EnterWith => {
                let value = self.pop()?;
                let result: Result<Value, RuntimeError> = match value {
                    Value::Null | Value::Undefined => Err(RuntimeError {
                        thrown: None,
                        message: "TypeError: cannot convert null or undefined to object".to_owned(),
                    }),
                    Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Boolean(_) => {
                        let env = self.context();
                        Ok(boxed_primitive(value, &env)
                            .expect("primitive value should box to object"))
                    }
                    other => Ok(other),
                };
                if let Some(object) = self.handle_runtime_result(result)? {
                    self.cold_mut().with_stack.push(object);
                }
            }
            Op::ExitWith => {
                if let Some(cold) = self.cold.as_deref_mut() {
                    cold.with_stack.pop();
                }
            }
            Op::LoadIdentWith {
                name,
                slot,
                is_strict,
            } => {
                let result = self.load_ident_with(&name, slot, is_strict);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::ResolveIdentWith {
                name, object_slot, ..
            } => {
                let result = self.resolve_ident_with(&name, object_slot);
                self.handle_runtime_result(result)?;
            }
            Op::LoadResolvedIdentWith {
                name,
                slot,
                object_slot,
                is_strict,
            } => {
                let result = self.load_resolved_ident_with(&name, slot, object_slot, is_strict);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::StoreIdentWith {
                name,
                slot,
                is_strict,
            } => {
                let value = self.pop()?;
                let result = self.store_ident_with(&name, slot, is_strict, value);
                self.handle_runtime_result(result)?;
            }
            Op::StoreResolvedIdentWith {
                name,
                slot,
                object_slot,
                is_strict,
            } => {
                let value = self.pop()?;
                let result =
                    self.store_resolved_ident_with(&name, slot, object_slot, is_strict, value);
                self.handle_runtime_result(result)?;
            }
            Op::TypeofIdentWith { name, slot } => {
                let result = self.typeof_ident_with(&name, slot);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::DeleteIdentWith { name, slot } => {
                self.delete_ident_with(&name, slot)?;
            }
            _ => unreachable!("run_with_op received a non-with opcode"),
        }
        Ok(())
    }

    fn with_name_reference(
        &mut self,
        name: &str,
        strict: bool,
    ) -> Result<ResolvedReference, RuntimeError> {
        let scope = self.bytecode.binding_plan.pc_scopes[self.ip.saturating_sub(1)];
        self.select_binding_reference(name, strict, scope)
    }
    fn with_name_needs_live_lookup(&self, slot: Option<usize>) -> bool {
        self.bytecode.has_dynamic_scope_tree()
            || self.env.scope_cursor().is_some()
            || !self.with_stack().is_empty()
            || slot.is_none_or(|slot| {
                self.bytecode.locals[slot].sloppy_global_fallback
                    || self.slot_is_realm_binding(slot)
            })
    }
    pub(super) fn load_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
        strict: bool,
    ) -> Result<Value, RuntimeError> {
        if !self.with_name_needs_live_lookup(slot) {
            return self.load_local(slot.expect("indexed binding"));
        }
        let reference = self.with_name_reference(name, strict)?;
        self.read_scope_binding(&reference)
    }
    pub(super) fn store_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
        strict: bool,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if !self.with_name_needs_live_lookup(slot) {
            return self.assign_local(slot.expect("indexed binding"), value);
        }
        let reference = self.with_name_reference(name, strict)?;
        self.assign_scope_binding(&reference, value, strict)
    }
    pub(super) fn resolve_ident_with(
        &mut self,
        name: &str,
        object_slot: usize,
    ) -> Result<(), RuntimeError> {
        let reference = self.with_name_reference(name, self.execution_is_strict())?;
        let object = match &reference {
            ResolvedReference::Property {
                base,
                with_environment: true,
                ..
            } => base.clone(),
            _ => Value::Undefined,
        };
        self.store_local(object_slot, object)?;
        let lifetime = self.bytecode.binding_plan.references.explicit[object_slot]
            .expect("explicit reference lifetime");
        self.save_reference(object_slot, lifetime, reference);
        Ok(())
    }
    pub(super) fn load_resolved_ident_with(
        &mut self,
        name: &str,
        _slot: Option<usize>,
        bank: usize,
        _strict: bool,
    ) -> Result<Value, RuntimeError> {
        let reference = self
            .cold()
            .and_then(|cold| cold.resolved_bindings.get(bank))
            .and_then(Option::as_ref)
            .cloned()
            .ok_or_else(|| missing_reference(name))?;
        self.read_scope_binding(&reference)
    }
    pub(super) fn store_resolved_ident_with(
        &mut self,
        name: &str,
        _slot: Option<usize>,
        bank: usize,
        strict: bool,
        value: Value,
    ) -> Result<(), RuntimeError> {
        let reference = self
            .cold
            .as_mut()
            .and_then(|cold| cold.resolved_bindings.get_mut(bank))
            .and_then(Option::take)
            .ok_or_else(|| missing_reference(name))?;
        self.assign_scope_binding(&reference, value, strict)
    }
    pub(super) fn typeof_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
    ) -> Result<Value, RuntimeError> {
        if !self.with_name_needs_live_lookup(slot) {
            return self
                .load_local(slot.expect("indexed binding"))
                .map(|value| Value::String(typeof_value(value)));
        }
        let reference = self.with_name_reference(name, false)?;
        if matches!(&reference, ResolvedReference::Unresolvable)
            || matches!(&reference, ResolvedReference::Global(global) if !global.is_bound())
        {
            return Ok(Value::String("undefined".into()));
        }
        self.read_scope_binding(&reference)
            .map(|value| Value::String(typeof_value(value)))
    }
    pub(super) fn delete_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
    ) -> Result<(), RuntimeError> {
        let deleted = if self.with_name_needs_live_lookup(slot) {
            let reference = self.with_name_reference(name, false)?;
            reference.delete(&mut self.context())?
        } else {
            false
        };
        self.stack.push(Value::Boolean(deleted));
        Ok(())
    }
}
fn missing_reference(name: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("bytecode reference for `{name}` was not selected"),
    }
}
