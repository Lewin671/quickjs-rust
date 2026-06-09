use std::collections::HashMap;

use crate::{GLOBAL_THIS_BINDING, Property, RuntimeError, Value};

use super::{
    ir::Bytecode,
    vm::{Slot, Vm},
};

impl Vm<'_> {
    pub(super) fn initial_slots(
        bytecode: &Bytecode,
        globals: &HashMap<String, Value>,
    ) -> Vec<Slot> {
        bytecode
            .locals
            .iter()
            .map(|local| {
                if local.from_env
                    && let Some(value) = globals.get(&local.name)
                {
                    Some(value.clone())
                } else if local.hoisted {
                    Some(Value::Undefined)
                } else {
                    None
                }
            })
            .collect()
    }

    pub(super) fn load_global(&self, name: &str) -> Result<Value, RuntimeError> {
        self.globals.get(name).cloned().ok_or_else(|| RuntimeError {
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
            && let Some(Value::Object(global_this)) = self.globals.get(GLOBAL_THIS_BINDING)
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
                if self.globals.contains_key(&name) {
                    self.store_global_sloppy(name, value)?;
                }
                Ok(())
            }
            Some(None) => {
                self.store_global_sloppy(name.clone(), value)?;
                self.record_sloppy_global_name(&name);
                let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
                    thrown: None,
                    message: "bytecode local index out of bounds".to_owned(),
                })?;
                *local = self.globals.get(&name).cloned();
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
        let Some(Value::Object(global_this)) = self.globals.get(GLOBAL_THIS_BINDING) else {
            return Err(RuntimeError {
                thrown: None,
                message: "global object binding is missing".to_owned(),
            });
        };
        global_this.define_property(name, Property::data(value, true, true, false));
        Ok(())
    }
}
