use std::collections::HashMap;

use crate::{
    GLOBAL_THIS_BINDING, Property, PropertyKey, RuntimeError, Value, function::CallEnv, is_truthy,
    property::has_property, property_value_key, symbol::unscopables_symbol,
};

use super::util::typeof_value;
use super::{
    ir::{Bytecode, Op},
    vm::{Slot, Vm},
    vm_props::{get_property, set_property_key},
};

impl Vm<'_> {
    /// Executes a `with`-related opcode: scope push/pop and the with-aware
    /// identifier load/store/typeof. Centralizing the stack interaction here
    /// keeps the main bytecode loop terse.
    pub(super) fn run_with_op(&mut self, op: Op) -> Result<(), RuntimeError> {
        match op {
            Op::EnterWith => {
                let object = self.pop()?;
                self.with_stack.push(object);
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

    pub(super) fn load_global(&self, name: &str) -> Result<Value, RuntimeError> {
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
        if let Some(value) = self.global_this_property(name) {
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
            Some(slot) => self.load_local_or_undefined(slot)?,
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
                self.store_local(slot, value.clone())?;
                if self.env.contains_key(&name) {
                    self.store_global_sloppy(name, value)?;
                }
                Ok(())
            }
            Some(None) => {
                self.store_global_sloppy(name.clone(), value)?;
                self.record_sloppy_global_name(&name);
                let global_value = self.env.get(&name);
                let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
                    thrown: None,
                    message: "bytecode local index out of bounds".to_owned(),
                })?;
                *local = global_value;
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
            }
            // Realm bindings need no write-back: the callee mutated the shared
            // cell directly.
        }
        self.refresh_derived_constructor_this_from_captured();
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

    pub(super) fn drain_promise_jobs(&mut self) -> Result<(), RuntimeError> {
        let mut env = self.current_env();
        crate::promise::drain_promise_jobs(&mut env)?;
        self.apply_env(env);
        Ok(())
    }
}
