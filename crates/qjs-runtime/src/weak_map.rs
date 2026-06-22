use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, RuntimeError, Value,
    array::for_each_iterable_value_with_env, call_function, property_value, symbol,
};

const WEAK_MAP_ENTRIES: &str = "\0weak_map_entries";
const WEAK_MAP_BRAND: &str = "\0weak_map";

pub(crate) fn install_weak_map(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let weak_map_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    weak_map_prototype.set_to_string_tag("WeakMap");
    symbol::define_well_known_to_string_tag(env, &weak_map_prototype, "WeakMap");
    let weak_map_function = Function::new_native(Some("WeakMap"), 0, NativeFunction::WeakMap, true);
    weak_map_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(weak_map_function.clone()),
    );
    define_weak_map_prototype_function(
        &weak_map_prototype,
        "delete",
        1,
        NativeFunction::WeakMapPrototypeDelete,
    );
    define_weak_map_prototype_function(
        &weak_map_prototype,
        "get",
        1,
        NativeFunction::WeakMapPrototypeGet,
    );
    define_weak_map_prototype_function(
        &weak_map_prototype,
        "getOrInsert",
        2,
        NativeFunction::WeakMapPrototypeGetOrInsert,
    );
    define_weak_map_prototype_function(
        &weak_map_prototype,
        "getOrInsertComputed",
        2,
        NativeFunction::WeakMapPrototypeGetOrInsertComputed,
    );
    define_weak_map_prototype_function(
        &weak_map_prototype,
        "has",
        1,
        NativeFunction::WeakMapPrototypeHas,
    );
    define_weak_map_prototype_function(
        &weak_map_prototype,
        "set",
        2,
        NativeFunction::WeakMapPrototypeSet,
    );
    weak_map_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        crate::Property::fixed_non_enumerable(Value::Object(weak_map_prototype)),
    );

    let value = Value::Function(weak_map_function);
    env.insert_realm("WeakMap".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("WeakMap".to_owned(), value);
    }
}

pub(crate) fn native_weak_map(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor WeakMap requires 'new'".to_owned(),
        });
    }

    let object = ObjectRef::with_prototype_slot(
        HashMap::new(),
        crate::native_construct_prototype_slot(function, env)?,
    );
    object.set_to_string_tag("WeakMap");
    object.define_non_enumerable(WEAK_MAP_BRAND.to_owned(), Value::Boolean(true));
    object.define_non_enumerable(
        WEAK_MAP_ENTRIES.to_owned(),
        Value::Array(ArrayRef::new(Vec::new())),
    );
    let weak_map = Value::Object(object);

    if let Some(iterable) = argument_values.first().cloned()
        && !matches!(iterable, Value::Undefined | Value::Null)
    {
        let adder = property_value(weak_map.clone(), "set", env)?;
        if !matches!(adder, Value::Function(_)) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: WeakMap constructor set adder must be callable".to_owned(),
            });
        }
        for_each_iterable_value_with_env(iterable, "WeakMap constructor", env, |entry, env| {
            let (key, value) = weak_map_entry(entry, env)?;
            call_function(
                adder.clone(),
                weak_map.clone(),
                vec![key, value],
                env,
                false,
            )?;
            Ok(())
        })?;
    }

    Ok(weak_map)
}

fn weak_map_entry(entry: Value, env: &mut CallEnv) -> Result<(Value, Value), RuntimeError> {
    match entry {
        Value::Object(_)
        | Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_) => {
            let key = property_value(entry.clone(), "0", env)?;
            let value = property_value(entry, "1", env)?;
            Ok((key, value))
        }
        Value::Null
        | Value::Undefined
        | Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_) => Err(RuntimeError {
            thrown: None,
            message: "TypeError: WeakMap constructor entry must be an object".to_owned(),
        }),
    }
}

pub(crate) fn native_weak_map_prototype_delete(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = weak_map_object(&this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_weak_map_key(&key) {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(weak_map_delete(object, &key)))
}

pub(crate) fn native_weak_map_prototype_get(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = weak_map_object(&this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_weak_map_key(&key) {
        return Ok(Value::Undefined);
    }
    Ok(weak_map_get(object, &key).unwrap_or(Value::Undefined))
}

pub(crate) fn native_weak_map_prototype_get_or_insert(
    this_value: Value,
    argument_values: &[Value],
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let object = weak_map_object(&this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !can_be_held_weakly(&key, env) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: WeakMap key must be an object".to_owned(),
        });
    }
    if let Some(value) = weak_map_get(object.clone(), &key) {
        return Ok(value);
    }
    let value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    weak_map_set(object, key, value.clone(), env)?;
    Ok(value)
}

