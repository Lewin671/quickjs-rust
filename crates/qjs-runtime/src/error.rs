use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    to_js_string,
};

const ERROR_DATA_PROPERTY: &str = "\0ErrorData";

pub(crate) fn install_error(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let error_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    let error_function = Function::new_native(Some("Error"), 1, NativeFunction::Error, true);
    error_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(error_function.clone()),
    );
    error_prototype.define_non_enumerable("name".to_owned(), Value::String("Error".to_owned()));
    error_prototype.define_non_enumerable("message".to_owned(), Value::String(String::new()));
    error_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::ErrorPrototypeToString,
            false,
        )),
    );
    error_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(error_prototype)),
    );

    let error_value = Value::Function(error_function);
    env.insert("Error".to_owned(), error_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Error".to_owned(), error_value);
    }
}

pub(crate) fn native_error(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    let object = match (is_construct, this_value) {
        (true, Value::Object(object)) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    define_error_data(&object);
    if let Some(message) = argument_values.first() {
        if !matches!(message, Value::Undefined) {
            object.define_property(
                "message".to_owned(),
                Property::data(
                    Value::String(to_js_string(message.clone())?),
                    false,
                    true,
                    true,
                ),
            );
        }
    }
    Ok(Value::Object(object))
}

pub(crate) fn native_error_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(RuntimeError {
            message: "Error.prototype.toString called on non-object".to_owned(),
        });
    };

    let name = match object.get("name") {
        Some(Value::Undefined) | None => "Error".to_owned(),
        Some(value) => to_js_string(value)?,
    };
    let message = match object.get("message") {
        Some(Value::Undefined) | None => String::new(),
        Some(value) => to_js_string(value)?,
    };

    Ok(Value::String(match (name.is_empty(), message.is_empty()) {
        (true, true) => String::new(),
        (true, false) => message,
        (false, true) => name,
        (false, false) => format!("{name}: {message}"),
    }))
}

pub(crate) fn is_error_object(object: &ObjectRef) -> bool {
    object.own_property(ERROR_DATA_PROPERTY).is_some()
}

pub(crate) fn error_object_to_string(object: &ObjectRef) -> Option<String> {
    if !is_error_object(object) {
        return None;
    }
    let name = match object.get("name") {
        Some(Value::String(value)) if !value.is_empty() => value,
        Some(Value::String(_)) => return object_message(object),
        Some(Value::Undefined) | None => "Error".to_owned(),
        _ => "Error".to_owned(),
    };
    let Some(message) = object_message(object) else {
        return Some(name);
    };
    if message.is_empty() {
        Some(name)
    } else {
        Some(format!("{name}: {message}"))
    }
}

fn define_error_data(object: &ObjectRef) {
    object.define_non_enumerable(ERROR_DATA_PROPERTY.to_owned(), Value::Boolean(true));
}

fn object_message(object: &ObjectRef) -> Option<String> {
    match object.get("message") {
        Some(Value::String(value)) => Some(value),
        Some(Value::Undefined) | None => Some(String::new()),
        _ => None,
    }
}
