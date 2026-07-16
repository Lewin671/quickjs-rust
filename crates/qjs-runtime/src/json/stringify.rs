use std::collections::HashMap;

use crate::{
    ObjectRef, Property, PropertyKey, RuntimeError, Value, call_function, number, property_value,
    string, to_js_string_with_env, to_number_with_env,
};

use super::raw_json_value;
use crate::CallEnv;

pub(crate) fn native_json_stringify(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let replacer = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let space = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let property_list = build_property_list(replacer.clone(), env)?;
    let replacer_fn = if is_callable(&replacer) {
        Some(replacer)
    } else {
        None
    };
    let ctx = StringifyContext {
        property_list,
        replacer_fn,
        gap: stringify_gap(space, env)?,
        stack: Vec::new(),
        indent: String::new(),
    };
    let wrapper = ObjectRef::with_prototype(
        HashMap::from([("".to_owned(), value)]),
        crate::object_prototype(env),
    );
    let mut state = ctx;
    match serialize_json_property("", Value::Object(wrapper), &mut state, env)? {
        Some(json) => Ok(Value::String(json.into())),
        None => Ok(Value::Undefined),
    }
}

struct StringifyContext {
    property_list: Option<Vec<String>>,
    replacer_fn: Option<Value>,
    gap: String,
    stack: Vec<Value>,
    indent: String,
}

fn build_property_list(
    replacer: Value,
    env: &mut CallEnv,
) -> Result<Option<Vec<String>>, RuntimeError> {
    if is_callable(&replacer) || !is_array_like(&replacer, env)? {
        return Ok(None);
    }

    let length = crate::to_length_with_env(property_value(replacer.clone(), "length", env)?, env)?;
    let mut list = Vec::new();
    for index in 0..length {
        let item = property_value(replacer.clone(), &index.to_string(), env)?;
        let key = match item {
            Value::String(value) => Some(value.to_string()),
            Value::Number(value) => Some(number::number_to_js_string(value)),
            Value::Object(object)
                if wrapper_string_value(&object).is_some()
                    || wrapper_number_value(&object).is_some() =>
            {
                Some(to_js_string_with_env(Value::Object(object), env)?)
            }
            _ => None,
        };
        if let Some(key) = key
            && !list.contains(&key)
        {
            list.push(key);
        }
    }
    Ok(Some(list))
}

fn stringify_gap(space: Value, env: &mut CallEnv) -> Result<String, RuntimeError> {
    match space {
        Value::Object(object) if wrapper_number_value(&object).is_some() => {
            let number = to_number_with_env(Value::Object(object), env)?;
            number_gap(number)
        }
        Value::Object(object) if wrapper_string_value(&object).is_some() => {
            let value = to_js_string_with_env(Value::Object(object), env)?;
            Ok(truncate_string_code_units(&value, 10))
        }
        Value::Number(number) => number_gap(number),
        Value::String(value) => Ok(truncate_string_code_units(&value, 10)),
        _ => Ok(String::new()),
    }
}

fn number_gap(number: f64) -> Result<String, RuntimeError> {
    if number.is_nan() || number <= 0.0 {
        return Ok(String::new());
    }
    let count = if number.is_infinite() {
        10
    } else {
        number.trunc().min(10.0) as usize
    };
    Ok(" ".repeat(count))
}

fn serialize_json_property(
    key: &str,
    holder: Value,
    ctx: &mut StringifyContext,
    env: &mut CallEnv,
) -> Result<Option<String>, RuntimeError> {
    let mut value = property_value(holder.clone(), key, env)?;
    if matches!(
        value,
        Value::Array(_)
            | Value::Object(_)
            | Value::Function(_)
            | Value::Proxy(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::BigInt(_)
    ) {
        let to_json = property_value(value.clone(), "toJSON", env)?;
        if is_callable(&to_json) {
            value = call_function(
                to_json,
                value,
                vec![Value::String(key.to_owned().into())],
                env,
                false,
            )?;
        }
    }
    if let Some(replacer) = &ctx.replacer_fn {
        value = call_function(
            replacer.clone(),
            holder,
            vec![Value::String(key.to_owned().into()), value],
            env,
            false,
        )?;
    }

    value = unbox_json_wrapper(value, env)?;
    match value {
        Value::String(value) => Ok(Some(quote_json_string(&value))),
        Value::Number(value) if value.is_finite() => Ok(Some(number::number_to_js_string(value))),
        Value::Number(_) | Value::Null => Ok(Some("null".to_owned())),
        Value::Boolean(true) => Ok(Some("true".to_owned())),
        Value::Boolean(false) => Ok(Some("false".to_owned())),
        Value::BigInt(_) => Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot serialize BigInt".to_owned(),
        }),
        Value::Array(_) | Value::Proxy(_) if is_array_like(&value, env)? => {
            serialize_json_array(value, ctx, env).map(Some)
        }
        Value::Array(_) => serialize_json_object(value, ctx, env).map(Some),
        Value::Object(object) => {
            if crate::symbol::is_symbol_primitive(&object) {
                return Ok(None);
            }
            if let Some(raw_json) = raw_json_value(&object) {
                Ok(Some(raw_json))
            } else {
                serialize_json_object(Value::Object(object), ctx, env).map(Some)
            }
        }
        Value::Map(_) | Value::Set(_) => serialize_json_object(value, ctx, env).map(Some),
        Value::Function(_) | Value::Undefined => Ok(None),
        Value::Proxy(_) => serialize_json_object(value, ctx, env).map(Some),
    }
}

