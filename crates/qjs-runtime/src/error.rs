use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    has_property, property_value, property_value_key, symbol, to_js_string, to_js_string_with_env,
};

const ERROR_DATA_PROPERTY: &str = "\0ErrorData";

const NATIVE_ERRORS: &[(&str, NativeFunction)] = &[
    ("EvalError", NativeFunction::EvalError),
    ("RangeError", NativeFunction::RangeError),
    ("ReferenceError", NativeFunction::ReferenceError),
    ("SyntaxError", NativeFunction::SyntaxError),
    ("TypeError", NativeFunction::TypeError),
    ("URIError", NativeFunction::UriError),
];

pub(crate) fn install_error(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
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
    let is_error_function =
        Function::new_native(Some("isError"), 1, NativeFunction::ErrorIsError, false);
    define_function_name(&is_error_function, "isError");
    error_function.properties.borrow_mut().insert(
        "isError".to_owned(),
        Property::non_enumerable(Value::Function(is_error_function)),
    );
    error_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(error_prototype)),
    );
    define_function_name(&error_function, "Error");

    let error_value = Value::Function(error_function);
    env.insert_realm("Error".to_owned(), error_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Error".to_owned(), error_value.clone());
    }

    let Value::Function(error_function) = error_value else {
        unreachable!("Error constructor must be a function");
    };
    let Some(Value::Object(error_prototype)) = error_function
        .properties
        .borrow()
        .get("prototype")
        .map(|property| property.value.clone())
    else {
        unreachable!("Error constructor must have a prototype");
    };

    for (name, native) in NATIVE_ERRORS {
        install_native_error(env, global_this, &error_prototype, name, *native);
    }
    install_native_error(
        env,
        global_this,
        &error_prototype,
        "AggregateError",
        NativeFunction::AggregateError,
    );
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

pub(crate) fn native_aggregate_error(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = match (is_construct, this_value) {
        (true, Value::Object(object)) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    define_error_data(&object);
    let errors = aggregate_error_errors(argument_values.first().cloned(), env)?;
    object.define_property(
        "errors".to_owned(),
        Property::data(errors, false, true, true),
    );
    if let Some(message) = argument_values.get(1) {
        if !matches!(message, Value::Undefined) {
            object.define_property(
                "message".to_owned(),
                Property::data(
                    Value::String(to_js_string_with_env(message.clone(), env)?),
                    false,
                    true,
                    true,
                ),
            );
        }
    }
    install_error_cause(&object, argument_values.get(2).cloned(), env)?;
    Ok(Value::Object(object))
}

fn aggregate_error_errors(errors: Option<Value>, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let Some(errors) = errors else {
        return Err(aggregate_error_errors_not_iterable());
    };
    match errors {
        Value::Array(errors) => Ok(Value::Array(errors)),
        Value::Undefined | Value::Null => Err(aggregate_error_errors_not_iterable()),
        value => {
            if aggregate_errors_is_iterable(value.clone(), env)? {
                let array_constructor = env.get("Array").unwrap_or(Value::Undefined);
                crate::array::native_array_from(array_constructor, &[value], env)
            } else {
                Err(aggregate_error_errors_not_iterable())
            }
        }
    }
}

fn aggregate_error_errors_not_iterable() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: AggregateError errors must be iterable".to_owned(),
    }
}

fn aggregate_errors_is_iterable(value: Value, env: &mut CallEnv) -> Result<bool, RuntimeError> {
    let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
        return Ok(false);
    };
    let iterator_method =
        property_value_key(value, &crate::PropertyKey::Symbol(iterator_symbol), env)?;
    Ok(!matches!(iterator_method, Value::Undefined | Value::Null))
}

fn install_error_cause(
    object: &ObjectRef,
    options: Option<Value>,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let Some(options) = options else {
        return Ok(());
    };
    if !is_object_value(&options) || !has_property(options.clone(), env, "cause")? {
        return Ok(());
    }
    let cause = property_value(options, "cause", env)?;
    object.define_property("cause".to_owned(), Property::data(cause, false, true, true));
    Ok(())
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}

