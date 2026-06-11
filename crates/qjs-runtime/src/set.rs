use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, Property, RuntimeError, SetRef, Value,
    array::iterable_values_with_env, call_function, property_value, symbol,
};

mod composition;
use crate::CallEnv;
use composition::SetRecord;

const SET_ITERATOR: &str = "\0set_iterator";
const SET_ITERATOR_NEXT_INDEX: &str = "\0set_iterator_next_index";
const SET_ITERATOR_DONE: &str = "\0set_iterator_done";
const SET_ITERATOR_KIND: &str = "\0set_iterator_kind";
const SET_ITERATOR_KIND_VALUE: &str = "value";
const SET_ITERATOR_KIND_KEY_VALUE: &str = "key+value";

pub(crate) fn install_set(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
    let set_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    set_prototype.set_to_string_tag("Set");
    symbol::define_well_known_to_string_tag(env, &set_prototype, "Set");
    let set_function = Function::new_native(Some("Set"), 0, NativeFunction::Set, true);
    set_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(set_function.clone()),
    );
    set_prototype.define_property(
        "size".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get size"),
                0,
                NativeFunction::SetPrototypeSize,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
    define_set_prototype_function(&set_prototype, "add", 1, NativeFunction::SetPrototypeAdd);
    define_set_prototype_function(
        &set_prototype,
        "clear",
        0,
        NativeFunction::SetPrototypeClear,
    );
    define_set_prototype_function(
        &set_prototype,
        "delete",
        1,
        NativeFunction::SetPrototypeDelete,
    );
    define_set_prototype_function(
        &set_prototype,
        "difference",
        1,
        NativeFunction::SetPrototypeDifference,
    );
    define_set_prototype_function(
        &set_prototype,
        "entries",
        0,
        NativeFunction::SetPrototypeEntries,
    );
    define_set_prototype_function(
        &set_prototype,
        "forEach",
        1,
        NativeFunction::SetPrototypeForEach,
    );
    define_set_prototype_function(&set_prototype, "has", 1, NativeFunction::SetPrototypeHas);
    define_set_prototype_function(
        &set_prototype,
        "intersection",
        1,
        NativeFunction::SetPrototypeIntersection,
    );
    define_set_prototype_function(
        &set_prototype,
        "isDisjointFrom",
        1,
        NativeFunction::SetPrototypeIsDisjointFrom,
    );
    define_set_prototype_function(
        &set_prototype,
        "isSubsetOf",
        1,
        NativeFunction::SetPrototypeIsSubsetOf,
    );
    define_set_prototype_function(
        &set_prototype,
        "isSupersetOf",
        1,
        NativeFunction::SetPrototypeIsSupersetOf,
    );
    let values_function = Value::Function(Function::new_native(
        Some("values"),
        0,
        NativeFunction::SetPrototypeValues,
        false,
    ));
    set_prototype.define_non_enumerable("keys".to_owned(), values_function.clone());
    define_set_prototype_function(
        &set_prototype,
        "symmetricDifference",
        1,
        NativeFunction::SetPrototypeSymmetricDifference,
    );
    define_set_prototype_function(
        &set_prototype,
        "union",
        1,
        NativeFunction::SetPrototypeUnion,
    );
    set_prototype.define_non_enumerable("values".to_owned(), values_function);
    symbol::define_well_known_iterator_alias(env, &set_prototype, "values");
    set_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(set_prototype)),
    );
    symbol::define_species_accessor(env, &set_function);

    let value = Value::Function(set_function);
    env.insert_realm("Set".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Set".to_owned(), value);
    }
}

pub(crate) fn native_set(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor Set requires 'new'".to_owned(),
        });
    }
    let set = SetRef::new(crate::function_prototype(function));
    let set_value = Value::Set(set);
    if let Some(iterable) = argument_values.first().cloned()
        && !matches!(iterable, Value::Undefined | Value::Null)
    {
        let adder = property_value(set_value.clone(), "add", env)?;
        if !matches!(adder, Value::Function(_)) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Set constructor add adder must be callable".to_owned(),
            });
        }
        for value in iterable_values_with_env(iterable, "Set constructor", env)? {
            call_function(adder.clone(), set_value.clone(), vec![value], env, false)?;
        }
    }
    Ok(set_value)
}

