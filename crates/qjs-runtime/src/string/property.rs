use crate::{Property, Value};

use super::{indexing::canonical_string_index, string_code_units, string_from_code_unit};

pub(crate) fn string_property(value: &str, key: &str) -> Option<Value> {
    let index = canonical_string_index(key)?;
    string_code_units(value)
        .get(index)
        .map(|code_unit| Value::String(string_from_code_unit(*code_unit).into()))
}

pub(crate) fn string_has_own_property(value: &str, key: &str) -> bool {
    key == "length"
        || canonical_string_index(key).is_some_and(|index| index < string_code_units(value).len())
}

pub(crate) fn string_own_property_descriptor(value: &str, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property::data(
            Value::Number(string_code_units(value).len() as f64),
            false,
            false,
            false,
        ));
    }
    string_property(value, key).map(|value| Property::data(value, true, false, false))
}

pub(crate) fn string_own_property_keys(value: &str) -> Vec<String> {
    (0..string_code_units(value).len())
        .map(|index| index.to_string())
        .collect()
}

pub(crate) fn string_own_property_names(value: &str) -> Vec<String> {
    let mut names = string_own_property_keys(value);
    names.push("length".to_owned());
    names
}
