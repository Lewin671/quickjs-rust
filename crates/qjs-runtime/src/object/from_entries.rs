use std::collections::HashMap;

use crate::{
    ArrayRef, Function, ObjectRef, Property, RuntimeError, Value, array_prototype_property,
    function_prototype_property, object_prototype, to_property_key_value,
};

use crate::array::array_like_values;

pub(crate) fn native_object_from_entries(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let iterable = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let result = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));

    for entry in array_like_values(iterable, "Object.fromEntries")? {
        let key = to_property_key_value(entry_component(entry.clone(), 0, env)?)?;
        let value = entry_component(entry, 1, env)?;
        match key {
            crate::PropertyKey::String(key) => {
                result.define_property(key, Property::enumerable(value));
            }
            crate::PropertyKey::Symbol(symbol) => {
                result.define_symbol_property(symbol, Property::enumerable(value));
            }
        }
    }

    Ok(Value::Object(result))
}

fn entry_component(
    entry: Value,
    index: usize,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match entry {
        Value::Array(array) => Ok(array_entry_component(&array, index, env)),
        Value::Object(object) => Ok(object.get(&index.to_string()).unwrap_or(Value::Undefined)),
        Value::Function(function) => Ok(function_entry_component(&function, index, env)),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Object.fromEntries entry must be an object".to_owned(),
        }),
    }
}

fn array_entry_component(array: &ArrayRef, index: usize, env: &HashMap<String, Value>) -> Value {
    array
        .get(index)
        .or_else(|| array_prototype_property(array, env, &index.to_string()))
        .unwrap_or(Value::Undefined)
}

fn function_entry_component(
    function: &Function,
    index: usize,
    env: &HashMap<String, Value>,
) -> Value {
    let key = index.to_string();
    function
        .properties
        .borrow()
        .get(&key)
        .map(|property| property.value.clone())
        .or_else(|| function_prototype_property(function, env, &key))
        .unwrap_or(Value::Undefined)
}
