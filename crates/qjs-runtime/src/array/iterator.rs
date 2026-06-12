use std::collections::HashMap;

use crate::{ArrayRef, ObjectRef, RuntimeError, Value, property_value, to_length_with_env};

use super::array_like::array_like_receiver;
use crate::CallEnv;

const ITERATED_OBJECT: &str = "\0array_iterator_object";
const ITERATOR_NEXT_INDEX: &str = "\0array_iterator_next_index";
const ITERATOR_DONE: &str = "\0array_iterator_done";
const ITERATOR_KIND: &str = "\0array_iterator_kind";
const ITERATOR_KIND_KEY: &str = "key";
const ITERATOR_KIND_VALUE: &str = "value";
const ITERATOR_KIND_KEY_VALUE: &str = "key+value";

pub(crate) fn native_array_prototype_entries(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    array_iterator(
        this_value,
        env,
        "Array.prototype.entries",
        ITERATOR_KIND_KEY_VALUE,
    )
}

pub(crate) fn native_array_prototype_keys(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    array_iterator(this_value, env, "Array.prototype.keys", ITERATOR_KIND_KEY)
}

pub(crate) fn native_array_prototype_values(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    array_iterator(
        this_value,
        env,
        "Array.prototype.values",
        ITERATOR_KIND_VALUE,
    )
}

fn array_iterator(
    this_value: Value,
    env: &mut CallEnv,
    context: &str,
    kind: &str,
) -> Result<Value, RuntimeError> {
    let receiver = match this_value {
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                thrown: None,
                message: format!("{context} called on null or undefined"),
            });
        }
        value => array_like_receiver(value, env),
    };
    // `%ArrayIteratorPrototype%` inherits `%Iterator.prototype%`, so the
    // instance gets `next`, `Symbol.iterator`, and the iterator helpers through
    // the chain rather than per-instance own properties.
    let prototype = crate::iterator::builtin_iterator_prototype(
        env,
        crate::iterator::BuiltinIteratorKind::Array,
    );
    let iterator = ObjectRef::with_prototype(HashMap::new(), prototype);
    iterator.define_non_enumerable(ITERATED_OBJECT.to_owned(), receiver);
    iterator.define_non_enumerable(ITERATOR_NEXT_INDEX.to_owned(), Value::Number(0.0));
    iterator.define_non_enumerable(ITERATOR_DONE.to_owned(), Value::Boolean(false));
    iterator.define_non_enumerable(ITERATOR_KIND.to_owned(), Value::String(kind.to_owned()));
    Ok(Value::Object(iterator))
}

pub(crate) fn native_array_iterator_next(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let Value::Object(iterator) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array iterator next called on non-object".to_owned(),
        });
    };
    if iterator_done(&iterator) {
        return Ok(iterator_result(Value::Undefined, true));
    }

    let target = iterator_slot(&iterator, ITERATED_OBJECT)?;
    let index = iterator_index(&iterator)?;
    if let Value::Object(object) = &target
        && crate::typed_array::is_typed_array_object(object)
        && crate::typed_array::typed_array_is_out_of_bounds(object)
    {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Array iterator next on out-of-bounds TypedArray".to_owned(),
        });
    }
    let length = to_length_with_env(property_value(target.clone(), "length", env)?, env)?;
    if index >= length {
        iterator.define_non_enumerable(ITERATOR_DONE.to_owned(), Value::Boolean(true));
        return Ok(iterator_result(Value::Undefined, true));
    }

    iterator.define_non_enumerable(
        ITERATOR_NEXT_INDEX.to_owned(),
        Value::Number((index + 1) as f64),
    );
    let key = Value::Number(index as f64);
    let value = match iterator_kind(&iterator)?.as_str() {
        ITERATOR_KIND_KEY => key,
        ITERATOR_KIND_VALUE => property_value(target, &index.to_string(), env)?,
        ITERATOR_KIND_KEY_VALUE => Value::Array(ArrayRef::new(vec![
            key,
            property_value(target, &index.to_string(), env)?,
        ])),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Array iterator kind is invalid".to_owned(),
            });
        }
    };
    Ok(iterator_result(value, false))
}

fn iterator_done(iterator: &ObjectRef) -> bool {
    matches!(
        iterator
            .own_property(ITERATOR_DONE)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    )
}

fn iterator_index(iterator: &ObjectRef) -> Result<usize, RuntimeError> {
    match iterator_slot(iterator, ITERATOR_NEXT_INDEX)? {
        Value::Number(index) if index >= 0.0 => Ok(index as usize),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Array iterator next index is invalid".to_owned(),
        }),
    }
}

fn iterator_slot(iterator: &ObjectRef, key: &str) -> Result<Value, RuntimeError> {
    iterator
        .own_property(key)
        .map(|property| property.value)
        .ok_or_else(|| RuntimeError {
            thrown: None,
            message: "Array iterator is missing internal state".to_owned(),
        })
}

fn iterator_kind(iterator: &ObjectRef) -> Result<String, RuntimeError> {
    match iterator_slot(iterator, ITERATOR_KIND)? {
        Value::String(kind) => Ok(kind),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Array iterator kind is invalid".to_owned(),
        }),
    }
}

fn iterator_result(value: Value, done: bool) -> Value {
    let mut properties = HashMap::new();
    properties.insert("value".to_owned(), value);
    properties.insert("done".to_owned(), Value::Boolean(done));
    Value::Object(ObjectRef::new(properties))
}
