use std::collections::HashMap;

use crate::reflect::ordinary_set;
use crate::string::{string_code_units, string_from_code_units};
use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    function_prototype, is_truthy, property_value, property_value_key, symbol,
    to_js_string_with_env, to_length_with_env,
};

mod escape;
mod formatting;
mod match_all;
mod matcher;
mod symbol_match;
mod symbol_replace;
mod symbol_search;
mod symbol_split;
mod unicode;
mod validation;

use crate::CallEnv;
pub(crate) use escape::native_regexp_escape;
use formatting::{canonical_regexp_flags, escape_regexp_source};
pub(crate) use match_all::{native_regexp_prototype_match_all, native_regexp_string_iterator_next};
pub(crate) use symbol_match::native_regexp_prototype_match;
pub(crate) use symbol_replace::native_regexp_prototype_replace;
pub(crate) use symbol_search::native_regexp_prototype_search;
pub(crate) use symbol_split::native_regexp_prototype_split;
use validation::validate_regexp_init;
pub(crate) use validation::validate_regexp_init as validate_regexp_literal;

const REGEXP_SOURCE_PROPERTY: &str = "\0RegExpSource";
const REGEXP_FLAGS_PROPERTY: &str = "\0RegExpFlags";
const REGEXP_PROTOTYPE_BINDING: &str = "\0RegExpPrototype";

