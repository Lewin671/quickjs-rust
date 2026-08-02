use std::collections::{HashMap, HashSet};

use crate::{
    GLOBAL_THIS_BINDING, Property, RuntimeError, Value,
    function::{CallEnv, Upvalue},
    property_value,
    value::OwnDataPropertyWrite,
};

use super::{
    ir::Bytecode,
    vm::{Slot, Vm},
};

/// A sloppy fallback binding that a typed loop proved is an existing writable
/// ordinary realm global. The executor publishes every write immediately, so
/// it retains the interpreter's observable per-iteration ordering.
#[derive(Clone, Debug)]
pub(super) struct TypedLoopSloppyGlobalWrite {
    slot: usize,
    name: String,
    cell: Upvalue,
    global_this: crate::ObjectRef,
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
        let Some(parameter_bindings) = self.env.deopt_bindings().cloned() else {
            return;
        };
        let split_slots = self
            .bytecode
            .locals
            .iter()
            .enumerate()
            .filter_map(|(slot, local)| {
                let marker = format!(
                    "{}{}",
                    crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX,
                    local.name
                );
                (local.hoisted && !local.parameter && parameter_bindings.contains_key(&marker))
                    .then_some(slot)
            })
            .collect::<Vec<_>>();
        if split_slots.is_empty() {
            return;
        }
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
            let upvalue = Upvalue::new(value.clone());
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

