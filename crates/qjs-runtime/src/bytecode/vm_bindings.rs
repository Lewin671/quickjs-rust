use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::{
    GLOBAL_THIS_BINDING, Property, PropertyKey, RuntimeError, Value, function::CallEnv, is_truthy,
    object::boxed_primitive, property::has_property, property_value, property_value_key,
    symbol::unscopables_symbol,
};

use super::util::typeof_value;
use super::{
    ir::{Bytecode, Op},
    vm::{Slot, Vm},
    vm_props::get_property,
    vm_set::set_property_key,
};

impl Vm<'_> {
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
                        let env = self.realm_env();
                        Ok(boxed_primitive(value, &env)
                            .expect("primitive value should box to object"))
                    }
                    other => Ok(other),
                };
                if let Some(object) = self.handle_runtime_result(result)? {
                    self.with_stack.push(object);
                }
            }
            Op::ExitWith => {
                self.with_stack.pop();
            }
            Op::LoadIdentWith { name, slot } => {
                let result = self.load_ident_with(&name, slot);
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

    pub(super) fn initialize_script_global_bindings(
        bytecode: &Bytecode,
        globals: &mut HashMap<String, Value>,
    ) -> Result<(), RuntimeError> {
        let global_this = globals
            .get(GLOBAL_THIS_BINDING)
            .and_then(|value| match value {
                Value::Object(object) => Some(object.clone()),
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
                globals.insert(name.to_owned(), property.value);
            } else {
                globals.entry(name.to_owned()).or_insert(Value::Undefined);
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
                // Global-scope `var`/function bindings live in the realm; the
                // vestigial slot stays empty so captures and loads route to
                // the shared cell instead of a frozen copy.
                if local.hoisted && bytecode.global_scope {
                    None
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
                } else if local.hoisted {
                    Some(Value::Undefined)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Keeps the frame's shared `captured_env` copy of a slot binding current
    /// when the slot changes, so closures sharing the Rc observe the update.
    pub(super) fn write_through_captured(&self, name: &str, value: Value) {
        let mut captured = self.captured_env.borrow_mut();
        if let Some(slot_value) = captured.get_mut(name) {
            *slot_value = value;
        }
    }

    pub(super) fn load_global(&mut self, name: &str) -> Result<Value, RuntimeError> {
        // `this` belongs to the frame: function frames bind it in their locals
        // layer (arrows inherit it through capture). Falling through to the
        // realm's global `this` would leak it into the derived-constructor
        // TDZ window, so only global script code reads `this` from the realm.
        if name == "this" && !self.bytecode.global_scope {
            return self.env.get_local(name).ok_or_else(|| RuntimeError {
                thrown: None,
                message: "ReferenceError: must call super constructor before accessing `this`"
                    .to_owned(),
            });
        }
        // A "global" name may actually be a caller-scope binding carried in this
        // frame's own locals layer (e.g. an outer `var`/`let` the body closes
        // over); check that first, then the shared realm, then a property
        // created directly on `globalThis` (`this.x = 1` and realm bindings
        // share one global namespace).
        if let Some(value) = self.env.get(name) {
            return Ok(value);
        }
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => Some(global_this.clone()),
            _ => None,
        };
        if let Some(global_this) = global_this
            && global_this.has_own_property(name)
        {
            let mut env = self.current_env();
            let value = property_value(Value::Object(global_this), name, &mut env)?;
            self.apply_env(env);
            return Ok(value);
        }
        Err(RuntimeError {
            thrown: None,
            message: format!("ReferenceError: undefined identifier `{name}`"),
        })
    }

    pub(super) fn load_new_target(&self) -> Value {
        self.env
            .get(crate::NEW_TARGET_BINDING)
            .unwrap_or(Value::Undefined)
    }

    /// Reads an own property of the realm's `globalThis` object, if any.
    pub(super) fn global_this_property(&self, name: &str) -> Option<Value> {
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => Some(global_this.clone()),
            _ => None,
        }?;
        global_this
            .own_property(name)
            .map(|property| property.value)
    }

    fn has_realm_or_global_this_binding(&self, name: &str) -> bool {
        self.realm.borrow().contains_key(name) || self.global_this_property(name).is_some()
    }

    fn store_realm_or_global_this_sloppy(
        &mut self,
        name: String,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if let Some(property) = self.global_this_own_property(&name)
            && !property.writable
        {
            return Ok(());
        }
        if self.env.locals().contains_key(&name) {
            self.env.insert(name.clone(), value.clone());
            self.write_through_captured(&name, value);
            return Ok(());
        }
        self.invalidate_array_prototype_cache(&name);
        if self.realm.borrow().contains_key(&name) {
            self.realm.borrow_mut().insert(name.clone(), value.clone());
            let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
                Some(Value::Object(global_this)) => Some(global_this.clone()),
                _ => None,
            };
            if let Some(global_this) = global_this
                && global_this.has_own_property(&name)
            {
                global_this.set(name, value);
            }
            return Ok(());
        }
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => Some(global_this.clone()),
            _ => None,
        };
        if let Some(global_this) = global_this {
            global_this.set(name.clone(), value.clone());
        }
        self.realm.borrow_mut().insert(name, value);
        Ok(())
    }

    /// Returns the full own-property descriptor of a `globalThis` property so
    /// callers can inspect attribute flags such as `writable`.
    pub(super) fn global_this_own_property(&self, name: &str) -> Option<Property> {
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => Some(global_this.clone()),
            _ => None,
        }?;
        global_this.own_property(name)
    }

    /// Resolves an identifier inside a `with` body to the innermost with-object
    /// that binds `name` (an own-or-inherited property not filtered out by the
    /// object's `Symbol.unscopables`). Returns `None` when no with-object binds
    /// it, in which case the caller falls back to ordinary scope resolution.
    fn with_binding_object(&self, name: &str) -> Result<Option<Value>, RuntimeError> {
        let env = self.realm_env();
        for object in self.with_stack.iter().rev() {
            if !has_property(object.clone(), &env, name)? {
                continue;
            }
            if self.is_unscopable(object, name)? {
                continue;
            }
            return Ok(Some(object.clone()));
        }
        Ok(None)
    }

    /// Whether `name` is excluded from a with-object's bindings by its
    /// `Symbol.unscopables` (a property whose value is truthy).
    fn is_unscopable(&self, object: &Value, name: &str) -> Result<bool, RuntimeError> {
        let mut env = self.current_env();
        let Some(symbol) = unscopables_symbol(&env) else {
            return Ok(false);
        };
        let unscopables =
            property_value_key(object.clone(), &PropertyKey::Symbol(symbol), &mut env)?;
        match unscopables {
            Value::Object(_) | Value::Function(_) | Value::Array(_) => {
                let blocked = get_property(unscopables, name, &mut env)?;
                Ok(is_truthy(&blocked))
            }
            _ => Ok(false),
        }
    }

    pub(super) fn load_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
    ) -> Result<Value, RuntimeError> {
        if let Some(object) = self.with_binding_object(name)? {
            let mut env = self.current_env();
            let value = get_property(object, name, &mut env)?;
            self.apply_env(env);
            return Ok(value);
        }
        match slot {
            Some(slot) => self.load_local(slot),
            None => self.load_global(name),
        }
    }

    pub(super) fn store_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
        is_strict: bool,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if let Some(object) = self.with_binding_object(name)? {
            let mut env = self.current_env();
            set_property_key(
                object,
                PropertyKey::String(name.to_owned()),
                value,
                &mut env,
            )?;
            self.apply_env(env);
            return Ok(());
        }
        match slot {
            Some(slot) => self.assign_local(slot, value),
            None if is_strict => self.store_global_strict(name.to_owned(), value),
            None => {
                self.store_global_sloppy(name.to_owned(), value)?;
                self.record_sloppy_global_name(name);
                Ok(())
            }
        }
    }

    pub(super) fn resolve_ident_with(
        &mut self,
        name: &str,
        object_slot: usize,
    ) -> Result<(), RuntimeError> {
        let value = self.with_binding_object(name)?.unwrap_or(Value::Undefined);
        self.store_local(object_slot, value)
    }

    pub(super) fn store_resolved_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
        object_slot: usize,
        is_strict: bool,
        value: Value,
    ) -> Result<(), RuntimeError> {
        match self.load_local(object_slot)? {
            Value::Undefined => match slot {
                Some(slot) => self.assign_local(slot, value),
                None if is_strict => self.store_global_strict(name.to_owned(), value),
                None => {
                    self.store_global_sloppy(name.to_owned(), value)?;
                    self.record_sloppy_global_name(name);
                    Ok(())
                }
            },
            object => {
                let mut env = self.current_env();
                if is_strict && !has_property(object.clone(), &env, name)? {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!("ReferenceError: undefined identifier `{name}`"),
                    });
                }
                set_property_key(
                    object,
                    PropertyKey::String(name.to_owned()),
                    value,
                    &mut env,
                )?;
                self.apply_env(env);
                Ok(())
            }
        }
    }

    pub(super) fn typeof_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
    ) -> Result<Value, RuntimeError> {
        if let Some(object) = self.with_binding_object(name)? {
            let mut env = self.current_env();
            let value = get_property(object, name, &mut env)?;
            self.apply_env(env);
            return Ok(Value::String(typeof_value(value)));
        }
        let value = match slot {
            Some(slot) => self.load_local(slot)?,
            None => self.env.get(name).unwrap_or(Value::Undefined),
        };
        Ok(Value::String(typeof_value(value)))
    }

    pub(super) fn load_local(&self, slot: usize) -> Result<Value, RuntimeError> {
        match self.locals.get(slot) {
            Some(Some(value)) => Ok(value.clone()),
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

    pub(super) fn load_local_or_undefined(&self, slot: usize) -> Result<Value, RuntimeError> {
        match self.locals.get(slot) {
            Some(Some(value)) => Ok(value.clone()),
            Some(None) => Ok(Value::Undefined),
            None => Err(RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    pub(super) fn store_local(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        let local_meta = self
            .bytecode
            .locals
            .get(slot)
            .cloned()
            .ok_or_else(|| RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            })?;
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        if !local_meta.mutable && local.is_some() {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        *local = Some(value.clone());
        // A closure created in this frame shares its captured-binding snapshot
        // through `captured_env`; keep a captured copy of this slot current so
        // later calls of that closure observe the new value.
        self.write_through_captured(&local_meta.name, value.clone());
        if local_meta.from_env
            && !local_meta.hoisted
            && let Some(Value::Object(global_this)) =
                self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned()
            && global_this.has_own_property(&local_meta.name)
        {
            global_this.set(local_meta.name, value);
        }
        Ok(())
    }

    pub(super) fn assign_local(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        match self.locals.get(slot) {
            Some(Some(_)) => self.store_local(slot, value),
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

    pub(super) fn store_local_or_global_sloppy(
        &mut self,
        slot: usize,
        name: String,
        value: Value,
    ) -> Result<(), RuntimeError> {
        match self.locals.get(slot) {
            Some(Some(_)) => {
                if self.has_realm_or_global_this_binding(&name) {
                    self.store_realm_or_global_this_sloppy(name.clone(), value)?;
                    let value = self.load_global(&name)?;
                    self.store_local(slot, value)?;
                } else {
                    self.store_local(slot, value)?;
                }
                Ok(())
            }
            Some(None) => {
                self.store_global_sloppy(name.clone(), value)?;
                self.record_sloppy_global_name(&name);
                let global_value = self.load_global(&name)?;
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
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        *local = None;
        Ok(())
    }

    pub(super) fn define_global_var(
        &mut self,
        name: String,
        value: Value,
    ) -> Result<(), RuntimeError> {
        let Some(Value::Object(global_this)) =
            self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned()
        else {
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
            self.invalidate_array_prototype_cache(&name);
            self.realm.borrow_mut().insert(name, value);
            return Ok(());
        }
        global_this.define_property(
            name.clone(),
            Property::data(value.clone(), true, true, false),
        );
        self.invalidate_array_prototype_cache(&name);
        self.realm.borrow_mut().insert(name, value);
        Ok(())
    }
    pub(super) fn apply_selected_env(
        &mut self,
        env: CallEnv,
        binding_names: &[String],
        injected: &HashMap<String, Value>,
    ) {
        for name in binding_names {
            let Some(value) = env.get(name) else {
                continue;
            };
            // An injected caller binding the callee never modified must not
            // write back: a newer value may have arrived through the shared
            // captured_env while this call was in flight.
            if injected.get(name) == Some(&value) {
                if let Some(captured) = self.captured_env.borrow().get(name).cloned()
                    && Some(&captured) != injected.get(name)
                {
                    if let Some(index) = self.bytecode.local_slot(name) {
                        self.locals[index] = Some(captured);
                    } else if self.env.locals().contains_key(name) {
                        self.env.insert(name.clone(), captured);
                    }
                }
                continue;
            }
            if let Some(index) = self.bytecode.local_slot(name) {
                self.locals[index] = Some(value.clone());
                self.write_through_captured(name, value);
                if self.env.locals().contains_key(name) {
                    let value = self.locals[index]
                        .as_ref()
                        .expect("slot was just assigned")
                        .clone();
                    self.env.insert(name.clone(), value);
                }
                if !self.env.locals().contains_key(name)
                    && !self.has_captured_binding(name)
                    && env.realm_contains(name)
                {
                    let value = self.locals[index]
                        .as_ref()
                        .expect("slot was just assigned")
                        .clone();
                    env.insert_realm(name.clone(), value);
                }
            } else if self.env.locals().contains_key(name) {
                // A caller-scope binding this frame itself carries in its
                // locals layer (e.g. an outer `let` riding through nested
                // calls): keep it current so it propagates further out.
                self.env.insert(name.clone(), value);
            } else if name == "this"
                && self
                    .env
                    .locals()
                    .contains_key(crate::SUPER_CONSTRUCTOR_BINDING)
            {
                self.env.insert(name.clone(), value);
            } else if !self.has_captured_binding(name) && env.realm_contains(name) {
                env.insert_realm(name.clone(), value);
            }
        }
        self.refresh_derived_constructor_this_from_captured();
    }

    fn has_captured_binding(&self, name: &str) -> bool {
        self.captured_env.borrow().contains_key(name)
            || self
                .env
                .captured_binding_source_env()
                .is_some_and(|source| source.borrow().contains_key(name))
    }

    pub(super) fn apply_env(&mut self, env: CallEnv) {
        // The realm layer is shared by `Rc`, so global writes are already live.
        // Write each non-realm local back to its slot, to the frame's own
        // internal/caller-scope binding layer, or (for a genuinely new binding)
        // to the shared realm.
        let locals = env.into_locals();
        for (name, value) in locals {
            if let Some(index) = self.bytecode.local_slot(&name) {
                if self.locals[index].is_some() {
                    self.locals[index] = Some(value.clone());
                    self.write_through_captured(&name, value);
                }
            } else if self.env.locals().contains_key(&name) {
                self.env.insert(name, value);
            } else if self.realm.borrow().contains_key(&name) {
                // Already a realm binding (shared cell) — leave it; a mutation
                // would have hit the cell directly.
            } else {
                self.env.insert(name, value);
            }
        }
        self.refresh_derived_constructor_this_from_captured();
    }

    fn refresh_derived_constructor_this_from_captured(&mut self) {
        if self.env.locals().contains_key("this")
            || !self
                .env
                .locals()
                .contains_key(crate::SUPER_CONSTRUCTOR_BINDING)
        {
            return;
        }
        let Some(value) = self.captured_env.borrow().get("this").cloned() else {
            return;
        };
        if matches!(
            &value,
            Value::Function(function) if function.is_uninitialized_lexical_marker()
        ) {
            return;
        }
        self.env.insert("this".to_owned(), value);
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
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned() {
            Some(Value::Object(obj)) => obj,
            _ => return true,
        };
        if !global_this.has_own_property(name) {
            // Name exists in realm but not on globalThis — it's a lexical
            // binding from a script-level `let`/`const`; undeletable.
            if self.realm.borrow().contains_key(name) {
                return false;
            }
            return true;
        }
        let deleted = global_this.delete_own_property(name);
        if deleted {
            self.realm.borrow_mut().remove(name);
            // Clear the cached local slot if the sloppy global was mirrored there.
            if let Some(slot) = self.bytecode.local_slot(name) {
                if let Some(local) = self.locals.get_mut(slot) {
                    *local = None;
                }
            }
        }
        deleted
    }

    /// `delete identifier` inside a `with` body in non-strict mode: checks the
    /// with-object stack first (deletes from the first binding object), then
    /// falls back to `delete_ident` for local/global scope.
    pub(super) fn delete_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
    ) -> Result<(), RuntimeError> {
        if let Some(object) = self.with_binding_object(name)? {
            let mut env = self.current_env();
            let result = super::vm_props::delete_property_key(
                object,
                &PropertyKey::String(name.to_owned()),
                &mut env,
            )?;
            self.apply_env(env);
            self.stack.push(result);
        } else {
            // Fall back to ordinary identifier deletion.
            let result = if let Some(s) = slot {
                if self.locals[s].is_some() {
                    false
                } else {
                    self.delete_ident(name)
                }
            } else {
                self.delete_ident(name)
            };
            self.stack.push(Value::Boolean(result));
        }
        Ok(())
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

    /// Creates a fresh captured-environment cell for per-iteration loop
    /// bindings. The new cell is seeded from the current local slot values,
    /// making it an independent snapshot. Closures created after this point
    /// capture from the new cell; closures from previous iterations retain
    /// their old cell. Write-through from the loop body still targets the
    /// new cell, which is fine since it belongs to THIS iteration's closures.
    pub(super) fn fresh_iteration_scope(&mut self, slots: &[usize]) {
        let mut new_env = self.captured_env.borrow().clone();
        for &slot in slots {
            if let Some(Some(value)) = self.locals.get(slot) {
                if let Some(name) = self.bytecode.local_name_at(slot) {
                    new_env.insert(name.to_owned(), value.clone());
                }
            }
        }
        self.captured_env = Rc::new(RefCell::new(new_env));
    }

    pub(super) fn push_captured_env(&mut self) {
        self.captured_env_stack.push(Rc::clone(&self.captured_env));
    }

    pub(super) fn pop_captured_env(&mut self) {
        if let Some(env) = self.captured_env_stack.pop() {
            self.captured_env = env;
        }
    }

    pub(super) fn drain_promise_jobs(&mut self) -> Result<(), RuntimeError> {
        let mut env = self.current_env();
        crate::promise::drain_promise_jobs(&mut env)?;
        self.apply_env(env);
        Ok(())
    }
}
