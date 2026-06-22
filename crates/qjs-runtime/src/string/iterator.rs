use std::collections::HashMap;

use crate::{ObjectRef, RuntimeError, Value};

use super::{string_code_units, string_from_code_units};
use crate::CallEnv;

const STRING_ITERATOR_STRING: &str = "\0string_iterator_string";
const STRING_ITERATOR_NEXT_INDEX: &str = "\0string_iterator_next_index";
const STRING_ITERATOR_DONE: &str = "\0string_iterator_done";

pub(crate) fn native_string_prototype_iterator(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let source = super::indexing::this_string_value(this_value, env)?;
    let prototype = crate::iterator::builtin_iterator_prototype(
        env,
        crate::iterator::BuiltinIteratorKind::String,
    );
    let iterator = ObjectRef::with_prototype(HashMap::new(), prototype);
    iterator.define_non_enumerable(
        STRING_ITERATOR_STRING.to_owned(),
        Value::String(source.into()),
    );
    iterator.define_non_enumerable(STRING_ITERATOR_NEXT_INDEX.to_owned(), Value::Number(0.0));
    iterator.define_non_enumerable(STRING_ITERATOR_DONE.to_owned(), Value::Boolean(false));
    Ok(Value::Object(iterator))
}

pub(crate) fn native_string_iterator_next(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Object(iterator) = this_value else {
        return Err(RuntimeError {
            thrown: None,
            message: "String iterator next called on non-object".to_owned(),
        });
    };
    if iterator_done(&iterator) {
        return Ok(iterator_result(Value::Undefined, true));
    }

    let string = iterator_string(&iterator)?;
    let index = iterator_index(&iterator)?;
    let code_units = string_code_units(&string);
    if index >= code_units.len() {
        iterator.define_non_enumerable(STRING_ITERATOR_DONE.to_owned(), Value::Boolean(true));
        return Ok(iterator_result(Value::Undefined, true));
    }

    let width = if is_leading_surrogate(code_units[index])
        && code_units
            .get(index + 1)
            .is_some_and(|unit| is_trailing_surrogate(*unit))
    {
        2
    } else {
        1
    };
    iterator.define_non_enumerable(
        STRING_ITERATOR_NEXT_INDEX.to_owned(),
        Value::Number((index + width) as f64),
    );
    let value = if width == 2 {
        string_from_surrogate_pair(code_units[index], code_units[index + 1])
    } else {
        string_from_code_units(&code_units[index..index + width])
    };
    Ok(iterator_result(Value::String(value.into()), false))
}

fn iterator_done(iterator: &ObjectRef) -> bool {
    matches!(
        iterator
            .own_property(STRING_ITERATOR_DONE)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    )
}

fn iterator_index(iterator: &ObjectRef) -> Result<usize, RuntimeError> {
    match iterator_slot(iterator, STRING_ITERATOR_NEXT_INDEX)? {
        Value::Number(index) if index >= 0.0 => Ok(index as usize),
        _ => Err(RuntimeError {
            thrown: None,
            message: "String iterator next index is invalid".to_owned(),
        }),
    }
}

fn iterator_string(iterator: &ObjectRef) -> Result<String, RuntimeError> {
    match iterator_slot(iterator, STRING_ITERATOR_STRING)? {
        Value::String(value) => Ok(value.to_string()),
        _ => Err(RuntimeError {
            thrown: None,
            message: "String iterator source is invalid".to_owned(),
        }),
    }
}

fn iterator_slot(iterator: &ObjectRef, key: &str) -> Result<Value, RuntimeError> {
    iterator
        .own_property(key)
        .map(|property| property.value)
        .ok_or_else(|| RuntimeError {
            thrown: None,
            message: "String iterator is missing internal state".to_owned(),
        })
}

fn iterator_result(value: Value, done: bool) -> Value {
    let mut properties = HashMap::new();
    properties.insert("value".to_owned(), value);
    properties.insert("done".to_owned(), Value::Boolean(done));
    Value::Object(ObjectRef::new(properties))
}

fn is_leading_surrogate(code_unit: u16) -> bool {
    (0xD800..=0xDBFF).contains(&code_unit)
}

fn is_trailing_surrogate(code_unit: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&code_unit)
}

fn string_from_surrogate_pair(high: u16, low: u16) -> String {
    let code_point = 0x10000 + ((u32::from(high) - 0xD800) << 10) + u32::from(low) - 0xDC00;
    char::from_u32(code_point)
        .unwrap_or(char::REPLACEMENT_CHARACTER)
        .to_string()
}