    pub(super) fn initial_slots(bytecode: &Bytecode, env: &CallEnv) -> Vec<Slot> {
        bytecode
            .locals
            .iter()
            .map(|local| {
                if local.compiler_temporary {
                    return Some(Value::Undefined);
                }
                // Global-scope `var`/function bindings live in the realm; the
                // vestigial slot stays empty so captures and loads route to
                // the shared cell instead of a frozen copy.
                if local.hoisted && bytecode.global_scope {
                    None
                } else if !local.from_env
                    && crate::function::is_call_frame_binding(&local.name)
                    && let Some(value) = env.get_local(&local.name)
                {
                    // A declaring function's materialized `this`/`arguments`
                    // slot is seeded by call setup. It is deliberately not an
                    // indexed upvalue; a nested arrow captures the resulting
                    // local cell as an ordinary ParentLocal source.
                    Some(value)
                } else if local.from_env
                    && let Some(value) = env.get_local(&local.name)
                    && !matches!(
                        &value,
                        Value::Function(function) if function.is_uninitialized_lexical_marker()
                    )
                {
                    // Only a binding the caller passed in the frame's locals
                    // layer seeds a from_env slot; realm globals stay in the
                    // shared cell so closures observe live values.
                    Some(value)
                } else if local.from_env
                    && !local.hoisted
                    && !crate::function::is_call_frame_binding(&local.name)
                    && let Some(value) = env.get_realm(&local.name)
                {
                    // A script-level `var`/function binding is realm-backed,
                    // not an incoming lexical upvalue. Seed the compatibility
                    // slot from the live realm; stores below synchronize the
                    // global object and realm rather than creating a closure
                    // snapshot.
                    Some(value)
                } else if local.hoisted {
                    Some(Value::Undefined)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(super) fn initial_local_upvalues(
        bytecode: &Bytecode,
        _locals: &[Slot],
        upvalues: &[Upvalue],
        env: &CallEnv,
    ) -> Vec<Option<Upvalue>> {
        let mut next_received = 0;
        let mut local_upvalues = vec![None; bytecode.locals.len()];
        let direct_eval_frame = matches!(
            env.get_local(crate::DIRECT_EVAL_BINDING),
            Some(Value::Boolean(true))
        );
        for (slot, local) in bytecode.locals.iter().enumerate() {
            if local.compiler_temporary {
                continue;
            }
            if let Some(upvalue) = env.module_import_cell(&local.name) {
                local_upvalues[slot] = Some(upvalue);
                if local.is_received_upvalue() {
                    next_received += 1;
                }
                continue;
            }
            // A direct-eval program is compiled as a global-scope bytecode,
            // but its imported `var` slots can still name the caller's local
            // binding. Preserve that binding identity before considering the
            // same-named realm cell.
            if direct_eval_frame
                && local.hoisted
                && let Some(upvalue) = env.local_binding_cell(&local.name)
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            if bytecode.global_scope
                && local.hoisted
                && let Some(upvalue) = env.realm_binding_cell(&local.name)
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            if local.sloppy_global_fallback {
                local_upvalues[slot] = if direct_eval_frame {
                    env.local_binding_cell(&local.name)
                        .or_else(|| env.realm_binding_cell(&local.name))
                } else {
                    env.realm_binding_cell(&local.name)
                };
                continue;
            }
            if local.is_received_upvalue() {
                if let Some(upvalue) = upvalues.get(next_received) {
                    local_upvalues[slot] = Some(upvalue.clone());
                } else if env.deopt_bindings().is_some() {
                    local_upvalues[slot] = env
                        .deopt_bindings()
                        .and_then(|bindings| bindings.cell(&local.name))
                        .or_else(|| env.frame_binding_cell(&local.name));
                }
                next_received += 1;
            }
        }
        let plan = super::upvalue_resolver::resolve_upvalues(bytecode);
        let mut cell_slots = plan.cell_slots;
        if bytecode.needs_arguments_object() {
            cell_slots.extend(
                bytecode
                    .locals
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, local)| {
                        (local.parameter && !local.compiler_temporary).then_some(slot)
                    }),
            );
        }
        if bytecode.contains_direct_eval()
            || bytecode.contains_with()
            || env.deopt_bindings().is_some()
        {
            cell_slots.extend(
                bytecode
                    .locals
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, local)| {
                        (!(local.compiler_temporary
                            || local.sloppy_global_fallback
                            || bytecode.global_scope && local.hoisted))
                            .then_some(slot)
                    }),
            );
            cell_slots.sort_unstable();
            cell_slots.dedup();
        }
        cell_slots.extend(
            bytecode
                .locals
                .iter()
                .enumerate()
                .filter_map(|(slot, local)| {
                    (!local.compiler_temporary
                        && bytecode.local_slot(&local.name) == Some(slot)
                        && env.module_live_binding_cell(&local.name).is_some())
                    .then_some(slot)
                }),
        );
        cell_slots.sort_unstable();
        cell_slots.dedup();
        for slot in cell_slots {
            let Some(local) = bytecode.locals.get(slot) else {
                continue;
            };
            if local.compiler_temporary {
                continue;
            }
            if local.is_received_upvalue() {
                continue;
            }
            if bytecode.global_scope
                && local.hoisted
                && let Some(upvalue) = env.realm_binding_cell(&local.name)
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            if local.parameter
                && bytecode.needs_arguments_object()
                && let Some(upvalue) = env.frame_binding_cell(&local.name)
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            if bytecode.local_slot(&local.name) == Some(slot)
                && let Some(upvalue) = env.module_live_binding_cell(&local.name)
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            if direct_eval_frame
                && local.hoisted
                && !bytecode
                    .global_lexical_names()
                    .iter()
                    .any(|name| name == &local.name)
                && !env.has_frame_binding(&local.name)
                && let Some(upvalue) = env
                    .deopt_bindings()
                    .and_then(|bindings| bindings.cell(&local.name))
            {
                local_upvalues[slot] = Some(upvalue);
                continue;
            }
            let value = _locals
                .get(slot)
                .and_then(Option::as_ref)
                .cloned()
                .unwrap_or_else(
                    || Value::Function(crate::Function::uninitialized_lexical_marker()),
                );
            local_upvalues[slot] = Some(Upvalue::new(value));
        }
        local_upvalues
    }