pub(crate) fn native_set_prototype_size(this_value: Value) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    Ok(Value::Number(set.len() as f64))
}

pub(crate) fn native_set_prototype_add(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value.clone())?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    set.add(value);
    Ok(this_value)
}

pub(crate) fn native_set_prototype_clear(this_value: Value) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    set.clear();
    Ok(Value::Undefined)
}

pub(crate) fn native_set_prototype_delete(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(set.delete(&value)))
}

pub(crate) fn native_set_prototype_entries(
    this_value: Value,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    set_iterator(this_value, env, SET_ITERATOR_KIND_KEY_VALUE)
}

pub(crate) fn native_set_prototype_union(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let other = SetRecord::from_arguments(argument_values, env)?;
    let result = new_set_like(&set);
    for value in set.values() {
        result.add(value);
    }
    for value in other.keys(env)? {
        result.add(value);
    }
    Ok(Value::Set(result))
}

pub(crate) fn native_set_prototype_intersection(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let other = SetRecord::from_arguments(argument_values, env)?;
    let result = new_set_like(&set);
    if (set.len() as f64) <= other.size() {
        for value in set.values() {
            if other.has(&value, env)? {
                result.add(value);
            }
        }
    } else {
        for value in other.keys(env)? {
            if set.has(&value) {
                result.add(value);
            }
        }
    }
    Ok(Value::Set(result))
}

pub(crate) fn native_set_prototype_difference(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let other = SetRecord::from_arguments(argument_values, env)?;
    let result = new_set_like(&set);
    if (set.len() as f64) <= other.size() {
        for value in set.values() {
            if !other.has(&value, env)? {
                result.add(value);
            }
        }
    } else {
        for value in set.values() {
            result.add(value);
        }
        for value in other.keys(env)? {
            result.delete(&value);
        }
    }
    Ok(Value::Set(result))
}

pub(crate) fn native_set_prototype_symmetric_difference(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let other = SetRecord::from_arguments(argument_values, env)?;
    let result = new_set_like(&set);
    for value in set.values() {
        result.add(value);
    }
    for value in other.keys(env)? {
        if set.has(&value) {
            result.delete(&value);
        } else {
            result.add(value);
        }
    }
    Ok(Value::Set(result))
}

pub(crate) fn native_set_prototype_is_subset_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let other = SetRecord::from_arguments(argument_values, env)?;
    if (set.len() as f64) > other.size() {
        return Ok(Value::Boolean(false));
    }
    for value in set.values() {
        if !other.has(&value, env)? {
            return Ok(Value::Boolean(false));
        }
    }
    Ok(Value::Boolean(true))
}

pub(crate) fn native_set_prototype_is_superset_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let other = SetRecord::from_arguments(argument_values, env)?;
    if (set.len() as f64) < other.size() {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(other.all_in_set(&set, env)?))
}

pub(crate) fn native_set_prototype_is_disjoint_from(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let other = SetRecord::from_arguments(argument_values, env)?;
    if (set.len() as f64) <= other.size() {
        for value in set.values() {
            if other.has(&value, env)? {
                return Ok(Value::Boolean(false));
            }
        }
    } else {
        if other.has_any_in_set(&set, env)? {
            return Ok(Value::Boolean(false));
        }
    }
    Ok(Value::Boolean(true))
}

pub(crate) fn native_set_prototype_for_each(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value.clone())?;
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Set.prototype.forEach callback must be callable".to_owned(),
        });
    }
    let this_arg = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let mut index = 0;
    loop {
        let values = set.values();
        let Some(value) = values.get(index).cloned() else {
            break;
        };
        let tail_values = values.iter().skip(index + 1).cloned().collect::<Vec<_>>();
        crate::call_function(
            callback.clone(),
            this_arg.clone(),
            vec![value.clone(), value.clone(), this_value.clone()],
            env,
            false,
        )?;
        let values = set.values();
        if let Some(next_index) = tail_values.iter().find_map(|tail_value| {
            values
                .iter()
                .position(|entry| entry.same_value_zero(tail_value))
        }) {
            index = next_index;
        } else if values
            .get(index)
            .is_some_and(|entry| entry.same_value_zero(&value))
        {
            index += 1;
        }
    }
    Ok(Value::Undefined)
}