pub(crate) fn native_weak_map_prototype_get_or_insert_computed(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = weak_map_object(&this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let callback = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: WeakMap.prototype.getOrInsertComputed callback must be callable"
                .to_owned(),
        });
    }
    if !can_be_held_weakly(&key, env) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: WeakMap key must be an object".to_owned(),
        });
    }
    if let Some(value) = weak_map_get(object.clone(), &key) {
        return Ok(value);
    }
    let value = crate::call_function(callback, Value::Undefined, vec![key.clone()], env, false)?;
    weak_map_set(object, key, value.clone(), env)?;
    Ok(value)
}

pub(crate) fn native_weak_map_prototype_has(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = weak_map_object(&this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_weak_map_key(&key) {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(weak_map_has(object, &key)))
}

pub(crate) fn native_weak_map_prototype_set(
    this_value: Value,
    argument_values: &[Value],
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let object = weak_map_object(&this_value)?;
    let key = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !can_be_held_weakly(&key, env) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: WeakMap key must be an object".to_owned(),
        });
    }
    let value = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    weak_map_set(object, key, value, env)?;
    Ok(this_value)
}

fn weak_map_object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    let Value::Object(object) = value else {
        return Err(incompatible_receiver());
    };
    if matches!(
        object
            .own_property(WEAK_MAP_BRAND)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    ) {
        Ok(object.clone())
    } else {
        Err(incompatible_receiver())
    }
}

fn weak_map_entries(object: &ObjectRef) -> Result<ArrayRef, RuntimeError> {
    match object
        .own_property(WEAK_MAP_ENTRIES)
        .map(|property| property.value)
    {
        Some(Value::Array(entries)) => Ok(entries),
        _ => Err(RuntimeError {
            thrown: None,
            message: "WeakMap is missing internal state".to_owned(),
        }),
    }
}

fn weak_map_get(object: ObjectRef, key: &Value) -> Option<Value> {
    weak_map_entries(&object)
        .ok()?
        .to_vec()
        .into_iter()
        .find_map(|entry| match entry {
            Value::Array(pair)
                if pair
                    .get(0)
                    .is_some_and(|entry_key| entry_key.same_value(key)) =>
            {
                pair.get(1)
            }
            _ => None,
        })
}

fn weak_map_has(object: ObjectRef, key: &Value) -> bool {
    weak_map_get(object, key).is_some()
}

fn weak_map_set(
    object: ObjectRef,
    key: Value,
    value: Value,
    env: &CallEnv,
) -> Result<(), RuntimeError> {
    if !can_be_held_weakly(&key, env) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: WeakMap key must be an object".to_owned(),
        });
    }
    let entries = weak_map_entries(&object)?;
    let mut values = entries.to_vec();
    if let Some(pair) = values.iter_mut().find_map(|entry| match entry {
        Value::Array(pair)
            if pair
                .get(0)
                .is_some_and(|entry_key| entry_key.same_value(&key)) =>
        {
            Some(pair)
        }
        _ => None,
    }) {
        pair.set(1, value);
        return Ok(());
    }
    values.push(Value::Array(ArrayRef::new(vec![key, value])));
    entries.replace_with(values);
    Ok(())
}

fn weak_map_delete(object: ObjectRef, key: &Value) -> bool {
    let Ok(entries) = weak_map_entries(&object) else {
        return false;
    };
    let mut values = entries.to_vec();
    let Some(index) = values.iter().position(|entry| match entry {
        Value::Array(pair) => pair
            .get(0)
            .is_some_and(|entry_key| entry_key.same_value(key)),
        _ => false,
    }) else {
        return false;
    };
    values.remove(index);
    entries.replace_with(values);
    true
}

fn is_weak_map_key(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}

fn can_be_held_weakly(value: &Value, env: &CallEnv) -> bool {
    match value {
        Value::Object(object) if symbol::is_symbol_primitive(object) => {
            !symbol::is_registered_symbol(object, env)
        }
        value => is_weak_map_key(value),
    }
}

fn incompatible_receiver() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: incompatible WeakMap receiver".to_owned(),
    }
}

fn define_weak_map_prototype_function(
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