    pub(super) fn initial_authoritative_slots(
        bytecode: &Bytecode,
        local_upvalues: &[Option<Upvalue>],
        env: &CallEnv,
    ) -> u128 {
        // A frame with no cells and an environment that supplies no name answers
        // the whole mask from the bytecode. Asking the environment about every
        // local's name, as the general path must, is what made building a frame
        // scale with its local count.
        if local_upvalues.is_empty() && env.supplies_no_named_binding() {
            return bytecode.authoritative_mask_clean();
        }
        bytecode
            .locals
            .iter()
            .enumerate()
            .take(u128::BITS as usize)
            .filter_map(|(slot, local)| {
                (!local.sloppy_global_fallback
                    && local_upvalues.get(slot).is_none_or(Option::is_none)
                    && (local.compiler_temporary
                        || !bytecode.global_scope && env.slot_is_authoritative(&local.name)))
                .then_some(1_u128 << slot)
            })
            .fold(0, |slots, slot| slots | slot)
    }

    pub(super) fn initial_realm_binding_slots(
        bytecode: &Bytecode,
        local_upvalues: &[Option<Upvalue>],
        env: &CallEnv,
    ) -> u128 {
        bytecode
            .locals
            .iter()
            .enumerate()
            .take(u128::BITS as usize)
            .filter_map(|(slot, local)| {
                local_upvalues
                    .get(slot)
                    .and_then(Option::as_ref)
                    .is_some_and(|cell| env.is_realm_binding_cell(&local.name, cell))
                    .then_some(1_u128 << slot)
            })
            .fold(0, |slots, slot| slots | slot)
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

    pub(super) fn load_global(&mut self, name: &str) -> Result<Value, RuntimeError> {
        // `this` belongs to the frame for function bodies (arrows inherit it
        // through capture). Module bodies provide their `this` binding through
        // the environment chain instead, while derived constructors without a
        // completed `super(...)` stay in their `this` TDZ.
        if name == "this" && !self.bytecode.global_scope {
            if let Some(value) = &self.direct_this {
                return Ok(value.clone());
            }
            if let Some(value) = self.env.get_local(name) {
                return Ok(value);
            }
            if !self.env.has_local_binding(crate::SUPER_CONSTRUCTOR_BINDING)
                && !self
                    .env
                    .has_local_binding(crate::ACTIVE_CONSTRUCTOR_BINDING)
                && let Some(value) = self.env.get(name)
            {
                return Ok(value);
            }
            return Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: must call super constructor before accessing `this`"
                    .to_owned(),
            });
        }
        if self.env.is_immutable_function_name(name)
            && let Some(value) = self.env.get_local(name)
        {
            return Ok(value);
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
        self.apply_env(env);
        Ok(Some(value))
    }