fn serialize_json_array(
    value: Value,
    ctx: &mut StringifyContext,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
    push_stack(value.clone(), ctx)?;
    let old_indent = ctx.indent.clone();
    ctx.indent.push_str(&ctx.gap);
    let length = crate::to_length_with_env(property_value(value.clone(), "length", env)?, env)?;
    let mut parts = Vec::with_capacity(length);
    for index in 0..length {
        parts.push(
            serialize_json_property(&index.to_string(), value.clone(), ctx, env)?
                .unwrap_or_else(|| "null".to_owned()),
        );
    }
    let result = join_json_array(&parts, &ctx.gap, &ctx.indent, &old_indent);
    ctx.indent = old_indent;
    ctx.stack.pop();
    Ok(result)
}

fn serialize_json_object(
    value: Value,
    ctx: &mut StringifyContext,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
    push_stack(value.clone(), ctx)?;
    let keys = if let Some(property_list) = &ctx.property_list {
        property_list.clone()
    } else {
        enumerable_own_string_keys(value.clone(), env)?
    };
    let old_indent = ctx.indent.clone();
    ctx.indent.push_str(&ctx.gap);
    let mut parts = Vec::new();
    for key in keys {
        let Some(json) = serialize_json_property(&key, value.clone(), ctx, env)? else {
            continue;
        };
        let member = if ctx.gap.is_empty() {
            format!("{}:{json}", quote_json_string(&key))
        } else {
            format!("{}: {json}", quote_json_string(&key))
        };
        parts.push(member);
    }
    let result = join_json_object(&parts, &ctx.gap, &ctx.indent, &old_indent);
    ctx.indent = old_indent;
    ctx.stack.pop();
    Ok(result)
}

fn enumerable_own_string_keys(
    value: Value,
    env: &mut CallEnv,
) -> Result<Vec<String>, RuntimeError> {
    let keys = own_property_keys(value.clone(), env)?;
    let mut strings = Vec::new();
    for key in keys {
        let PropertyKey::String(name) = key else {
            continue;
        };
        if own_property_descriptor(value.clone(), &PropertyKey::String(name.clone()), env)?
            .is_some_and(|property| property.enumerable)
        {
            strings.push(name);
        }
    }
    Ok(strings)
}

fn own_property_keys(value: Value, env: &mut CallEnv) -> Result<Vec<PropertyKey>, RuntimeError> {
    match value {
        Value::Proxy(proxy) => crate::proxy::proxy_own_keys(proxy, env),
        Value::Object(_) | Value::Map(_) | Value::Set(_) | Value::Function(_) | Value::Array(_) => {
            Ok(crate::object::own_property_names(value)
                .into_iter()
                .map(PropertyKey::String)
                .collect())
        }
        Value::String(value) => Ok(crate::string::string_own_property_names(&value)
            .into_iter()
            .map(PropertyKey::String)
            .collect()),
        _ => Ok(Vec::new()),
    }
}

fn own_property_descriptor(
    value: Value,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<Option<Property>, RuntimeError> {
    match value {
        Value::Proxy(proxy) => {
            crate::proxy::proxy_get_own_property_descriptor(proxy, key, env, |target, env| {
                crate::object::own_property_descriptor_key(target, key, env)
            })
        }
        value => crate::object::own_property_descriptor_key(value, key, env),
    }
}

fn join_json_array(parts: &[String], gap: &str, indent: &str, old_indent: &str) -> String {
    if parts.is_empty() {
        return "[]".to_owned();
    }
    if gap.is_empty() {
        return format!("[{}]", parts.join(","));
    }
    format!("[\n{}\n{}]", join_indented(parts, indent), old_indent)
}

fn join_json_object(parts: &[String], gap: &str, indent: &str, old_indent: &str) -> String {
    if parts.is_empty() {
        return "{}".to_owned();
    }
    if gap.is_empty() {
        return format!("{{{}}}", parts.join(","));
    }
    format!("{{\n{}\n{}}}", join_indented(parts, indent), old_indent)
}

fn join_indented(parts: &[String], indent: &str) -> String {
    parts
        .iter()
        .map(|part| format!("{indent}{part}"))
        .collect::<Vec<_>>()
        .join(",\n")
}

fn push_stack(value: Value, ctx: &mut StringifyContext) -> Result<(), RuntimeError> {
    if ctx.stack.iter().any(|seen| same_json_object(seen, &value)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: cyclic object value".to_owned(),
        });
    }
    ctx.stack.push(value);
    Ok(())
}

