use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    inherited_object_prototype_property, is_truthy,
};

pub(crate) const BOOLEAN_DATA_PROPERTY: &str = "\0BooleanData";

pub(super) fn install_boolean(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let boolean_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    let boolean_function = Function::new_native(Some("Boolean"), 1, NativeFunction::Boolean, true);
    boolean_prototype
        .define_non_enumerable(BOOLEAN_DATA_PROPERTY.to_owned(), Value::Boolean(false));
    boolean_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(boolean_function.clone()),
    );
    boolean_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::BooleanPrototypeToString,
            false,
        )),
    );
    boolean_prototype.define_non_enumerable(
        "valueOf".to_owned(),
        Value::Function(Function::new_native(
            Some("valueOf"),
            0,
            NativeFunction::BooleanPrototypeValueOf,
            false,
        )),
    );
    boolean_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::data(Value::Object(boolean_prototype), false, false, false),
    );
    let boolean_value = Value::Function(boolean_function);
    env.insert("Boolean".to_owned(), boolean_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Boolean".to_owned(), boolean_value);
    }
}

fn boolean_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(boolean_function)) = env.get("Boolean") else {
        return None;
    };
    function_prototype(boolean_function)
}

pub(super) fn native_boolean(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().is_some_and(is_truthy);
    if !is_construct {
        return Ok(Value::Boolean(value));
    }

    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    object.define_non_enumerable(BOOLEAN_DATA_PROPERTY.to_owned(), Value::Boolean(value));
    Ok(Value::Object(object))
}

pub(super) fn native_boolean_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::String(if this_boolean_value(this_value)? {
        "true".to_owned()
    } else {
        "false".to_owned()
    }))
}

pub(super) fn native_boolean_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(this_boolean_value(this_value)?))
}

fn this_boolean_value(value: Value) -> Result<bool, RuntimeError> {
    match value {
        Value::Boolean(value) => Ok(value),
        Value::Object(object) => match object.own_property(BOOLEAN_DATA_PROPERTY) {
            Some(Property {
                value: Value::Boolean(value),
                ..
            }) => Ok(value),
            _ => Err(RuntimeError {
                thrown: None,
                message: "Boolean.prototype method called on non-boolean object".to_owned(),
            }),
        },
        _ => Err(RuntimeError {
            thrown: None,
            message: "Boolean.prototype method called on non-boolean".to_owned(),
        }),
    }
}

pub(super) fn inherited_boolean_prototype_property(
    env: &HashMap<String, Value>,
    key: &str,
) -> Option<Value> {
    boolean_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}

pub(super) fn is_boolean_object(object: &ObjectRef) -> bool {
    matches!(
        object.own_property(BOOLEAN_DATA_PROPERTY),
        Some(Property {
            value: Value::Boolean(_),
            ..
        })
    )
}
