use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, Property, RuntimeError, Value,
    function_prototype, to_js_string_with_env, to_length_with_env,
};

mod matcher;

const REGEXP_SOURCE_PROPERTY: &str = "\0RegExpSource";
const REGEXP_FLAGS_PROPERTY: &str = "\0RegExpFlags";

pub(crate) fn install_regexp(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let regexp_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    regexp_prototype.set_to_string_tag("RegExp");

    let regexp_function = Function::new_native(Some("RegExp"), 2, NativeFunction::RegExp, true);
    regexp_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(regexp_function.clone()),
    );
    regexp_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::RegExpPrototypeToString,
            false,
        )),
    );
    regexp_prototype.define_non_enumerable(
        "exec".to_owned(),
        Value::Function(Function::new_native(
            Some("exec"),
            1,
            NativeFunction::RegExpPrototypeExec,
            false,
        )),
    );
    regexp_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(regexp_prototype)),
    );

    let regexp_value = Value::Function(regexp_function);
    env.insert("RegExp".to_owned(), regexp_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("RegExp".to_owned(), regexp_value);
    }
}

pub(crate) fn native_regexp(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let pattern = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let flags_value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let source = regexp_source(pattern.clone(), env)?;
    let flags = regexp_flags(pattern.clone(), flags_value, env)?;

    if !is_construct {
        let object = ObjectRef::with_prototype(HashMap::new(), function_prototype(function));
        define_regexp_data(&object, &source, &flags);
        return Ok(Value::Object(object));
    }

    let Value::Object(object) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "RegExp constructor requires an object receiver".to_owned(),
        });
    };
    define_regexp_data(&object, &source, &flags);
    Ok(Value::Object(object))
}

pub(crate) fn native_regexp_prototype_exec(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "RegExp.prototype.exec requires an object receiver".to_owned(),
        });
    };
    let source =
        regexp_string_data(&object, REGEXP_SOURCE_PROPERTY).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "RegExp.prototype.exec requires a RegExp receiver".to_owned(),
        })?;
    let input = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let global = regexp_flags_contains(&object, 'g');
    let ignore_case = regexp_flags_contains(&object, 'i');
    let start = if global {
        regexp_last_index(&object, env)?
    } else {
        0
    };

    let Some(match_result) = matcher::regexp_match_range(&source, &input, start, ignore_case)
    else {
        if global {
            object.set("lastIndex".to_owned(), Value::Number(0.0));
        }
        return Ok(Value::Null);
    };
    if global {
        object.set(
            "lastIndex".to_owned(),
            Value::Number(match_result.end as f64),
        );
    }
    Ok(regexp_match_array(&input, match_result))
}

pub(crate) fn native_regexp_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "RegExp.prototype.toString requires an object receiver".to_owned(),
        });
    };
    Ok(Value::String(format!(
        "/{}/{}",
        regexp_string_data(&object, REGEXP_SOURCE_PROPERTY).unwrap_or_default(),
        regexp_string_data(&object, REGEXP_FLAGS_PROPERTY).unwrap_or_default()
    )))
}

fn define_regexp_data(object: &ObjectRef, source: &str, flags: &str) {
    object.define_non_enumerable(
        REGEXP_SOURCE_PROPERTY.to_owned(),
        Value::String(source.to_owned()),
    );
    object.define_non_enumerable(
        REGEXP_FLAGS_PROPERTY.to_owned(),
        Value::String(flags.to_owned()),
    );
    object.define_non_enumerable("lastIndex".to_owned(), Value::Number(0.0));
}

fn regexp_source(pattern: Value, env: &mut HashMap<String, Value>) -> Result<String, RuntimeError> {
    match pattern {
        Value::Undefined => Ok("(?:)".to_owned()),
        Value::Object(object) => {
            if let Some(source) = regexp_string_data(&object, REGEXP_SOURCE_PROPERTY) {
                Ok(source)
            } else {
                to_js_string_with_env(Value::Object(object), env)
            }
        }
        value => to_js_string_with_env(value, env),
    }
}

fn regexp_flags(
    pattern: Value,
    flags_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    match flags_value {
        Value::Undefined => match pattern {
            Value::Object(object) => {
                Ok(regexp_string_data(&object, REGEXP_FLAGS_PROPERTY).unwrap_or_default())
            }
            _ => Ok(String::new()),
        },
        value => to_js_string_with_env(value, env),
    }
}

fn regexp_string_data(object: &ObjectRef, key: &str) -> Option<String> {
    match object.own_property(key) {
        Some(Property {
            value: Value::String(value),
            ..
        }) => Some(value),
        _ => None,
    }
}

pub(crate) fn regexp_is_global(value: &Value) -> bool {
    let Value::Object(object) = value else {
        return false;
    };
    regexp_flags_contains(object, 'g')
}

pub(crate) fn regexp_is_regexp(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(object) if regexp_string_data(object, REGEXP_SOURCE_PROPERTY).is_some()
    )
}

pub(crate) fn regexp_set_last_index(value: &Value, index: usize) {
    if let Value::Object(object) = value {
        if regexp_string_data(object, REGEXP_SOURCE_PROPERTY).is_some() {
            object.set("lastIndex".to_owned(), Value::Number(index as f64));
        }
    }
}

pub(crate) fn native_regexp_global_match(
    regexp: Value,
    input: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    regexp_set_last_index(&regexp, 0);
    let mut matches = Vec::new();
    loop {
        let result =
            native_regexp_prototype_exec(regexp.clone(), &[Value::String(input.to_owned())], env)?;
        let Value::Array(array) = result else {
            break;
        };
        let Some(Value::String(matched)) = array.get(0) else {
            break;
        };
        let empty = matched.is_empty();
        matches.push(Value::String(matched));
        if empty {
            let next = regexp_last_index_value(&regexp, env)?.saturating_add(1);
            regexp_set_last_index(&regexp, next);
        }
    }

    if matches.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(Value::Array(ArrayRef::new(matches)))
    }
}

fn regexp_flags_contains(object: &ObjectRef, flag: char) -> bool {
    regexp_string_data(object, REGEXP_FLAGS_PROPERTY).is_some_and(|flags| flags.contains(flag))
}

fn regexp_last_index(
    object: &ObjectRef,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    to_length_with_env(object.get("lastIndex").unwrap_or(Value::Undefined), env)
}

fn regexp_last_index_value(
    value: &Value,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let Value::Object(object) = value else {
        return Ok(0);
    };
    regexp_last_index(object, env)
}

fn regexp_match_array(input: &str, match_result: matcher::RegexpMatch) -> Value {
    let mut values = Vec::with_capacity(1 + match_result.captures.len());
    values.push(Value::String(input_slice(
        input,
        match_result.start,
        match_result.end,
    )));
    values.extend(match_result.captures.into_iter().map(|capture| {
        capture
            .map(|(start, end)| Value::String(input_slice(input, start, end)))
            .unwrap_or(Value::Undefined)
    }));
    let result = ArrayRef::new(values);
    result.set_property("index".to_owned(), Value::Number(match_result.start as f64));
    result.set_property("input".to_owned(), Value::String(input.to_owned()));
    Value::Array(result)
}

fn input_slice(input: &str, start: usize, end: usize) -> String {
    input.chars().skip(start).take(end - start).collect()
}