    pub(super) fn load_new_target(&self) -> Value {
        self.env
            .get(crate::NEW_TARGET_BINDING)
            .unwrap_or(Value::Undefined)
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
        if self.env.is_immutable_lexical_binding(&name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
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

    /// Prepares one sloppy fallback slot for a typed-loop store. This is
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
            || !self.with_stack.is_empty()
            || self.bytecode.contains_direct_eval()
            || self.bytecode.contains_with()
            || self.env.deopt_bindings().is_some()
            || self.env.has_module_imports()
            || self.env.dynamic_function_realm_global().is_some()
            || self.env.has_local_binding(name)
            || self.env.has_module_import(name)
            || self.env.is_global_lexical_binding(name)
            || self.env.is_immutable_lexical_binding(name)
            || self.env.is_immutable_function_name(name)
        {
            return None;
        }
        let local = self.bytecode.locals.get(slot)?;
        if local.name != name
            || !local.mutable
            || !local.sloppy_global_fallback
            || local.compiler_temporary
            || self.bytecode.local_slot(name) != Some(slot)
        {
            return None;
        }
        // The generic store's fast branch only applies once the frame holds a
        // value and the global property already exists. Keeping that same
        // boundary means the first creation and every unusual reconfiguration
        // retain the ordinary interpreter path.
        let value = self.locals.get(slot)?.as_ref()?;
        if !matches!(
            value,
            Value::Number(_) | Value::Boolean(_) | Value::Undefined
        ) {
            return None;
        }
        let global_this = self.cached_global_this()?;
        let property = global_this.own_property(name)?;
        if property.is_accessor() || !property.writable || !property.value.same_value(value) {
            return None;
        }
        let cell = self.env.realm_binding_cell(name)?;
        if !self.env.is_realm_binding_cell(name, &cell)
            || !cell.get().same_value(value)
            || !self.env.get_realm(name)?.same_value(value)
        {
            return None;
        }
        if let Some(local_cell) = self.local_upvalue_cell(slot)
            && !local_cell.ptr_eq(&cell)
        {
            return None;
        }
        Some(TypedLoopSloppyGlobalWrite {
            slot,
            name: name.to_owned(),
            cell,
            global_this,
        })
    }

    /// Publishes one already-guarded typed-loop sloppy-global value. No user
    /// code can run in an admitted region, so the entry proof stays valid until
    /// the loop yields; if the data-property write nevertheless declines, the
    /// caller deoptimizes before the bytecode store and replays it normally.
    pub(super) fn write_typed_loop_sloppy_global(
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
        self.local_upvalue_cell(slot).map(Upvalue::get)
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
        // Read only the `Copy` slot metadata up front so the hot local write
        // never clones the `Local` (its owned `name` would be a heap
        // allocation on every assignment); the binding name is resolved by
        // reference, and only on the cold capture/global-sync paths.
        let (mutable, from_env, hoisted, module_import, immutable_env_binding) = {
            let local_meta = self.bytecode.locals.get(slot).ok_or_else(|| RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            })?;
            (
                local_meta.mutable,
                local_meta.from_env,
                local_meta.hoisted,
                self.env.has_module_import(&local_meta.name),
                local_meta.from_env
                    && !local_meta.parameter
                    && !local_meta.hoisted
                    && (self.env.is_immutable_lexical_binding(&local_meta.name)
                        || self.env.is_immutable_function_name(&local_meta.name)),
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
            .map(|upvalue| upvalue.get())
            .is_some_and(|value| {
                !matches!(
                    value,
                    Value::Function(function) if function.is_uninitialized_lexical_marker()
                )
            });
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
        if (local.is_some() || upvalue_initialized) && immutable_env_binding {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        *local = Some(value.clone());
        let uses_shared_cell = if let Some(upvalue) = self.local_upvalue_cell(slot) {
            upvalue.set(value.clone());
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
        if self.bytecode.global_scope
            && self.persist_global_lexicals
            && !hoisted
            && self.bytecode.local_slot(&self.bytecode.locals[slot].name) == Some(slot)
            && self
                .bytecode
                .global_lexical_names()
                .iter()
                .any(|name| name == &self.bytecode.locals[slot].name)
            && !self.bytecode.local_is_compiler_temporary(slot)
        {
            let name = self.bytecode.locals[slot].name.clone();
            self.env
                .set_global_lexical_value(name.clone(), value.clone());
            self.env.mark_global_lexical_binding(name.clone());
            if !mutable {
                self.env.mark_immutable_lexical_binding(name);
            }
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

    pub(super) fn store_local_or_global_sloppy(
        &mut self,
        slot: usize,
        name: &str,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if self.env.has_module_import(name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        // The inner name of a named function expression is immutable; a sloppy
        // assignment to it is a silent no-op.
        if self.env.is_immutable_function_name(name) {
            return Ok(());
        }
        let is_sloppy_global_fallback = self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| local.sloppy_global_fallback);
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
            *upvalue = Some(Upvalue::new(Value::Function(
                crate::Function::uninitialized_lexical_marker(),
            )));
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
    /// Resolves a dynamic environment name to the innermost currently-active
    /// frame slot. Static bytecode is already slot-indexed; this reverse lookup
    /// is only for `CallEnv` round-trips used by direct eval and native calls.
    /// Later slots correspond to inner lexical declarations, so searching in
    /// reverse preserves shadowing without encoding slot identity in the name.
    fn active_local_slot_for_env_name(&self, name: &str) -> Option<usize> {
        self.bytecode
            .locals
            .iter()
            .enumerate()
            .rev()
            .find(|(slot, local)| {
                local.name == name
                    && (self.locals.get(*slot).is_some_and(Option::is_some)
                        || self.local_upvalues.get(*slot).is_some_and(Option::is_some))
            })
            .map(|(slot, _)| slot)
            .or_else(|| self.bytecode.local_slot(name))
    }

    pub(super) fn apply_env(&mut self, env: CallEnv) {
        // A realm-only environment carries no frame binding to write back, and
        // the write-back path allocates a name vector and a conflict set before
        // discovering that. Ordinary frames hand builtins, accessors, coercion
        // hooks, and iteration exactly such an environment, so this is the
        // common case now rather than a corner.
        if !env.has_writable_frame_state() {
            return;
        }
        // The realm layer is shared by `Rc`, so global writes are already live.
        // Write each non-realm local back to its slot, to the frame's own
        // internal/caller-scope binding layer, or (for a genuinely new binding)
        // to the shared realm.
        let direct_parameter_eval_values = env
            .deopt_bindings()
            .map(|bindings| {
                bindings
                    .names()
                    .into_iter()
                    .filter_map(|name| {
                        name.strip_prefix(crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX)
                            .and_then(|parameter| {
                                bindings
                                    .get(parameter)
                                    .map(|value| (parameter.to_owned(), value))
                            })
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        let locals = env.visible_local_entries();
        let direct_parameter_eval_vars = locals
            .iter()
            .filter_map(|(name, _)| {
                name.strip_prefix(crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX)
                    .map(str::to_owned)
            })
            .collect::<HashSet<_>>();
        for (name, mut value) in locals {
            if name.starts_with(crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX) {
                self.env.insert_deopt(name, value);
                continue;
            }
            if self.in_parameter_prologue() && direct_parameter_eval_vars.contains(&name) {
                if let Some(parameter_value) = direct_parameter_eval_values.get(&name) {
                    value = parameter_value.clone();
                }
                if let Some(parameter_var_slot) = self.active_local_slot_for_env_name(&name) {
                    if let Some(local) = self.locals.get_mut(parameter_var_slot) {
                        *local = Some(value.clone());
                    }
                    if let Some(upvalue) = self
                        .local_upvalues
                        .get(parameter_var_slot)
                        .and_then(Option::as_ref)
                    {
                        upvalue.set(value.clone());
                    }
                }
                self.env.insert_deopt(name, value);
                continue;
            }
            if let Some(index) = self.active_local_slot_for_env_name(&name) {
                if self.in_parameter_prologue()
                    && !self.bytecode.local_is_parameter(index)
                    && (is_call_frame_binding(&name) || !self.bytecode.local_is_from_env(index))
                {
                    self.env.insert(name, value);
                    continue;
                }
                let syncs_global_this = self.bytecode.local_is_sloppy_global_fallback(index)
                    || (self.bytecode.global_scope
                        && self.bytecode.local_is_body_hoist_only(index)
                        && !self.bytecode.local_is_compiler_temporary(index));
                let value = if syncs_global_this {
                    self.global_this_property(&name).unwrap_or(value)
                } else {
                    value
                };
                if self.locals[index].is_some()
                    || self.bytecode.local_is_from_env(index)
                    || syncs_global_this
                {
                    if self.locals[index]
                        .as_ref()
                        .is_some_and(|current| !is_uninitialized_lexical_value(current))
                        && is_uninitialized_lexical_value(&value)
                    {
                        continue;
                    }
                    self.locals[index] = Some(value.clone());
                    if let Some(upvalue) = self.local_upvalues.get(index).and_then(Option::as_ref) {
                        upvalue.set(value.clone());
                    }
                    self.write_through_module_live_binding(&name, value.clone());
                    let realm_backed_slot = (self.bytecode.global_scope
                        && self.bytecode.local_is_body_hoist_only(index)
                        && !self.bytecode.local_is_compiler_temporary(index))
                        || self
                            .local_upvalues
                            .get(index)
                            .and_then(Option::as_ref)
                            .is_some_and(|cell| self.env.is_realm_binding_cell(&name, cell));
                    if realm_backed_slot && self.realm.contains(&name) {
                        self.env.insert_realm(name, value);
                    } else if syncs_global_this {
                        self.sync_global_this_own_property(&name, value);
                    }
                } else if self.env.has_local_binding(&name) {
                    self.env.insert(name.clone(), value.clone());
                    self.write_through_module_live_binding(&name, value);
                }
            } else if self.env.has_local_binding(&name)
                || (self.in_parameter_prologue()
                    && !is_call_frame_binding(&name)
                    && !is_compiler_temporary(&name))
            {
                self.env.insert(name, value);
            } else if self.realm.contains(&name) {
                // Already a realm binding (shared cell) — leave it; a mutation
                // would have hit the cell directly.
            } else {
                self.env.insert(name, value);
            }
        }
    }

    fn sync_global_this_own_property(&self, name: &str, value: Value) {
        let Some(global_this) = self.cached_global_this() else {
            return;
        };
        if global_this.has_own_property(name) {
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
    pub(super) fn delete_ident(&mut self, name: &str) -> bool {
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
                    if self.bytecode.local_is_eval_deletable(slot) {
                        self.locals[slot] = None;
                        self.local_upvalues[slot] = None;
                        self.env.remove(name);
                        return true;
                    }
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
    }

    pub(super) fn record_sloppy_global_name(&mut self, name: &str) {
        if !self
            .sloppy_global_names
            .iter()
            .any(|existing| existing == name)
        {
            self.sloppy_global_names.push(name.to_owned());
        }
    }

    /// Replaces each captured loop binding with a fresh per-iteration cell.
    pub(super) fn fresh_iteration_scope(&mut self, slots: &[usize]) {
        for &slot in slots {
            if !self.local_upvalues.get(slot).is_some_and(Option::is_some) {
                continue;
            }
            let value = self
                .locals
                .get(slot)
                .and_then(Option::as_ref)
                .cloned()
                .unwrap_or_else(
                    || Value::Function(crate::Function::uninitialized_lexical_marker()),
                );
            if let Some(upvalue) = self.local_upvalues.get_mut(slot) {
                *upvalue = Some(Upvalue::new(value));
            }
        }
    }

    pub(super) fn drain_promise_jobs(&mut self) -> Result<(), RuntimeError> {
        let mut env = self.current_env();
        crate::promise::drain_promise_jobs(&mut env)?;
        self.apply_env(env);
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

pub(super) fn is_compiler_temporary(name: &str) -> bool {
    name.starts_with("\0\0")
}

pub(super) fn is_call_frame_binding(name: &str) -> bool {
    matches!(
        name,
        crate::GLOBAL_THIS_BINDING
            | crate::DIRECT_EVAL_STRICT_BINDING
            | crate::DIRECT_EVAL_ARGUMENTS_BINDING
            | crate::DIRECT_EVAL_FUNCTION_CONTEXT_BINDING
            | crate::FIELD_INITIALIZER_EVAL_BINDING
            | crate::HOME_OBJECT_BINDING
            | crate::NEW_TARGET_BINDING
            | crate::SUPER_CONSTRUCTOR_BINDING
            | crate::ACTIVE_CONSTRUCTOR_BINDING
            | "this"
            | "arguments"
    )
}

fn is_uninitialized_lexical_value(value: &Value) -> bool {
    matches!(value, Value::Function(function) if function.is_uninitialized_lexical_marker())
}
