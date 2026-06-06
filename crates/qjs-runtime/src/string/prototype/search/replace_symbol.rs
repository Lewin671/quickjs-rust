use std::collections::HashMap;

use crate::{PropertyKey, RuntimeError, Value, has_property_key, property_value_key, symbol};

pub(super) struct SymbolReplaceMethod {
    pub(super) present: bool,
    pub(super) method: Option<Value>,
}

pub(super) fn symbol_replace_method(
    search_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<SymbolReplaceMethod, RuntimeError> {
    let absent = SymbolReplaceMethod {
        present: false,
        method: None,
    };
    if !is_object_value(&search_value) {
        return Ok(absent);
    }
    let Some(symbol) = symbol::replace_symbol(env) else {
        return Ok(absent);
    };
    let key = PropertyKey::Symbol(symbol);
    if !has_property_key(search_value.clone(), env, &key)? {
        return Ok(absent);
    }
    let method = property_value_key(search_value, &key, env)?;
    if matches!(method, Value::Null | Value::Undefined) {
        return Ok(SymbolReplaceMethod {
            present: true,
            method: None,
        });
    }
    if !matches!(method, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Symbol.replace method is not callable".to_owned(),
        });
    }
    Ok(SymbolReplaceMethod {
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
