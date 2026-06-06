use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    call_function, property_value, reflect, symbol, to_js_string_with_env, to_length_with_env,
    to_number_with_env, to_uint32, to_uint32_number,
};

pub(crate) fn install_regexp_prototype_split(env: &HashMap<String, Value>, prototype: &ObjectRef) {
    if let Some(symbol) = symbol::split_symbol(env) {
        prototype.define_symbol_property(
            symbol,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.split]"),
                2,
                NativeFunction::RegExpPrototypeSplit,
                false,
            ))),
        );
    }
}

pub(crate) fn native_regexp_prototype_split(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_object_value(&this_value) {
        return Err(RuntimeError {
            thrown: None,
            message: "RegExp.prototype[Symbol.split] requires an object receiver".to_owned(),
        });
    }

    let input = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let splitter = split_regexp_clone(this_value, env)?;
    let limit = split_limit(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let mut parts = Vec::new();
    if limit == 0 {
        return Ok(Value::Array(ArrayRef::new(parts)));
    }

    let input_len = input.chars().count();
    if input_len == 0 {
        set_last_index(splitter.clone(), Value::Number(0.0), env)?;
        let result = regexp_exec(splitter, &input, env)?;
        if matches!(result, Value::Null) {
            parts.push(Value::String(String::new()));
        } else {
            ensure_exec_result_object(result)?;
        }
        return Ok(Value::Array(ArrayRef::new(parts)));
    }

    let mut segment_start = 0usize;
    let mut search_index = 0usize;
    while search_index < input_len {
        set_last_index(splitter.clone(), Value::Number(search_index as f64), env)?;
        let result = regexp_exec(splitter.clone(), &input, env)?;
        if matches!(result, Value::Null) {
            search_index += 1;
            continue;
        }

        let match_result = ensure_exec_result_object(result)?;
        let match_end = regexp_last_index(splitter.clone(), env)?.min(input_len);
        if match_end == segment_start {
            search_index += 1;
            continue;
        }

        parts.push(Value::String(input_slice(
            &input,
            segment_start,
            search_index,
        )));
        if parts.len() == limit {
            return Ok(Value::Array(ArrayRef::new(parts)));
        }

        append_captures(match_result, &mut parts, limit, env)?;
        if parts.len() == limit {
            return Ok(Value::Array(ArrayRef::new(parts)));
        }

        segment_start = match_end;
        search_index = match_end;
    }

    parts.push(Value::String(input_slice(&input, segment_start, input_len)));
    Ok(Value::Array(ArrayRef::new(parts)))
}

fn split_regexp_clone(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    validate_species_constructor(value.clone(), env)?;
    let flags = to_js_string_with_env(property_value(value.clone(), "flags", env)?, env)?;
    let mut split_flags = flags;
    if !split_flags.contains('y') {
        split_flags.push('y');
    }
    let constructor = env.get("RegExp").cloned().ok_or_else(|| RuntimeError {
        thrown: None,
        message: "RegExp constructor is not available".to_owned(),
    })?;
    call_function(
        constructor,
        Value::Undefined,
        vec![value, Value::String(split_flags)],
        env,
        false,
    )
}

fn validate_species_constructor(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let constructor = property_value(value, "constructor", env)?;
    if matches!(constructor, Value::Undefined) || is_object_value(&constructor) {
        return Ok(());
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: RegExp species constructor must be an object".to_owned(),
    })
}

fn regexp_exec(
    splitter: Value,
    input: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let exec = property_value(splitter.clone(), "exec", env)?;
    call_function(
        exec,
        splitter,
        vec![Value::String(input.to_owned())],
        env,
        false,
    )
}

fn append_captures(
    result: Value,
    parts: &mut Vec<Value>,
    limit: usize,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let length = to_length_with_env(property_value(result.clone(), "length", env)?, env)?;
    for index in 1..length {
        parts.push(property_value(result.clone(), &index.to_string(), env)?);
        if parts.len() == limit {
            break;
        }
    }
    Ok(())
}

fn ensure_exec_result_object(value: Value) -> Result<Value, RuntimeError> {
    if is_object_value(&value) {
        Ok(value)
    } else {
        Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp exec must return an object or null".to_owned(),
        })
    }
}

fn regexp_last_index(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    to_length_with_env(property_value(value, "lastIndex", env)?, env)
}

fn split_limit(value: Value, env: &mut HashMap<String, Value>) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(u32::MAX as usize);
    }
    match value {
        Value::Object(_) | Value::Function(_) | Value::Array(_) => {
            Ok(to_uint32_number(to_number_with_env(value, env)?) as usize)
        }
        value => Ok(to_uint32(value)? as usize),
    }
}

fn set_last_index(
    receiver: Value,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    if reflect::ordinary_set(
        receiver.clone(),
        &PropertyKey::String("lastIndex".to_owned()),
        value,
        receiver,
        env,
    )? {
        Ok(())
    } else {
        Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp.prototype[Symbol.split] cannot set lastIndex".to_owned(),
        })
    }
}

fn input_slice(input: &str, start: usize, end: usize) -> String {
    input.chars().skip(start).take(end - start).collect()
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(object) if !symbol::is_symbol_primitive(object)
    ) || matches!(
        value,
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}
