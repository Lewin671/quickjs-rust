use crate::{ArrayRef, ObjectRef, RuntimeError, Value, call_function, number};

use super::raw_json_value;
use crate::CallEnv;

pub(crate) fn native_json_stringify(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let replacer = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let property_list = build_property_list(&replacer, env)?;
    let replacer_fn = if matches!(&replacer, Value::Function(_)) {
        Some(replacer)
    } else {
        None
    };
    let ctx = StringifyContext {
        property_list,
        replacer_fn,
    };
    match stringify_value_ctx(value, "", false, &ctx, env)? {
        Some(json) => Ok(Value::String(json)),
        None => Ok(Value::Undefined),
    }
}

struct StringifyContext {
    property_list: Option<Vec<String>>,
    replacer_fn: Option<Value>,
}

fn build_property_list(
    replacer: &Value,
    _env: &mut CallEnv,
) -> Result<Option<Vec<String>>, RuntimeError> {
    let Value::Array(elements) = replacer else {
        return Ok(None);
    };
    let mut list = Vec::new();
    for i in 0..elements.len() {
        let item = elements.get(i).unwrap_or(Value::Undefined);
        let key = match item {
            Value::String(s) => s,
            Value::Number(n) => crate::number::number_to_js_string(n),
            _ => continue,
        };
        if !list.contains(&key) {
            list.push(key);
        }
    }
    Ok(Some(list))
}

fn stringify_value_ctx(
    value: Value,
    key: &str,
    in_array: bool,
    ctx: &StringifyContext,
    env: &mut CallEnv,
) -> Result<Option<String>, RuntimeError> {
    let value = apply_to_json(value, key, env)?;
    let value = if let Some(ref replacer) = ctx.replacer_fn {
        let holder = Value::Object(ObjectRef::new(std::collections::HashMap::new()));
        call_function(
            replacer.clone(),
            holder,
            vec![Value::String(key.to_owned()), value],
            env,
            false,
        )?
    } else {
        value
    };
    match &value {
        Value::String(value) => Ok(Some(quote_json_string(value))),
        Value::Number(number) if number.is_finite() => {
            Ok(Some(number::number_to_js_string(*number)))
        }
        Value::Number(_) | Value::Null => Ok(Some("null".to_owned())),
        Value::BigInt(_) => Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot serialize BigInt".to_owned(),
        }),
        Value::Boolean(true) => Ok(Some("true".to_owned())),
        Value::Boolean(false) => Ok(Some("false".to_owned())),
        Value::Array(array) => stringify_array_ctx(array, ctx, env).map(Some),
        Value::Object(object) => {
            if let Some(raw_json) = raw_json_value(object) {
                Ok(Some(raw_json))
            } else {
                stringify_object_ctx(object, ctx, env).map(Some)
            }
        }
        Value::Map(map) => stringify_object_ctx(&map.object(), ctx, env).map(Some),
        Value::Set(set) => stringify_object_ctx(&set.object(), ctx, env).map(Some),
        Value::Proxy(proxy) => stringify_value_ctx(proxy.target(), key, in_array, ctx, env),
        Value::Undefined | Value::Function(_) if in_array => Ok(Some("null".to_owned())),
        Value::Undefined | Value::Function(_) => Ok(None),
    }
}

fn stringify_array_ctx(
    array: &ArrayRef,
    ctx: &StringifyContext,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
    let mut parts = Vec::new();
    for (index, element) in array.to_vec().into_iter().enumerate() {
        parts.push(
            stringify_value_ctx(element, &index.to_string(), true, ctx, env)?
                .unwrap_or_else(|| "null".to_owned()),
        );
    }
    Ok(format!("[{}]", parts.join(",")))
}

fn stringify_object_ctx(
    object: &ObjectRef,
    ctx: &StringifyContext,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
    let keys: Vec<String> = if let Some(ref list) = ctx.property_list {
        list.clone()
    } else {
        object.own_property_keys()
    };
    let mut parts = Vec::new();
    for key in keys {
        let Some(value) = object.own_property(&key).map(|property| property.value) else {
            continue;
        };
        let Some(json) = stringify_value_ctx(value, &key, false, ctx, env)? else {
            continue;
        };
        parts.push(format!("{}:{json}", quote_json_string(&key)));
    }
    Ok(format!("{{{}}}", parts.join(",")))
}

fn apply_to_json(value: Value, key: &str, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let Value::Object(object) = &value else {
        return Ok(value);
    };
    let Some(to_json) = object.get("toJSON") else {
        return Ok(value);
    };
    call_function(
        to_json,
        value,
        vec![Value::String(key.to_owned())],
        env,
        false,
    )
}

fn quote_json_string(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch <= '\u{1f}' => output.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}
