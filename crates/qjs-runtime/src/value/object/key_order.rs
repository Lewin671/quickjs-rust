//! Numeric indices first, then strings and symbols in insertion order. Virtual
//! String indices participate without materializing a named key or descriptor.
use super::{ObjectRef, is_internal_property_key};
use std::{collections::BTreeMap, rc::Rc};
impl ObjectRef {
    pub(crate) fn own_property_keys(&self) -> Vec<String> {
        self.own_string_keys_with_enumerability()
            .into_iter()
            .filter(|(_, enumerable)| *enumerable)
            .map(|(key, _)| key.to_string())
            .collect()
    }
    pub(crate) fn own_property_names(&self) -> Vec<String> {
        self.own_string_keys_with_enumerability()
            .into_iter()
            .map(|(key, _)| key.to_string())
            .collect()
    }
    pub(crate) fn own_string_keys_with_enumerability(&self) -> Vec<(Rc<str>, bool)> {
        let virtual_len = if self.0.flag(super::flags::STRING_INDICES) {
            self.string_data()
                .map_or(0, |text| crate::string::js_string_code_unit_len(&text))
        } else {
            0
        };
        let store = self.0.properties.borrow();
        let mut indices = BTreeMap::new();
        let mut strings = Vec::new();
        for (key, enumerable) in store.key_enumerability() {
            let Some(text) = key.string_text() else {
                continue;
            };
            if is_internal_property_key(text) {
                continue;
            }
            if let Some(index) = key.array_index() {
                indices.insert(index as usize, (key.text().clone(), enumerable));
            } else if crate::string::canonical_string_index(text)
                .is_none_or(|index| index >= virtual_len)
            {
                strings.push((key.text().clone(), enumerable));
            }
        }
        drop(store);
        for index in 0..virtual_len {
            indices.insert(index, (Rc::from(index.to_string()), true));
        }
        indices.into_values().chain(strings).collect()
    }
}
