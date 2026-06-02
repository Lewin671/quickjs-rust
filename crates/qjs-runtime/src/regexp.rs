use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    to_js_string_with_env,
};

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
