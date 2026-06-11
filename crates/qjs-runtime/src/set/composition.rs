use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    MapRef, RuntimeError, SetRef, Value, array::array_like_values_with_env, call_function,
    is_truthy, property_value, to_number_with_env,
};

pub(super) enum SetRecord {
    Set(SetRef),
    Map(MapRef),
    SetLike {
        object: Value,
        size: f64,
        has: Box<Value>,
        keys: Box<Value>,
    },
}

impl SetRecord {
    pub(super) fn from_arguments(
        argument_values: &[Value],
        env: &mut CallEnv,
    ) -> Result<Self, RuntimeError> {
        match argument_values.first().cloned().unwrap_or(Value::Undefined) {
            Value::Set(set) => Ok(Self::Set(set)),
            Value::Map(map) => Ok(Self::Map(map)),
            Value::Object(object) => Self::from_set_like_object(Value::Object(object), env),
            Value::Array(array) => Self::from_set_like_object(Value::Array(array), env),
            value => Err(RuntimeError {
                thrown: None,
                message: format!("TypeError: Set composition argument must be set-like: {value:?}"),
            }),
        }
    }

    pub(super) fn has(&self, value: &Value, env: &mut CallEnv) -> Result<bool, RuntimeError> {
        match self {
            Self::Set(set) => Ok(set.has(value)),
            Self::Map(map) => Ok(map.has(value)),
            Self::SetLike { object, has, .. } => {
                let result = call_function(
                    (**has).clone(),
                    object.clone(),
                    vec![value.clone()],
                    env,
                    false,
                )?;
                Ok(is_truthy(&result))
            }
        }
    }

    pub(super) fn keys(&self, env: &mut CallEnv) -> Result<Vec<Value>, RuntimeError> {
        match self {
            Self::Set(set) => Ok(set.values()),
            Self::Map(map) => Ok(map.entries().into_iter().map(|(key, _)| key).collect()),
            Self::SetLike { object, keys, .. } => {
                let values =
                    call_function((**keys).clone(), object.clone(), Vec::new(), env, false)?;
                iterator_values(values, "Set-like keys", env)
            }
        }
    }

    pub(super) fn has_any_in_set(
        &self,
        set: &SetRef,
        env: &mut CallEnv,
    ) -> Result<bool, RuntimeError> {
        match self {
            Self::Set(other) => Ok(other.values().into_iter().any(|value| set.has(&value))),
            Self::Map(map) => Ok(map.entries().into_iter().any(|(key, _)| set.has(&key))),
            Self::SetLike { object, keys, .. } => {
                let values =
                    call_function((**keys).clone(), object.clone(), Vec::new(), env, false)?;
                iterator_has_value_in_set(values, set, env)
            }
        }
    }

    pub(super) fn all_in_set(&self, set: &SetRef, env: &mut CallEnv) -> Result<bool, RuntimeError> {
        match self {
            Self::Set(other) => Ok(other.values().into_iter().all(|value| set.has(&value))),
            Self::Map(map) => Ok(map.entries().into_iter().all(|(key, _)| set.has(&key))),
            Self::SetLike { object, keys, .. } => {
                let values =
                    call_function((**keys).clone(), object.clone(), Vec::new(), env, false)?;
                iterator_all_values_in_set(values, set, env)
            }
        }
    }

    pub(super) fn size(&self) -> f64 {
        match self {
            Self::Set(set) => set.len() as f64,
            Self::Map(map) => map.len() as f64,
            Self::SetLike { size, .. } => *size,
        }
    }

    fn from_set_like_object(object: Value, env: &mut CallEnv) -> Result<Self, RuntimeError> {
        let size = set_record_size(property_value(object.clone(), "size", env)?, env)?;
        let has = property_value(object.clone(), "has", env)?;
        if !matches!(has, Value::Function(_)) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Set-like has must be callable".to_owned(),
            });
        }
        let keys = property_value(object.clone(), "keys", env)?;
        if !matches!(keys, Value::Function(_)) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Set-like keys must be callable".to_owned(),
            });
        }
        Ok(Self::SetLike {
            object,
            size,
            has: Box::new(has),
            keys: Box::new(keys),
        })
    }
}

fn iterator_all_values_in_set(
    value: Value,
    set: &SetRef,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    if !matches!(value, Value::Object(_)) {
        return Ok(array_like_values_with_env(value, "Set-like keys", env)?
            .into_iter()
            .all(|value| set.has(&value)));
    }
    let next = property_value(value.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return Ok(array_like_values_with_env(value, "Set-like keys", env)?
            .into_iter()
            .all(|value| set.has(&value)));
    }

    loop {
        let step = call_function(next.clone(), value.clone(), Vec::new(), env, false)?;
        let done = is_truthy(&property_value(step.clone(), "done", env)?);
        if done {
            return Ok(true);
        }
        if !set.has(&property_value(step, "value", env)?) {
            close_iterator(value, env)?;
            return Ok(false);
        }
    }
}

fn set_record_size(value: Value, env: &mut CallEnv) -> Result<f64, RuntimeError> {
    let size = to_number_with_env(value, env)?;
    if size.is_nan() {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Set-like size must not be NaN".to_owned(),
        });
    }
    let integer_size = if size.is_finite() { size.trunc() } else { size };
    if integer_size < 0.0 {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: Set-like size must be non-negative".to_owned(),
        });
    }
    Ok(integer_size)
}

fn iterator_has_value_in_set(
    value: Value,
    set: &SetRef,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    if !matches!(value, Value::Object(_)) {
        return Ok(array_like_values_with_env(value, "Set-like keys", env)?
            .into_iter()
            .any(|value| set.has(&value)));
    }
    let next = property_value(value.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return Ok(array_like_values_with_env(value, "Set-like keys", env)?
            .into_iter()
            .any(|value| set.has(&value)));
    }

    loop {
        let step = call_function(next.clone(), value.clone(), Vec::new(), env, false)?;
        let done = is_truthy(&property_value(step.clone(), "done", env)?);
        if done {
            return Ok(false);
        }
        if set.has(&property_value(step, "value", env)?) {
            close_iterator(value, env)?;
            return Ok(true);
        }
    }
}

fn close_iterator(iterator: Value, env: &mut CallEnv) -> Result<(), RuntimeError> {
    let return_method = property_value(iterator.clone(), "return", env)?;
    match return_method {
        Value::Undefined => Ok(()),
        Value::Function(_) => {
            let _ = call_function(return_method, iterator, Vec::new(), env, false)?;
            Ok(())
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: iterator return must be callable".to_owned(),
        }),
    }
}

fn iterator_values(
    value: Value,
    context: &str,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    if !matches!(value, Value::Object(_)) {
        return array_like_values_with_env(value, context, env);
    }
    let next = property_value(value.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return array_like_values_with_env(value, context, env);
    }

    let mut values = Vec::new();
    loop {
        let step = call_function(next.clone(), value.clone(), Vec::new(), env, false)?;
        let done = is_truthy(&property_value(step.clone(), "done", env)?);
        if done {
            return Ok(values);
        }
        values.push(property_value(step, "value", env)?);
    }
}
