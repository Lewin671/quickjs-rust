use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
};

const SYMBOL_DATA_PROPERTY: &str = "\0SymbolData";

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

pub(crate) fn native_symbol(function: &Function) -> Result<Value, RuntimeError> {
    let object = ObjectRef::with_prototype(HashMap::new(), function_prototype(function));
    object.define_non_enumerable(SYMBOL_DATA_PROPERTY.to_owned(), Value::Boolean(true));
    Ok(Value::Object(object))
}

pub(crate) fn is_symbol_object(object: &ObjectRef) -> bool {
    object.own_property(SYMBOL_DATA_PROPERTY).is_some()
}