fn same_json_object(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Array(left), Value::Array(right)) => left.ptr_eq(right),
        (Value::Object(left), Value::Object(right)) => left.ptr_eq(right),
        (Value::Function(left), Value::Function(right)) => left.ptr_eq(right),
        (Value::Proxy(left), Value::Proxy(right)) => left.ptr_eq(right),
        (Value::Map(left), Value::Map(right)) => left.ptr_eq(right),
        (Value::Set(left), Value::Set(right)) => left.ptr_eq(right),
        _ => false,
    }
}

fn unbox_json_wrapper(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    match value {
        Value::Object(object) if wrapper_number_value(&object).is_some() => Ok(Value::Number(
            to_number_with_env(Value::Object(object), env)?,
        )),
        Value::Object(object) if wrapper_string_value(&object).is_some() => Ok(Value::String(
            to_js_string_with_env(Value::Object(object), env)?.into(),
        )),
        Value::Object(object) => Ok(wrapper_boolean_value(&object)
            .map(Value::Boolean)
            .or_else(|| wrapper_bigint_value(&object).map(Value::bigint))
            .unwrap_or(Value::Object(object))),
        value => Ok(value),
    }
}

fn wrapper_number_value(object: &ObjectRef) -> Option<f64> {
    match object.own_property(crate::number::NUMBER_DATA_PROPERTY) {
        Some(Property {
            value: Value::Number(value),
            ..
        }) => Some(value),
        _ => None,
    }
}

fn wrapper_string_value(object: &ObjectRef) -> Option<String> {
    crate::string_object_value(object)
}

fn wrapper_boolean_value(object: &ObjectRef) -> Option<bool> {
    match object.own_property(crate::boolean::BOOLEAN_DATA_PROPERTY) {
        Some(Property {
            value: Value::Boolean(value),
            ..
        }) => Some(value),
        _ => None,
    }
}

fn wrapper_bigint_value(object: &ObjectRef) -> Option<num_bigint::BigInt> {
    match object.own_property(crate::bigint::BIGINT_DATA_PROPERTY) {
        Some(Property {
            value: Value::BigInt(value),
            ..
        }) => Some(value.as_ref().clone()),
        _ => None,
    }
}

fn is_callable(value: &Value) -> bool {
    match value {
        Value::Function(_) => true,
        Value::Proxy(proxy) => crate::proxy::proxy_is_callable(proxy),
        _ => false,
    }
}

fn is_array_like(value: &Value, env: &mut CallEnv) -> Result<bool, RuntimeError> {
    match value {
        Value::Array(_) => Ok(true),
        Value::Proxy(proxy) => crate::proxy::proxy_target_is_array_result(proxy),
        Value::Object(object) if crate::symbol::is_symbol_primitive(object) => Ok(false),
        Value::Object(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) => Ok(false),
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => {
            let _ = env;
            Ok(false)
        }
    }
}

fn truncate_string_code_units(value: &str, max_units: usize) -> String {
    let units = string::string_code_units(value);
    string::string_from_code_units(&units[..units.len().min(max_units)])
}

fn quote_json_string(value: &str) -> String {
    let mut output = String::from("\"");
    let units = string::string_code_units(value);
    let mut index = 0;
    while index < units.len() {
        let code_unit = units[index];
        match code_unit {
            0x22 => output.push_str("\\\""),
            0x5c => output.push_str("\\\\"),
            0x08 => output.push_str("\\b"),
            0x0c => output.push_str("\\f"),
            0x0a => output.push_str("\\n"),
            0x0d => output.push_str("\\r"),
            0x09 => output.push_str("\\t"),
            0x00..=0x1f => output.push_str(&format!("\\u{code_unit:04x}")),
            0xD800..=0xDBFF if index + 1 < units.len() => {
                let next = units[index + 1];
                if (0xDC00..=0xDFFF).contains(&next) {
                    output.push_str(&string::string_from_code_units(&[code_unit, next]));
                    index += 1;
                } else {
                    output.push_str(&format!("\\u{code_unit:04x}"));
                }
            }
            0xD800..=0xDFFF => output.push_str(&format!("\\u{code_unit:04x}")),
            _ => output.push_str(&string::string_from_code_units(&[code_unit])),
        }
        index += 1;
    }
    output.push('"');
    output
}
