use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value, call_function,
    property_value, reflect, symbol, to_js_string_with_env, to_length_with_env,
};

pub(crate) fn install_regexp_prototype_replace(env: &CallEnv, prototype: &ObjectRef) {
    if let Some(symbol) = symbol::replace_symbol(env) {
        prototype.define_symbol_property(
            symbol,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.replace]"),
                2,
                NativeFunction::RegExpPrototypeReplace,
                false,
            ))),
        );
    }
}

pub(crate) fn native_regexp_prototype_replace(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_object_value(&this_value) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp.prototype[Symbol.replace] requires an object receiver"
                .to_owned(),
        });
    }

    let input = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let replace_value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let replacement = if matches!(replace_value, Value::Function(_)) {
        Replacement::Function(Box::new(replace_value))
    } else {
        Replacement::String(to_js_string_with_env(replace_value, env)?)
    };

    let flags = to_js_string_with_env(property_value(this_value.clone(), "flags", env)?, env)?;
    let global = flags.contains('g');
    let unicode = flags.contains('u');
    if global {
        set_last_index(this_value.clone(), Value::Number(0.0), env)?;
    }

    let matches = collect_matches(this_value, &input, global, unicode, env)?;
    replace_matches(input, matches, replacement, env)
}

enum Replacement {
    Function(Box<Value>),
    String(String),
}

struct MatchRecord {
    start: usize,
    end: usize,
    matched: String,
    captures: Vec<Value>,
    groups: Value,
}

fn collect_matches(
    regexp: Value,
    input: &str,
    global: bool,
    unicode: bool,
    env: &mut CallEnv,
) -> Result<Vec<MatchRecord>, RuntimeError> {
    let mut matches = Vec::new();
    loop {
        let exec_result = regexp_exec(regexp.clone(), input, env)?;
        if matches!(exec_result, Value::Null) {
            break;
        }
        let match_record = match_record(exec_result, input, env)?;
        let empty = match_record.matched.is_empty();
        matches.push(match_record);
        if !global {
            break;
        }
        if empty {
            let last_index =
                to_length_with_env(property_value(regexp.clone(), "lastIndex", env)?, env)?;
            let next_index = advance_string_index(input, last_index, unicode);
            set_last_index(regexp.clone(), Value::Number(next_index as f64), env)?;
        }
    }
    Ok(matches)
}

fn regexp_exec(regexp: Value, input: &str, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let exec = property_value(regexp.clone(), "exec", env)?;
    if !matches!(exec, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp exec method is not callable".to_owned(),
        });
    }
    let result = call_function(
        exec,
        regexp,
        vec![Value::String(input.to_owned().into())],
        env,
        false,
    )?;
    if matches!(result, Value::Null) || is_object_value(&result) {
        Ok(result)
    } else {
        Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp exec must return an object or null".to_owned(),
        })
    }
}

fn match_record(
    exec_result: Value,
    input: &str,
    env: &mut CallEnv,
) -> Result<MatchRecord, RuntimeError> {
    let matched = to_js_string_with_env(property_value(exec_result.clone(), "0", env)?, env)?;
    let position = to_length_with_env(property_value(exec_result.clone(), "index", env)?, env)?
        .min(input.chars().count());
    let length = to_length_with_env(property_value(exec_result.clone(), "length", env)?, env)?;
    let mut captures = Vec::new();
    for index in 1..length {
        let capture = property_value(exec_result.clone(), &index.to_string(), env)?;
        captures.push(match capture {
            Value::Undefined => Value::Undefined,
            value => Value::String(to_js_string_with_env(value, env)?.into()),
        });
    }
    let groups = property_value(exec_result, "groups", env)?;
    Ok(MatchRecord {
        start: position,
        end: position + matched.chars().count(),
        matched,
        captures,
        groups,
    })
}

fn replace_matches(
    input: String,
    matches: Vec<MatchRecord>,
    replacement: Replacement,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let mut result = String::new();
    let mut copied_until = 0usize;
    for match_record in matches {
        if match_record.start < copied_until {
            continue;
        }
        result.push_str(&input_char_slice(&input, copied_until, match_record.start));
        let replacement_string = match &replacement {
            Replacement::Function(function) => {
                functional_replacement((**function).clone(), &match_record, input.clone(), env)?
            }
            Replacement::String(replacement) => get_substitution(
                replacement,
                &match_record.matched,
                match_record.start,
                &input,
                &match_record.captures,
                &match_record.groups,
                env,
            )?,
        };
        result.push_str(&replacement_string);
        copied_until = match_record.end;
    }
    result.push_str(&input_char_slice(
        &input,
        copied_until,
        input.chars().count(),
    ));
    Ok(Value::String(result.into()))
}