pub(crate) fn native_set_prototype_has(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(set.has(&value)))
}

pub(crate) fn native_set_prototype_values(
    this_value: Value,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    set_iterator(this_value, env, SET_ITERATOR_KIND_VALUE)
}

pub(crate) fn native_set_iterator_next(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Object(iterator) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Set iterator next called on non-object".to_owned(),
        });
    };
    if iterator_done(&iterator) {
        return Ok(iterator_result(Value::Undefined, true));
    }

    let set = match iterator_slot(&iterator, SET_ITERATOR)? {
        Value::Set(set) => set,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Set iterator target is invalid".to_owned(),
            });
        }
    };
    let values = set.values();
    let index = iterator_index(&iterator)?;
    if index >= values.len() {
        iterator.define_non_enumerable(SET_ITERATOR_DONE.to_owned(), Value::Boolean(true));
        return Ok(iterator_result(Value::Undefined, true));
    }
    iterator.define_non_enumerable(
        SET_ITERATOR_NEXT_INDEX.to_owned(),
        Value::Number((index + 1) as f64),
    );

    let value = values[index].clone();
    let item = match iterator_kind(&iterator)?.as_str() {
        SET_ITERATOR_KIND_VALUE => value,
        SET_ITERATOR_KIND_KEY_VALUE => Value::Array(ArrayRef::new(vec![value.clone(), value])),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Set iterator kind is invalid".to_owned(),
            });
        }
    };
    Ok(iterator_result(item, false))
}

fn this_set(this_value: Value) -> Result<SetRef, RuntimeError> {
    match this_value {
        Value::Set(set) => Ok(set),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: incompatible Set receiver".to_owned(),
        }),
    }
}

fn new_set_like(set: &SetRef) -> SetRef {
    SetRef::new(set.object().prototype())
}

fn set_iterator(this_value: Value, env: &CallEnv, kind: &str) -> Result<Value, RuntimeError> {
    this_set(this_value.clone())?;
    let iterator = ObjectRef::new(HashMap::new());
    iterator.define_non_enumerable(SET_ITERATOR.to_owned(), this_value);
    iterator.define_non_enumerable(SET_ITERATOR_NEXT_INDEX.to_owned(), Value::Number(0.0));
    iterator.define_non_enumerable(SET_ITERATOR_DONE.to_owned(), Value::Boolean(false));
    iterator.define_non_enumerable(SET_ITERATOR_KIND.to_owned(), Value::String(kind.to_owned()));
    iterator.define_non_enumerable(
        "next".to_owned(),
        Value::Function(Function::new_native(
            Some("next"),
            0,
            NativeFunction::SetIteratorPrototypeNext,
            false,
        )),
    );
    symbol::define_iterator_identity(env, &iterator);
    Ok(Value::Object(iterator))
}

fn iterator_done(iterator: &ObjectRef) -> bool {
    matches!(
        iterator
            .own_property(SET_ITERATOR_DONE)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    )
}

fn iterator_index(iterator: &ObjectRef) -> Result<usize, RuntimeError> {
    match iterator_slot(iterator, SET_ITERATOR_NEXT_INDEX)? {
        Value::Number(index) if index >= 0.0 => Ok(index as usize),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Set iterator next index is invalid".to_owned(),
        }),
    }
}

fn iterator_slot(iterator: &ObjectRef, key: &str) -> Result<Value, RuntimeError> {
    iterator
        .own_property(key)
        .map(|property| property.value)
        .ok_or_else(|| RuntimeError {
            thrown: None,
            message: "Set iterator is missing internal state".to_owned(),
        })
}

fn iterator_kind(iterator: &ObjectRef) -> Result<String, RuntimeError> {
    match iterator_slot(iterator, SET_ITERATOR_KIND)? {
        Value::String(kind) => Ok(kind),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Set iterator kind is invalid".to_owned(),
        }),
    }
}

fn iterator_result(value: Value, done: bool) -> Value {
    let mut properties = HashMap::new();
    properties.insert("value".to_owned(), value);
    properties.insert("done".to_owned(), Value::Boolean(done));
    Value::Object(ObjectRef::new(properties))
}

fn define_set_prototype_function(
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
