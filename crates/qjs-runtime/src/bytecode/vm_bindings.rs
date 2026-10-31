//! Binding access for the interpreter: local
//! and global loads and stores through their canonical locations.
//!
//! A local slot is *authoritative* (a bit in `authoritative_slots`) when no
//! name-addressed state can supersede it; such a slot is read and written
//! directly, and the accelerator tiers rely on that bit. Every other slot --
//! one captured in an upvalue cell, a realm global, a module binding, a
//! sloppy-global fallback, anything under direct eval or `with` -- takes the
//! slow path, which keeps the slot, its cell and the realm in step.
//! `refresh_authoritative_slots` recomputes the masks after a frame's cells
//! or environment change.

use crate::{
    GLOBAL_THIS_BINDING, Property, RuntimeError, Value, function::BindingCell, property_value,
    value::OwnDataPropertyWrite,
};

use super::{ir::Bytecode, vm::Vm};

/// A fallback or hoisted binding proved to be a writable ordinary realm
/// global. Publication uses its canonical cell and guarded property slot;
/// scalar regions with no observers may publish once before continuation.
#[derive(Clone, Debug)]
pub(super) struct TypedLoopSloppyGlobalWrite {
    slot: usize,
    name: String,
    cell: BindingCell,
    global_this: crate::ObjectRef,
    /// The global object's own-property slot for `name`, with the layout
    /// revision it was resolved at: while the layout holds, each write goes
    /// straight to the slot instead of hashing the name into the global
    /// object's table. `None` for a name whose store has a side effect of
    /// its own.
    property_slot: Option<(usize, crate::value::layout::LayoutVersion)>,
}

