use crate::{Function, Property, Value};

pub(crate) fn function_own_property_keys(function: &Function) -> Vec<String> {
    let mut keys: Vec<_> = function
        .properties
        .borrow()
        .iter()
        .filter(|(_, property)| property.enumerable)
        .map(|(key, _)| key.clone())
        .collect();
    keys.sort();
    keys
}

pub(crate) fn function_own_property_descriptor(function: &Function, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property {
            value: Value::Number(function.params.len() as f64),
            enumerable: false,
            writable: false,
            configurable: !function.is_sealed(),
        });
    }
    function.properties.borrow().get(key).cloned()
}

pub(crate) fn function_delete_own_property(function: &Function, key: &str) -> bool {
    if key == "length" {
        return false;
    }
    let mut properties = function.properties.borrow_mut();
    if properties
        .get(key)
        .is_some_and(|property| !property.configurable)
    {
        return false;
    }
    properties.remove(key);
    true
}

pub(crate) fn function_own_property_names(function: &Function) -> Vec<String> {
    let mut names: Vec<_> = function.properties.borrow().keys().cloned().collect();
    names.push("length".to_owned());
    names.sort();
    names
}
