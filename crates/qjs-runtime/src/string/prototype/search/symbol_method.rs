use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    ObjectRef, PropertyKey, RuntimeError, Value, has_property_key, property_value_key, symbol,
};

pub(super) struct SymbolMethod {
    pub(super) present: bool,
    pub(super) method: Option<Value>,
}

pub(super) fn symbol_replace_method(
    search_value: Value,
    env: &mut CallEnv,
) -> Result<SymbolMethod, RuntimeError> {
    symbol_method(
        search_value,
        symbol::replace_symbol(env),
        "Symbol.replace",
        env,
    )
}

pub(super) fn symbol_match_method(
    value: Value,
    env: &mut CallEnv,
) -> Result<SymbolMethod, RuntimeError> {
    symbol_method(value, symbol::match_symbol(env), "Symbol.match", env)
}

pub(super) fn symbol_match_all_method(
    value: Value,
    env: &mut CallEnv,
) -> Result<SymbolMethod, RuntimeError> {
    symbol_method(value, symbol::match_all_symbol(env), "Symbol.matchAll", env)
}

pub(super) fn symbol_search_method(
    value: Value,
    env: &mut CallEnv,
) -> Result<SymbolMethod, RuntimeError> {
    symbol_method(value, symbol::search_symbol(env), "Symbol.search", env)
}

fn symbol_method(
    value: Value,
    symbol: Option<ObjectRef>,
    name: &str,
    env: &mut CallEnv,
) -> Result<SymbolMethod, RuntimeError> {
    let absent = SymbolMethod {
        present: false,
        method: None,
    };
    if !is_object_value(&value) {
        return Ok(absent);
    }
    let Some(symbol) = symbol else {
        return Ok(absent);
    };
    let key = PropertyKey::Symbol(symbol);
    if !has_property_key(value.clone(), env, &key)? {
        return Ok(absent);
    }
    let method = property_value_key(value, &key, env)?;
    if matches!(method, Value::Null | Value::Undefined) {
        return Ok(SymbolMethod {
            present: true,
            method: None,
        });
    }
    if !matches!(method, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: {name} method is not callable"),
        });
    }
    Ok(SymbolMethod {
        present: true,
        method: Some(method),
    })
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}
