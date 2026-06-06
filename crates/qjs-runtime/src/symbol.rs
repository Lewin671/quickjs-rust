use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    to_js_string_with_env,
};

const SYMBOL_DATA_PROPERTY: &str = "\0SymbolData";
const SYMBOL_DESCRIPTION_PROPERTY: &str = "\0SymbolDescription";

pub(crate) fn install_symbol(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let symbol_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    symbol_prototype.set_to_string_tag("Symbol");

    let symbol_function = Function::new_native(Some("Symbol"), 0, NativeFunction::Symbol, false);
    symbol_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(symbol_function.clone()),
    );
    symbol_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::SymbolPrototypeToString,
            false,
        )),
    );
    symbol_prototype.define_non_enumerable(
        "valueOf".to_owned(),
        Value::Function(Function::new_native(
            Some("valueOf"),
            0,
            NativeFunction::SymbolPrototypeValueOf,
            false,
        )),
    );
    symbol_prototype.define_property(
        "description".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get description"),
                0,
                NativeFunction::SymbolPrototypeDescription,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
    symbol_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(symbol_prototype)),
    );

    let symbol_value = Value::Function(symbol_function);
    env.insert("Symbol".to_owned(), symbol_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Symbol".to_owned(), symbol_value);
    }
}

pub(crate) fn native_symbol(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let description = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => Value::Undefined,
        value => Value::String(to_js_string_with_env(value, env)?),
    };
    let object = ObjectRef::with_prototype(HashMap::new(), function_prototype(function));
    object.define_non_enumerable(SYMBOL_DATA_PROPERTY.to_owned(), Value::Boolean(true));
    object.define_non_enumerable(SYMBOL_DESCRIPTION_PROPERTY.to_owned(), description);
    Ok(Value::Object(object))
}

pub(crate) fn is_symbol_object(object: &ObjectRef) -> bool {
    object.own_property(SYMBOL_DATA_PROPERTY).is_some()
}

pub(crate) fn native_symbol_prototype_description(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    this_symbol_description(this_value)
}

pub(crate) fn native_symbol_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::String(symbol_descriptive_string(
        this_symbol_description(this_value)?,
    )))
}

pub(crate) fn native_symbol_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(symbol_method_error());
    };
    if !is_symbol_object(&object) {
        return Err(symbol_method_error());
    }
    Ok(Value::Object(object))
}

fn this_symbol_description(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(symbol_method_error());
    };
    if !is_symbol_object(&object) {
        return Err(symbol_method_error());
    }
    Ok(object
        .own_property(SYMBOL_DESCRIPTION_PROPERTY)
        .map(|property| property.value)
        .unwrap_or(Value::Undefined))
}

fn symbol_descriptive_string(description: Value) -> String {
    match description {
        Value::String(description) => format!("Symbol({description})"),
        _ => "Symbol()".to_owned(),
    }
}

fn symbol_method_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "Symbol.prototype method called on non-symbol".to_owned(),
    }
}
