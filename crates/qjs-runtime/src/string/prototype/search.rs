use std::collections::HashMap;

use crate::{RuntimeError, Value, call_function, property_value, regexp, to_js_string_with_env};

use super::super::indexing::{
    string_end_position, string_last_search_position, string_search_start, this_string_value,
};

mod symbol_method;
use symbol_method::{symbol_match_method, symbol_replace_method};

pub(crate) fn native_string_prototype_ends_with(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search_value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    reject_regexp_search_value(search_value.clone(), "String.prototype.endsWith", env)?;
    let search = to_js_string_with_env(search_value, env)?;
    let end = string_end_position(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let prefix = value.chars().take(end).collect::<String>();
    Ok(Value::Boolean(prefix.ends_with(&search)))
}

pub(crate) fn native_string_prototype_includes(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search_value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    reject_regexp_search_value(search_value.clone(), "String.prototype.includes", env)?;
    let search = to_js_string_with_env(search_value, env)?;
    let start = string_search_start(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::Boolean(
        value
            .chars()
            .skip(start)
            .collect::<String>()
            .contains(&search),
    ))
}

pub(crate) fn native_string_prototype_index_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let start = string_search_start(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let haystack = value.chars().skip(start).collect::<String>();
    let Some(byte_index) = haystack.find(&search) else {
        return Ok(Value::Number(-1.0));
    };
    let char_offset = haystack[..byte_index].chars().count();
    Ok(Value::Number((start + char_offset) as f64))
}

pub(crate) fn native_string_prototype_last_index_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let chars: Vec<_> = value.chars().collect();
    let search_chars: Vec<_> = search.chars().collect();
    let position = string_last_search_position(
        chars.len(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    if search_chars.is_empty() {
        return Ok(Value::Number(position as f64));
    }
    if search_chars.len() > chars.len() {
        return Ok(Value::Number(-1.0));
    }

    let start = position.min(chars.len() - search_chars.len());
    for candidate in (0..=start).rev() {
        if chars[candidate..candidate + search_chars.len()] == search_chars {
            return Ok(Value::Number(candidate as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

pub(crate) fn native_string_prototype_match(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "String.prototype method called on null or undefined".to_owned(),
        });
    }
    let pattern = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(pattern, Value::Null | Value::Undefined) {
        if let Some(matcher) = symbol_match_method(pattern.clone(), env)?.method {
            return call_function(matcher, pattern, vec![this_value], env, false);
        }
    }
    let input = this_string_value(this_value, env)?;
    let regexp = regexp_value(pattern, env)?;
    if let Some(matcher) = symbol_match_method(regexp.clone(), env)?.method {
        return call_function(matcher, regexp, vec![Value::String(input)], env, false);
    }
    if regexp::regexp_is_global(&regexp) {
        return regexp::native_regexp_global_match(regexp, &input, env);
    }
    let exec = property_value(regexp.clone(), "exec", env)?;
    call_function(exec, regexp, vec![Value::String(input)], env, false)
}

pub(crate) fn native_string_prototype_replace_all(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if matches!(this_value, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "String.prototype method called on null or undefined".to_owned(),
        });
    }
    let search_value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let is_regexp = regexp::regexp_is_regexp_with_env(search_value.clone(), env)?;
    if is_regexp {
        let flags_value = property_value(search_value.clone(), "flags", env)?;
        if matches!(flags_value, Value::Null | Value::Undefined) {
            return Err(replace_all_regexp_flags_error());
        }
        let flags = to_js_string_with_env(flags_value, env)?;
        if !flags.contains('g') {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: String.prototype.replaceAll called with a non-global RegExp"
                    .to_owned(),
            });
        }
    }
    let replace_method = symbol_replace_method(search_value.clone(), env)?;
    if let Some(replacer) = replace_method.method {
        return call_function(
            replacer,
            search_value,
            vec![
                this_value,
                argument_values.get(1).cloned().unwrap_or(Value::Undefined),
            ],
            env,
            false,
        );
    }
    if is_regexp && regexp::regexp_is_regexp(&search_value) && !replace_method.present {
        let input = this_string_value(this_value, env)?;
        return regexp_replace_all(
            input,
            search_value,
            argument_values.get(1).cloned().unwrap_or(Value::Undefined),
            env,
        );
    }

    let input = this_string_value(this_value, env)?;
    let search = to_js_string_with_env(search_value, env)?;
    let replacement_value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let replacement = if matches!(replacement_value, Value::Function(_)) {
        Replacement::Function(replacement_value)
    } else {
        Replacement::String(to_js_string_with_env(replacement_value, env)?)
    };
    string_replace_all(input, search, replacement, env)
}

