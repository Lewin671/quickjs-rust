use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    call_function, property_value, reflect, symbol, to_js_string_with_env, to_length_with_env,
};

pub(crate) fn install_regexp_prototype_match(env: &HashMap<String, Value>, prototype: &ObjectRef) {
    if let Some(symbol) = symbol::match_symbol(env) {
        prototype.define_symbol_property(
            symbol,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.match]"),
                1,
                NativeFunction::RegExpPrototypeMatch,
                false,
            ))),
        );
    }
}

pub(crate) fn native_regexp_prototype_match(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_object_value(&this_value) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: RegExp.prototype[Symbol.match] requires an object receiver"
                .to_owned(),
        });
    }

    let input = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let flags = to_js_string_with_env(property_value(this_value.clone(), "flags", env)?, env)?;
    let global = flags.contains('g');
    if !global {
        return regexp_exec(this_value, &input, env);
    }

    let unicode = flags.contains('u');
    set_last_index(this_value.clone(), Value::Number(0.0), env)?;
    global_match(this_value, &input, unicode, env)
}

fn global_match(
    regexp: Value,
    input: &str,
    unicode: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let mut matches = Vec::new();
    loop {
        let result = regexp_exec(regexp.clone(), input, env)?;
        if matches!(result, Value::Null) {
            return if matches.is_empty() {
                Ok(Value::Null)
            } else {
                Ok(Value::Array(ArrayRef::new(matches)))
            };
        }

        let match_string = to_js_string_with_env(property_value(result, "0", env)?, env)?;
        let empty = match_string.is_empty();
        matches.push(Value::String(match_string));
        if empty {
            let last_index =
                to_length_with_env(property_value(regexp.clone(), "lastIndex", env)?, env)?;
            let next_index = advance_string_index(input, last_index, unicode);
            set_last_index(regexp.clone(), Value::Number(next_index as f64), env)?;
        }
    }
}

fn regexp_exec(
    regexp: Value,
    input: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
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
        vec![Value::String(input.to_owned())],
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
            message: "TypeError: RegExp.prototype[Symbol.match] cannot set lastIndex".to_owned(),
        })
    }
}

fn advance_string_index(input: &str, index: usize, unicode: bool) -> usize {
    let chars: Vec<_> = input.chars().collect();
    crate::string::advance_string_index(&chars, index, unicode)
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
