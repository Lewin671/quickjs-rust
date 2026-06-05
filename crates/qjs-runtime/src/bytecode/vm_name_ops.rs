use crate::{GLOBAL_THIS_BINDING, LOCAL_CAPTURE_PREFIX, ObjectRef, RuntimeError, Value};

use super::vm::Vm;
use super::vm_props::{delete_property, get_property, set_property};

#[derive(Clone)]
pub(super) enum NameReference {
    WithObject { name: String, object: Value },
    Local { name: String, slot: usize },
    Global { name: String, loaded_value: Value },
    GlobalObject { name: String, object: ObjectRef },
}

impl Vm<'_> {
    pub(super) fn resolve_name(&mut self, name: &str) {
        if let Some(object) = self.resolve_with_object(name) {
            self.name_references.push(NameReference::WithObject {
                name: name.to_owned(),
                object,
            });
            return;
        }
        if let Some(slot) = self.bytecode.local_slot(name) {
            self.name_references.push(NameReference::Local {
                name: name.to_owned(),
                slot,
            });
            return;
        }
        if let Some(value) = self.globals.get(name).cloned() {
            self.name_references.push(NameReference::Global {
                name: name.to_owned(),
                loaded_value: value,
            });
            return;
        }
        if let Some(object) = self.global_object_with_property(name) {
            self.name_references.push(NameReference::GlobalObject {
                name: name.to_owned(),
                object,
            });
        }
    }

    pub(super) fn load_name(&mut self, name: &str) -> Result<Value, RuntimeError> {
        if let Some(object) = self.resolve_with_object(name) {
            let mut env = self.current_env();
            let value = get_property(object.clone(), name, &mut env)?;
            self.apply_env(env);
            self.name_references.push(NameReference::WithObject {
                name: name.to_owned(),
                object,
            });
            return Ok(value);
        }
        if let Some(slot) = self.bytecode.local_slot(name) {
            let value = self.load_local(slot)?;
            self.name_references.push(NameReference::Local {
                name: name.to_owned(),
                slot,
            });
            return Ok(value);
        }
        if self.is_uninitialized_captured_binding(name) {
            return Err(reference_error(name));
        }
        if let Some(value) = self.globals.get(name).cloned() {
            self.name_references.push(NameReference::Global {
                name: name.to_owned(),
                loaded_value: value.clone(),
            });
            return Ok(value);
        }
        if let Some(object) = self.global_object_with_property(name) {
            let mut env = self.current_env();
            let value = get_property(Value::Object(object.clone()), name, &mut env)?;
            self.apply_env(env);
            self.name_references.push(NameReference::GlobalObject {
                name: name.to_owned(),
                object,
            });
            return Ok(value);
        }
        Err(RuntimeError {
            thrown: None,
            message: format!("ReferenceError: undefined identifier `{name}`"),
        })
    }

    pub(super) fn store_name(
        &mut self,
        name: &str,
        value: Value,
        strict: bool,
    ) -> Result<(), RuntimeError> {
        match self.take_name_reference(name) {
            Some(NameReference::WithObject { object, .. }) => {
                if strict && !self.object_environment_has_property(&object, name) {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!("ReferenceError: undefined identifier `{name}`"),
                    });
                }
                let mut env = self.current_env();
                let updated = set_property(object, name.to_owned(), value, &mut env)?;
                self.apply_env(env);
                if strict && !updated {
                    Err(RuntimeError {
                        thrown: None,
                        message: "TypeError: cannot assign to property".to_owned(),
                    })
                } else {
                    Ok(())
                }
            }
            Some(NameReference::Local { slot, .. }) => self.store_local(slot, value),
            Some(NameReference::Global { loaded_value, .. }) => {
                if self
                    .globals
                    .get(name)
                    .is_some_and(|current| current != &loaded_value)
                {
                    self.binding_overrides.insert(name.to_owned(), value);
                } else {
                    self.globals.insert(name.to_owned(), value);
                }
                Ok(())
            }
            Some(NameReference::GlobalObject { object, .. }) => {
                if strict && !object.contains_property(name) {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!("ReferenceError: undefined identifier `{name}`"),
                    });
                }
                let mut env = self.current_env();
                let updated =
                    set_property(Value::Object(object), name.to_owned(), value, &mut env)?;
                self.apply_env(env);
                if strict && !updated {
                    Err(RuntimeError {
                        thrown: None,
                        message: "TypeError: cannot assign to property".to_owned(),
                    })
                } else {
                    Ok(())
                }
            }
            None => self.store_resolved_name(name, value, strict),
        }
    }

    pub(super) fn enter_with(&mut self, cleanup_ip: usize) -> Result<(), RuntimeError> {
        let object = self.pop()?;
        if matches!(object, Value::Null | Value::Undefined)
            && self
                .handle_runtime_result::<()>(Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: with object cannot be null or undefined".to_owned(),
                }))?
                .is_none()
        {
            return Ok(());
        }
        self.with_stack.push(object);
        self.with_cleanup_stack.push(cleanup_ip);
        self.stack.push(Value::Undefined);
        Ok(())
    }

    pub(super) fn exit_with(&mut self) -> Result<(), RuntimeError> {
        self.with_cleanup_stack.pop().ok_or_else(|| RuntimeError {
            thrown: None,
            message: "with cleanup stack underflow".to_owned(),
        })?;
        self.with_stack.pop().ok_or_else(|| RuntimeError {
            thrown: None,
            message: "with stack underflow".to_owned(),
        })?;
        Ok(())
    }

    pub(super) fn cleanup_with_for_jump(&mut self, target: usize) {
        while self
            .with_cleanup_stack
            .last()
            .is_some_and(|cleanup_ip| *cleanup_ip != usize::MAX && target >= *cleanup_ip)
        {
            self.with_cleanup_stack.pop();
            self.with_stack.pop();
        }
    }

    pub(super) fn cleanup_active_with_scopes(&mut self) {
        while self
            .with_cleanup_stack
            .last()
            .is_some_and(|cleanup_ip| *cleanup_ip != usize::MAX)
        {
            self.with_cleanup_stack.pop();
            self.with_stack.pop();
        }
    }

    pub(super) fn delete_name(&mut self, name: &str) -> Result<(), RuntimeError> {
        if let Some(object) = self.resolve_with_object(name) {
            let deleted = delete_property(object, name)?;
            self.stack.push(deleted);
            return Ok(());
        }
        if self.bytecode.local_slot(name).is_some() {
            self.stack.push(Value::Boolean(false));
            return Ok(());
        }
        if let Some(object) = self.global_object_with_property(name) {
            let deleted = delete_property(Value::Object(object), name)?;
            self.globals.remove(name);
            self.stack.push(deleted);
            return Ok(());
        }
        if self.globals.remove(name).is_some() {
            if let Some(Value::Object(global_object)) = self.globals.get(GLOBAL_THIS_BINDING) {
                global_object.delete_own_property(name);
            }
            self.stack.push(Value::Boolean(true));
            return Ok(());
        }
        self.stack.push(Value::Boolean(true));
        Ok(())
    }

    fn take_name_reference(&mut self, name: &str) -> Option<NameReference> {
        let index = self.name_references.iter().rposition(|reference| {
            matches!(
                reference,
                NameReference::WithObject { name: candidate, .. }
                    | NameReference::Local {
                        name: candidate, ..
                    }
                    | NameReference::Global {
                        name: candidate, ..
                    }
                    | NameReference::GlobalObject {
                        name: candidate, ..
                    }
                    if candidate == name
            )
        })?;
        Some(self.name_references.remove(index))
    }

    fn store_resolved_name(
        &mut self,
        name: &str,
        value: Value,
        strict: bool,
    ) -> Result<(), RuntimeError> {
        if let Some(object) = self.resolve_with_object(name) {
            let mut env = self.current_env();
            let updated = set_property(object, name.to_owned(), value, &mut env)?;
            self.apply_env(env);
            if strict && !updated {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: cannot assign to property".to_owned(),
                });
            }
            return Ok(());
        }
        if let Some(slot) = self.bytecode.local_slot(name) {
            return self.store_local(slot, value);
        }
        if self.is_uninitialized_captured_binding(name) {
            return Err(reference_error(name));
        }
        if let Some(object) = self.global_object_with_property(name) {
            let mut env = self.current_env();
            let updated = set_property(Value::Object(object), name.to_owned(), value, &mut env)?;
            self.apply_env(env);
            if strict && !updated {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: cannot assign to property".to_owned(),
                });
            }
            return Ok(());
        }
        if strict {
            return Err(RuntimeError {
                thrown: None,
                message: format!("ReferenceError: undefined identifier `{name}`"),
            });
        }
        if let Some(Value::Object(global_object)) = self.globals.get(GLOBAL_THIS_BINDING) {
            global_object.set(name.to_owned(), value.clone());
        }
        self.globals.insert(name.to_owned(), value);
        Ok(())
    }

    fn global_object_with_property(&self, name: &str) -> Option<ObjectRef> {
        let Some(Value::Object(object)) = self.globals.get(GLOBAL_THIS_BINDING) else {
            return None;
        };
        object.contains_property(name).then(|| object.clone())
    }

    fn is_uninitialized_captured_binding(&self, name: &str) -> bool {
        self.globals
            .get(&format!("{LOCAL_CAPTURE_PREFIX}{name}"))
            .is_some_and(|value| matches!(value, Value::Boolean(true)))
            && !self.globals.contains_key(name)
    }

    fn resolve_with_object(&self, name: &str) -> Option<Value> {
        self.with_stack
            .iter()
            .rev()
            .find(|object| self.with_object_has_property(object, name))
            .cloned()
    }

    fn with_object_has_property(&self, object: &Value, name: &str) -> bool {
        self.object_environment_has_property(object, name)
    }

    fn object_environment_has_property(&self, object: &Value, name: &str) -> bool {
        match object {
            Value::Object(object) => object.contains_property(name),
            Value::Array(array) => name == "length" || array.property(name).is_some(),
            Value::Function(function) => function.properties.borrow().contains_key(name),
            Value::String(value) => {
                name == "length" || crate::string::string_property(value, name).is_some()
            }
            _ => false,
        }
    }
}

fn reference_error(name: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("ReferenceError: undefined identifier `{name}`"),
    }
}
