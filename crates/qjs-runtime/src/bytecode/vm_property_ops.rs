use crate::{GLOBAL_THIS_BINDING, RuntimeError, Value, call_function, to_property_key_with_env};

use super::vm::{Vm, property_base_error};
use super::vm_props::{delete_property, get_property, set_property};

impl Vm<'_> {
    pub(super) fn get_prop(&mut self) -> Result<(), RuntimeError> {
        let key_value = self.pop()?;
        let object = self.pop()?;
        if matches!(object, Value::Null | Value::Undefined) {
            if self
                .handle_runtime_result::<()>(Err(property_base_error()))?
                .is_none()
            {
                return Ok(());
            }
            return Err(RuntimeError {
                thrown: None,
                message: "property base error did not throw".to_owned(),
            });
        }
        let mut key_env = self.current_env();
        let key_result = to_property_key_with_env(key_value, &mut key_env);
        self.apply_env(key_env);
        let Some(key) = self.handle_runtime_result(key_result)? else {
            return Ok(());
        };
        let mut env = self.current_env();
        let value_result = get_property(object, &key, &mut env);
        self.apply_env(env);
        if let Some(value) = self.handle_runtime_result(value_result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    pub(super) fn check_object_coercible(&mut self) -> Result<(), RuntimeError> {
        if matches!(self.stack.last(), Some(Value::Null | Value::Undefined)) {
            if self
                .handle_runtime_result::<()>(Err(property_base_error()))?
                .is_none()
            {
                return Ok(());
            }
            return Err(RuntimeError {
                thrown: None,
                message: "property base error did not throw".to_owned(),
            });
        }
        Ok(())
    }

    pub(super) fn coerce_property_key(&mut self) -> Result<(), RuntimeError> {
        let key_value = self.pop()?;
        let mut key_env = self.current_env();
        let key_result = to_property_key_with_env(key_value, &mut key_env);
        self.apply_env(key_env);
        if let Some(key) = self.handle_runtime_result(key_result)? {
            self.stack.push(Value::String(key));
        }
        Ok(())
    }

    pub(super) fn set_prop(&mut self, strict: bool) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let key_value = self.pop()?;
        let object = self.pop()?;
        if matches!(object, Value::Null | Value::Undefined) {
            if self
                .handle_runtime_result::<()>(Err(property_base_error()))?
                .is_none()
            {
                return Ok(());
            }
            return Err(RuntimeError {
                thrown: None,
                message: "property base error did not throw".to_owned(),
            });
        }
        let mut key_env = self.current_env();
        let key_result = to_property_key_with_env(key_value, &mut key_env);
        self.apply_env(key_env);
        let Some(key) = self.handle_runtime_result(key_result)? else {
            return Ok(());
        };
        let mut env = self.current_env();
        let set_result = set_property(object.clone(), key.clone(), value.clone(), &mut env);
        self.apply_env(env);
        let Some(did_set) = self.handle_runtime_result(set_result)? else {
            return Ok(());
        };
        if strict && !did_set {
            if self
                .handle_runtime_result::<()>(Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: cannot assign to read-only property".to_owned(),
                }))?
                .is_none()
            {
                return Ok(());
            }
            return Err(RuntimeError {
                thrown: None,
                message: "strict property assignment failed without throwing".to_owned(),
            });
        }
        if did_set {
            self.sync_global_object_property_to_local(&object, &key, &value);
        }
        self.stack.push(value);
        Ok(())
    }

    pub(super) fn delete_prop(&mut self) -> Result<(), RuntimeError> {
        let mut key_env = self.current_env();
        let key = to_property_key_with_env(self.pop()?, &mut key_env)?;
        self.apply_env(key_env);
        let object = self.pop()?;
        self.stack.push(delete_property(object, &key)?);
        Ok(())
    }

    fn sync_global_object_property_to_local(&mut self, object: &Value, key: &str, value: &Value) {
        if !self.sync_var_to_global_object {
            return;
        }
        let Value::Object(object) = object else {
            return;
        };
        let Some(Value::Object(global_object)) = self.globals.get(GLOBAL_THIS_BINDING) else {
            return;
        };
        if !object.ptr_eq(global_object) {
            return;
        }
        let Some(slot) = self.bytecode.local_slot(key) else {
            return;
        };
        if self
            .bytecode
            .locals
            .get(slot)
            .is_some_and(|local| local.hoisted)
            && let Some(property) = global_object.own_property(key)
            && !property.is_accessor()
        {
            self.locals[slot] = Some(value.clone());
        }
    }

    pub(super) fn iterator_close_for_throw(
        &mut self,
        iterator_slot: usize,
    ) -> Result<(), RuntimeError> {
        let Some(iterator) = self.load_local_or_undefined(iterator_slot).ok() else {
            return Ok(());
        };
        let mut env = self.current_env();
        let return_method = get_property(iterator.clone(), "return", &mut env);
        self.apply_env(env);
        let Ok(Value::Function(function)) = return_method else {
            return Ok(());
        };
        let mut env = self.current_env();
        let _ = call_function(
            Value::Function(function),
            iterator,
            Vec::new(),
            &mut env,
            false,
        );
        self.apply_env(env);
        Ok(())
    }
}