pub(crate) fn is_native_error(native: NativeFunction) -> bool {
    native_error_name(native).is_some()
}

pub(crate) fn native_error_constructor_parent(function: &Function, env: &CallEnv) -> Option<Value> {
    function
        .native
        .filter(|native| is_native_error(*native))
        .and_then(|_| env.get("Error"))
}

pub(crate) fn native_error_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(RuntimeError {
            thrown: None,
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

pub(crate) fn native_error_is_error(argument_values: &[Value]) -> Value {
    Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Object(object)) if is_error_object(object)
    ))
}

pub(crate) fn is_error_object(object: &ObjectRef) -> bool {
    object.own_property(ERROR_DATA_PROPERTY).is_some()
}

/// Materializes the thrown JavaScript value for a `RuntimeError`.
///
/// Engine-internal errors carry their reason in `thrown` when one exists;
/// otherwise the `message` encodes a native error (`"TypeError: ..."`) that is
/// reconstructed into a real Error object using the realm's constructors. This
/// mirrors the VM's catch-path reification so values rejected from native code
/// (combinators, async functions) are observably `instanceof TypeError`.
pub(crate) fn runtime_error_to_value(error: RuntimeError, env: &CallEnv) -> Value {
    if let Some(thrown) = error.thrown {
        return *thrown;
    }
    let message = error
        .message
        .trim_start_matches("throw statement executed: ");
    let (constructor_name, detail) = split_native_error_message(message);
    if let Some(Value::Function(function)) = env.get(constructor_name) {
        if let Ok(value) = native_error(
            &function,
            Value::Undefined,
            &[Value::String(detail.clone())],
            false,
        ) {
            return value;
        }
    }
    Value::String(message.to_owned())
}

/// Splits a `"TypeError: detail"`-style message into its constructor name and
/// detail, defaulting to `TypeError` when no recognised prefix is present.
fn split_native_error_message(message: &str) -> (&'static str, String) {
    for (name, _) in NATIVE_ERRORS {
        if let Some(detail) = message
            .strip_prefix(name)
            .and_then(|rest| rest.strip_prefix(": "))
        {
            return (name, detail.to_owned());
        }
    }
    if let Some(detail) = message
        .strip_prefix("Error")
        .and_then(|rest| rest.strip_prefix(": "))
    {
        return ("Error", detail.to_owned());
    }
    ("TypeError", message.to_owned())
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

fn install_native_error(
    env: &mut CallEnv,
    global_this: &Value,
    error_prototype: &ObjectRef,
    name: &str,
    native: NativeFunction,
) {
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(error_prototype.clone()));
    let length = if name == "AggregateError" { 2 } else { 1 };
    let function = Function::new_native(Some(name), length, native, true);
    prototype.define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
    prototype.define_non_enumerable("name".to_owned(), Value::String(name.to_owned()));
    prototype.define_non_enumerable("message".to_owned(), Value::String(String::new()));
    function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(prototype)),
    );
    define_function_name(&function, name);

    let value = Value::Function(function);
    env.insert_realm(name.to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable(name.to_owned(), value);
    }
}

fn native_error_name(native: NativeFunction) -> Option<&'static str> {
    match native {
        NativeFunction::EvalError => Some("EvalError"),
        NativeFunction::RangeError => Some("RangeError"),
        NativeFunction::ReferenceError => Some("ReferenceError"),
        NativeFunction::SyntaxError => Some("SyntaxError"),
        NativeFunction::TypeError => Some("TypeError"),
        NativeFunction::UriError => Some("URIError"),
        NativeFunction::AggregateError => Some("AggregateError"),
        _ => None,
    }
}

fn define_function_name(function: &Function, name: &str) {
    function.properties.borrow_mut().insert(
        "name".to_owned(),
        Property::data(Value::String(name.to_owned()), false, false, true),
    );
}

fn object_message(object: &ObjectRef) -> Option<String> {
    match object.get("message") {
        Some(Value::String(value)) => Some(value),
        Some(Value::Undefined) | None => Some(String::new()),
        _ => None,
    }
}
