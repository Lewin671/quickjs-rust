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
                if local.from_env
                    && let Some(value) = env.get(&local.name)
                {
                    Some(value)
                } else if local.hoisted {
                    Some(Value::Undefined)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(super) fn load_global(&self, name: &str) -> Result<Value, RuntimeError> {
        // A "global" name may actually be a caller-scope binding carried in this
        // frame's own locals layer (e.g. an outer `var`/`let` the body closes
        // over); check that first, then the shared realm.
        self.env.get(name).ok_or_else(|| RuntimeError {
            thrown: None,
            message: format!("ReferenceError: undefined identifier `{name}`"),
        })
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
        global_this.define_property(name, Property::data(value, true, true, false));
        Ok(())
    }
}
