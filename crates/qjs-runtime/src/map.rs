use std::collections::HashMap;

use crate::{Function, MapRef, NativeFunction, ObjectRef, Property, RuntimeError, Value};

pub(crate) fn install_map(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let map_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    map_prototype.set_to_string_tag("Map");
    let map_function = Function::new_native(Some("Map"), 0, NativeFunction::Map, true);
    map_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(map_function.clone()),
    );
    map_prototype.define_property(
        "size".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get size"),
                0,
                NativeFunction::MapPrototypeSize,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
    define_map_prototype_function(
        &map_prototype,
        "clear",
        0,
        NativeFunction::MapPrototypeClear,
    );
    define_map_prototype_function(
        &map_prototype,
        "delete",
        1,
        NativeFunction::MapPrototypeDelete,
    );
    define_map_prototype_function(&map_prototype, "get", 1, NativeFunction::MapPrototypeGet);
    define_map_prototype_function(&map_prototype, "has", 1, NativeFunction::MapPrototypeHas);
    define_map_prototype_function(&map_prototype, "set", 2, NativeFunction::MapPrototypeSet);
    map_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(map_prototype)),
    );

    let value = Value::Function(map_function);
    env.insert("Map".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Map".to_owned(), value);
    }
}

pub(crate) fn native_map(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor Map requires 'new'".to_owned(),
        });
    }
    if !matches!(argument_values.first(), None | Some(Value::Undefined)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Map iterable constructor arguments are not implemented".to_owned(),
        });
    }
    Ok(Value::Map(MapRef::new(crate::function_prototype(function))))
}

pub(crate) fn native_map_prototype_size(this_value: Value) -> Result<Value, RuntimeError> {
    let map = this_map(this_value)?;
    Ok(Value::Number(map.len() as f64))
}

pub(crate) fn native_map_prototype_clear(this_value: Value) -> Result<Value, RuntimeError> {
    let map = this_map(this_value)?;
    map.clear();
    Ok(Value::Undefined)
}

pub(crate) fn native_map_prototype_delete(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let map = this_map(this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(map.delete(&key)))
}

pub(crate) fn native_map_prototype_get(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let map = this_map(this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(map.get(&key).unwrap_or(Value::Undefined))
}

pub(crate) fn native_map_prototype_has(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let map = this_map(this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(map.has(&key)))
}

pub(crate) fn native_map_prototype_set(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let map = this_map(this_value.clone())?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    map.set(key, value);
    Ok(this_value)
}

fn this_map(this_value: Value) -> Result<MapRef, RuntimeError> {
    match this_value {
        Value::Map(map) => Ok(map),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: incompatible Map receiver".to_owned(),
        }),
    }
}

fn define_map_prototype_function(
    prototype: &ObjectRef,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    prototype.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}
