use std::collections::HashMap;

use crate::{
    ArrayRef, Function, MapRef, NativeFunction, ObjectRef, Property, RuntimeError, Value,
    array::array_like_values_with_env, call_function, property_value, symbol,
};

const MAP_ITERATOR: &str = "\0map_iterator";
const MAP_ITERATOR_NEXT_INDEX: &str = "\0map_iterator_next_index";
const MAP_ITERATOR_DONE: &str = "\0map_iterator_done";
const MAP_ITERATOR_KIND: &str = "\0map_iterator_kind";
const MAP_ITERATOR_KIND_KEY: &str = "key";
const MAP_ITERATOR_KIND_VALUE: &str = "value";
const MAP_ITERATOR_KIND_KEY_VALUE: &str = "key+value";

pub(crate) fn install_map(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let map_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    map_prototype.set_to_string_tag("Map");
    symbol::define_well_known_to_string_tag(env, &map_prototype, "Map");
    let map_function = Function::new_native(Some("Map"), 0, NativeFunction::Map, true);
    define_map_function(&map_function, "groupBy", 2, NativeFunction::MapGroupBy);
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
    define_map_prototype_function(
        &map_prototype,
        "entries",
        0,
        NativeFunction::MapPrototypeEntries,
    );
    define_map_prototype_function(
        &map_prototype,
        "forEach",
        1,
        NativeFunction::MapPrototypeForEach,
    );
    define_map_prototype_function(&map_prototype, "get", 1, NativeFunction::MapPrototypeGet);
    define_map_prototype_function(
        &map_prototype,
        "getOrInsert",
        2,
        NativeFunction::MapPrototypeGetOrInsert,
    );
    define_map_prototype_function(
        &map_prototype,
        "getOrInsertComputed",
        2,
        NativeFunction::MapPrototypeGetOrInsertComputed,
    );
    define_map_prototype_function(&map_prototype, "has", 1, NativeFunction::MapPrototypeHas);
    define_map_prototype_function(&map_prototype, "keys", 0, NativeFunction::MapPrototypeKeys);
    define_map_prototype_function(&map_prototype, "set", 2, NativeFunction::MapPrototypeSet);
    define_map_prototype_function(
        &map_prototype,
        "values",
        0,
        NativeFunction::MapPrototypeValues,
    );
    symbol::define_well_known_iterator_alias(env, &map_prototype, "entries");
    map_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(map_prototype)),
    );

    let value = Value::Function(map_function);
    env.insert("Map".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Map".to_owned(), value);
    }
}

pub(crate) fn native_map(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor Map requires 'new'".to_owned(),
        });
    }
    let map = MapRef::new(crate::function_prototype(function));
    let map_value = Value::Map(map);
    if let Some(iterable) = argument_values.first().cloned()
        && !matches!(iterable, Value::Undefined | Value::Null)
    {
        let adder = property_value(map_value.clone(), "set", env)?;
        if !matches!(adder, Value::Function(_)) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Map constructor set adder must be callable".to_owned(),
            });
        }
        for entry in array_like_values_with_env(iterable, "Map constructor", env)? {
            let (key, value) = map_entry(entry, env)?;
            call_function(
                adder.clone(),
                map_value.clone(),
                vec![key, value],
                env,
                false,
            )?;
        }
    }
    Ok(map_value)
}

pub(crate) fn native_map_group_by(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let items = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let callback = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Map.groupBy callback must be callable".to_owned(),
        });
    }

    let map = MapRef::new(map_prototype(env));
    for (index, value) in array_like_values_with_env(items, "Map.groupBy", env)?
        .into_iter()
        .enumerate()
    {
        let key = crate::call_function(
            callback.clone(),
            Value::Undefined,
            vec![value.clone(), Value::Number(index as f64)],
            env,
            false,
        )?;
        append_map_group(&map, key, value);
    }

    Ok(Value::Map(map))
}

fn map_entry(
    entry: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(Value, Value), RuntimeError> {
    match entry {
        Value::Object(object) if symbol::is_symbol_primitive(&object) => Err(RuntimeError {
            thrown: None,
            message: "TypeError: Map constructor entry must be an object".to_owned(),
        }),
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) => {
            let key = property_value(entry.clone(), "0", env)?;
            let value = property_value(entry, "1", env)?;
            Ok((key, value))
        }
        Value::Null
        | Value::Undefined
        | Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_) => Err(RuntimeError {
            thrown: None,
            message: "TypeError: Map constructor entry must be an object".to_owned(),
        }),
    }
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

pub(crate) fn native_map_prototype_get_or_insert(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let map = this_map(this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if let Some(value) = map.get(&key) {
        return Ok(value);
    }
    let value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    map.set(key, value.clone());
    Ok(value)
}

pub(crate) fn native_map_prototype_get_or_insert_computed(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let map = this_map(this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let callback = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Map.prototype.getOrInsertComputed callback must be callable"
                .to_owned(),
        });
    }
    if let Some(value) = map.get(&key) {
        return Ok(value);
    }
    let canonical_key = canonical_map_key_value(key);
    let value = crate::call_function(
        callback,
        Value::Undefined,
        vec![canonical_key.clone()],
        env,
        false,
    )?;
    map.delete(&canonical_key);
    map.set(canonical_key, value.clone());
    Ok(value)
}

