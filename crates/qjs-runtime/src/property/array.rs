use crate::{ArrayRef, Property, Value};

pub(crate) fn array_has_own_property(elements: &ArrayRef, key: &str) -> bool {
    key == "length"
        || key
            .parse::<usize>()
            .is_ok_and(|index| index < elements.len())
}

pub(crate) fn array_own_property_descriptor(elements: &ArrayRef, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property::data(
            Value::Number(elements.len() as f64),
            false,
            !elements.is_frozen(),
            false,
        ));
    }
    let index = key.parse::<usize>().ok()?;
    elements
        .get(index)
        .map(|value| Property::data(value, true, !elements.is_frozen(), !elements.is_sealed()))
}

pub(crate) fn array_own_property_keys(elements: &ArrayRef) -> Vec<String> {
    (0..elements.len()).map(|index| index.to_string()).collect()
}

pub(crate) fn array_own_property_names(elements: &ArrayRef) -> Vec<String> {
    let mut names = array_own_property_keys(elements);
    names.push("length".to_owned());
    names
}
