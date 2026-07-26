use crate::{JsString, Property, Value};

use super::{
    indexing::canonical_string_index, js_string_code_unit_at, js_string_code_unit_len,
    string_from_code_unit,
};

pub(crate) fn string_property(value: &JsString, key: &str) -> Option<Value> {
    let index = canonical_string_index(key)?;
    js_string_code_unit_at(value, index)
        .map(|code_unit| Value::String(string_from_code_unit(code_unit).into()))
}

pub(crate) fn string_has_own_property(value: &JsString, key: &str) -> bool {
    key == "length"
        || canonical_string_index(key).is_some_and(|index| index < js_string_code_unit_len(value))
}

pub(crate) fn string_own_property_descriptor(value: &JsString, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property::data(
            Value::Number(js_string_code_unit_len(value) as f64),
            false,
            false,
            false,
        ));
    }
    string_property(value, key).map(|value| Property::data(value, true, false, false))
}

pub(crate) fn string_own_property_keys(value: &JsString) -> Vec<String> {
    (0..js_string_code_unit_len(value))
        .map(|index| index.to_string())
        .collect()
}

pub(crate) fn string_own_property_names(value: &JsString) -> Vec<String> {
    let mut names = string_own_property_keys(value);
    names.push("length".to_owned());
    names
}