fn replace_all_regexp_flags_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: String.prototype.replaceAll RegExp flags are null or undefined"
            .to_owned(),
    }
}

pub(crate) fn native_string_prototype_replace(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let input = this_string_value(this_value, env)?;
    let search_value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let replacement_value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let replacement = if matches!(replacement_value, Value::Function(_)) {
        Replacement::Function(replacement_value)
    } else {
        Replacement::String(to_js_string_with_env(replacement_value, env)?)
    };
    let matches = if regexp::regexp_is_regexp(&search_value) {
        regexp_first_match_position(&input, search_value, env)?
            .into_iter()
            .collect()
    } else {
        let search = to_js_string_with_env(search_value, env)?;
        string_first_match_position(&input, &search)
            .into_iter()
            .collect()
    };
    replace_matches(input, matches, replacement, env)
}

pub(crate) fn native_string_prototype_search(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let input = this_string_value(this_value, env)?;
    let pattern = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let regexp = regexp_value(pattern, env)?;
    let exec = property_value(regexp.clone(), "exec", env)?;
    match call_function(exec, regexp, vec![Value::String(input)], env, false)? {
        Value::Array(array) => property_value(Value::Array(array), "index", env),
        Value::Null => Ok(Value::Number(-1.0)),
        _ => Ok(Value::Number(-1.0)),
    }
}

