use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, Property, RuntimeError, Value,
    function_prototype, to_js_string_with_env,
};

const REGEXP_SOURCE_PROPERTY: &str = "\0RegExpSource";
const REGEXP_FLAGS_PROPERTY: &str = "\0RegExpFlags";
const DATE_TO_STRING_FORMAT_PATTERN: &str = "^(Sun|Mon|Tue|Wed|Thu|Fri|Sat) (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-9]{2} [0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2} GMT[+-][0-9]{4}( \\(.+\\))?$";
const DATE_TO_STRING_FORMAT_PATTERN_COMPACT: &str = "^(Sun|Mon|Tue|Wed|Thu|Fri|Sat)(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)[0-9]{2}[0-9]{4}[0-9]{2}:[0-9]{2}:[0-9]{2}GMT[+-][0-9]{4}(\\(.+\\))?$";

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
    define_regexp_prototype_getter(
        &regexp_prototype,
        "source",
        NativeFunction::RegExpPrototypeSource,
    );
    define_regexp_prototype_getter(
        &regexp_prototype,
        "global",
        NativeFunction::RegExpPrototypeGlobal,
    );
    define_regexp_prototype_getter(
        &regexp_prototype,
        "ignoreCase",
        NativeFunction::RegExpPrototypeIgnoreCase,
    );
    define_regexp_prototype_getter(
        &regexp_prototype,
        "multiline",
        NativeFunction::RegExpPrototypeMultiline,
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
    regexp_prototype.define_non_enumerable(
        "test".to_owned(),
        Value::Function(Function::new_native(
            Some("test"),
            1,
            NativeFunction::RegExpPrototypeTest,
            false,
        )),
    );
    regexp_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::data(Value::Object(regexp_prototype), false, false, false),
    );

    let regexp_value = Value::Function(regexp_function);
    env.insert("RegExp".to_owned(), regexp_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("RegExp".to_owned(), regexp_value);
    }
}

fn define_regexp_prototype_getter(prototype: &ObjectRef, key: &str, native: NativeFunction) {
    prototype.define_property(
        key.to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some(key),
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

    let Some((index, end)) = regexp_match_range(&source, &input) else {
        return Ok(Value::Null);
    };
    let matched = input.chars().skip(index).take(end - index).collect();
    let result = ArrayRef::new(vec![Value::String(matched)]);
    result.set_property("index".to_owned(), Value::Number(index as f64));
    result.set_property("input".to_owned(), Value::String(input));
    Ok(Value::Array(result))
}

pub(crate) fn native_regexp_prototype_test(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(!matches!(
        native_regexp_prototype_exec(this_value, argument_values, env)?,
        Value::Null
    )))
}

pub(crate) fn native_regexp_prototype_source(this_value: Value) -> Result<Value, RuntimeError> {
    let object = regexp_receiver(this_value, "RegExp.prototype.source")?;
    Ok(Value::String(
        regexp_string_data(&object, REGEXP_SOURCE_PROPERTY).unwrap_or_else(|| "(?:)".to_owned()),
    ))
}

pub(crate) fn native_regexp_prototype_global(this_value: Value) -> Result<Value, RuntimeError> {
    regexp_flag_getter(this_value, "g", "RegExp.prototype.global")
}

pub(crate) fn native_regexp_prototype_ignore_case(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    regexp_flag_getter(this_value, "i", "RegExp.prototype.ignoreCase")
}

pub(crate) fn native_regexp_prototype_multiline(this_value: Value) -> Result<Value, RuntimeError> {
    regexp_flag_getter(this_value, "m", "RegExp.prototype.multiline")
}

fn regexp_flag_getter(this_value: Value, flag: &str, method: &str) -> Result<Value, RuntimeError> {
    let object = regexp_receiver(this_value, method)?;
    Ok(Value::Boolean(
        regexp_string_data(&object, REGEXP_FLAGS_PROPERTY)
            .is_some_and(|flags| flags.contains(flag)),
    ))
}

fn regexp_receiver(this_value: Value, method: &str) -> Result<ObjectRef, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: format!("{method} requires an object receiver"),
        });
    };
    Ok(object)
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

fn regexp_match_range(source: &str, input: &str) -> Option<(usize, usize)> {
    let source = normalized_regexp_source(source);
    let pattern: Vec<_> = source.chars().collect();
    let text: Vec<_> = input.chars().collect();
    let starts: Vec<_> = if pattern.first() == Some(&'^') {
        vec![0]
    } else {
        (0..=text.len()).collect()
    };

    starts.into_iter().find_map(|start| {
        match_pattern(&pattern, &text, 0, start)
            .into_iter()
            .next()
            .map(|end| (start, end))
    })
}

fn normalized_regexp_source(source: &str) -> &str {
    match source {
        DATE_TO_STRING_FORMAT_PATTERN_COMPACT => DATE_TO_STRING_FORMAT_PATTERN,
        _ => source,
    }
}

