//! Dynamic identifier resolution through a `with` object environment.
//!
//! `with` is the construct that can put a name's meaning outside the frame's
//! slot-indexed locals: a free name inside the block is a property of the
//! object environment first and a binding second. Everything that has to
//! consult that object before -- or instead of -- a slot lives here, including
//! the opcode family the compiler emits inside the block and `delete` of a
//! name that may resolve to one.
//!
//! Split out of `vm_bindings` so that file stays the frame's own binding
//! model: slots, upvalue cells, and the realm's globals.

use crate::{
    PropertyKey, RuntimeError, Value, is_truthy, object::boxed_primitive, property::has_property,
    property_value_key, symbol::unscopables_symbol,
};

use super::util::typeof_value;
use super::{ir::Op, vm::Vm, vm_props::get_property, vm_set::set_property_key};

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
            Op::LoadIdentWith {
                name,
                slot,
                is_strict,
            } => {
                let result = self.load_ident_with(&name, slot, is_strict);
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
            Op::LoadResolvedIdentWith {
                name,
                slot,
                object_slot,
                is_strict,
            } => {
                let result = self.load_resolved_ident_with(&name, slot, object_slot, is_strict);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
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
        is_strict: bool,
    ) -> Result<Value, RuntimeError> {
        if let Some(object) = self.with_binding_object(name)? {
            let mut env = self.current_env();
            // GetBindingValue re-checks HasProperty (step 2) before the Get
            // (step 4); the binding may have been deleted by the @@unscopables
            // getter. A false result throws in strict mode and otherwise yields
            // undefined for the loose with-binding.
            if !has_property(object.clone(), &env, name)? {
                self.apply_env(env);
                if is_strict {
                    return Err(RuntimeError {
                        thrown: None,
                        message: format!("ReferenceError: undefined identifier `{name}`"),
                    });
                }
                return Ok(Value::Undefined);
            }
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
            // SetMutableBinding step 1 re-checks HasProperty (observable on a
            // Proxy, and the binding may have been deleted by the @@unscopables
            // getter). A false result throws only in strict mode; the Set in
            // step 3 runs otherwise, recreating the property in sloppy mode.
            if !has_property(object.clone(), &env, name)? && is_strict {
                self.apply_env(env);
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
            return Ok(());
        }
        match slot {
            Some(slot) => self.assign_local(slot, value),
            None if is_strict => self.store_global_strict(name, value),
            None => {
                self.store_global_sloppy(name, value)?;
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

    pub(super) fn load_resolved_ident_with(
        &mut self,
        name: &str,
        slot: Option<usize>,
        object_slot: usize,
        is_strict: bool,
    ) -> Result<Value, RuntimeError> {
        match self.load_local(object_slot)? {
            Value::Undefined => match slot {
                Some(slot) => self.load_local(slot),
                None => self.load_global(name),
            },
            object => {
                let mut env = self.current_env();
                // GetBindingValue re-checks HasProperty (step 2) before the Get;
                // a false result throws in strict mode, else yields undefined.
                if !has_property(object.clone(), &env, name)? {
                    self.apply_env(env);
                    if is_strict {
                        return Err(RuntimeError {
                            thrown: None,
                            message: format!("ReferenceError: undefined identifier `{name}`"),
                        });
                    }
                    return Ok(Value::Undefined);
                }
                let value = get_property(object, name, &mut env)?;
                self.apply_env(env);
                Ok(value)
            }
        }
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
                None if is_strict => self.store_global_strict(name, value),
                None => {
                    self.store_global_sloppy(name, value)?;
                    self.record_sloppy_global_name(name);
                    Ok(())
                }
            },
            object => {
                let mut env = self.current_env();
                // SetMutableBinding step 1: HasProperty always runs (observable
                // on a Proxy); a false result throws only in strict mode, while
                // the Set in step 3 still recreates the property in sloppy mode.
                if !has_property(object.clone(), &env, name)? && is_strict {
                    self.apply_env(env);
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
            return Ok(Value::String(typeof_value(value).into()));
        }
        let value = match slot {
            Some(slot) => self.load_local(slot)?,
            None => {
                if let Some(value) = self.env.module_import_value(name) {
                    if value.is_uninitialized_lexical_marker() {
                        return Err(RuntimeError {
                            thrown: None,
                            message: format!("ReferenceError: undefined identifier `{name}`"),
                        });
                    }
                    value
                } else {
                    self.env.get(name).unwrap_or(Value::Undefined)
                }
            }
        };
        let value = if matches!(
            &value,
            Value::Function(function) if function.is_uninitialized_lexical_marker()
        ) {
            Value::Undefined
        } else {
            value
        };
        Ok(Value::String(typeof_value(value).into()))
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
}