impl Vm<'_> {
    /// Realm `globalThis` object handle. `CallEnv::global_this` already
    /// resolves this from a plain `Option<Value>` field set once at realm
    /// construction (see `RealmState::new`), so this is just the `Value` ->
    /// `ObjectRef` narrowing — no `RefCell` borrow or `HashMap` lookup, unlike
    /// `self.realm.borrow().get(GLOBAL_THIS_BINDING)`.
    #[inline(always)]
    pub(super) fn cached_global_this(&self) -> Option<crate::ObjectRef> {
        match self.env.global_this() {
            Some(Value::Object(global_this)) => Some(global_this),
            _ => None,
        }
    }

    pub(super) fn enter_body_deopt_scope(&mut self) {
        if let Some(scope) = self.bytecode.binding_plan.pc_scopes.get(self.ip) {
            let target = self.bytecode.binding_plan.scopes[scope.0].variable_target;
            self.env.select_variable_boundary(target);
        }
        let Some(parameter_bindings) = self.env.deopt_bindings().cloned() else {
            return;
        };
        self.split_body_deopt_scope(parameter_bindings);
    }

    /// Splits a direct eval's parameter environment from the body's.
    ///
    /// Out of line and cold: every frame in the program runs the check above,
    /// and only a frame whose function has both parameters and a direct eval
    /// reaches this.
    #[cold]
    #[inline(never)]
    fn split_body_deopt_scope(&mut self, parameter_bindings: crate::function::DynamicBindings) {
        // Every frame in a script that contains a direct eval anywhere reaches
        // this, so the discovery below must not allocate when it finds nothing.
        // Building the `PREFIX + name` marker for each local would allocate
        // per local, usually to discover an empty set (a script that evals
        // a JSON payload is the case). The bindings are the smaller side and
        // already carry the marker, so strip the prefix from them instead.
        let mut split_slots: Vec<usize> = Vec::new();
        {
            let bytecode = &self.current.bytecode;
            parameter_bindings.for_each_prefixed_name(
                crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX,
                |name| {
                    if let Some(slot) = bytecode.local_slot(name)
                        && let Some(local) = bytecode.locals.get(slot)
                        && local.hoisted
                        && !local.parameter
                    {
                        split_slots.push(slot);
                    }
                },
            );
        }
        if split_slots.is_empty() {
            return;
        }
        // The old walk visited locals in slot order; keep that, since the
        // cells published below are inserted in this order.
        split_slots.sort_unstable();
        split_slots.dedup();
        let body_bindings = crate::function::DynamicBindings::new();
        for (name, upvalue) in parameter_bindings.cells() {
            if split_slots
                .iter()
                .any(|slot| self.bytecode.locals[*slot].name == name)
            {
                continue;
            }
            body_bindings.insert_cell(name, upvalue);
        }
        for slot in split_slots {
            let name = self.bytecode.locals[slot].name.clone();
            let value = Value::Undefined;
            let upvalue = BindingCell::new(value.clone());
            self.locals[slot] = Some(value);
            self.local_upvalues[slot] = Some(upvalue.clone());
            body_bindings.insert_cell(name, upvalue);
        }
        self.env.set_deopt_bindings(body_bindings);
    }

    pub(super) fn initialize_script_global_bindings(
        bytecode: &Bytecode,
        realm: &crate::function::Realm,
    ) -> Result<(), RuntimeError> {
        let global_this = realm
            .get_value(GLOBAL_THIS_BINDING)
            .and_then(|value| match value {
                Value::Object(object) => Some(object),
                _ => None,
            });
        if let Some(global_this) = &global_this {
            for name in bytecode.global_lexical_names() {
                if global_this
                    .own_property(name)
                    .is_some_and(|property| !property.configurable)
                {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!(
                            "SyntaxError: global lexical declaration `{name}` conflicts with an existing var binding"
                        ),
                    });
                }
            }
        }
        for name in bytecode.hoisted_local_names() {
            if let Some(property) = global_this
                .as_ref()
                .and_then(|object| object.own_property(name))
            {
                realm.insert_value(name.to_owned(), property.value);
            } else {
                realm.entry_or_insert_value(name.to_owned(), Value::Undefined);
                if let Some(global_this) = &global_this {
                    global_this.define_property(
                        name.to_owned(),
                        Property::data(Value::Undefined, true, true, false),
                    );
                }
            }
        }
        Ok(())
    }

    pub(super) fn refresh_authoritative_slots(&mut self) {
        self.authoritative_slots =
            Self::initial_authoritative_slots(&self.bytecode, &self.local_upvalues, &self.env)
                & !self.direct_readonly_upvalue_slots;
        let direct_realm_binding_slots =
            self.realm_binding_slots & self.direct_readonly_upvalue_slots;
        self.realm_binding_slots =
            Self::initial_realm_binding_slots(&self.bytecode, &self.local_upvalues, &self.env)
                | direct_realm_binding_slots;
        self.refresh_virtual_object_execution();
    }

    /// Keeps a module's exported binding cell current for name-based writes.
    pub(super) fn write_through_module_live_binding(&self, name: &str, value: Value) {
        if let Some(binding) = self.env.module_live_binding_cell(name) {
            binding.set(value);
        }
    }

    /// Slot-addressed module live-binding update.
    pub(super) fn write_through_module_live_binding_slot(&self, slot: usize, value: &Value) {
        if let Some(name) = self.bytecode.locals.get(slot).map(|local| &local.name) {
            // Check the cheap, almost-always-`None` live-binding lookup first:
            // non-module scripts never pay for the `local_slot` name-table
            // hash lookup below. Module live bindings describe the module's
            // top-level declaration slot; a nested lexical may reuse the same
            // source name but owns a distinct cell and must never update the
            // export by coincidence.
            if let Some(binding) = self.env.module_live_binding_cell(name)
                && self.bytecode.local_slot(name) == Some(slot)
            {
                binding.set(value.clone());
            }
        }
    }

    /// Own lexical/import/arguments routes precede the selected global object.
    pub(super) fn uses_selected_object_binding(&self, name: &str) -> bool {
        !(self.env.can_cache_realm_globals()
            || self.env.has_module_import(name)
            || name == "arguments" && self.env.has_local_binding(name))
    }

    pub(super) fn load_global(&mut self, name: &str) -> Result<Value, RuntimeError> {
        if name != "this"
            && let Some(reference) = self.resolve_scope_binding(name, self.execution_is_strict())?
        {
            return self.read_scope_binding(&reference);
        }
        if name != "this" && !self.env.can_cache_realm_globals() {
            if let Some(value) = self.env.module_import_value(name).or_else(|| {
                (name == "arguments")
                    .then(|| self.env.get_local(name))
                    .flatten()
            }) {
                if value.is_uninitialized_lexical_marker() {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!("ReferenceError: undefined identifier `{name}`"),
                    });
                }
                return Ok(value);
            }
            return self.read_global_binding(name)?.ok_or_else(|| RuntimeError {
                thrown: None,
                message: format!("ReferenceError: undefined identifier `{name}`"),
            });
        }
        if name != "this" && self.env.has_global_binding_root() {
            return self.read_global_binding(name)?.ok_or_else(|| RuntimeError {
                thrown: None,
                message: format!("ReferenceError: undefined identifier `{name}`"),
            });
        }
        if name != "this" && self.env.scope_cursor().is_some() {
            if let Some(value) = self.env.module_import_value(name) {
                return if value.is_uninitialized_lexical_marker() {
                    Err(RuntimeError {
                        thrown: None,
                        message: format!("ReferenceError: undefined identifier `{name}`"),
                    })
                } else {
                    Ok(value)
                };
            }
            if let Some(value) = self.env.get_realm(name)
                && !value.is_uninitialized_lexical_marker()
            {
                return Ok(value);
            }
            if let Some(value) = self.global_this_own_value(name)? {
                return Ok(value);
            }
            return Err(RuntimeError {
                thrown: None,
                message: format!("ReferenceError: undefined identifier `{name}`"),
            });
        }
        if name == "this" {
            return self.env.this_value();
        }
        let deoptimized_sloppy_global = if !self.bytecode.global_scope
            && let Some(slot) = self.bytecode.local_slot(name)
            && self.bytecode.local_is_sloppy_global_fallback(slot)
            && let Some(value) = self.local_slot_value(slot)
        {
            if !value.is_uninitialized_lexical_marker() {
                return Ok(value);
            }
            true
        } else {
            false
        };
        if let Some(value) = self.env.module_import_value(name) {
            if value.is_uninitialized_lexical_marker() {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{name}`"),
                });
            }
            return Ok(value);
        }
        if self.bytecode.global_scope
            && let Some(slot) = self.bytecode.local_slot(name)
            && self.bytecode.local_is_body_hoist_only(slot)
            && !self.bytecode.local_is_compiler_temporary(slot)
            && let Some(value) = self.global_this_property(name)
        {
            return Ok(value);
        }
        // A "global" name may actually be a caller-scope binding carried in this
        // frame's own locals layer (e.g. an outer `var`/`let` the body closes
        // over); check that first, then the shared realm, then a property
        // created directly on `globalThis` (`this.x = 1` and realm bindings
        // share one global namespace).
        // Reconfiguring a captured sloppy global into an accessor detaches its
        // realm cell. The frame still owns that now-invalidated cell, so skip
        // its marker and resolve the accessor on `globalThis` below.
        if !deoptimized_sloppy_global && let Some(value) = self.env.get(name) {
            if matches!(
                &value,
                Value::Function(function) if function.is_uninitialized_lexical_marker()
            ) {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{name}`"),
                });
            }
            return Ok(value);
        }
        if let Some(value) = self.global_this_own_value(name)? {
            return Ok(value);
        }
        Err(RuntimeError {
            thrown: None,
            message: format!("ReferenceError: undefined identifier `{name}`"),
        })
    }

    /// Reads an own property of `globalThis` by name, invoking any getter, when
    /// the name resolves to a property created directly on the global object
    /// (e.g. `this.x = 1`, a realm binding, or `Object.defineProperty`). Returns
    /// `None` when no such own property exists, so the caller can decide whether
    /// that is a ReferenceError (a bare read) or "undefined" (a `typeof`).
    pub(super) fn global_this_own_value(
        &mut self,
        name: &str,
    ) -> Result<Option<Value>, RuntimeError> {
        let Some(global_this) = self.cached_global_this() else {
            return Ok(None);
        };
        if !global_this.has_own_property(name) {
            return Ok(None);
        }
        let mut env = self.current_env();
        let value = property_value(Value::Object(global_this), name, &mut env)?;
        Ok(Some(value))
    }

    /// Assigns an accessor own property of `globalThis` -- one made by
    /// `Object.defineProperty(globalThis, name, { get, set })` -- by calling
    /// its setter with the global object as `this`, as the global
    /// environment record's SetMutableBinding does. Without a setter the
    /// assignment fails: a TypeError in strict code, nothing in sloppy code.
    /// `None` when the property is absent or a data property, which the
    /// caller's own store handles.
    pub(super) fn store_global_this_accessor(
        &mut self,
        name: &str,
        value: &Value,
        strict: bool,
    ) -> Option<Result<(), RuntimeError>> {
        let global_this = self.cached_global_this()?;
        let property = global_this.own_property(name)?;
        if !property.is_accessor() {
            return None;
        }
        let (_, setter) = property.into_accessor_parts()?;
        let Some(setter) = setter else {
            return Some(if strict {
                Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "TypeError: Cannot set property '{name}' which has only a getter"
                    ),
                })
            } else {
                Ok(())
            });
        };
        let mut env = self.current_env();
        let result = crate::call_function(
            setter,
            Value::Object(global_this),
            vec![value.clone()],
            &mut env,
            false,
        );
        Some(result.map(|_| ()))
    }

    pub(super) fn load_new_target(&self) -> Value {
        self.env.lexical_new_target().unwrap_or(Value::Undefined)
    }

    /// Reads an own property of the realm's `globalThis` object, if any.
    pub(super) fn global_this_property(&self, name: &str) -> Option<Value> {
        let global_this = self.cached_global_this()?;
        global_this
            .own_property(name)
            .map(|property| property.value)
    }

    fn has_realm_or_global_this_binding(&self, name: &str) -> bool {
        self.realm.contains(name) || self.global_this_property(name).is_some()
    }

    fn store_realm_or_global_this_sloppy(
        &mut self,
        name: String,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if !self.env.can_cache_realm_globals() {
            return self.store_global_binding(&name, value, false);
        }
        if self.env.is_immutable_lexical_binding(&name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        if let Some(result) = self.store_global_this_accessor(&name, &value, false) {
            return result;
        }
        if let Some(property) = self.global_this_own_property(&name)
            && !property.writable
        {
            return Ok(());
        }
        if self.env.has_local_binding(&name) {
            self.env.insert(name.clone(), value.clone());
            self.write_through_module_live_binding(&name, value);
            self.sync_marked_dynamic_global(&name);
            return Ok(());
        }
        if self.realm.contains(&name) {
            self.env.insert_realm(name.clone(), value.clone());
            if self.env.has_local_binding(&name) {
                self.env.insert(name.clone(), value.clone());
            }
            self.write_through_module_live_binding(&name, value.clone());
            let global_this = self.cached_global_this();
            if let Some(global_this) = global_this
                && global_this.has_own_property(&name)
            {
                global_this.set(name.clone(), value);
            }
            self.sync_marked_dynamic_global(&name);
            return Ok(());
        }
        let global_this = self.cached_global_this();
        if let Some(global_this) = global_this {
            global_this.set(name.clone(), value.clone());
        }
        self.env.insert_realm(name.clone(), value.clone());
        if self.env.has_local_binding(&name) {
            self.env.insert(name.clone(), value.clone());
        }
        self.write_through_module_live_binding(&name, value);
        self.sync_marked_dynamic_global(&name);
        Ok(())
    }

    /// Returns the full own-property descriptor of a `globalThis` property so
    /// callers can inspect attribute flags such as `writable`.
    /// The realm binding's current value for `name`, which is the authority a
    /// global read resolves to.
    pub(super) fn global_this_own_property(&self, name: &str) -> Option<Property> {
        let global_this = self.cached_global_this()?;
        global_this.own_property(name)
    }

    pub(super) fn local_slot_targets_non_writable_global(&self, slot: usize, name: &str) -> bool {
        let is_global_shadow = self.bytecode.global_scope
            && self.bytecode.local_is_body_hoist_only(slot)
            && !self.bytecode.local_is_compiler_temporary(slot);
        let is_sloppy_fallback = self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| local.sloppy_global_fallback);
        (is_global_shadow || is_sloppy_fallback)
            && self
                .global_this_own_property(name)
                .is_some_and(|property| !property.writable)
    }

    #[inline(always)]
    pub(super) fn load_local(&mut self, slot: usize) -> Result<Value, RuntimeError> {
        if self.slot_is_authoritative(slot) {
            return match self.locals.get(slot) {
                Some(Some(value)) => self.checked_local_value(slot, clone_local_value(value)),
                Some(None) => Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "ReferenceError: undefined identifier `{}`",
                        self.bytecode.locals[slot].name
                    ),
                }),
                None => Err(RuntimeError {
                    thrown: None,
                    message: "bytecode local index out of bounds".to_owned(),
                }),
            };
        }
        self.load_local_slow(slot)
    }

    #[inline(never)]
    fn load_local_slow(&mut self, slot: usize) -> Result<Value, RuntimeError> {
        if self.bytecode.is_direct_eval_code()
            && self.env.scope_cursor().is_some()
            && self.bytecode.locals[slot].hoisted
            && !self.bytecode.locals[slot].compiler_temporary
        {
            let name = self.bytecode.locals[slot].name.clone();
            return self.load_global(&name);
        }
        if (self.bytecode.locals[slot].is_received_upvalue()
            && (self.bytecode.contains_direct_eval() || self.env.scope_cursor().is_some()))
            || (self.bytecode.locals[slot].parameter
                && self.bytecode.contains_direct_eval()
                && !self.in_parameter_prologue()
                && self
                    .bytecode
                    .binding_plan
                    .scopes
                    .get(1)
                    .is_some_and(|scope| scope.variable_target.0 == 1))
        {
            let name = &self.bytecode.locals[slot].name;
            if let Some(reference) = self.resolve_scope_binding(name, self.execution_is_strict())? {
                return self.read_scope_binding(&reference);
            }
        }
        if self.slot_is_realm_binding(slot)
            && let Some(cell) = self.local_upvalue_cell(slot)
        {
            let value = cell.get();
            if !value.is_uninitialized_lexical_marker() {
                return Ok(value);
            }
            let name = self.bytecode.locals[slot].name.clone();
            if let Some(value) = self.global_this_own_value(&name)? {
                return Ok(value);
            }
            return Err(RuntimeError {
                thrown: None,
                message: format!("ReferenceError: undefined identifier `{name}`"),
            });
        }
        if self.env.has_global_binding_root()
            && self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.hoisted && !local.parameter && !local.compiler_temporary)
        {
            let name = self.bytecode.locals[slot].name.clone();
            return self
                .read_global_binding(&name)?
                .ok_or_else(|| RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{name}`"),
                });
        }
        if self.bytecode.locals[slot].name != "this"
            && let Some(cell) = self.local_upvalue_cell(slot)
        {
            return cell.read();
        }
        if let Some(value) = self.upvalue_slot_value(slot) {
            return self.checked_local_value(slot, value);
        }
        if let Some(local) = self.bytecode.locals.get(slot)
            && local.from_env
            && let Some(value) = self.env.module_import_value(&local.name)
        {
            if value.is_uninitialized_lexical_marker() {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: undefined identifier `{}`", local.name),
                });
            }
            return Ok(value);
        }
        match self.locals.get(slot) {
            Some(Some(value)) => self.checked_local_value(slot, value.clone()),
            Some(None) => Err(RuntimeError {
                thrown: None,
                message: format!(
                    "ReferenceError: undefined identifier `{}`",
                    self.bytecode.locals[slot].name
                ),
            }),
            None => Err(RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    /// Whether a typed loop program may write `slot` directly: the slot must be
    /// an ordinary mutable frame local with no shared cell and no realm
    /// binding behind it, so a plain store is the whole observable effect.
    pub(super) fn slot_accepts_typed_loop_write(&self, slot: usize) -> bool {
        // Exactly the condition `store_local` uses for a plain slot write: an
        // authoritative slot has no environment binding and no shared cell
        // behind it, so the write is the whole observable effect.
        self.slot_is_authoritative(slot)
            && self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.mutable && !local.sloppy_global_fallback)
    }

    /// Prepares a fallback or hoisted realm slot for a typed-loop store. This is
    /// deliberately stricter than the ordinary store: the typed program can
    /// only enter after the fallback already names the same writable own data
    /// property, realm cell, and local value. Missing globals and every dynamic
    /// binding path stay on the generic bytecode operation that creates or
    /// resolves them.
    pub(super) fn prepare_typed_loop_sloppy_global_write(
        &self,
        slot: usize,
        name: &str,
    ) -> Option<TypedLoopSloppyGlobalWrite> {
        if self.direct_eval_with_stack
            || !self.with_stack().is_empty()
            || self.bytecode.contains_direct_eval()
            || self.bytecode.contains_with()
            || self.env.deopt_bindings().is_some()
            || self.env.has_module_imports()
            || self.env.has_dynamic_function_realm_global()
            || self.env.has_local_binding(name)
            || self.env.has_module_import(name)
            || self.env.is_global_lexical_binding(name)
            || self.env.is_immutable_lexical_binding(name)
        {
            return None;
        }
        let local = self.bytecode.locals.get(slot)?;
        if local.name != name
            || !local.mutable
            || !(local.sloppy_global_fallback
                || self.bytecode.global_scope && local.hoisted && !local.parameter)
            || local.compiler_temporary
            || self.bytecode.local_slot(name) != Some(slot)
        {
            return None;
        }
        // The generic store's fast branch only applies once the frame holds a
        // value (a global hoist reads its canonical realm cell instead) and
        // the global property already exists. Keeping that same
        // boundary means the first creation and every unusual reconfiguration
        // retain the ordinary interpreter path.
        let value = if self.bytecode.global_scope && local.hoisted {
            self.env.get_realm(name)?
        } else {
            self.locals.get(slot)?.as_ref()?.clone()
        };
        if !matches!(
            value,
            Value::Number(_) | Value::Boolean(_) | Value::Undefined
        ) {
            return None;
        }
        let global_this = self.cached_global_this()?;
        let property = global_this.own_property(name)?;
        if property.is_accessor() || !property.writable || !property.value.same_value(&value) {
            return None;
        }
        let cell = self.env.realm_binding_cell(name)?;
        if !self.env.is_realm_binding_cell(name, &cell)
            || !cell.get().same_value(&value)
            || !self.env.get_realm(name)?.same_value(&value)
        {
            return None;
        }
        if let Some(local_cell) = self.local_upvalue_cell(slot)
            && !local_cell.ptr_eq(&cell)
        {
            return None;
        }
        // `globalThis` itself and the dynamic-function realm marker have
        // effects beyond the value (`write_existing_own_data_property`,
        // `replace_existing_realm_with_cell`); they keep the named path.
        let property_slot = (name != "globalThis"
            && name != "__quickjsRustDynamicFunctionRealm"
            && self.env.realm().atoms().ic_policy().enabled)
            .then(|| global_this.any_storage_data_slot(name))
            .flatten()
            .map(|slot| (slot, global_this.layout_revision()));
        Some(TypedLoopSloppyGlobalWrite {
            slot,
            name: name.to_owned(),
            cell,
            global_this,
            property_slot,
        })
    }

    /// Publishes one already-guarded typed-loop sloppy-global value. No user
    /// code can run in an admitted region, so the entry proof stays valid until
    /// the loop yields; if the data-property write nevertheless declines, the
    /// caller deoptimizes before the bytecode store and replays it normally.
    #[inline(always)]
    pub(super) fn write_typed_loop_sloppy_global(
        &mut self,
        target: &TypedLoopSloppyGlobalWrite,
        value: Value,
    ) -> bool {
        if let Some((slot, layout)) = target.property_slot.as_ref()
            && target.global_this.with_named_mut(|store| {
                // Admission proved an own writable data slot. Guard and publish
                // under one borrow; no descriptor reprobe or callback intervenes.
                store.matches_version(layout) && store.overwrite_guarded_data(*slot, value.clone())
            })
        {
            debug_assert!(self.env.is_realm_binding_cell(&target.name, &target.cell));
            target.cell.set(value.clone());
            self.locals[target.slot] = Some(value);
            return true;
        }
        self.write_typed_loop_sloppy_global_slow(target, value)
    }
    #[cold]
    #[inline(never)]
    fn write_typed_loop_sloppy_global_slow(
        &mut self,
        target: &TypedLoopSloppyGlobalWrite,
        value: Value,
    ) -> bool {
        if !matches!(
            target
                .global_this
                .write_existing_own_data_property(&target.name, &value),
            OwnDataPropertyWrite::Written
        ) {
            return false;
        }
        debug_assert!(self.env.is_realm_binding_cell(&target.name, &target.cell));
        let replaced =
            self.env
                .replace_existing_realm_with_cell(&target.name, value.clone(), &target.cell);
        debug_assert!(
            replaced,
            "prepared sloppy-global cell must remain canonical"
        );
        self.locals[target.slot] = Some(value);
        true
    }

    /// Writes a slot a typed loop program owns for the duration of the loop.
    pub(super) fn write_typed_loop_slot(&mut self, slot: usize, value: Value) {
        if let Some(local) = self.locals.get_mut(slot) {
            *local = Some(value);
        }
    }

    pub(super) fn local_slot_value(&self, slot: usize) -> Option<Value> {
        self.upvalue_slot_value(slot)
            .or_else(|| self.locals.get(slot).and_then(Option::as_ref).cloned())
    }

    pub(super) fn upvalue_slot_value(&self, slot: usize) -> Option<Value> {
        self.local_upvalue_cell(slot).map(BindingCell::get)
    }

    #[inline(always)]
    fn checked_local_value(&self, slot: usize, value: Value) -> Result<Value, RuntimeError> {
        if !value.is_uninitialized_lexical_marker() {
            return Ok(value);
        }
        self.uninitialized_local_value(slot)
    }

    #[cold]
    #[inline(never)]
    fn uninitialized_local_value(&self, slot: usize) -> Result<Value, RuntimeError> {
        if self.bytecode.local_is_compiler_temporary(slot) {
            return Ok(Value::Undefined);
        }
        Err(RuntimeError {
            thrown: None,
            message: format!(
                "ReferenceError: undefined identifier `{}`",
                self.bytecode.locals[slot].name
            ),
        })
    }

    pub(super) fn load_local_or_undefined(&self, slot: usize) -> Result<Value, RuntimeError> {
        if let Some(value) = self.upvalue_slot_value(slot) {
            return Ok(value);
        }
        if let Some(local) = self.bytecode.locals.get(slot)
            && local.from_env
            && let Some(value) = self.env.module_import_value(&local.name)
        {
            return Ok(value);
        }
        match self.locals.get(slot) {
            Some(Some(value)) => Ok(value.clone()),
            Some(None) => Ok(Value::Undefined),
            None => Err(RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    #[inline(always)]
    pub(super) fn store_local(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        if self.slot_is_authoritative(slot)
            && self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.mutable)
        {
            self.locals[slot] = Some(value);
            return Ok(());
        }
        self.store_local_slow(slot, value)
    }

    #[inline(never)]
    fn store_local_slow(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        if self.env.has_global_binding_root()
            && self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.hoisted && !local.parameter && !local.compiler_temporary)
        {
            let name = self.bytecode.locals[slot].name.clone();
            if self.bytecode.locals[slot].hoisted_function
                && let Some(Value::Object(global)) = self.env.selected_global_this()
            {
                let property = match global.own_property(&name) {
                    Some(mut property) if !property.configurable => {
                        property.value = value.clone();
                        property
                    }
                    _ => Property::data(
                        value.clone(),
                        true,
                        true,
                        self.env.global_function_configurable(),
                    ),
                };
                global.define_property(name.clone(), property);
                if self.env.can_cache_realm_globals() {
                    self.env.insert_realm(name, value);
                }
                return Ok(());
            }
            return self.store_global_binding(&name, value, self.execution_is_strict());
        }
        // Read only the `Copy` slot metadata up front so the hot local write
        // never clones the `Local` (its owned `name` would be a heap
        // allocation on every assignment); the binding name is resolved by
        // reference, and only on the cold capture/global-sync paths.
        let (mutable, from_env, hoisted, module_import) = {
            let local_meta = self.bytecode.locals.get(slot).ok_or_else(|| RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            })?;
            (
                local_meta.mutable,
                local_meta.from_env,
                local_meta.hoisted,
                self.env
                    .module_import_cell(&local_meta.name)
                    .zip(self.local_upvalue_cell(slot))
                    .is_some_and(|(import, selected)| import.ptr_eq(selected)),
            )
        };
        if module_import {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        let upvalue_initialized = self
            .local_upvalue_cell(slot)
            .is_some_and(BindingCell::is_initialized);
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        if !mutable && (local.is_some() || upvalue_initialized) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        *local = Some(value.clone());
        let uses_shared_cell = if let Some(upvalue) = self.local_upvalue_cell(slot) {
            if upvalue.is_initialized() {
                upvalue.assign(value.clone(), self.execution_is_strict())?;
            } else {
                upvalue.initialize(value.clone())?;
            }
            true
        } else {
            false
        };
        if !uses_shared_cell {
            // Binding classes not migrated to cells still use the coexistence
            // snapshot/writeback path. A cell-backed lexical must not also write
            // by name: same-named shadowed bindings are distinct slots/cells.
            self.write_through_module_live_binding_slot(slot, &value);
        } else if !from_env {
            // A declaring frame still mirrors its own cell into the coexistence
            // map for module live exports and not-yet-migrated consumers. A
            // received upvalue must never take this name-keyed path: its parent
            // binding is already updated through the shared cell, and a
            // same-named outer binding can be a different cell.
            self.write_through_module_live_binding_slot(slot, &value);
        }
        let mut env_mirror_synced = false;
        if !uses_shared_cell
            && (from_env || self.bytecode.local_is_body_hoist_only(slot))
            && self.env.has_local_binding(&self.bytecode.locals[slot].name)
        {
            let name = self.bytecode.locals[slot].name.clone();
            self.env.insert(name, value.clone());
            env_mirror_synced = true;
        }
        // `shared_realm_cell` only matters through the first `syncs_global_var`
        // disjunct, which is unreachable when `hoisted` (its `!hoisted` term is
        // false) — skip the hash lookup entirely for the common hoisted-var
        // case instead of computing a value that can't affect the result.
        let syncs_global_var = if hoisted {
            self.bytecode.global_scope
                && self.bytecode.local_is_body_hoist_only(slot)
                && !self.bytecode.local_is_compiler_temporary(slot)
        } else {
            let shared_realm_cell = self.local_upvalue_cell(slot).is_some_and(|cell| {
                self.env
                    .is_realm_binding_cell(&self.bytecode.locals[slot].name, cell)
            });
            (from_env && (!uses_shared_cell || shared_realm_cell))
                || (self.bytecode.global_scope
                    && self.bytecode.local_is_body_hoist_only(slot)
                    && !self.bytecode.local_is_compiler_temporary(slot))
        };
        let global_this = if syncs_global_var {
            self.cached_global_this()
        } else {
            None
        };
        if let Some(global_this) = global_this {
            // A plain writable data property (the common case for a hoisted
            // `var`'s global slot) is a single hashed lookup here instead of
            // the `has_own_property` existence check plus a second hashed
            // `set()` lookup; accessors and non-existent/read-only properties
            // fall back to the original two-lookup path unchanged.
            match global_this
                .write_existing_own_data_property(&self.bytecode.locals[slot].name, &value)
            {
                OwnDataPropertyWrite::Written => {
                    let name = self.bytecode.locals[slot].name.clone();
                    if self.realm.contains(&name) {
                        self.env.insert_realm(name.clone(), value.clone());
                    }
                    if !env_mirror_synced && self.env.has_local_binding(&name) {
                        self.env.insert(name, value);
                    }
                }
                OwnDataPropertyWrite::ReadOnly => {}
                OwnDataPropertyWrite::NeedsSlowPath => {
                    if global_this.has_own_property(&self.bytecode.locals[slot].name) {
                        let name = self.bytecode.locals[slot].name.clone();
                        global_this.set(name.clone(), value.clone());
                        if self.realm.contains(&name) {
                            self.env.insert_realm(name.clone(), value.clone());
                        }
                        if !env_mirror_synced && self.env.has_local_binding(&name) {
                            self.env.insert(name, value);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub(super) fn assign_local(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        if (self.bytecode.locals[slot].is_received_upvalue()
            && (self.bytecode.contains_direct_eval() || self.env.scope_cursor().is_some()))
            || (self.bytecode.locals[slot].parameter
                && self.bytecode.contains_direct_eval()
                && !self.in_parameter_prologue()
                && self
                    .bytecode
                    .binding_plan
                    .scopes
                    .get(1)
                    .is_some_and(|scope| scope.variable_target.0 == 1))
        {
            let name = &self.bytecode.locals[slot].name;
            if let Some(reference) = self.resolve_scope_binding(name, self.execution_is_strict())? {
                return self.assign_scope_binding(&reference, value, self.execution_is_strict());
            }
        }
        if self.slot_is_authoritative(slot)
            && self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.mutable)
            && let Some(Some(local)) = self.locals.get_mut(slot)
            && !local.is_uninitialized_lexical_marker()
        {
            *local = value;
            return Ok(());
        }
        self.assign_local_slow(slot, value)
    }

    #[inline(never)]
    fn assign_local_slow(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        if self.slot_is_authoritative(slot)
            && self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.mutable)
        {
            return match self.locals.get_mut(slot) {
                Some(Some(Value::Function(function)))
                    if function.is_uninitialized_lexical_marker() =>
                {
                    Err(RuntimeError {
                        thrown: None,
                        message: format!(
                            "ReferenceError: undefined identifier `{}`",
                            self.bytecode.locals[slot].name
                        ),
                    })
                }
                Some(Some(local)) => {
                    *local = value;
                    Ok(())
                }
                Some(None) => Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "ReferenceError: undefined identifier `{}`",
                        self.bytecode.locals[slot].name
                    ),
                }),
                None => Err(RuntimeError {
                    thrown: None,
                    message: "bytecode local index out of bounds".to_owned(),
                }),
            };
        }
        let current = self
            .upvalue_slot_value(slot)
            .or_else(|| self.locals.get(slot).and_then(Option::as_ref).cloned());
        match current {
            Some(Value::Function(function)) if function.is_uninitialized_lexical_marker() => {
                Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "ReferenceError: undefined identifier `{}`",
                        self.bytecode.locals[slot].name
                    ),
                })
            }
            Some(_) => self.store_local(slot, value),
            None => Err(RuntimeError {
                thrown: None,
                message: format!(
                    "ReferenceError: undefined identifier `{}`",
                    self.bytecode.locals[slot].name
                ),
            }),
        }
    }

    /// True when this frame has no name-addressed state that can supersede the
    /// indexed local. Captures, dynamic scope, modules, globals, and sloppy
    /// fallback bindings all retain the full synchronization path.
    #[inline(always)]
    pub(super) fn slot_is_authoritative(&self, slot: usize) -> bool {
        slot < u128::BITS as usize && self.authoritative_slots & (1_u128 << slot) != 0
    }

    #[inline(always)]
    pub(super) fn slot_is_realm_binding(&self, slot: usize) -> bool {
        slot < u128::BITS as usize && self.realm_binding_slots & (1_u128 << slot) != 0
    }

    pub(super) fn check_dynamic_assignment(
        &self,
        name: &str,
        strict: bool,
    ) -> Result<(), RuntimeError> {
        if let Some(cell) = self.env.local_binding_cell(name) {
            cell.check_assignment(strict)?;
        }
        Ok(())
    }

    pub(super) fn store_local_or_global_sloppy(
        &mut self,
        slot: usize,
        name: &str,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if let Some(reference) = self.resolve_scope_binding(name, false)? {
            return self.assign_scope_binding(&reference, value, false);
        }
        if self.uses_selected_object_binding(name) {
            return self.store_global_binding(name, value, false);
        }
        self.check_dynamic_assignment(name, false)?;
        if self.env.has_module_import(name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        let is_sloppy_global_fallback = self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| local.sloppy_global_fallback);
        // A `var` a direct eval declared in this function shadows the global.
        if is_sloppy_global_fallback
            && self.env.has_local_binding(name)
            && self.env.has_local_binding(&format!(
                "{}{name}",
                crate::DIRECT_EVAL_FUNCTION_VAR_BINDING_PREFIX
            ))
        {
            self.env.insert(name.to_owned(), value.clone());
            return self.store_local(slot, value);
        }
        if is_sloppy_global_fallback
            && self.locals.get(slot).is_some_and(Option::is_some)
            && let Some(Value::Object(global_this)) = self.env.global_this()
        {
            match global_this.write_existing_own_data_property(name, &value) {
                OwnDataPropertyWrite::Written => {
                    if !self.env.replace_existing_realm(name, value.clone()) {
                        self.env.insert_realm(name.to_owned(), value.clone());
                    }
                    self.locals[slot] = Some(value);
                    self.sync_marked_dynamic_global(name);
                    return Ok(());
                }
                OwnDataPropertyWrite::ReadOnly => return Ok(()),
                OwnDataPropertyWrite::NeedsSlowPath => {}
            }
        }
        if is_sloppy_global_fallback
            && let Some(result) = self.store_global_this_accessor(name, &value, false)
        {
            return result;
        }
        match self.locals.get(slot) {
            Some(Some(_)) => {
                if self.local_slot_targets_non_writable_global(slot, name) {
                    return Ok(());
                }
                if is_sloppy_global_fallback || self.has_realm_or_global_this_binding(name) {
                    let syncs_global_snapshot = is_sloppy_global_fallback
                        && self.captured_or_local_matches_global_this(name);
                    if syncs_global_snapshot {
                        self.record_sloppy_global_name(name);
                    }
                    self.store_realm_or_global_this_sloppy(name.to_owned(), value.clone())?;
                    self.store_local(slot, value)?;
                    if syncs_global_snapshot && let Some(value) = self.locals[slot].clone() {
                        self.sync_global_this_own_property(name, value);
                    }
                } else {
                    self.store_local(slot, value)?;
                }
                Ok(())
            }
            Some(None) => {
                if is_sloppy_global_fallback {
                    if self.local_slot_targets_non_writable_global(slot, name) {
                        return Ok(());
                    }
                    let syncs_global_snapshot = self.captured_or_local_matches_global_this(name);
                    if syncs_global_snapshot {
                        self.record_sloppy_global_name(name);
                    }
                    self.store_realm_or_global_this_sloppy(name.to_owned(), value.clone())?;
                    self.store_local(slot, value)?;
                    if syncs_global_snapshot && let Some(value) = self.locals[slot].clone() {
                        self.sync_global_this_own_property(name, value);
                    }
                    return Ok(());
                }
                self.store_global_sloppy(name, value)?;
                self.record_sloppy_global_name(name);
                let global_value = self.load_global(name)?;
                let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
                    thrown: None,
                    message: "bytecode local index out of bounds".to_owned(),
                })?;
                *local = Some(global_value);
                Ok(())
            }
            None => Err(RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    pub(super) fn clear_local(&mut self, slot: usize) -> Result<(), RuntimeError> {
        let deactivated_cell = self
            .local_upvalues
            .get(slot)
            .and_then(Option::as_ref)
            .cloned();
        let deactivates_lexical = self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| !local.hoisted);
        let local_is_some = self
            .locals
            .get(slot)
            .ok_or_else(|| RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            })?
            .is_some();
        let refresh_upvalue = self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| !local.from_env)
            && self
                .local_upvalues
                .get(slot)
                .and_then(Option::as_ref)
                .is_some_and(|upvalue| local_is_some || !upvalue.is_shared());
        self.locals[slot] = None;
        if refresh_upvalue && let Some(upvalue) = self.local_upvalues.get_mut(slot) {
            *upvalue = upvalue.as_ref().map(BindingCell::fresh_uninitialized);
        }
        if let Some(name) = self.current.bytecode.local_name_at(slot) {
            if deactivates_lexical && let Some(cell) = &deactivated_cell {
                self.current.env.remove_deopt_cell_if(name, cell);
            }
            if self
                .current
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.catch_binding)
            {
                self.current.env.remove(name);
                self.current.env.unmark_catch_binding(name);
            }
        }
        Ok(())
    }

    pub(super) fn define_global_var(
        &mut self,
        name: String,
        value: Value,
    ) -> Result<(), RuntimeError> {
        let Some(global_this) = self.cached_global_this() else {
            return Err(RuntimeError {
                thrown: None,
                message: "global object binding is missing".to_owned(),
            });
        };
        if global_this.has_own_property(&name) {
            global_this.set(name.clone(), value.clone());
            let value = global_this
                .own_property(&name)
                .map(|property| property.value)
                .unwrap_or(value);
            self.env.insert_realm(name.clone(), value.clone());
            if self.env.has_local_binding(&name) {
                self.env.insert(name.clone(), value.clone());
            }
            self.clear_global_var_local(&name);
            self.write_through_module_live_binding(&name, value);
            return Ok(());
        }
        global_this.define_property(
            name.clone(),
            Property::data(value.clone(), true, true, false),
        );
        self.env.insert_realm(name.clone(), value.clone());
        if self.env.has_local_binding(&name) {
            self.env.insert(name.clone(), value.clone());
        }
        self.clear_global_var_local(&name);
        self.write_through_module_live_binding(&name, value);
        Ok(())
    }

    fn clear_global_var_local(&mut self, name: &str) {
        if !self.bytecode.global_scope {
            return;
        }
        let Some(slot) = self.bytecode.local_slot(name) else {
            return;
        };
        if self.bytecode.local_is_body_hoist_only(slot)
            && let Some(local) = self.locals.get_mut(slot)
        {
            *local = None;
        }
    }
    fn sync_global_this_own_property(&self, name: &str, value: Value) {
        let Some(global_this) = self.cached_global_this() else {
            return;
        };
        let Some(property) = global_this.own_property(name) else {
            return;
        };
        if property.is_accessor() {
            // The setter has already run. A value projected by the caller's
            // compatibility view is neither the getter result nor a data
            // binding: keep identifier reads on the semantic global Get path.
            self.env.remove_realm(name);
        } else {
            global_this.set(name.to_owned(), value.clone());
            self.env.insert_realm(name.to_owned(), value);
        }
    }

    fn captured_or_local_matches_global_this(&self, name: &str) -> bool {
        let Some(global_value) = self.global_this_property(name) else {
            return false;
        };
        self.env.get_local(name) == Some(global_value)
    }

    /// `delete identifier` in non-strict mode (sloppy): attempts to delete the
    /// binding. Declared variables (var/let/const/function in any scope) cannot
    /// be deleted (returns false). Global properties on `globalThis` that are
    /// configurable can be deleted (returns true). A non-existent binding also
    /// returns true.
    pub(super) fn delete_ident(&mut self, name: &str) -> Result<bool, RuntimeError> {
        if let Some(reference) = self.resolve_scope_binding(name, false)? {
            return reference.delete(&mut self.realm_env());
        }
        if self.prepare_scope_cursor().is_some() {
            return Ok(true);
        }
        if self.uses_selected_object_binding(name) {
            let Some(base) = self.env.selected_global_this() else {
                return Ok(true);
            };
            return crate::bytecode::delete_object_property(
                base,
                &crate::PropertyKey::String(name.to_owned()),
                &mut self.realm_env(),
            );
        }
        let deleted = (|| {
            let is_sloppy_global = self
                .bytecode
                .sloppy_global_assignment_names()
                .contains(&name.to_owned());
            // Local scope bindings (var/let/const/param) are never deletable,
            // but sloppy global assignments that happen to occupy a local slot
            // are configurable properties on globalThis and CAN be deleted.
            if !is_sloppy_global {
                if let Some(slot) = self.bytecode.local_slot(name) {
                    if self.locals[slot].is_some() {
                        return false;
                    }
                }
                // Non-global frame locals (e.g. captured from outer function scope)
                // are also undeletable.
                if self.env.get_local(name).is_some() {
                    return false;
                }
            }
            // For globals, check the globalThis property descriptor. Only
            // configurable properties (bare assignments like `x = 1`) can be
            // deleted. `var` declarations are non-configurable.
            let Some(global_this) = self.cached_global_this() else {
                return true;
            };
            if !global_this.has_own_property(name) {
                // Name exists in realm but not on globalThis — it's a lexical
                // binding from a script-level `let`/`const`; undeletable.
                if self.realm.contains(name) {
                    return false;
                }
                return true;
            }
            let deleted = global_this.delete_own_property(name);
            if deleted {
                self.env.remove_realm(name);
                // Clear the cached local slot if the sloppy global was mirrored there.
                if let Some(slot) = self.bytecode.local_slot(name) {
                    if let Some(local) = self.locals.get_mut(slot) {
                        *local = None;
                    }
                    if let Some(upvalue) = self.local_upvalues.get_mut(slot) {
                        *upvalue = None;
                    }
                }
            }
            deleted
        })();
        Ok(deleted)
    }

    pub(super) fn record_sloppy_global_name(&mut self, name: &str) {
        if !self
            .sloppy_global_names()
            .iter()
            .any(|existing| existing == name)
        {
            self.cold_mut().sloppy_global_names.push(name.to_owned());
        }
    }

    /// Replaces each captured loop binding with a fresh per-iteration cell.
    pub(super) fn fresh_iteration_scope(&mut self, slots: &[usize]) {
        for &slot in slots {
            if !self.local_upvalues.get(slot).is_some_and(Option::is_some) {
                continue;
            }
            let value = self.local_slot_value(slot).unwrap_or_else(|| {
                Value::Function(crate::Function::uninitialized_lexical_marker())
            });
            if let Some(upvalue) = self.local_upvalues.get_mut(slot) {
                *upvalue = upvalue.as_ref().map(|cell| cell.fresh_with_value(value));
            }
        }
    }

    pub(super) fn drain_promise_jobs(&mut self) -> Result<(), RuntimeError> {
        let mut env = self.current_env();
        crate::promise::drain_promise_jobs(&mut env)?;
        Ok(())
    }
}

/// Clones a slot value, keeping the four immediate variants off the general
/// `Value::clone` path so a local read never reaches refcount code it cannot
/// need. Shared with the interpreter's inline `LoadLocal` fast path.
#[inline(always)]
pub(super) fn clone_local_value(value: &Value) -> Value {
    match value {
        Value::Number(value) => Value::Number(*value),
        Value::Boolean(value) => Value::Boolean(*value),
        Value::Null => Value::Null,
        Value::Undefined => Value::Undefined,
        value => value.clone(),
    }
}
