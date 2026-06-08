use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    function_prototype, is_truthy, property_value, property_value_key, symbol,
    to_js_string_with_env, to_length_with_env,
};

mod escape;
mod formatting;
mod matcher;
mod symbol_search;
mod symbol_split;
mod validation;

pub(crate) use escape::native_regexp_escape;
use formatting::{canonical_regexp_flags, escape_regexp_source};
pub(crate) use symbol_search::native_regexp_prototype_search;
pub(crate) use symbol_split::native_regexp_prototype_split;
use validation::validate_regexp_init;

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
    regexp_function.properties.borrow_mut().insert(
        "escape".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("escape"),
            1,
            NativeFunction::RegExpEscape,
            false,
        ))),
    );
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
        "compile".to_owned(),
        Value::Function(Function::new_native(
            Some("compile"),
            2,
            NativeFunction::RegExpPrototypeCompile,
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
    regexp_prototype.define_non_enumerable(
        "test".to_owned(),
        Value::Function(Function::new_native(
            Some("test"),
            1,
            NativeFunction::RegExpPrototypeTest,
            false,
        )),
    );
    if let Some(symbol) = symbol::search_symbol(env) {
        regexp_prototype.define_symbol_property(
            symbol,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.search]"),
                1,
                NativeFunction::RegExpPrototypeSearch,
                false,
            ))),
        );
    }
    symbol_split::install_regexp_prototype_split(env, &regexp_prototype);
    define_regexp_accessor(
        &regexp_prototype,
        "source",
        NativeFunction::RegExpPrototypeSource,
    );
    define_regexp_accessor(
        &regexp_prototype,
        "flags",
        NativeFunction::RegExpPrototypeFlags,
    );
    define_regexp_accessor(
        &regexp_prototype,
        "global",
        NativeFunction::RegExpPrototypeGlobal,
    );
    define_regexp_accessor(
        &regexp_prototype,
        "dotAll",
        NativeFunction::RegExpPrototypeDotAll,
    );
    define_regexp_accessor(
        &regexp_prototype,
        "ignoreCase",
        NativeFunction::RegExpPrototypeIgnoreCase,
    );
    define_regexp_accessor(
        &regexp_prototype,
        "multiline",
        NativeFunction::RegExpPrototypeMultiline,
    );
    define_regexp_accessor(
        &regexp_prototype,
        "sticky",
        NativeFunction::RegExpPrototypeSticky,
    );
    define_regexp_accessor(
        &regexp_prototype,
        "unicode",
        NativeFunction::RegExpPrototypeUnicode,
    );
    define_regexp_accessor(
        &regexp_prototype,
        "hasIndices",
        NativeFunction::RegExpPrototypeHasIndices,
    );
    regexp_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(regexp_prototype)),
    );

    let regexp_value = Value::Function(regexp_function);
    env.insert("RegExp".to_owned(), regexp_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("RegExp".to_owned(), regexp_value);
    }
}

fn define_regexp_accessor(prototype: &ObjectRef, name: &str, native: NativeFunction) {
    prototype.define_property(
        name.to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some(&format!("get {name}")),
                0,
                native,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
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
    let pattern_is_regexp = regexp_is_regexp_with_env(pattern.clone(), env)?;
    if !is_construct && pattern_is_regexp && matches!(flags_value, Value::Undefined) {
        let pattern_constructor = property_value(pattern.clone(), "constructor", env)?;
        if pattern_constructor.same_value(&Value::Function(function.clone())) {
            return Ok(pattern);
        }
    }

    let source = regexp_source(pattern.clone(), pattern_is_regexp, env)?;
    let flags = regexp_flags(pattern.clone(), pattern_is_regexp, flags_value, env)?;
    validate_regexp_init(&source, &flags)?;

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

pub(crate) fn native_regexp_prototype_compile(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Object(object) = &this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp.prototype.compile requires an object receiver".to_owned(),
        });
    };
    if regexp_string_data(object, REGEXP_SOURCE_PROPERTY).is_none() {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp.prototype.compile requires a RegExp receiver".to_owned(),
        });
    }

    let pattern = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let flags_value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let (source, flags) = match &pattern {
        Value::Object(pattern_object)
            if regexp_string_data(pattern_object, REGEXP_SOURCE_PROPERTY).is_some() =>
        {
            if !matches!(flags_value, Value::Undefined) {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: flags must be undefined".to_owned(),
                });
            }
            (
                regexp_string_data(pattern_object, REGEXP_SOURCE_PROPERTY).unwrap_or_default(),
                regexp_string_data(pattern_object, REGEXP_FLAGS_PROPERTY).unwrap_or_default(),
            )
        }
        _ => {
            let source = regexp_source(pattern, false, env)?;
            let flags = regexp_flags(Value::Undefined, false, flags_value, env)?;
            (source, flags)
        }
    };
    validate_regexp_init(&source, &flags)?;

    define_regexp_data_without_last_index(object, &source, &flags);
    regexp_set_last_index_object(object, 0)?;
    Ok(this_value)
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
    let sticky = regexp_flags_contains(&object, 'y');
    let ignore_case = regexp_flags_contains(&object, 'i');
    let unicode = regexp_flags_contains(&object, 'u');
    let stateful = global || sticky;
    let start = if stateful {
        regexp_last_index(&object, env)?
    } else {
        0
    };

    let match_result = if sticky {
        matcher::regexp_match_at(&source, &input, start, ignore_case, unicode)
    } else {
        matcher::regexp_match_range(&source, &input, start, ignore_case, unicode)
    };

    let Some(match_result) = match_result else {
        if stateful {
            regexp_set_last_index_object(&object, 0)?;
        }
        return Ok(Value::Null);
    };
    if stateful {
        regexp_set_last_index_object(&object, match_result.end)?;
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
        regexp_string_data(&object, REGEXP_SOURCE_PROPERTY)
            .map(|source| escape_regexp_source(&source))
            .unwrap_or_default(),
        regexp_string_data(&object, REGEXP_FLAGS_PROPERTY)
            .map(|flags| canonical_regexp_flags(&flags))
            .unwrap_or_default()
    )))
}

