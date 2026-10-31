//! Instantiate immutable scope descriptors into escaping positional cell leases.
//!
//! The compiler's PC map chooses active scopes after normal or abrupt control
//! flow. Cached leases retain cells only, never frame storage or name values.

use super::vm::Vm;
use crate::binding::{
    AccessPolicy, CellReference, ScopeCursor, ScopeDescriptorRef, ScopeId, ScopeKind, ScopeLease,
    VarHolder, VariableEnvIdentity,
};
use std::rc::Rc;

#[derive(Default)]
pub(super) struct ScopeInstances {
    activation: u64,
    leases: Vec<Option<Rc<ScopeLease>>>,
    holders: Vec<Option<Rc<VarHolder>>>,
    with_base: usize,
}

impl Vm<'_> {
    pub(super) fn read_global_binding(
        &mut self,
        name: &str,
    ) -> Result<Option<crate::Value>, crate::RuntimeError> {
        if self.env.can_cache_realm_globals()
            && let Some(value) = self.env.get_realm(name)
        {
            if !value.is_uninitialized_lexical_marker() {
                return Ok(Some(value));
            }
            if self.env.can_cache_realm_globals() && self.env.is_global_lexical_binding(name) {
                return Err(crate::RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{name}`"),
                });
            }
        }
        let Some(base) = self.env.selected_global_this() else {
            return Ok(None);
        };
        if name == "this" {
            return Ok(Some(base));
        }
        if name == "globalThis"
            && !self.env.can_cache_realm_globals()
            && matches!(&base, crate::Value::Object(global) if !global.has_own_property(name))
        {
            return Ok(Some(base));
        }
        let mut env = self.realm_env();
        if !crate::property::has_property(base.clone(), &env, name)? {
            return Ok(None);
        }
        crate::property_value(base, name, &mut env).map(Some)
    }

    pub(super) fn store_global_binding(
        &mut self,
        name: &str,
        value: crate::Value,
        strict: bool,
    ) -> Result<(), crate::RuntimeError> {
        if self.env.can_cache_realm_globals()
            && self.env.is_global_lexical_binding(name)
            && let Some(cell) = self.env.realm_binding_cell(name)
        {
            return cell.assign(value, strict);
        }
        let Some(base) = self.env.selected_global_this() else {
            if strict && !self.env.realm_contains(name) {
                return Err(crate::RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{name}`"),
                });
            }
            self.env.insert_realm(name.to_owned(), value);
            return Ok(());
        };
        if let crate::Value::Object(global) = &base {
            match global.write_existing_own_data_property(name, &value) {
                crate::value::OwnDataPropertyWrite::Written => {
                    if self.env.can_cache_realm_globals() {
                        self.env.insert_realm(name.to_owned(), value);
                    }
                    return Ok(());
                }
                crate::value::OwnDataPropertyWrite::ReadOnly => {
                    if strict {
                        return Err(crate::RuntimeError {
                            thrown: None,
                            message: format!(
                                "TypeError: Cannot assign to read only property '{name}'"
                            ),
                        });
                    }
                    return Ok(());
                }
                crate::value::OwnDataPropertyWrite::NeedsSlowPath => {}
            }
        }
        let mut env = self.realm_env();
        if strict && !crate::property::has_property(base.clone(), &env, name)? {
            return Err(crate::RuntimeError {
                thrown: None,
                message: format!("ReferenceError: undefined identifier `{name}`"),
            });
        }
        let written =
            crate::bytecode::set_object_property(base.clone(), name.to_owned(), value, &mut env)?;
        if !written && strict {
            return Err(crate::RuntimeError {
                thrown: None,
                message: "TypeError: cannot set global binding".to_owned(),
            });
        }
        if self.env.can_cache_realm_globals()
            && !self.env.is_global_lexical_binding(name)
            && let crate::Value::Object(global) = base
        {
            match global.own_property(name) {
                Some(property) if !property.is_accessor() => {
                    self.env.insert_realm(name.to_owned(), property.value);
                }
                _ => {
                    self.env.remove_realm(name);
                }
            }
        }
        Ok(())
    }

    pub(super) fn resolve_scope_binding(
        &self,
        name: &str,
        strict: bool,
    ) -> Result<Option<crate::binding::ResolvedReference>, crate::RuntimeError> {
        let Some(cursor) = self.prepare_scope_cursor() else {
            return Ok(None);
        };
        let mut env = self.realm_env();
        match cursor.resolve(name, strict, &mut env)? {
            crate::binding::ResolvedReference::Unresolvable => {
                if self.env.can_cache_realm_globals()
                    && self.env.is_global_lexical_binding(name)
                    && let Some(cell) = self.env.realm_binding_cell(name)
                {
                    return Ok(Some(crate::binding::ResolvedReference::Cell {
                        binding: CellReference {
                            cell,
                            access: AccessPolicy::Mutable,
                        },
                    }));
                }
                if let Some(base) = self.env.selected_global_this()
                    && crate::property::has_property(base.clone(), &env, name)?
                {
                    return Ok(Some(crate::binding::ResolvedReference::Property {
                        base,
                        name: name.to_owned(),
                        strict,
                        with_environment: false,
                    }));
                }
                Ok(None)
            }
            reference => Ok(Some(reference)),
        }
    }

    pub(super) fn read_scope_binding(
        &self,
        reference: &crate::binding::ResolvedReference,
    ) -> Result<crate::Value, crate::RuntimeError> {
        reference.read(&mut self.realm_env())
    }

    pub(super) fn assign_scope_binding(
        &self,
        reference: &crate::binding::ResolvedReference,
        value: crate::Value,
        strict: bool,
    ) -> Result<(), crate::RuntimeError> {
        reference.assign(value, strict, &mut self.realm_env())
    }

    pub(super) fn prepare_scope_cursor(&self) -> Option<ScopeCursor> {
        if !self.bytecode.has_dynamic_scope_tree()
            && self.env.scope_cursor().is_none()
            && self.with_stack().is_empty()
        {
            return None;
        }
        let id = self
            .bytecode
            .binding_plan
            .pc_scopes
            .get(self.ip.saturating_sub(1))
            .copied()
            .unwrap_or(ScopeId(0));
        if self.env.has_global_binding_root()
            && id.0 == 0
            && self.env.scope_cursor().is_none()
            && !self.bytecode.binding_plan.scopes[0]
                .declarations
                .iter()
                .any(|id| {
                    let meta = &self.bytecode.binding_plan.bindings[id.0];
                    meta.kind == crate::binding::BindingKind::Lexical
                        && !self.bytecode.binding_plan.realm_lexical_slots[meta.slot.0]
                })
        {
            return None;
        }
        let mut storage = self.scope_instances.borrow_mut();
        let instances = storage.get_or_insert_with(|| {
            Box::new(ScopeInstances {
                activation: self.env.variable_identity().activation,
                ..ScopeInstances::default()
            })
        });
        let mut current = Some(id);
        let mut with_depth = 0;
        while let Some(scope) = current {
            let descriptor = &self.bytecode.binding_plan.scopes[scope.0];
            with_depth += usize::from(descriptor.kind == ScopeKind::With);
            current = descriptor.parent;
        }
        instances.with_base = self.with_stack().len().saturating_sub(with_depth);
        let head = self.instantiate_scope(id, instances);
        let variable_id = self.bytecode.binding_plan.scopes[id.0].variable_target;
        let variable = if self.bytecode.is_direct_eval_code() {
            self.env
                .scope_cursor()
                .map(|cursor| cursor.variable.clone())
                .unwrap_or_else(|| self.instantiate_scope(variable_id, instances))
        } else {
            self.instantiate_scope(variable_id, instances)
        };
        Some(ScopeCursor { head, variable })
    }

    pub(super) fn closure_scope_cursor(&self, child: &super::ir::Bytecode) -> Option<ScopeCursor> {
        if child.has_dynamic_scope_tree()
            || self.env.scope_cursor().is_some()
            || !self.with_stack().is_empty()
            || !self.bytecode.global_scope && self.bytecode.contains_direct_eval()
        {
            self.prepare_scope_cursor()
        } else {
            None
        }
    }

    fn instantiate_scope(&self, id: ScopeId, instances: &mut ScopeInstances) -> Rc<ScopeLease> {
        let descriptor = &self.bytecode.binding_plan.scopes[id.0];
        if id.0 == 0
            && let Some(root) = self.env.entry_scope()
        {
            return root.clone();
        }
        let parent = descriptor
            .parent
            .map(|id| self.instantiate_scope(id, instances))
            .or_else(|| self.env.scope_cursor().map(|cursor| cursor.head.clone()));
        if let Some(lease) = instances.leases.get(id.0).and_then(Option::as_ref) {
            let same_parent = match (&lease.parent, &parent) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            };
            if same_parent
                && descriptor.kind != ScopeKind::With
                && lease
                    .cells
                    .iter()
                    .zip(&descriptor.declarations)
                    .all(|(binding, id)| {
                        let slot = self.bytecode.binding_plan.bindings[id.0].slot.0;
                        self.local_upvalue_cell(slot)
                            .is_some_and(|cell| cell.ptr_eq(&binding.cell))
                    })
            {
                return lease.clone();
            }
        }
        let cells: Box<[_]> = descriptor
            .declarations
            .iter()
            .map(|id| {
                let meta = &self.bytecode.binding_plan.bindings[id.0];
                let cell = self
                    .local_upvalue_cell(meta.slot.0)
                    .cloned()
                    .or_else(|| self.env.frame_binding_cell(&meta.name))
                    .unwrap_or_else(|| {
                        if descriptor.kind == ScopeKind::Eval
                            && matches!(
                                meta.kind,
                                crate::binding::BindingKind::Var
                                    | crate::binding::BindingKind::Function
                            )
                        {
                            crate::binding::BindingCell::with_metadata(
                                self.bytecode
                                    .binding_meta(meta.slot.0)
                                    .expect("binding metadata"),
                                None,
                            )
                        } else {
                            panic!(
                                "exposure missing cell: {} slot {} pc {} scope {}",
                                meta.name, meta.slot.0, self.ip, id.0
                            )
                        }
                    });
                let access = if self
                    .env
                    .module_import_cell(&meta.name)
                    .is_some_and(|import| import.ptr_eq(&cell))
                {
                    AccessPolicy::Import
                } else {
                    AccessPolicy::Mutable
                };
                CellReference { cell, access }
            })
            .collect();
        let own_vars = (descriptor.variable_target == id && !self.env.has_global_binding_root())
            .then(|| {
                if instances.holders.len() <= id.0 {
                    instances.holders.resize_with(id.0 + 1, || None);
                }
                instances.holders[id.0]
                    .get_or_insert_with(|| {
                        Rc::new(VarHolder::new(VariableEnvIdentity {
                            activation: instances.activation,
                            boundary: id,
                        }))
                    })
                    .clone()
            });
        let object = (descriptor.kind == ScopeKind::With).then(|| {
            let mut current = Some(id);
            let mut depth = 0;
            while let Some(id) = current {
                let scope = &self.bytecode.binding_plan.scopes[id.0];
                depth += usize::from(scope.kind == ScopeKind::With);
                current = scope.parent;
            }
            // Captured with objects live in the parent lease. While the old
            // driver still passes its retained vector, its own with regions
            // occupy the tail; select the current region by lexical depth.
            self.with_stack()
                .get(instances.with_base + depth - 1)
                .cloned()
                .expect("active with region")
        });
        let lease = Rc::new(ScopeLease {
            descriptor: ScopeDescriptorRef {
                owner: self.bytecode.binding_plan.scopes.clone(),
                id,
            },
            parent,
            cells,
            object,
            own_vars,
        });
        if instances.leases.len() <= id.0 {
            instances.leases.resize_with(id.0 + 1, || None);
        }
        instances.leases[id.0] = Some(lease.clone());
        lease
    }
}