fn functional_replacement(
    function: Value,
    match_record: &MatchRecord,
    input: String,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
    let mut arguments = Vec::with_capacity(4 + match_record.captures.len());
    arguments.push(Value::String(match_record.matched.clone().into()));
    arguments.extend(match_record.captures.iter().cloned());
    arguments.push(Value::Number(match_record.start as f64));
    arguments.push(Value::String(input.into()));
    if !matches!(match_record.groups, Value::Undefined) {
        arguments.push(match_record.groups.clone());
    }
    let value = call_function(function, Value::Undefined, arguments, env, false)?;
    to_js_string_with_env(value, env)
}

#[allow(clippy::too_many_arguments)]
fn get_substitution(
    replacement: &str,
    matched: &str,
    position: usize,
    input: &str,
    captures: &[Value],
    named_captures: &Value,
    env: &mut CallEnv,
) -> Result<String, RuntimeError> {
    let mut result = String::new();
    let mut chars = replacement.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '$' {
            result.push(character);
            continue;
        }
        let Some(next) = chars.next() else {
            result.push('$');
            break;
        };
        match next {
            '$' => result.push('$'),
            '&' => result.push_str(matched),
            '`' => result.push_str(&input_char_slice(input, 0, position)),
            '\'' => result.push_str(&input_char_slice(
                input,
                position + matched.chars().count(),
                input.chars().count(),
            )),
            '0'..='9' => substitute_capture(&mut result, &mut chars, captures, next),
            '<' if !matches!(named_captures, Value::Undefined) => {
                substitute_named_capture(&mut result, &mut chars, named_captures, env)?;
            }
            _ => {
                result.push('$');
                result.push(next);
            }
        }
    }
    Ok(result)
}

/// Handle a `$<name>` substitution. `named_captures` is known to be defined.
/// An unterminated `$<` (no closing `>`) is emitted literally per spec.
fn substitute_named_capture(
    result: &mut String,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    named_captures: &Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let mut name = String::new();
    let mut closed = false;
    for value in chars.by_ref() {
        if value == '>' {
            closed = true;
            break;
        }
        name.push(value);
    }
    if !closed {
        result.push_str("$<");
        result.push_str(&name);
        return Ok(());
    }
    let capture = property_value(named_captures.clone(), &name, env)?;
    if !matches!(capture, Value::Undefined) {
        result.push_str(&to_js_string_with_env(capture, env)?);
    }
    Ok(())
}

fn substitute_capture(
    result: &mut String,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    captures: &[Value],
    first_digit: char,
) {
    let first = first_digit.to_digit(10).unwrap() as usize;
    if let Some(second) = chars.peek().and_then(|value| value.to_digit(10)) {
        let two_digit = first * 10 + second as usize;
        if (1..=captures.len()).contains(&two_digit) {
            chars.next();
            push_capture(result, captures, two_digit);
            return;
        }
        if first == 0 {
            chars.next();
            result.push('$');
            result.push(first_digit);
            result.push(char::from_digit(second, 10).unwrap());
            return;
        }
    }
    if (1..=captures.len()).contains(&first) {
        push_capture(result, captures, first);
    } else {
        result.push('$');
        result.push(first_digit);
    }
}

fn push_capture(result: &mut String, captures: &[Value], index: usize) {
    if let Some(Value::String(capture)) = captures.get(index - 1) {
        result.push_str(capture);
    }
}

fn set_last_index(receiver: Value, value: Value, env: &mut CallEnv) -> Result<(), RuntimeError> {
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
            message: "TypeError: RegExp.prototype[Symbol.replace] cannot set lastIndex".to_owned(),
        })
    }
}

fn advance_string_index(input: &str, index: usize, unicode: bool) -> usize {
    let chars: Vec<_> = input.chars().collect();
    crate::string::advance_string_index(&chars, index, unicode)
}

fn input_char_slice(input: &str, start: usize, end: usize) -> String {
    let input_len = input.chars().count();
    let start = start.min(input_len);
    let end = end.min(input_len);
    input
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}