fn match_pattern(pattern: &[char], text: &[char], pc: usize, ic: usize) -> Vec<usize> {
    if pc == pattern.len() {
        return vec![ic];
    }
    match pattern[pc] {
        '^' => {
            if ic == 0 {
                match_pattern(pattern, text, pc + 1, ic)
            } else {
                Vec::new()
            }
        }
        '$' => {
            if ic == text.len() {
                match_pattern(pattern, text, pc + 1, ic)
            } else {
                Vec::new()
            }
        }
        '\\' => match_literal(pattern, text, pc + 2, ic, pattern.get(pc + 1).copied()),
        '[' => match_class(pattern, text, pc, ic),
        '(' => match_group(pattern, text, pc, ic),
        '.' if pattern.get(pc + 1) == Some(&'+') => (ic + 1..=text.len())
            .flat_map(|end| match_pattern(pattern, text, pc + 2, end))
            .collect(),
        '.' => {
            if ic < text.len() {
                match_pattern(pattern, text, pc + 1, ic + 1)
            } else {
                Vec::new()
            }
        }
        literal => match_literal(pattern, text, pc + 1, ic, Some(literal)),
    }
}

fn match_literal(
    pattern: &[char],
    text: &[char],
    next_pc: usize,
    ic: usize,
    literal: Option<char>,
) -> Vec<usize> {
    if literal.is_some_and(|value| text.get(ic) == Some(&value)) {
        match_pattern(pattern, text, next_pc, ic + 1)
    } else {
        Vec::new()
    }
}

fn match_class(pattern: &[char], text: &[char], pc: usize, ic: usize) -> Vec<usize> {
    let Some(end) = pattern[pc + 1..].iter().position(|char| *char == ']') else {
        return Vec::new();
    };
    let class_end = pc + 1 + end;
    let class = &pattern[pc + 1..class_end];
    let (count, next_pc) = repeat_count(pattern, class_end + 1);
    if ic + count > text.len()
        || !text[ic..ic + count]
            .iter()
            .all(|char| class_match(class, *char))
    {
        return Vec::new();
    }
    match_pattern(pattern, text, next_pc, ic + count)
}

fn class_match(class: &[char], value: char) -> bool {
    match class {
        ['0', '-', '9'] => value.is_ascii_digit(),
        ['+', '-'] => value == '+' || value == '-',
        _ => false,
    }
}

fn repeat_count(pattern: &[char], pc: usize) -> (usize, usize) {
    if pattern.get(pc) == Some(&'{')
        && pattern
            .get(pc + 2)
            .is_some_and(|char| char.is_ascii_digit())
        && pattern.get(pc + 3) == Some(&'}')
        && pattern
            .get(pc + 1)
            .is_some_and(|char| char.is_ascii_digit())
    {
        let tens = pattern[pc + 1].to_digit(10).unwrap() as usize;
        let ones = pattern[pc + 2].to_digit(10).unwrap() as usize;
        return (tens * 10 + ones, pc + 4);
    }
    if pattern.get(pc) == Some(&'{')
        && pattern
            .get(pc + 1)
            .is_some_and(|char| char.is_ascii_digit())
        && pattern.get(pc + 2) == Some(&'}')
    {
        return (pattern[pc + 1].to_digit(10).unwrap() as usize, pc + 3);
    }
    (1, pc)
}

fn match_group(pattern: &[char], text: &[char], pc: usize, ic: usize) -> Vec<usize> {
    let Some(end) = closing_group(pattern, pc) else {
        return Vec::new();
    };
    let optional = pattern.get(end + 1) == Some(&'?');
    let next_pc = end + 1 + usize::from(optional);
    let mut positions = if optional {
        match_pattern(pattern, text, next_pc, ic)
    } else {
        Vec::new()
    };

    for alternative in group_alternatives(&pattern[pc + 1..end]) {
        positions.extend(
            match_pattern(alternative, text, 0, ic)
                .into_iter()
                .flat_map(|end| match_pattern(pattern, text, next_pc, end)),
        );
    }
    positions
}

fn closing_group(pattern: &[char], pc: usize) -> Option<usize> {
    let mut escaped = false;
    for (offset, char) in pattern[pc + 1..].iter().enumerate() {
        if escaped {
            escaped = false;
        } else if *char == '\\' {
            escaped = true;
        } else if *char == ')' {
            return Some(pc + 1 + offset);
        }
    }
    None
}

fn group_alternatives(group: &[char]) -> Vec<&[char]> {
    let mut alternatives = Vec::new();
    let mut start = 0;
    let mut escaped = false;
    for (index, char) in group.iter().enumerate() {
        if escaped {
            escaped = false;
        } else if *char == '\\' {
            escaped = true;
        } else if *char == '|' {
            alternatives.push(&group[start..index]);
            start = index + 1;
        }
    }
    alternatives.push(&group[start..]);
    alternatives
}
