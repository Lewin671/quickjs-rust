use crate::{ArrayRef, Property, Value};

pub(crate) fn array_has_own_property(elements: &ArrayRef, key: &str) -> bool {
    key == "length"
        || key
            .parse::<usize>()
            .is_ok_and(|index| elements.has_index(index))
        || elements.property(key).is_some()
}

pub(crate) fn array_own_property_descriptor(elements: &ArrayRef, key: &str) -> Option<Property> {
    if key == "length" {
        return Some(Property::data(
            Value::Number(elements.len() as f64),
            false,
            elements.is_length_writable(),
            false,
        ));
    }
    if let Ok(index) = key.parse::<usize>()
        && let Some(value) = elements.get(index)
    {
        return Some(Property::data(
            value,
            true,
            !elements.is_frozen(),
            !elements.is_sealed(),
        ));
    }
    elements.property(key)
}

pub(crate) fn array_own_property_keys(elements: &ArrayRef) -> Vec<String> {
    let mut keys: Vec<_> = (0..elements.len())
        .filter(|index| elements.has_index(*index))
        .map(|index| index.to_string())
        .collect();
    keys.extend(elements.property_keys());
    keys
}

pub(crate) fn array_own_property_names(elements: &ArrayRef) -> Vec<String> {
    let mut names = array_own_property_keys(elements);
    names.extend(elements.property_names());
    names.push("length".to_owned());
    names.sort();
    names.dedup();
    names
}
