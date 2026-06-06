use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    to_js_string_with_env,
};

const SYMBOL_DATA_PROPERTY: &str = "\0SymbolData";
const SYMBOL_DESCRIPTION_PROPERTY: &str = "\0SymbolDescription";
const SYMBOL_REGISTRY_BINDING: &str = "\0SymbolRegistry";

pub(crate) fn install_symbol(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let symbol_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    symbol_prototype.set_to_string_tag("Symbol");

    let symbol_function = Function::new_native(Some("Symbol"), 0, NativeFunction::Symbol, false);
    symbol_function.properties.borrow_mut().insert(
        "for".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("for"),
            1,
            NativeFunction::SymbolFor,
            false,
        ))),
    );
    symbol_function.properties.borrow_mut().insert(
        "keyFor".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("keyFor"),
            1,
            NativeFunction::SymbolKeyFor,
            false,
        ))),
    );
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
    Ok(Value::Object(symbol_object(
        function_prototype(function),
        description,
    )))
}

pub(crate) fn native_symbol_for(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let key = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let registry = symbol_registry(env);
    if let Some(property) = registry.own_property(&key) {
        return Ok(property.value);
    }

    let symbol = Value::Object(symbol_object(
        symbol_prototype(env),
        Value::String(key.clone()),
    ));
    registry.define_non_enumerable(key, symbol.clone());
    Ok(symbol)
}

pub(crate) fn native_symbol_key_for(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Value::Object(target_object) = target else {
        return Err(symbol_method_error());
    };
    if !is_symbol_object(&target_object) {
        return Err(symbol_method_error());
    }

    let registry = symbol_registry(env);
    for key in registry.own_property_names() {
        if matches!(
            registry.own_property(&key).map(|property| property.value),
            Some(Value::Object(symbol)) if symbol.ptr_eq(&target_object)
        ) {
            return Ok(Value::String(key));
        }
    }
    Ok(Value::Undefined)
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

fn symbol_object(prototype: Option<ObjectRef>, description: Value) -> ObjectRef {
    let object = ObjectRef::with_prototype(HashMap::new(), prototype);
    object.define_non_enumerable(SYMBOL_DATA_PROPERTY.to_owned(), Value::Boolean(true));
    object.define_non_enumerable(SYMBOL_DESCRIPTION_PROPERTY.to_owned(), description);
    object
}

fn symbol_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(symbol_function)) = env.get("Symbol") else {
        return None;
    };
    function_prototype(symbol_function)
}

fn symbol_registry(env: &mut HashMap<String, Value>) -> ObjectRef {
    if let Some(Value::Object(registry)) = env.get(SYMBOL_REGISTRY_BINDING) {
        return registry.clone();
    }
    let registry = ObjectRef::new(HashMap::new());
    env.insert(
        SYMBOL_REGISTRY_BINDING.to_owned(),
        Value::Object(registry.clone()),
    );
    registry
}
