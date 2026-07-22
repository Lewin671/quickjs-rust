use std::collections::HashMap;

use crate::{
    Function, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    bigint::BIGINT_DATA_PROPERTY,
    boolean::BOOLEAN_DATA_PROPERTY,
    function_prototype,
    number::NUMBER_DATA_PROPERTY,
    string::{self, STRING_DATA_PROPERTY},
    symbol,
};

use super::descriptor::native_object_define_properties;
use super::enumeration::enumerable_property_entries_with_symbols;
use crate::CallEnv;

pub(crate) fn native_object_assign(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Object(object) if symbol::is_symbol_primitive(&object) => {
            symbol::boxed_symbol(&object, env)
        }
        value @ (Value::Array(_)
        | Value::Object(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_)) => value,
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.assign target must not be null or undefined".to_owned(),
            });
        }
        value @ (Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Boolean(_)) => {
            boxed_primitive(value, env).expect("primitive value should box")
        }
    };

    for source in argument_values.iter().skip(1).cloned() {
        if matches!(source, Value::Null | Value::Undefined) {
            continue;
        }
        for (key, value) in enumerable_property_entries_with_symbols(source, env)? {
            assign_property(target.clone(), key, value, env)?;
        }
    }
    Ok(target)
}

pub(crate) fn native_object(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    // Object([value]) step 1: when NewTarget is a subclass (not the active
    // Object constructor), return OrdinaryCreateFromConstructor(NewTarget,
    // "%Object.prototype%") and ignore `value`. The receiver was already
    // allocated from NewTarget.prototype, so return it as-is.
    if is_construct
        && let Some(Value::Function(new_target)) = env.get(crate::NEW_TARGET_BINDING)
        && !new_target.ptr_eq(function)
    {
        return Ok(this_value);
    }
    match argument_values.first() {
        Some(Value::Object(object)) if symbol::is_symbol_primitive(object) => {
            Ok(symbol::boxed_symbol(object, env))
        }
        Some(
            Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Object(_)
            | Value::Proxy(_),
        ) => Ok(argument_values[0].clone()),
        Some(Value::Boolean(value)) => Ok(boxed_boolean(*value, env)),
        Some(Value::BigInt(value)) => Ok(boxed_bigint(value.as_ref().clone(), env)),
        Some(Value::Number(value)) => Ok(boxed_number(*value, env)),
        Some(Value::String(value)) => Ok(boxed_string(value, env)),
        _ if is_construct => Ok(this_value),
        _ => Ok(Value::Object(ObjectRef::with_prototype(
            HashMap::new(),
            function_prototype(function),
        ))),
    }
}

pub(crate) fn boxed_primitive(value: Value, env: &CallEnv) -> Option<Value> {
    match value {
        Value::Boolean(value) => Some(boxed_boolean(value, env)),
        Value::BigInt(value) => Some(boxed_bigint(std::rc::Rc::unwrap_or_clone(value), env)),
        Value::Number(value) => Some(boxed_number(value, env)),
        Value::String(value) => Some(boxed_string(&value, env)),
        Value::Object(object) if symbol::is_symbol_primitive(&object) => {
            Some(symbol::boxed_symbol(&object, env))
        }
        _ => None,
    }
}

fn boxed_bigint(value: num_bigint::BigInt, env: &CallEnv) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), constructor_prototype("BigInt", env));
    object.define_non_enumerable(BIGINT_DATA_PROPERTY.to_owned(), Value::bigint(value));
    Value::Object(object)
}

fn boxed_boolean(value: bool, env: &CallEnv) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), constructor_prototype("Boolean", env));
    object.define_non_enumerable(BOOLEAN_DATA_PROPERTY.to_owned(), Value::Boolean(value));
    Value::Object(object)
}

fn boxed_number(value: f64, env: &CallEnv) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), constructor_prototype("Number", env));
    object.define_non_enumerable(NUMBER_DATA_PROPERTY.to_owned(), Value::Number(value));
    Value::Object(object)
}

fn boxed_string(value: &str, env: &CallEnv) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), constructor_prototype("String", env));
    object.define_non_enumerable(
        STRING_DATA_PROPERTY.to_owned(),
        Value::String(value.to_owned().into()),
    );
    object.define_property(
        "length".to_owned(),
        Property::data(
            Value::Number(string::string_code_unit_len(value) as f64),
            false,
            false,
            false,
        ),
    );
    for (index, code_unit) in string::string_code_units(value).into_iter().enumerate() {
        object.define_property(
            index.to_string(),
            Property::data(
                Value::String(string::string_from_code_unit(code_unit).into()),
                true,
                false,
                false,
            ),
        );
    }
    Value::Object(object)
}

fn constructor_prototype(name: &str, env: &CallEnv) -> Option<ObjectRef> {
    let Some(Value::Function(function)) = env.get(name) else {
        return None;
    };
    function_prototype(&function)
}

pub(crate) fn native_object_create(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = match argument_values.first() {
        Some(Value::Object(prototype)) => Value::Object(ObjectRef::with_prototype_slot(
            HashMap::new(),
            Some(crate::Prototype::Object(prototype.clone())),
        )),
        Some(Value::Function(prototype)) => Value::Object(ObjectRef::with_prototype_slot(
            HashMap::new(),
            Some(crate::Prototype::Function(prototype.clone())),
        )),
        Some(Value::Proxy(prototype)) => Value::Object(ObjectRef::with_prototype_slot(
            HashMap::new(),
            Some(crate::Prototype::Proxy(prototype.clone())),
        )),
        Some(Value::Array(prototype)) => Value::Object(ObjectRef::with_prototype_slot(
            HashMap::new(),
            Some(crate::array_as_prototype_slot(prototype, env)),
        )),
        Some(Value::Null) => Value::Object(ObjectRef::new(HashMap::new())),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.create prototype must be an object or null".to_owned(),
            });
        }
    };

    if !matches!(argument_values.get(1), None | Some(Value::Undefined)) {
        native_object_define_properties(
            &[
                object.clone(),
                argument_values.get(1).cloned().unwrap_or(Value::Undefined),
            ],
            env,
        )?;
    }
    Ok(object)
}

pub(crate) fn native_object_is(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let left = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let right = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(left.same_value(&right)))
}

fn assign_property(
    target: Value,
    key: PropertyKey,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    if crate::reflect::ordinary_set(target.clone(), &key, value, target, env)? {
        return Ok(());
    }
    let key = match key {
        PropertyKey::String(key) => key,
        PropertyKey::Symbol(_) => "[symbol]".to_owned(),
    };
    Err(RuntimeError {
        thrown: None,
        message: format!("Object.assign could not set property `{key}`"),
    })
}
