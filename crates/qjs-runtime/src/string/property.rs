use crate::{Property, Value};

use super::indexing::canonical_string_index;

pub(crate) fn string_property(value: &str, key: &str) -> Option<Value> {
    let index = canonical_string_index(key)?;
    value
        .chars()
        .nth(index)
        .map(|character| Value::String(character.to_string()))
}

pub(crate) fn string_has_own_property(value: &str, key: &str) -> bool {
    key == "length"
        || canonical_string_index(key).is_some_and(|index| index < value.chars().count())
}

pub(crate) fn string_own_property_descriptor(value: &str, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property {
            value: Value::Number(value.chars().count() as f64),
            enumerable: false,
            writable: false,
            configurable: false,
        });
    }
    string_property(value, key).map(|value| Property {
        value,
        enumerable: true,
        writable: false,
        configurable: false,
    })
}

pub(crate) fn string_own_property_keys(value: &str) -> Vec<String> {
    (0..value.chars().count())
        .map(|index| index.to_string())
        .collect()
}

pub(crate) fn string_own_property_names(value: &str) -> Vec<String> {
    let mut names = string_own_property_keys(value);
    names.push("length".to_owned());
    names
}