pub(crate) fn native_map_prototype_entries(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    map_iterator(this_value, env, MAP_ITERATOR_KIND_KEY_VALUE)
}

pub(crate) fn native_map_prototype_for_each(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let map = this_map(this_value.clone())?;
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Map.prototype.forEach callback must be callable".to_owned(),
        });
    }
    let this_arg = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    for (key, value) in map.entries() {
        crate::call_function(
            callback.clone(),
            this_arg.clone(),
            vec![value, key, this_value.clone()],
            env,
            false,
        )?;
    }
    Ok(Value::Undefined)
}

pub(crate) fn native_map_prototype_has(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let map = this_map(this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(map.has(&key)))
}

pub(crate) fn native_map_prototype_keys(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    map_iterator(this_value, env, MAP_ITERATOR_KIND_KEY)
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

pub(crate) fn native_map_prototype_values(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    map_iterator(this_value, env, MAP_ITERATOR_KIND_VALUE)
}

pub(crate) fn native_map_iterator_next(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Object(iterator) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Map iterator next called on non-object".to_owned(),
        });
    };
    if iterator_done(&iterator) {
        return Ok(iterator_result(Value::Undefined, true));
    }

    let map = match iterator_slot(&iterator, MAP_ITERATOR)? {
        Value::Map(map) => map,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Map iterator target is invalid".to_owned(),
            });
        }
    };
    let entries = map.entries();
    let index = iterator_index(&iterator)?;
    if index >= entries.len() {
        iterator.define_non_enumerable(MAP_ITERATOR_DONE.to_owned(), Value::Boolean(true));
        return Ok(iterator_result(Value::Undefined, true));
    }
    iterator.define_non_enumerable(
        MAP_ITERATOR_NEXT_INDEX.to_owned(),
        Value::Number((index + 1) as f64),
    );

    let (key, value) = entries[index].clone();
    let item = match iterator_kind(&iterator)?.as_str() {
        MAP_ITERATOR_KIND_KEY => key,
        MAP_ITERATOR_KIND_VALUE => value,
        MAP_ITERATOR_KIND_KEY_VALUE => Value::Array(ArrayRef::new(vec![key, value])),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Map iterator kind is invalid".to_owned(),
            });
        }
    };
    Ok(iterator_result(item, false))
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

fn map_iterator(
    this_value: Value,
    env: &HashMap<String, Value>,
    kind: &str,
) -> Result<Value, RuntimeError> {
    this_map(this_value.clone())?;
    let iterator = ObjectRef::new(HashMap::new());
    iterator.define_non_enumerable(MAP_ITERATOR.to_owned(), this_value);
    iterator.define_non_enumerable(MAP_ITERATOR_NEXT_INDEX.to_owned(), Value::Number(0.0));
    iterator.define_non_enumerable(MAP_ITERATOR_DONE.to_owned(), Value::Boolean(false));
    iterator.define_non_enumerable(MAP_ITERATOR_KIND.to_owned(), Value::String(kind.to_owned()));
    iterator.define_non_enumerable(
        "next".to_owned(),
        Value::Function(Function::new_native(
            Some("next"),
            0,
            NativeFunction::MapIteratorPrototypeNext,
            false,
        )),
    );
    symbol::define_iterator_identity(env, &iterator);
    Ok(Value::Object(iterator))
}

fn iterator_done(iterator: &ObjectRef) -> bool {
    matches!(
        iterator
            .own_property(MAP_ITERATOR_DONE)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    )
}

fn iterator_index(iterator: &ObjectRef) -> Result<usize, RuntimeError> {
    match iterator_slot(iterator, MAP_ITERATOR_NEXT_INDEX)? {
        Value::Number(index) if index >= 0.0 => Ok(index as usize),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Map iterator next index is invalid".to_owned(),
        }),
    }
}

fn iterator_slot(iterator: &ObjectRef, key: &str) -> Result<Value, RuntimeError> {
    iterator
        .own_property(key)
        .map(|property| property.value)
        .ok_or_else(|| RuntimeError {
            thrown: None,
            message: "Map iterator is missing internal state".to_owned(),
        })
}

fn iterator_kind(iterator: &ObjectRef) -> Result<String, RuntimeError> {
    match iterator_slot(iterator, MAP_ITERATOR_KIND)? {
        Value::String(kind) => Ok(kind),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Map iterator kind is invalid".to_owned(),
        }),
    }
}

fn iterator_result(value: Value, done: bool) -> Value {
    let mut properties = HashMap::new();
    properties.insert("value".to_owned(), value);
    properties.insert("done".to_owned(), Value::Boolean(done));
    Value::Object(ObjectRef::new(properties))
}

fn canonical_map_key_value(key: Value) -> Value {
    if matches!(key, Value::Number(value) if value == 0.0) {
        Value::Number(0.0)
    } else {
        key
    }
}

fn append_map_group(map: &MapRef, key: Value, value: Value) {
    match map.get(&key) {
        Some(Value::Array(group)) => group.set(group.len(), value),
        _ => map.set(key, Value::Array(ArrayRef::new(vec![value]))),
    }
}

fn map_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    match env.get("Map") {
        Some(Value::Function(function)) => crate::function_prototype(function),
        _ => None,
    }
}

fn define_map_function(function: &Function, key: &str, length: usize, native: NativeFunction) {
    function.properties.borrow_mut().insert(
        key.to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some(key),
            length,
            native,
            false,
        ))),
    );
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