pub(crate) fn native_regexp_prototype_test(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let result = native_regexp_prototype_exec(this_value, argument_values, env)?;
    Ok(Value::Boolean(!matches!(result, Value::Null)))
}

pub(crate) fn native_regexp_prototype_source(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let source = regexp_accessor_data(&this_value, env, REGEXP_SOURCE_PROPERTY, "(?:)")?;
    Ok(Value::String(escape_regexp_source(&source)))
}

pub(crate) fn native_regexp_prototype_flags(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    regexp_accessor_data(&this_value, env, REGEXP_FLAGS_PROPERTY, "")
        .map(|flags| Value::String(canonical_regexp_flags(&flags)))
}

pub(crate) fn native_regexp_prototype_flag(
    this_value: Value,
    env: &HashMap<String, Value>,
    flag: char,
) -> Result<Value, RuntimeError> {
    let flags = regexp_accessor_data(&this_value, env, REGEXP_FLAGS_PROPERTY, "")?;
    if flags.is_empty() && is_regexp_prototype_value(&this_value, env) {
        return Ok(Value::Undefined);
    }
    Ok(Value::Boolean(flags.contains(flag)))
}

fn regexp_accessor_data(
    this_value: &Value,
    env: &HashMap<String, Value>,
    key: &str,
    prototype_value: &str,
) -> Result<String, RuntimeError> {
    let Value::Object(object) = &this_value else {
        return Err(regexp_receiver_error());
    };
    if let Some(value) = regexp_string_data(object, key) {
        return Ok(value);
    }
    if is_regexp_prototype_value(this_value, env) {
        return Ok(prototype_value.to_owned());
    }
    Err(regexp_receiver_error())
}

fn is_regexp_prototype_value(value: &Value, env: &HashMap<String, Value>) -> bool {
    let Value::Object(object) = value else {
        return false;
    };
    env.get("RegExp")
        .and_then(|constructor| match constructor {
            Value::Function(function) => function_prototype(function),
            _ => None,
        })
        .is_some_and(|prototype| object.ptr_eq(&prototype))
}

fn regexp_receiver_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: RegExp prototype accessor requires a RegExp receiver".to_owned(),
    }
}

fn define_regexp_data(object: &ObjectRef, source: &str, flags: &str) {
    define_regexp_data_without_last_index(object, source, flags);
    object.define_non_enumerable("lastIndex".to_owned(), Value::Number(0.0));
}

fn define_regexp_data_without_last_index(object: &ObjectRef, source: &str, flags: &str) {
    object.define_non_enumerable(
        REGEXP_SOURCE_PROPERTY.to_owned(),
        Value::String(source.to_owned()),
    );
    object.define_non_enumerable(
        REGEXP_FLAGS_PROPERTY.to_owned(),
        Value::String(flags.to_owned()),
    );
}

fn regexp_source(
    pattern: Value,
    pattern_is_regexp: bool,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    if pattern_is_regexp {
        return to_js_string_with_env(property_value(pattern, "source", env)?, env);
    }
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
    pattern_is_regexp: bool,
    flags_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    match flags_value {
        Value::Undefined if pattern_is_regexp => {
            to_js_string_with_env(property_value(pattern, "flags", env)?, env)
        }
        Value::Undefined => Ok(String::new()),
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

pub(crate) fn regexp_is_regexp_with_env(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    let is_object = matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    );
    if !is_object {
        return Ok(false);
    }
    if let Some(symbol) = symbol::match_symbol(env) {
        let matcher = property_value_key(value.clone(), &PropertyKey::Symbol(symbol), env)?;
        if !matches!(matcher, Value::Undefined) {
            return Ok(is_truthy(&matcher));
        }
    }
    Ok(regexp_is_regexp(&value))
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

fn regexp_set_last_index_object(object: &ObjectRef, index: usize) -> Result<(), RuntimeError> {
    if object
        .own_property("lastIndex")
        .is_some_and(|property| property.is_accessor() || !property.writable)
        || !object.has_own_property("lastIndex") && !object.is_extensible()
    {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp.prototype.exec cannot set lastIndex".to_owned(),
        });
    }
    object.set("lastIndex".to_owned(), Value::Number(index as f64));
    Ok(())
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
