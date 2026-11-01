//! A selected global environment base, including its original resolvability.
//! Put consults the current declarative/object subrecord of that same base;
//! an originally unresolvable sloppy write targets only the global object.
//! [`set_bound_global`] is the same Put for a reference known to be bound to
//! the selected global environment, which every tier's global store uses.
use crate::{RuntimeError, Value, function::Realm, value::OwnDataPropertyWrite};
use std::rc::Rc;

/// GlobalEnv.SetMutableBinding(N, V, S) (ES2025 9.1.1.4.5) for a reference
/// bound to the selected global environment, decided entirely at store time.
/// A global lexical takes the write, including one another script or host
/// `evalScript` declared after this code was compiled. Otherwise the object
/// record runs HasProperty over the global object's whole prototype chain
/// (Proxy traps included), throws ReferenceError in strict code when the
/// binding no longer exists, and `[[Set]]`s the global object, so an
/// inherited setter runs and an inherited non-writable property refuses the
/// write (TypeError in strict code). Only an own data property of the global
/// object, found before any prototype, skips that walk: HasProperty is then
/// true without observable work and `[[Set]]` is exactly the slot write or,
/// read-only, a refusal. The realm table mirrors the resulting own data
/// property, or drops a mirror the write invalidated.
pub(crate) fn set_bound_global(
    name: &str,
    value: Value,
    strict: bool,
    env: &mut crate::context::Context<'_>,
) -> Result<(), RuntimeError> {
    if env.can_cache_realm_globals()
        && let Some(cell) = env.realm_binding_cell(name)
        && cell.metadata().kind == super::BindingKind::Lexical
    {
        return cell.assign(value, strict);
    }
    let Some(base) = env.selected_global_this() else {
        if strict && !env.realm_contains(name) {
            return Err(missing(name));
        }
        env.insert_realm(name.to_owned(), value);
        return Ok(());
    };
    if let Value::Object(global) = &base {
        match global.write_existing_own_data_property(name, &value) {
            OwnDataPropertyWrite::Written => {
                if env.can_cache_realm_globals() {
                    env.insert_realm(name.to_owned(), value);
                }
                return Ok(());
            }
            OwnDataPropertyWrite::ReadOnly if strict => return Err(read_only(name)),
            OwnDataPropertyWrite::ReadOnly => return Ok(()),
            OwnDataPropertyWrite::NeedsSlowPath => {}
        }
    }
    if !crate::property::has_property(base.clone(), env, name)? && strict {
        return Err(missing(name));
    }
    let written = crate::bytecode::set_object_property(base.clone(), name.to_owned(), value, env)?;
    if !written && strict {
        return Err(read_only(name));
    }
    // A setter may itself have declared a global lexical of this name.
    if env.can_cache_realm_globals()
        && !env.is_global_lexical_binding(name)
        && let Value::Object(global) = base
    {
        match global.own_property(name) {
            Some(property) if !property.is_accessor() => {
                env.insert_realm(name.to_owned(), property.value);
            }
            _ => {
                env.remove_realm(name);
            }
        }
    }
    Ok(())
}

fn missing(name: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("ReferenceError: undefined identifier `{name}`"),
    }
}

fn read_only(name: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("TypeError: Cannot assign to read only property '{name}'"),
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GlobalReference {
    realm: Realm,
    object: Option<Value>,
    name: String,
    strict: bool,
    declarative: bool,
    bound: bool,
}

impl GlobalReference {
    pub(crate) fn select(
        name: &str,
        strict: bool,
        env: &mut crate::context::Context<'_>,
    ) -> Result<Self, RuntimeError> {
        let object = env.selected_global_this();
        let declarative = env.can_cache_realm_globals();
        let lexical = declarative && env.is_global_lexical_binding(name);
        let bound = if lexical {
            true
        } else if let Some(object) = &object {
            crate::property::has_property(object.clone(), env, name)?
        } else {
            env.realm_contains(name)
        };
        Ok(Self {
            realm: env.realm_rc(),
            object,
            name: name.to_owned(),
            strict,
            declarative,
            bound,
        })
    }

    pub(crate) fn is_bound(&self) -> bool {
        self.bound
    }

    fn lexical(&self) -> Option<super::BindingCell> {
        self.declarative
            .then(|| self.realm.cell(&self.name))
            .flatten()
            .filter(|cell| cell.metadata().kind == super::BindingKind::Lexical)
    }

    pub(crate) fn read(
        &self,
        env: &mut crate::context::Context<'_>,
    ) -> Result<Value, RuntimeError> {
        if !self.bound {
            return Err(self.missing());
        }
        if let Some(cell) = self.lexical() {
            return cell.read();
        }
        if let Some(object) = &self.object {
            if !crate::property::has_property(object.clone(), env, &self.name)? {
                return if self.strict {
                    Err(self.missing())
                } else {
                    Ok(Value::Undefined)
                };
            }
            return crate::property_value(object.clone(), &self.name, env);
        }
        self.realm
            .get_value(&self.name)
            .ok_or_else(|| self.missing())
    }

    pub(crate) fn assign(
        &self,
        value: Value,
        env: &mut crate::context::Context<'_>,
    ) -> Result<(), RuntimeError> {
        if !self.bound && self.strict {
            return Err(self.missing());
        }
        if self.bound
            && let Some(cell) = self.lexical()
        {
            return cell.assign(value, self.strict);
        }
        let Some(object) = &self.object else {
            self.realm.insert_value(self.name.clone(), value);
            return Ok(());
        };
        if self.bound
            && !crate::property::has_property(object.clone(), env, &self.name)?
            && self.strict
        {
            return Err(self.missing());
        }
        let written =
            crate::bytecode::set_object_property(object.clone(), self.name.clone(), value, env)?;
        if self.declarative
            && Rc::ptr_eq(env.realm(), &self.realm)
            && let Value::Object(object) = object
        {
            // Accessors stay semantic Get paths; an independent lexical of the
            // same name must never receive an unresolvable property write.
            if !env.is_global_lexical_binding(&self.name) && env.is_realm_global_object(object) {
                match object.own_property(&self.name) {
                    Some(property) if !property.is_accessor() => {
                        env.insert_realm(self.name.clone(), property.value);
                    }
                    _ => {
                        env.remove_realm(&self.name);
                    }
                }
            }
        }
        if !written && self.strict {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot set global binding".into(),
            });
        }
        Ok(())
    }

    pub(crate) fn delete(
        &self,
        env: &mut crate::context::Context<'_>,
    ) -> Result<bool, RuntimeError> {
        if !self.bound {
            return Ok(true);
        }
        if self.lexical().is_some() {
            return Ok(false);
        }
        match &self.object {
            Some(object) => crate::bytecode::delete_object_property(
                object.clone(),
                &crate::PropertyKey::String(self.name.clone()),
                env,
            ),
            None => Ok(self.realm.remove_value(&self.name).is_some()),
        }
    }

    fn missing(&self) -> RuntimeError {
        RuntimeError {
            thrown: None,
            message: format!("ReferenceError: undefined identifier `{}`", self.name),
        }
    }
}
