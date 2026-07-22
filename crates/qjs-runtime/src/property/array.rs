use crate::{ArrayRef, Property, Value};

pub(crate) fn array_has_own_property(elements: &ArrayRef, key: &str) -> bool {
    key == "length"
        || crate::array_index_property_key(key).is_some_and(|index| elements.has_index(index))
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
    if let Some(index) = crate::array_index_property_key(key)
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
    let mut keys: Vec<_> = elements
        .present_indices()
        .into_iter()
        .filter_map(|index| {
            let key = index.to_string();
            array_own_property_descriptor(elements, &key)
                .is_some_and(|property| property.enumerable)
                .then_some(key)
        })
        .collect();
    keys.extend(
        elements
            .property_keys()
            .into_iter()
            .filter(|key| crate::array_index_property_key(key).is_none()),
    );
    keys
}

pub(crate) fn array_own_property_names(elements: &ArrayRef) -> Vec<String> {
    let mut names: Vec<_> = elements
        .present_indices()
        .into_iter()
        .map(|index| index.to_string())
        .collect();
    names.push("length".to_owned());
    names.extend(
        elements
            .property_names()
            .into_iter()
            .filter(|key| crate::array_index_property_key(key).is_none()),
    );
    names
}
