//! A selected global environment base, including its original resolvability.
//! Put consults the current declarative/object subrecord of that same base;
//! an originally unresolvable sloppy write targets only the global object.
use crate::{
    RuntimeError, Value,
    function::{CallEnv, Realm},
};
use std::rc::Rc;

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
    pub(crate) fn select(name: &str, strict: bool, env: &CallEnv) -> Result<Self, RuntimeError> {
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

    pub(crate) fn read(&self, env: &mut CallEnv) -> Result<Value, RuntimeError> {
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

    pub(crate) fn assign(&self, value: Value, env: &mut CallEnv) -> Result<(), RuntimeError> {
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

    pub(crate) fn delete(&self, env: &mut CallEnv) -> Result<bool, RuntimeError> {
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
