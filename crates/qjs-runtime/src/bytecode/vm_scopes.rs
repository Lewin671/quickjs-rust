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

impl Vm<'_, '_> {
    /// Re-entry creates new private lexical locations even when the previous
    /// instance escaped while still in TDZ. Declaration Init does not replace
    /// a cell already captured in this same scope instance.
    #[inline(never)]
    pub(super) fn update_binding_scope(&mut self, pc: usize) {
        let Some(next) = self.bytecode.binding_plan.pc_scopes.get(pc).copied() else {
            return;
        };
        let Some(previous) = self.binding_scope else {
            return;
        };
        if previous == next {
            return;
        }
        let owner = self.bytecode.clone();
        let scopes = &owner.binding_plan.scopes;
        if self.runtime.binding_debug.poisoning() {
            let mut exited = Some(previous);
            while let Some(id) = exited {
                let mut target = Some(next);
                let mut still_active = false;
                while let Some(active) = target {
                    if active == id {
                        still_active = true;
                        break;
                    }
                    target = scopes[active.0].parent;
                }
                if still_active {
                    break;
                }
                if matches!(
                    scopes[id.0].kind,
                    ScopeKind::Block | ScopeKind::Catch | ScopeKind::Loop | ScopeKind::ClassName
                ) {
                    for binding in &scopes[id.0].declarations {
                        let slot = owner.binding_plan.bindings[binding.0].slot.0;
                        if owner.locals[slot].from_env || owner.locals[slot].compiler_temporary {
                            continue;
                        }
                        // A released private slot has no observer. Escaped
                        // cells and leases retain their own storage untouched.
                        self.locals[slot] = Some(crate::Value::Number(f64::NAN));
                        self.locals[slot] = None;
                        if let Some(route) = self.local_upvalues.get_mut(slot) {
                            *route = None;
                        }
                        if slot < u128::BITS as usize {
                            self.authoritative_slots |= 1_u128 << slot;
                        }
                    }
                    if let Some(instances) = self.scope_instances.borrow_mut().as_mut() {
                        if let Some(lease) = instances.leases.get_mut(id.0) {
                            *lease = None;
                        }
                        if let Some(holder) = instances.holders.get_mut(id.0) {
                            *holder = None;
                        }
                    }
                }
                exited = scopes[id.0].parent;
            }
        }
        let mut current = Some(next);
        while let Some(id) = current {
            let mut old = Some(previous);
            let mut common = false;
            while let Some(prior) = old {
                if prior == id {
                    common = true;
                    break;
                }
                old = scopes[prior.0].parent;
            }
            if common {
                break;
            }
            current = scopes[id.0].parent;
            if !matches!(
                scopes[id.0].kind,
                ScopeKind::Block | ScopeKind::Catch | ScopeKind::Loop | ScopeKind::ClassName
            ) {
                continue;
            }
            for binding in &scopes[id.0].declarations {
                let meta = &owner.binding_plan.bindings[binding.0];
                let slot = meta.slot.0;
                if owner.locals[slot].from_env || owner.locals[slot].compiler_temporary {
                    continue;
                }
                self.locals[slot] = None;
                if let Some(route) = self.local_upvalues.get_mut(slot) {
                    *route = None;
                }
                if slot < u128::BITS as usize {
                    self.authoritative_slots |= 1_u128 << slot;
                }
            }
        }
        self.binding_scope = Some(next);
        self.force_binding_exposure(next);
        self.verify_binding_authority();
    }

