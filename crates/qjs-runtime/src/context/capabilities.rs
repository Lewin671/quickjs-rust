//! Narrow typed capabilities on the borrowed facade; canonical tables/cells
//! remain their only authority. Storage borrows end before observable hooks.
use super::{Context, Realm};
use crate::{
    Bytecode, Function, JsString, ObjectRef, Value, binding::DynamicBindings,
    function::BindingCell, private::PrivateEnvironment,
};
use qjs_parser::EvalParseContext;
use std::rc::Rc;
impl Context<'_> {
    pub(crate) fn primitive_prototype(&self, name: &str) -> Option<ObjectRef> {
        let kind = crate::context::PrimitiveIntrinsic::from_name(name)?;
        self.creation_context_ref().intrinsics.primitive[kind.index()]
            .clone()
            .or_else(|| self.realm().primitive_prototype(kind))
    }

    pub(crate) fn creation_context(&self) -> Rc<crate::context::CreationContext> {
        self.creation_context_ref().clone()
    }

    pub(crate) fn set_call_owner(&mut self, owner: crate::context::CallOwner) {
        self.activation.set_call_owner(owner)
    }

    pub(crate) fn capture_call_owner(&mut self) -> Option<Rc<crate::context::CallOwner>> {
        self.activation.capture_call_owner()
    }

    pub(crate) fn home_object(&self) -> Option<Value> {
        self.activation.home_object()
    }

    pub(crate) fn super_constructor(&self) -> Option<Value> {
        self.activation.super_constructor()
    }

    pub(crate) fn owner_kind(&self) -> Option<crate::context::OwnerKind> {
        self.activation.owner_kind()
    }

    pub(crate) fn explicit_global_selection_value(&self) -> Option<Value> {
        self.activation.explicit_global_selection_value()
    }

    pub(crate) fn dynamic_function_realm_value(&self) -> Option<Value> {
        self.activation.dynamic_function_realm_value()
    }

    pub(crate) fn module_host(&self) -> Option<crate::module::ModuleHostRef> {
        self.activation.module_host()
    }

    pub(crate) fn module_host_ref(&self) -> Option<&crate::module::ModuleHostRef> {
        self.runtime.host.module(&self.activation.creation)
    }

    pub(crate) fn set_module_host(&mut self, host: crate::module::ModuleHostRef) {
        self.activation.set_module_host(host)
    }

    pub(crate) fn module_import_cell(&self, local_name: &str) -> Option<BindingCell> {
        self.activation.module_import_cell(local_name)
    }

    pub(crate) fn has_module_import(&self, local_name: &str) -> bool {
        self.activation.has_module_import(local_name)
    }

    pub(crate) fn module_live_binding_cell(&self, name: &str) -> Option<BindingCell> {
        self.activation.module_live_binding_cell(name)
    }

    pub(crate) fn realm_rc(&self) -> Realm {
        self.activation.realm_rc()
    }

    pub(crate) fn is_global_lexical_binding(&self, name: &str) -> bool {
        self.creation_context_ref()
            .realm
            .cell(name)
            .is_some_and(|cell| cell.metadata().kind == crate::binding::BindingKind::Lexical)
    }

    pub(crate) fn is_immutable_lexical_binding(&self, name: &str) -> bool {
        self.creation_context_ref()
            .realm
            .cell(name)
            .is_some_and(|cell| {
                cell.metadata().kind == crate::binding::BindingKind::Lexical
                    && !cell.metadata().mutable
            })
    }

    pub(crate) fn private_environment(&self) -> Option<PrivateEnvironment> {
        self.native
            .as_ref()
            .and_then(|site| site.direct_eval)
            .and_then(|site| site.private.cloned())
            .or_else(|| self.activation.private_environment())
    }

    pub(crate) fn selected_global_this(&self) -> Option<Value> {
        match &self.creation_context_ref().global {
            crate::context::GlobalSelection::Override(global) => {
                Some(Value::Object(global.clone()))
            }
            crate::context::GlobalSelection::Default
            | crate::context::GlobalSelection::Disabled => self.realm().global_this(),
        }
    }

    pub(crate) fn can_cache_realm_globals(&self) -> bool {
        match &self.creation_context_ref().global {
            crate::context::GlobalSelection::Override(global) => {
                self.realm().is_global_object(global)
            }
            _ => true,
        }
    }

    pub(crate) fn has_global_binding_root(&self) -> bool {
        self.activation.has_global_binding_root()
    }

    pub(crate) fn scope_cursor(&self) -> Option<&crate::binding::ScopeCursor> {
        self.native
            .as_ref()
            .and_then(|site| site.direct_eval)
            .and_then(|site| site.scope)
            .or_else(|| self.activation.scope_cursor())
    }

    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        if name == crate::GLOBAL_THIS_BINDING {
            return self.selected_global_this();
        }
        if name == "this"
            && let Some(value) = self.activation.receiver_seed()
        {
            return Some(value);
        }
        if let Some(value) = self.creation_context_ref().realm.get_value(name) {
            return Some(value);
        }
        None
    }

    pub(crate) fn realm_read_layer(&self) -> Option<&DynamicBindings> {
        if !self.can_cache_realm_globals() {
            return None;
        }
        Some(self.creation_context_ref().realm.binding_cells())
    }

    pub(crate) fn get_realm(&self, name: &str) -> Option<Value> {
        self.creation_context_ref().realm.get_value(name)
    }

    pub(crate) fn dynamic_function_realm_global(&self) -> Option<ObjectRef> {
        match &self.creation_context_ref().global {
            crate::context::GlobalSelection::Override(global) => Some(global.clone()),
            crate::context::GlobalSelection::Disabled => None,
            crate::context::GlobalSelection::Default => {
                self.realm().dynamic_function_realm_global()
            }
        }
    }

    pub(crate) fn has_dynamic_function_realm_global(&self) -> bool {
        match &self.creation_context_ref().global {
            crate::context::GlobalSelection::Override(_) => true,
            crate::context::GlobalSelection::Disabled => false,
            crate::context::GlobalSelection::Default => {
                self.realm().has_dynamic_function_realm_global()
            }
        }
    }

    pub(crate) fn cached_direct_eval_bytecode(
        &self,
        source: &JsString,
        context: &EvalParseContext,
    ) -> Option<Rc<Bytecode>> {
        self.creation_context_ref()
            .realm
            .cached_direct_eval_bytecode(source, context)
    }

    pub(crate) fn cache_direct_eval_bytecode(
        &self,
        source: JsString,
        context: EvalParseContext,
        bytecode: Rc<Bytecode>,
    ) {
        self.creation_context_ref()
            .realm
            .cache_direct_eval_bytecode(source, context, bytecode);
    }

    #[cfg(test)]
    pub(crate) fn direct_eval_cache_len(&self) -> usize {
        self.creation_context_ref().realm.direct_eval_cache_len()
    }

    pub(crate) fn object_prototype_intrinsic_override(&self) -> Option<ObjectRef> {
        self.creation_context_ref().intrinsics.object.clone()
    }

    pub(crate) fn array_prototype_intrinsic_override(&self) -> Option<ObjectRef> {
        self.creation_context_ref().intrinsics.array.clone()
    }

    pub(crate) fn global_this(&self) -> Option<Value> {
        self.selected_global_this()
    }

    pub(crate) fn realm_binding_cell(&self, name: &str) -> Option<BindingCell> {
        self.creation_context_ref().realm.cell(name)
    }

    pub(crate) fn remove_realm(&self, name: &str) -> Option<Value> {
        // Capture the pre-removal value before marking the cell, matching the
        // old two-map model's behavior: a caller that already holds this
        // cell (e.g. a closure that captured the binding) observes the
        // uninitialized marker, while the returned value here is the value
        // that was live immediately before removal.
        let removed = self.creation_context_ref().realm.get_value(name);
        if let Some(cell) = self.creation_context_ref().realm.cell(name) {
            cell.set(Value::Function(Function::uninitialized_lexical_marker()));
        }
        self.creation_context_ref().realm.remove_value(name);
        self.creation_context_ref()
            .realm
            .sync_dynamic_function_realm_binding(name, None);
        removed
    }

    pub(crate) fn insert_realm(&self, name: String, value: Value) -> Option<Value> {
        self.creation_context_ref()
            .realm
            .sync_dynamic_function_realm_binding(&name, Some(&value));
        self.creation_context_ref().realm.insert_value(name, value)
    }

    pub(crate) fn replace_existing_realm_with_cell(
        &self,
        name: &str,
        value: Value,
        cell: &BindingCell,
    ) -> bool {
        cell.set(value.clone());
        self.creation_context_ref()
            .realm
            .sync_dynamic_function_realm_binding(name, Some(&value));
        true
    }

    pub(crate) fn is_realm_global_object(&self, object: &ObjectRef) -> bool {
        let realm = self.realm();
        realm.is_global_object(object)
            || realm.group().has_foreign_realms() && realm.group().global_realm(object).is_some()
            || !realm.group().ptr_eq(&self.runtime.group)
                && self.runtime.group.global_realm(object).is_some()
    }
    pub(crate) fn sync_realm_global_object_property(&self, object: &ObjectRef, name: &str) {
        let Some(realm) = self
            .realm()
            .group()
            .global_realm(object)
            .or_else(|| self.runtime.group.global_realm(object))
        else {
            return;
        };
        if !realm.contains(name)
            || realm
                .cell(name)
                .is_some_and(|cell| cell.metadata().kind == crate::binding::BindingKind::Lexical)
        {
            return;
        }
        match object.own_property(name) {
            Some(property) if !property.is_accessor() => {
                realm.insert_value(name.to_owned(), property.value);
            }
            _ => {
                realm.remove_value(name);
            }
        }
    }
    pub(crate) fn forget_deleted_global_object_property(&self, object: &ObjectRef, name: &str) {
        let Some(realm) = self
            .realm()
            .group()
            .global_realm(object)
            .or_else(|| self.runtime.group.global_realm(object))
        else {
            return;
        };
        if realm
            .cell(name)
            .is_some_and(|cell| cell.metadata().kind == crate::binding::BindingKind::Lexical)
        {
            return;
        }
        realm.remove_value(name);
    }

    pub(crate) fn realm_contains(&self, name: &str) -> bool {
        self.creation_context_ref().realm.contains(name)
    }

    pub(crate) fn direct_eval_site(&self) -> Option<&super::DirectEvalSite<'_>> {
        self.native.as_ref().and_then(|site| site.direct_eval)
    }
    pub(crate) fn construct_target(&self) -> Option<Value> {
        self.new_target().cloned()
    }
    #[cfg(feature = "agents")]
    pub(crate) fn agent_context(&self) -> Option<crate::agent::AgentContextRef> {
        self.runtime.agent.clone()
    }
}
