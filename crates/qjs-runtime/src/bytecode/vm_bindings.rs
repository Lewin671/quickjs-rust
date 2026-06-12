use std::collections::HashMap;

use crate::{GLOBAL_THIS_BINDING, Property, RuntimeError, Value, function::CallEnv};

use super::{
    ir::Bytecode,
    vm::{Slot, Vm},
};

impl Vm<'_> {
    pub(super) fn initialize_script_global_bindings(
        bytecode: &Bytecode,
        globals: &mut HashMap<String, Value>,
    ) {
        let global_this = globals
            .get(GLOBAL_THIS_BINDING)
            .and_then(|value| match value {
                Value::Object(object) => Some(object.clone()),
                _ => None,
            });
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
        {
            global_this.set(local_meta.name, value);
        }
        Ok(())
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
            }
            // Realm bindings need no write-back: the callee mutated the shared
            // cell directly.
        }
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
    }

    pub(super) fn drain_promise_jobs(&mut self) -> Result<(), RuntimeError> {
        let mut env = self.current_env();
        crate::promise::drain_promise_jobs(&mut env)?;
        self.apply_env(env);
        Ok(())
    }
}