    pub(super) fn read_global_binding(
        &mut self,
        name: &str,
    ) -> Result<Option<crate::Value>, crate::RuntimeError> {
        if self.env.can_cache_realm_globals()
            && let Some(cell) = self.env.realm_binding_cell(name)
        {
            if cell.metadata().kind == crate::binding::BindingKind::Lexical {
                return cell.read().map(Some);
            }
            if let Some(value) = cell.value_if_initialized() {
                return Ok(Some(value));
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
        let mut env = self.context();
        if !crate::property::has_property(base.clone(), &mut env, name)? {
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
        let mut env = self.context();
        if strict && !crate::property::has_property(base.clone(), &mut env, name)? {
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
        &mut self,
        name: &str,
        strict: bool,
    ) -> Result<Option<crate::binding::ResolvedReference>, crate::RuntimeError> {
        let Some(cursor) = self.prepare_scope_cursor() else {
            return Ok(None);
        };
        let reference = cursor.resolve(name, strict, &mut self.context())?;
        if !matches!(reference, crate::binding::ResolvedReference::Unresolvable) {
            return Ok(Some(reference));
        }
        // A retained dynamic/self-name scope may not contain the statically
        // captured enclosing scope. Its positional cell still precedes globals.
        if let Some(binding) = self.received_binding_reference(name) {
            return Ok(Some(crate::binding::ResolvedReference::Cell { binding }));
        }
        if !self.env.has_global_binding_root()
            && let Some(cell) = self.env.module_import_cell(name)
        {
            return Ok(Some(crate::binding::ResolvedReference::Cell {
                binding: CellReference {
                    cell,
                    access: AccessPolicy::Import,
                },
            }));
        }
        let reference = crate::binding::GlobalReference::select(name, strict, &mut self.context())?;
        Ok(reference
            .is_bound()
            .then_some(crate::binding::ResolvedReference::Global(reference)))
    }

    /// Static captures are a fallback after live lexical crossing, before the
    /// realm. Their declaration need not have required an escaping scope lease.
    fn received_binding_reference(&self, name: &str) -> Option<CellReference> {
        let slot = *self
            .bytecode
            .received_upvalue_slots()
            .iter()
            .find(|&&slot| {
                self.bytecode
                    .binding_meta(slot)
                    .is_some_and(|meta| meta.name == name)
            })?;
        let cell = self.local_upvalue_cell(slot)?.clone();
        let access = if self
            .env
            .module_import_cell(name)
            .is_some_and(|import| import.ptr_eq(&cell))
        {
            AccessPolicy::Import
        } else {
            AccessPolicy::Mutable
        };
        Some(CellReference { cell, access })
    }

    pub(super) fn select_binding_reference(
        &mut self,
        name: &str,
        strict: bool,
        scope: ScopeId,
    ) -> Result<crate::binding::ResolvedReference, crate::RuntimeError> {
        let cursor = self.prepare_scope_cursor_at(scope);
        if let Some(cursor) = cursor {
            let reference = cursor.resolve(name, strict, &mut self.context())?;
            if !matches!(reference, crate::binding::ResolvedReference::Unresolvable) {
                return Ok(reference);
            }
        }
        if let Some(binding) = self.received_binding_reference(name) {
            return Ok(crate::binding::ResolvedReference::Cell { binding });
        }
        let mut services = self.context();
        if !services.has_global_binding_root()
            && let Some(cell) = services.module_import_cell(name)
        {
            return Ok(crate::binding::ResolvedReference::Cell {
                binding: CellReference {
                    cell,
                    access: AccessPolicy::Import,
                },
            });
        }
        Ok(crate::binding::ResolvedReference::Global(
            crate::binding::GlobalReference::select(name, strict, &mut services)?,
        ))
    }

    pub(super) fn read_scope_binding(
        &mut self,
        reference: &crate::binding::ResolvedReference,
    ) -> Result<crate::Value, crate::RuntimeError> {
        reference.read(&mut self.context())
    }

    pub(super) fn assign_scope_binding(
        &mut self,
        reference: &crate::binding::ResolvedReference,
        value: crate::Value,
        strict: bool,
    ) -> Result<(), crate::RuntimeError> {
        reference.assign(value, strict, &mut self.context())
    }

    pub(super) fn prepare_scope_cursor(&mut self) -> Option<ScopeCursor> {
        let id = self
            .bytecode
            .binding_plan
            .pc_scopes
            .get(self.ip.saturating_sub(1))
            .copied()
            .unwrap_or(ScopeId(0));
        self.prepare_scope_cursor_at(id)
    }

    pub(super) fn prepare_scope_cursor_at(&mut self, id: ScopeId) -> Option<ScopeCursor> {
        if !self.bytecode.has_dynamic_scope_tree()
            && self.env.scope_cursor().is_none()
            && self.with_stack().is_empty()
        {
            return None;
        }

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
        // Promote before borrowing lease storage. The lease owns only cells;
        // no pointer into the declaring frame can escape.
        let mut scope = Some(id);
        while let Some(id) = scope {
            let owner = self.bytecode.clone();
            let descriptor = &owner.binding_plan.scopes[id.0];
            for binding in &descriptor.declarations {
                let meta = &owner.binding_plan.bindings[binding.0];
                if self.local_upvalue_cell(meta.slot.0).is_none()
                    && !(descriptor.kind == ScopeKind::Eval
                        && matches!(
                            meta.kind,
                            crate::binding::BindingKind::Var
                                | crate::binding::BindingKind::Function
                        ))
                {
                    self.ensure_upvalue_for_parent_slot(meta.slot.0);
                }
            }
            scope = descriptor.parent;
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

    pub(super) fn closure_scope_cursor(
        &mut self,
        child: &super::ir::Bytecode,
    ) -> Option<ScopeCursor> {
        if child.has_dynamic_scope_tree()
            || self.env.scope_cursor().is_some_and(|cursor| {
                cursor.needs_crossing_lookup(|name| child.references_free_name_tree(name))
            })
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
                    .or_else(|| {
                        (meta.name == "this")
                            .then(|| self.env.derived_this_cell())
                            .flatten()
                    })
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
        let own_vars = (descriptor.variable_target == id
            && !self.env.has_global_binding_root()
            && self.bytecode.contains_direct_eval())
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
        crate::diagnostics::count!(scope_lease_allocations);
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
