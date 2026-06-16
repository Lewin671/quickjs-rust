use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value, call_function,
    construct_function, property_value, reflect, symbol, to_js_string_with_env, to_length_with_env,
};

const REGEXP_STRING_ITERATOR_REGEXP: &str = "\0regexp_string_iterator_regexp";
const REGEXP_STRING_ITERATOR_STRING: &str = "\0regexp_string_iterator_string";
const REGEXP_STRING_ITERATOR_GLOBAL: &str = "\0regexp_string_iterator_global";
const REGEXP_STRING_ITERATOR_UNICODE: &str = "\0regexp_string_iterator_unicode";
const REGEXP_STRING_ITERATOR_DONE: &str = "\0regexp_string_iterator_done";

pub(crate) fn install_regexp_prototype_match_all(env: &CallEnv, prototype: &ObjectRef) {
    if let Some(symbol) = symbol::match_all_symbol(env) {
        prototype.define_symbol_property(
            symbol,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.matchAll]"),
                1,
                NativeFunction::RegExpPrototypeMatchAll,
                false,
            ))),
        );
    }
}

pub(crate) fn native_regexp_prototype_match_all(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_object_value(&this_value) {
        return Err(RuntimeError {
            thrown: None,
            message: "RegExp.prototype[Symbol.matchAll] requires an object receiver".to_owned(),
        });
    }

    let input = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    // C = SpeciesConstructor(R, %RegExp%) is resolved before reading R's flags,
    // matching the spec's observable order (@@species lookup, then Get(R, flags)).
    let constructor = super::regexp_species_constructor(this_value.clone(), env)?;
    let flags = to_js_string_with_env(property_value(this_value.clone(), "flags", env)?, env)?;
    let global = flags.contains('g');
    let unicode = flags.contains('u');
    let matcher = construct_function(
        constructor.clone(),
        constructor,
        vec![this_value.clone(), Value::String(flags)],
        env,
    )?;
    // lastIndex = ToLength(? Get(R, "lastIndex")) — the coercion is observable
    // (a throwing valueOf must propagate) and runs before the matcher copy is set.
    let last_index = to_length_with_env(property_value(this_value, "lastIndex", env)?, env)?;
    set_last_index(matcher.clone(), Value::Number(last_index as f64), env)?;

    let iterator = ObjectRef::new(HashMap::new());
    iterator.define_non_enumerable(REGEXP_STRING_ITERATOR_REGEXP.to_owned(), matcher);
    iterator.define_non_enumerable(
        REGEXP_STRING_ITERATOR_STRING.to_owned(),
        Value::String(input),
    );
    iterator.define_non_enumerable(
        REGEXP_STRING_ITERATOR_GLOBAL.to_owned(),
        Value::Boolean(global),
    );
    iterator.define_non_enumerable(
        REGEXP_STRING_ITERATOR_UNICODE.to_owned(),
        Value::Boolean(unicode),
    );
    iterator.define_non_enumerable(
        REGEXP_STRING_ITERATOR_DONE.to_owned(),
        Value::Boolean(false),
    );
    iterator.define_non_enumerable(
        "next".to_owned(),
        Value::Function(Function::new_native(
            Some("next"),
            0,
            NativeFunction::RegExpStringIteratorPrototypeNext,
            false,
        )),
    );
    symbol::define_iterator_identity(env, &iterator);
    Ok(Value::Object(iterator))
}

pub(crate) fn native_regexp_string_iterator_next(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let Value::Object(iterator) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "RegExp String iterator next called on non-object".to_owned(),
        });
    };
    if iterator_done(&iterator) {
        return Ok(iterator_result(Value::Undefined, true));
    }

    let regexp = iterator_slot(&iterator, REGEXP_STRING_ITERATOR_REGEXP)?;
    let input = iterator_string(&iterator)?;
    let result = regexp_exec(regexp.clone(), &input, env)?;
    if matches!(result, Value::Null) {
        iterator
            .define_non_enumerable(REGEXP_STRING_ITERATOR_DONE.to_owned(), Value::Boolean(true));
        return Ok(iterator_result(Value::Undefined, true));
    }

    let match_result = ensure_exec_result_object(result)?;
    if !iterator_boolean(&iterator, REGEXP_STRING_ITERATOR_GLOBAL)? {
        iterator
            .define_non_enumerable(REGEXP_STRING_ITERATOR_DONE.to_owned(), Value::Boolean(true));
        return Ok(iterator_result(match_result, false));
    }

    let match_string = match_result_value(match_result.clone(), env)?;
    if match_string.is_empty() {
        let last_index =
            to_length_with_env(property_value(regexp.clone(), "lastIndex", env)?, env)?;
        let next_index = advance_string_index(
            &input,
            last_index,
            iterator_boolean(&iterator, REGEXP_STRING_ITERATOR_UNICODE)?,
        );
        set_last_index(regexp, Value::Number(next_index as f64), env)?;
    }
    Ok(iterator_result(match_result, false))
}

fn regexp_exec(regexp: Value, input: &str, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let exec = property_value(regexp.clone(), "exec", env)?;
    if !matches!(exec, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp exec method is not callable".to_owned(),
        });
    }
    call_function(
        exec,
        regexp,
        vec![Value::String(input.to_owned())],
        env,
        false,
    )
}

fn ensure_exec_result_object(value: Value) -> Result<Value, RuntimeError> {
    if is_object_value(&value)
        && !matches!(&value, Value::Object(object) if symbol::is_symbol_primitive(object))
    {
        Ok(value)
    } else {
        Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp exec must return an object or null".to_owned(),
        })
    }
}

fn match_result_value(value: Value, env: &mut CallEnv) -> Result<String, RuntimeError> {
    to_js_string_with_env(property_value(value, "0", env)?, env)
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
            message: "TypeError: RegExp.prototype[Symbol.matchAll] cannot set lastIndex".to_owned(),
        })
    }
}

fn advance_string_index(input: &str, index: usize, unicode: bool) -> usize {
    let chars: Vec<_> = input.chars().collect();
    crate::string::advance_string_index(&chars, index, unicode)
}

fn iterator_done(iterator: &ObjectRef) -> bool {
    matches!(
        iterator
            .own_property(REGEXP_STRING_ITERATOR_DONE)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    )
}

fn iterator_string(iterator: &ObjectRef) -> Result<String, RuntimeError> {
    match iterator_slot(iterator, REGEXP_STRING_ITERATOR_STRING)? {
        Value::String(value) => Ok(value),
        _ => Err(RuntimeError {
            thrown: None,
            message: "RegExp String iterator source is invalid".to_owned(),
        }),
    }
}

fn iterator_boolean(iterator: &ObjectRef, key: &str) -> Result<bool, RuntimeError> {
    match iterator_slot(iterator, key)? {
        Value::Boolean(value) => Ok(value),
        _ => Err(RuntimeError {
            thrown: None,
            message: "RegExp String iterator flag is invalid".to_owned(),
        }),
    }
}

fn iterator_slot(iterator: &ObjectRef, key: &str) -> Result<Value, RuntimeError> {
    iterator
        .own_property(key)
        .map(|property| property.value)
        .ok_or_else(|| RuntimeError {
            thrown: None,
            message: "RegExp String iterator is missing internal state".to_owned(),
        })
}

fn iterator_result(value: Value, done: bool) -> Value {
    let mut properties = HashMap::new();
    properties.insert("value".to_owned(), value);
    properties.insert("done".to_owned(), Value::Boolean(done));
    Value::Object(ObjectRef::new(properties))
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}
