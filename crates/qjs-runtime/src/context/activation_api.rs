//! Canonical typed activation queries and declaration services. No factory here
//! copies lexical names/values or manufactures another execution environment.
use super::{ActivationContext, Realm};
use crate::{
    ObjectRef, Value, binding::DynamicBindings, function::BindingCell, private::PrivateEnvironment,
};
use std::rc::Rc;

impl ActivationContext {
    pub(crate) fn creation_context(&self) -> Rc<crate::context::CreationContext> {
        self.creation.clone()
    }

    pub(crate) fn creation_context_ref(&self) -> &Rc<crate::context::CreationContext> {
        &self.creation
    }

    pub(crate) fn set_call_owner(&mut self, owner: crate::context::CallOwner) {
        self.mode = owner.kind.into();
        if owner.kind == crate::context::OwnerKind::Module
            && !matches!(
                self.creation.referrer,
                crate::context::ScriptOrModule::Module(_)
            )
        {
            Rc::make_mut(&mut self.creation).referrer.select_module();
        }
        self.call = crate::context::LexicalCallContext::Inline(owner);
    }

    pub(crate) fn inherit_call_owner(&mut self, owner: Rc<crate::context::CallOwner>) {
        self.call = crate::context::LexicalCallContext::Shared(owner);
    }

    pub(crate) fn capture_call_owner(&mut self) -> Option<Rc<crate::context::CallOwner>> {
        self.call.capture()
    }

    pub(crate) fn receiver_seed(&self) -> Option<Value> {
        self.call.owner().and_then(|owner| owner.this.read().ok())
    }

    /// Borrow the canonical initialized receiver for callback-free fast paths.
    /// Derived-this cells keep their checked read path and cannot be borrowed.
    pub(crate) fn initialized_receiver(&self) -> Option<&Value> {
        match &self.call.owner()?.this {
            crate::context::ThisBinding::Initialized(value) => Some(value),
            crate::context::ThisBinding::Derived(_) => None,
        }
    }

    pub(crate) fn this_value(&self) -> Result<Value, crate::RuntimeError> {
        self.call
            .owner()
            .map_or(Ok(Value::Undefined), |owner| owner.this.read())
    }

    pub(crate) fn lexical_new_target(&self) -> Option<Value> {
        self.call.owner().and_then(|owner| owner.new_target.clone())
    }

    pub(crate) fn home_object(&self) -> Option<Value> {
        self.call.owner().and_then(|owner| owner.home.clone())
    }

    pub(crate) fn super_constructor(&self) -> Option<Value> {
        self.call
            .owner()
            .and_then(|owner| owner.super_constructor.clone())
    }

    pub(crate) fn active_constructor(&self) -> Option<Value> {
        self.call
            .owner()
            .and_then(|owner| owner.active_constructor.clone())
    }

    pub(crate) fn owner_kind(&self) -> Option<crate::context::OwnerKind> {
        self.call.owner().map(|owner| owner.kind)
    }

    pub(crate) fn set_global_selection(&mut self, value: Value) {
        let selected = Rc::make_mut(&mut self.creation);
        selected.intrinsics = match &value {
            Value::Object(global) => crate::context::IntrinsicOverrides::for_marked_global(global),
            _ => Default::default(),
        };
        selected.global = crate::context::GlobalSelection::from_control(value);
    }

    pub(crate) fn explicit_global_selection_value(&self) -> Option<Value> {
        self.creation.global.explicit_value()
    }

    pub(crate) fn dynamic_function_realm_value(&self) -> Option<Value> {
        self.explicit_global_selection_value().or_else(|| {
            self.realm()
                .dynamic_function_realm_global()
                .map(Value::Object)
        })
    }

    pub(crate) fn select_variable_boundary(&mut self, scope: crate::binding::ScopeId) {
        self.variable.boundary = scope;
    }

    pub(crate) fn variable_identity(&self) -> crate::binding::VariableEnvIdentity {
        self.variable
    }

    pub(crate) fn execution_state(&self) -> (crate::context::ExecutionMode, bool) {
        (self.mode, self.strict)
    }

    pub(crate) fn set_execution_state(
        &mut self,
        mode: crate::context::ExecutionMode,
        strict: bool,
    ) {
        self.mode = mode;
        self.strict = strict;
    }

    pub(crate) fn module_host(&self) -> Option<crate::module::ModuleHostRef> {
        self.module_host_ref().cloned()
    }

    pub(crate) fn module_host_ref(&self) -> Option<&crate::module::ModuleHostRef> {
        self.creation.referrer.host()
    }