pub(crate) fn native_string_prototype_starts_with(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = this_string_value(this_value, env)?;
    let search_value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    reject_regexp_search_value(search_value.clone(), "String.prototype.startsWith", env)?;
    let search = to_js_string_with_env(search_value, env)?;
    let start = string_search_start(
        value.chars().count(),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    Ok(Value::Boolean(
        value
            .chars()
            .skip(start)
            .collect::<String>()
            .starts_with(&search),
    ))
}

enum Replacement {
    Function(Value),
    String(String),
}

struct StringMatch {
    start: usize,
    end: usize,
    matched: String,
    captures: Vec<Value>,
}

fn string_replace_all(
    input: String,
    search: String,
    replacement: Replacement,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let matches = string_match_positions(&input, &search);
    replace_matches(input, matches, replacement, env)
}

fn regexp_replace_all(
    input: String,
    regexp: Value,
    replacement_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let replacement = if matches!(replacement_value, Value::Function(_)) {
        Replacement::Function(replacement_value)
    } else {
        Replacement::String(to_js_string_with_env(replacement_value, env)?)
    };
    let matches = regexp_match_positions(&input, regexp, env)?;
    replace_matches(input, matches, replacement, env)
}

fn string_match_positions(input: &str, search: &str) -> Vec<StringMatch> {
    let input_len = input.chars().count();
    if search.is_empty() {
        return (0..=input_len)
            .map(|position| StringMatch {
                start: position,
                end: position,
                matched: String::new(),
                captures: Vec::new(),
            })
            .collect();
    }

    let search_len = search.chars().count();
    let mut matches = Vec::new();
    let mut next_search = 0usize;
    while next_search <= input_len {
        let suffix = input_char_slice(input, next_search, input_len);
        let Some(byte_index) = suffix.find(search) else {
            break;
        };
        let start = next_search + suffix[..byte_index].chars().count();
        let end = start + search_len;
        matches.push(StringMatch {
            start,
            end,
            matched: search.to_owned(),
            captures: Vec::new(),
        });
        next_search = end;
    }
    matches
}

fn string_first_match_position(input: &str, search: &str) -> Option<StringMatch> {
    if search.is_empty() {
        return Some(StringMatch {
            start: 0,
            end: 0,
            matched: String::new(),
            captures: Vec::new(),
        });
    }
    let byte_index = input.find(search)?;
    let start = input[..byte_index].chars().count();
    Some(StringMatch {
        start,
        end: start + search.chars().count(),
        matched: search.to_owned(),
        captures: Vec::new(),
    })
}

fn regexp_match_positions(
    input: &str,
    regexp: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<StringMatch>, RuntimeError> {
    regexp::regexp_set_last_index(&regexp, 0);
    let mut matches = Vec::new();
    loop {
        let exec = property_value(regexp.clone(), "exec", env)?;
        let result = call_function(
            exec,
            regexp.clone(),
            vec![Value::String(input.to_owned())],
            env,
            false,
        )?;
        let Value::Array(array) = result else {
            break;
        };
        let Some(Value::String(matched)) = array.get(0) else {
            break;
        };
        let start = regexp_match_index(&Value::Array(array.clone()), env)?;
        let end = start + matched.chars().count();
        let captures = (1..array.len())
            .map(|index| array.get(index).unwrap_or(Value::Undefined))
            .collect();
        let empty = matched.is_empty();
        matches.push(StringMatch {
            start,
            end,
            matched,
            captures,
        });
        if empty {
            regexp::regexp_set_last_index(&regexp, end.saturating_add(1));
        }
    }
    Ok(matches)
}

fn regexp_first_match_position(
    input: &str,
    regexp: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Option<StringMatch>, RuntimeError> {
    regexp::regexp_set_last_index(&regexp, 0);
    let exec = property_value(regexp.clone(), "exec", env)?;
    let result = call_function(
        exec,
        regexp.clone(),
        vec![Value::String(input.to_owned())],
        env,
        false,
    )?;
    let Value::Array(array) = result else {
        return Ok(None);
    };
    let Some(Value::String(matched)) = array.get(0) else {
        return Ok(None);
    };
    let start = regexp_match_index(&Value::Array(array.clone()), env)?;
    let captures = (1..array.len())
        .map(|index| array.get(index).unwrap_or(Value::Undefined))
        .collect();
    Ok(Some(StringMatch {
        start,
        end: start + matched.chars().count(),
        matched,
        captures,
    }))
}

fn replace_matches(
    input: String,
    matches: Vec<StringMatch>,
    replacement: Replacement,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let mut result = String::new();
    let mut copied_until = 0usize;
    for string_match in matches {
        result.push_str(&input_char_slice(&input, copied_until, string_match.start));
        let replacement_string = match &replacement {
            Replacement::Function(function) => {
                functional_replacement(function.clone(), &string_match, input.clone(), env)?
            }
            Replacement::String(replacement) => get_substitution(
                replacement,
                &string_match.matched,
                string_match.start,
                &input,
                &string_match.captures,
            ),
        };
        result.push_str(&replacement_string);
        copied_until = string_match.end;
    }
    result.push_str(&input_char_slice(
        &input,
        copied_until,
        input.chars().count(),
    ));
    Ok(Value::String(result))
}

fn functional_replacement(
    function: Value,
    string_match: &StringMatch,
    input: String,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    let mut arguments = Vec::with_capacity(3 + string_match.captures.len());
    arguments.push(Value::String(string_match.matched.clone()));
    arguments.extend(string_match.captures.iter().cloned());
    arguments.push(Value::Number(string_match.start as f64));
    arguments.push(Value::String(input));
    let value = call_function(function, Value::Undefined, arguments, env, false)?;
    to_js_string_with_env(value, env)
}

fn get_substitution(
    replacement: &str,
    matched: &str,
    position: usize,
    input: &str,
    captures: &[Value],
) -> String {
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
            '1'..='9' => {
                let first = next.to_digit(10).unwrap() as usize;
                if let Some(second) = chars.peek().and_then(|value| value.to_digit(10)) {
                    let two_digit = first * 10 + second as usize;
                    if two_digit <= captures.len() {
                        chars.next();
                        push_capture(&mut result, captures, two_digit);
                        continue;
                    }
                }
                if first <= captures.len() {
                    push_capture(&mut result, captures, first);
                } else {
                    result.push('$');
                    result.push(next);
                }
            }
            _ => {
                result.push('$');
                result.push(next);
            }
        }
    }
    result
}

fn push_capture(result: &mut String, captures: &[Value], index: usize) {
    if let Some(Value::String(capture)) = captures.get(index - 1) {
        result.push_str(capture);
    }
}

fn regexp_match_index(
    match_value: &Value,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let index = property_value(match_value.clone(), "index", env)?;
    match index {
        Value::Number(number) if number.is_finite() && number > 0.0 => Ok(number.trunc() as usize),
        _ => Ok(0),
    }
}

fn input_char_slice(input: &str, start: usize, end: usize) -> String {
    input.chars().skip(start).take(end - start).collect()
}

fn regexp_value(pattern: Value, env: &mut HashMap<String, Value>) -> Result<Value, RuntimeError> {
    if regexp::regexp_is_regexp(&pattern) {
        return Ok(pattern);
    }
    let constructor = env.get("RegExp").cloned().ok_or_else(|| RuntimeError {
        thrown: None,
        message: "RegExp constructor is not available".to_owned(),
    })?;
    call_function(constructor, Value::Undefined, vec![pattern], env, false)
}

fn reject_regexp_search_value(
    value: Value,
    method: &str,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    if regexp::regexp_is_regexp_with_env(value, env)? {
        return Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: {method} search string must not be a RegExp"),
        });
    }
    Ok(())
}