pub(crate) fn install_regexp(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
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
    env.insert_realm(
        REGEXP_PROTOTYPE_BINDING.to_owned(),
        Value::Object(regexp_prototype.clone()),
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
    symbol_match::install_regexp_prototype_match(env, &regexp_prototype);
    match_all::install_regexp_prototype_match_all(env, &regexp_prototype);
    symbol_replace::install_regexp_prototype_replace(env, &regexp_prototype);
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
    env.insert_realm("RegExp".to_owned(), regexp_value.clone());
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
    env: &mut CallEnv,
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

pub(crate) fn regexp_literal_value(
    source: &str,
    flags: &str,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    validate_regexp_init(source, flags)?;
    let prototype = match env.get(REGEXP_PROTOTYPE_BINDING) {
        Some(Value::Object(prototype)) => Some(prototype),
        _ => None,
    };
    let object = ObjectRef::with_prototype(HashMap::new(), prototype);
    define_regexp_data(&object, source, flags);
    Ok(Value::Object(object))
}

pub(crate) fn native_regexp_prototype_compile(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
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
    regexp_set_last_index_object(object, 0, env)?;
    Ok(this_value)
}

pub(crate) fn native_regexp_prototype_exec(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let Value::Object(object) = this_value.clone() else {
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
    let dot_all = regexp_flags_contains(&object, 's');
    let multiline = regexp_flags_contains(&object, 'm');
    let has_indices = regexp_flags_contains(&object, 'd');
    let stateful = global || sticky;
    let last_index = regexp_last_index(&this_value, env)?;
    let start_code_unit = if stateful { last_index } else { 0 };
    let start = if unicode {
        char_index_from_code_unit_index(&input, start_code_unit)
    } else {
        start_code_unit
    };

    let match_result = if sticky {
        matcher::regexp_match_at(
            &source,
            &input,
            start,
            ignore_case,
            unicode,
            dot_all,
            multiline,
        )
    } else {
        matcher::regexp_match_range(
            &source,
            &input,
            start,
            ignore_case,
            unicode,
            dot_all,
            multiline,
        )
    };

    let Some(match_result) = match_result else {
        if stateful {
            regexp_set_last_index_object(&object, 0, env)?;
        }
        return Ok(Value::Null);
    };
    if stateful {
        let last_index = if unicode {
            code_unit_index_for_char_index(&input, match_result.end)
        } else {
            match_result.end
        };
        regexp_set_last_index_object(&object, last_index, env)?;
    }
    let group_names = matcher::regexp_group_names(&source);
    Ok(regexp_match_array(
        &input,
        match_result,
        unicode,
        &group_names,
        has_indices,
    ))
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let result = native_regexp_prototype_exec(this_value, argument_values, env)?;
    Ok(Value::Boolean(!matches!(result, Value::Null)))
}

pub(crate) fn native_regexp_prototype_source(
    this_value: Value,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let source = regexp_accessor_data(&this_value, env, REGEXP_SOURCE_PROPERTY, "(?:)")?;
    Ok(Value::String(escape_regexp_source(&source)))
}

pub(crate) fn native_regexp_prototype_flags(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_regexp_accessor_object_receiver(&this_value) {
        return Err(regexp_receiver_error());
    }
    let mut flags = String::new();
    for (name, flag) in [
        ("hasIndices", 'd'),
        ("global", 'g'),
        ("ignoreCase", 'i'),
        ("multiline", 'm'),
        ("dotAll", 's'),
        ("unicode", 'u'),
        ("sticky", 'y'),
    ] {
        if is_truthy(&property_value(this_value.clone(), name, env)?) {
            flags.push(flag);
        }
    }
    Ok(Value::String(flags))
}

pub(crate) fn native_regexp_prototype_flag(
    this_value: Value,
    env: &CallEnv,
    flag: char,
) -> Result<Value, RuntimeError> {
    let flags = regexp_accessor_data(&this_value, env, REGEXP_FLAGS_PROPERTY, "")?;
    if flags.is_empty() && is_regexp_prototype_value(&this_value, env) {
        return Ok(Value::Undefined);
    }
    Ok(Value::Boolean(flags.contains(flag)))
}

pub(crate) fn default_regexp_source_accessor_value(
    object: &ObjectRef,
    key: &str,
    env: &CallEnv,
) -> Option<Value> {
    if key != "source" || object.own_property(key).is_some() {
        return None;
    }
    let source = regexp_string_data(object, REGEXP_SOURCE_PROPERTY)?;
    let prototype = match env.get(REGEXP_PROTOTYPE_BINDING) {
        Some(Value::Object(prototype)) => prototype,
        _ => return None,
    };
    let descriptor = prototype.own_property("source")?;
    match descriptor.get {
        Some(Value::Function(getter))
            if getter.native_kind() == Some(NativeFunction::RegExpPrototypeSource) =>
        {
            Some(Value::String(escape_regexp_source(&source)))
        }
        _ => None,
    }
}

fn regexp_accessor_data(
    this_value: &Value,
    env: &CallEnv,
    key: &str,
    prototype_value: &str,
) -> Result<String, RuntimeError> {
    if !is_regexp_accessor_object_receiver(this_value) {
        return Err(regexp_receiver_error());
    };
    let Value::Object(object) = &this_value else {
        unreachable!("RegExp accessor receiver object was checked above")
    };
    if let Some(value) = regexp_string_data(object, key) {
        return Ok(value);
    }
    if is_regexp_prototype_value(this_value, env) {
        return Ok(prototype_value.to_owned());
    }
    Err(regexp_receiver_error())
}

fn is_regexp_accessor_object_receiver(value: &Value) -> bool {
    matches!(value, Value::Object(object) if !symbol::is_symbol_primitive(object))
}

fn is_regexp_prototype_value(value: &Value, env: &CallEnv) -> bool {
    let Value::Object(object) = value else {
        return false;
    };
    env.get("RegExp")
        .and_then(|constructor| match constructor {
            Value::Function(function) => function_prototype(&function),
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
    object.define_property(
        "lastIndex".to_owned(),
        Property::data(Value::Number(0.0), false, true, false),
    );
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
    env: &mut CallEnv,
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
    env: &mut CallEnv,
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
    env: &mut CallEnv,
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
    env: &mut CallEnv,
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

fn regexp_last_index(value: &Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    to_length_with_env(property_value(value.clone(), "lastIndex", env)?, env)
}

fn regexp_last_index_value(value: &Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    regexp_last_index(value, env)
}

fn regexp_set_last_index_object(
    object: &ObjectRef,
    index: usize,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let receiver = Value::Object(object.clone());
    let key = PropertyKey::String("lastIndex".to_owned());
    if !ordinary_set(
        receiver.clone(),
        &key,
        Value::Number(index as f64),
        receiver,
        env,
    )? {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp.prototype.exec cannot set lastIndex".to_owned(),
        });
    }
    Ok(())
}

fn regexp_match_array(
    input: &str,
    match_result: matcher::RegexpMatch,
    unicode: bool,
    group_names: &[Option<String>],
    has_indices: bool,
) -> Value {
    let captures = match_result.captures.clone();
    let mut values = Vec::with_capacity(1 + captures.len());
    values.push(Value::String(input_slice(
        input,
        match_result.start,
        match_result.end,
        unicode,
    )));
    values.extend(captures.iter().map(|capture| {
        capture
            .map(|(start, end)| Value::String(input_slice(input, start, end, unicode)))
            .unwrap_or(Value::Undefined)
    }));
    let result = ArrayRef::new(values);
    let index = if unicode {
        code_unit_index_for_char_index(input, match_result.start)
    } else {
        match_result.start
    };
    result.set_property("index".to_owned(), Value::Number(index as f64));
    result.set_property("input".to_owned(), Value::String(input.to_owned()));
    result.set_property(
        "groups".to_owned(),
        regexp_groups_object(input, &captures, unicode, group_names),
    );
    if has_indices {
        result.set_property(
            "indices".to_owned(),
            regexp_indices_array(
                input,
                (match_result.start, match_result.end),
                &captures,
                unicode,
                group_names,
            ),
        );
    }
    Value::Array(result)
}

/// Build the `indices` array for the `d` flag: a parallel array of
/// `[startCodeUnit, endCodeUnit]` pairs (or `undefined` for unmatched groups),
/// with a `groups` property mirroring the named captures.
fn regexp_indices_array(
    input: &str,
    whole: (usize, usize),
    captures: &[Option<(usize, usize)>],
    unicode: bool,
    group_names: &[Option<String>],
) -> Value {
    let mut entries = Vec::with_capacity(1 + captures.len());
    entries.push(index_pair_value(input, Some(whole), unicode));
    entries.extend(
        captures
            .iter()
            .map(|capture| index_pair_value(input, *capture, unicode)),
    );
    let indices = ArrayRef::new(entries);

    let groups = if group_names.is_empty() {
        Value::Undefined
    } else {
        let object = ObjectRef::with_prototype(HashMap::new(), None);
        for (capture_index, name) in group_names.iter().enumerate() {
            let Some(name) = name else { continue };
            let value = index_pair_value(
                input,
                captures.get(capture_index).copied().flatten(),
                unicode,
            );
            object.set(name.clone(), value);
        }
        Value::Object(object)
    };
    indices.set_property("groups".to_owned(), groups);
    Value::Array(indices)
}

/// Convert a char-index range into a `[start, end]` array of code-unit
/// positions, or `undefined` when the range is absent.
fn index_pair_value(input: &str, range: Option<(usize, usize)>, unicode: bool) -> Value {
    let Some((start, end)) = range else {
        return Value::Undefined;
    };
    let to_units = |char_index: usize| -> f64 {
        if unicode {
            code_unit_index_for_char_index(input, char_index) as f64
        } else {
            char_index as f64
        }
    };
    Value::Array(ArrayRef::new(vec![
        Value::Number(to_units(start)),
        Value::Number(to_units(end)),
    ]))
}

/// Build the `groups` property for a match result: `undefined` when the pattern
/// has no named groups, otherwise a null-prototype object mapping each name to
/// its captured substring (or `undefined` when the group did not participate).
fn regexp_groups_object(
    input: &str,
    captures: &[Option<(usize, usize)>],
    unicode: bool,
    group_names: &[Option<String>],
) -> Value {
    if group_names.is_empty() {
        return Value::Undefined;
    }
    let groups = ObjectRef::with_prototype(HashMap::new(), None);
    for (capture_index, name) in group_names.iter().enumerate() {
        let Some(name) = name else { continue };
        let value = captures
            .get(capture_index)
            .copied()
            .flatten()
            .map(|(start, end)| Value::String(input_slice(input, start, end, unicode)))
            .unwrap_or(Value::Undefined);
        groups.set(name.clone(), value);
    }
    Value::Object(groups)
}

fn input_slice(input: &str, start: usize, end: usize, unicode: bool) -> String {
    if unicode {
        input.chars().skip(start).take(end - start).collect()
    } else {
        string_from_code_units(&string_code_units(input)[start..end])
    }
}

fn code_unit_index_for_char_index(input: &str, char_index: usize) -> usize {
    input.chars().take(char_index).map(char_code_unit_len).sum()
}

fn char_index_from_code_unit_index(input: &str, code_unit_index: usize) -> usize {
    let mut units = 0usize;
    let mut chars = 0usize;
    for (char_index, character) in input.chars().enumerate() {
        chars = char_index + 1;
        if units >= code_unit_index {
            return char_index;
        }
        units += char_code_unit_len(character);
    }
    if code_unit_index <= units {
        chars
    } else {
        chars + 1
    }
}

fn char_code_unit_len(character: char) -> usize {
    if crate::string::surrogate_escape_code_unit(character).is_some() {
        1
    } else {
        character.len_utf16()
    }
}