    pub(crate) fn set_module_host(&mut self, host: crate::module::ModuleHostRef) {
        if self
            .module_host_ref()
            .is_some_and(|old| Rc::ptr_eq(old, &host))
        {
            return;
        }
        Rc::make_mut(&mut self.creation).referrer.set_host(host);
    }

    pub(crate) fn set_module_import(
        &mut self,
        local_name: String,
        bindings: DynamicBindings,
        exported: String,
    ) {
        let module = Rc::make_mut(&mut self.creation).module_mut();
        Rc::make_mut(&mut module.imports).insert(local_name, (bindings, exported));
    }

    pub(crate) fn module_import_value(
        &self,
        local_name: &str,
    ) -> Option<Result<Value, crate::RuntimeError>> {
        let (bindings, exported) = self.creation.imports().get(local_name)?;
        Some(bindings.cell(exported).map_or_else(
            || {
                Err(crate::RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{local_name}`"),
                })
            },
            |cell| cell.read(),
        ))
    }

    pub(crate) fn module_import_cell(&self, local_name: &str) -> Option<BindingCell> {
        let (bindings, exported) = self.creation.imports().get(local_name)?;
        bindings.cell(exported)
    }

    pub(crate) fn has_module_import(&self, local_name: &str) -> bool {
        self.creation.imports().contains_key(local_name)
    }

    pub(crate) fn has_module_imports(&self) -> bool {
        !self.creation.imports().is_empty()
    }

    pub(crate) fn set_module_live_bindings(&mut self, bindings: DynamicBindings) {
        self.mode = crate::context::ExecutionMode::Module;
        self.scopes.module_root = true;
        Rc::make_mut(&mut self.creation).module_mut().live = Some(bindings);
    }

    pub(crate) fn has_module_live_bindings(&self) -> bool {
        self.scopes.module_root
            && self
                .creation
                .module
                .as_ref()
                .is_some_and(|module| module.live.is_some())
    }

    pub(crate) fn module_live_binding_cell(&self, name: &str) -> Option<BindingCell> {
        if !self.has_module_live_bindings() {
            return None;
        }
        self.creation.module.as_ref()?.live.as_ref()?.cell(name)
    }

    pub(crate) fn realm(&self) -> &Realm {
        &self.creation.realm
    }

    pub(crate) fn realm_rc(&self) -> Realm {
        Rc::clone(&self.creation.realm)
    }

    pub(crate) fn declare_global_lexical(
        &self,
        meta: crate::binding::BindingMetaRef,
    ) -> BindingCell {
        if let Some(cell) = self.creation.realm.cell(&meta.name) {
            if cell.metadata().kind == crate::binding::BindingKind::Lexical {
                return cell;
            }
            cell.invalidate_object_route();
        }
        let name = meta.name.clone();
        let cell = BindingCell::with_metadata(meta, None);
        self.creation
            .realm
            .binding_cells()
            .insert_cell(name, cell.clone());
        cell
    }

    pub(crate) fn is_global_lexical_binding(&self, name: &str) -> bool {
        self.creation
            .realm
            .cell(name)
            .is_some_and(|cell| cell.metadata().kind == crate::binding::BindingKind::Lexical)
    }

    pub(crate) fn is_immutable_lexical_binding(&self, name: &str) -> bool {
        self.creation.realm.cell(name).is_some_and(|cell| {
            cell.metadata().kind == crate::binding::BindingKind::Lexical && !cell.metadata().mutable
        })
    }

    pub(crate) fn private_environment(&self) -> Option<PrivateEnvironment> {
        self.creation.private.clone()
    }

    pub(crate) fn select_global_bindings(&mut self, global: bool) {
        self.scopes.global = global.then_some(super::activation::GlobalCodeMode::Eval);
        self.scopes.entry = None;
    }

    pub(crate) fn select_script_bindings(&mut self) {
        self.select_global_bindings(true);
        self.scopes.global = Some(super::activation::GlobalCodeMode::Script);
    }

    pub(crate) fn global_function_configurable(&self) -> bool {
        self.scopes.global != Some(super::activation::GlobalCodeMode::Script)
    }

    pub(crate) fn selected_global_this(&self) -> Option<Value> {
        match &self.creation.global {
            crate::context::GlobalSelection::Override(global) => {
                Some(Value::Object(global.clone()))
            }
            crate::context::GlobalSelection::Default
            | crate::context::GlobalSelection::Disabled => self.realm().global_this(),
        }
    }

    pub(crate) fn can_cache_realm_globals(&self) -> bool {
        match &self.creation.global {
            crate::context::GlobalSelection::Override(global) => {
                self.realm().is_global_object(global)
            }
            _ => true,
        }
    }

    pub(crate) fn has_global_binding_root(&self) -> bool {
        self.scopes.global.is_some()
    }

    pub(crate) fn entry_scope(&self) -> Option<&Rc<crate::binding::ScopeLease>> {
        self.scopes.entry.as_ref()
    }

    pub(crate) fn set_eval_scope(&mut self, cursor: crate::binding::ScopeCursor, global: bool) {
        self.mode = crate::context::ExecutionMode::Eval;
        self.scopes.module_root = false;
        if let Some(holder) = &cursor.variable.own_vars {
            self.variable = holder.identity;
        }
        self.scopes.entry = Some(cursor.head.clone());
        self.scopes.cursor = Some(cursor);
        self.scopes.global = global.then_some(super::activation::GlobalCodeMode::Eval);
    }

    pub(crate) fn scope_cursor(&self) -> Option<&crate::binding::ScopeCursor> {
        self.scopes.cursor.as_ref()
    }

    pub(crate) fn set_scope_cursor(&mut self, cursor: Option<crate::binding::ScopeCursor>) {
        self.scopes.cursor = cursor;
    }

    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        if name == "this"
            && let Some(value) = self.receiver_seed()
        {
            return Some(value);
        }
        if let Some(value) = self.creation.realm.get_value(name) {
            return Some(value);
        }
        None
    }

    pub(crate) fn get_realm(&self, name: &str) -> Option<Value> {
        self.creation.realm.get_value(name)
    }

    pub(crate) fn dynamic_function_realm_global(&self) -> Option<ObjectRef> {
        match &self.creation.global {
            crate::context::GlobalSelection::Override(global) => Some(global.clone()),
            crate::context::GlobalSelection::Disabled => None,
            crate::context::GlobalSelection::Default => {
                self.realm().dynamic_function_realm_global()
            }
        }
    }

    pub(crate) fn has_dynamic_function_realm_global(&self) -> bool {
        match &self.creation.global {
            crate::context::GlobalSelection::Override(_) => true,
            crate::context::GlobalSelection::Disabled => false,
            crate::context::GlobalSelection::Default => {
                self.realm().has_dynamic_function_realm_global()
            }
        }
    }

    pub(crate) fn global_this(&self) -> Option<Value> {
        self.selected_global_this()
    }

    pub(crate) fn realm_binding_cell(&self, name: &str) -> Option<BindingCell> {
        self.creation.realm.cell(name)
    }

    pub(crate) fn remove_realm(&self, name: &str) -> Option<Value> {
        // Capture the pre-removal value before marking the cell, matching the
        // old two-map model's behavior: a caller that already holds this
        // cell (e.g. a closure that captured the binding) observes the
        // uninitialized marker, while the returned value here is the value
        // that was live immediately before removal.
        let removed = self.creation.realm.get_value(name);
        if let Some(cell) = self.creation.realm.cell(name) {
            cell.invalidate_object_route();
        }
        self.creation.realm.remove_value(name);
        self.creation
            .realm
            .sync_dynamic_function_realm_binding(name, None);
        removed
    }

    pub(crate) fn is_realm_binding_cell(&self, name: &str, cell: &BindingCell) -> bool {
        self.creation.realm.is_binding_cell(name, cell)
    }

    pub(crate) fn insert_realm(&self, name: String, value: Value) -> Option<Value> {
        self.creation
            .realm
            .sync_dynamic_function_realm_binding(&name, Some(&value));
        self.creation.realm.insert_value(name, value)
    }

    pub(crate) fn replace_existing_realm(&self, name: &str, value: Value) -> bool {
        let Some(cell) = self.creation.realm.cell(name) else {
            return false;
        };
        if !cell.publish_object_value(value.clone()) {
            return false;
        }
        self.creation
            .realm
            .sync_dynamic_function_realm_binding(name, Some(&value));
        true
    }

    pub(crate) fn replace_existing_realm_with_cell(
        &self,
        name: &str,
        value: Value,
        cell: &BindingCell,
    ) -> bool {
        if !cell.publish_object_value(value.clone()) {
            return false;
        }
        self.creation
            .realm
            .sync_dynamic_function_realm_binding(name, Some(&value));
        true
    }

    pub(crate) fn realm_contains(&self, name: &str) -> bool {
        self.creation.realm.contains(name)
    }
    pub(crate) fn derived_this_cell(&self) -> Option<BindingCell> {
        self.call
            .owner()
            .and_then(|owner| owner.this.derived_cell())
    }

    pub(crate) fn indexed_bindings_only(&self) -> bool {
        matches!(self.creation.global, super::GlobalSelection::Default)
            && self.scopes.cursor.is_none()
            && self.creation.imports().is_empty()
            && !self.has_module_live_bindings()
    }
    pub(crate) fn permits_owned_slots(&self) -> bool {
        self.creation.imports().is_empty() && !self.has_module_live_bindings()
    }
}
