//! An object's own string keys in property-key order: array indices
//! ascending, then insertion order.

use std::rc::Rc;

use super::{ObjectRef, PropertyStorage, array_index_property_key, is_internal_property_key};
use crate::Property;

impl ObjectRef {
    pub(crate) fn own_property_keys(&self) -> Vec<String> {
        self.ordered_property_names(|property| property.enumerable)
    }

    pub(crate) fn own_property_names(&self) -> Vec<String> {
        self.ordered_property_names(|_| true)
    }

    /// Own string keys in property-key order -- array indices ascending, then
    /// insertion order -- each with its enumerable flag, without cloning a
    /// descriptor or a key's text. Internal keys are omitted. `for-in` walks
    /// every layer of the prototype chain with this.
    pub(crate) fn own_string_keys_with_enumerability(&self) -> Vec<(Rc<str>, bool)> {
        let properties = self.properties().borrow();
        let has_indices = self.0.index_property_count.get() > 0;
        let mut indices: Vec<(u32, Rc<str>, bool)> = Vec::new();
        let mut strings: Vec<(Rc<str>, bool)> = Vec::new();
        let mut visit = |key: &Rc<str>, enumerable: bool| {
            if is_internal_property_key(key) {
                return;
            }
            if has_indices && let Some(index) = array_index_property_key(key) {
                indices.push((index, key.clone(), enumerable));
            } else {
                strings.push((key.clone(), enumerable));
            }
        };
        if let PropertyStorage::Small { entries } = &*properties {
            for (key, property) in entries {
                visit(key, property.enumerable);
            }
        } else {
            let order = properties
                .order()
                .expect("non-small property storage has a separate order");
            for key in order {
                if let Some(enumerable) = properties.enumerable(key) {
                    visit(key, enumerable);
                }
            }
        }
        if indices.is_empty() {
            return strings;
        }
        indices.sort_by_key(|(index, _, _)| *index);
        indices
            .into_iter()
            .map(|(_, key, enumerable)| (key, enumerable))
            .chain(strings)
            .collect()
    }

    pub(super) fn ordered_property_names(
        &self,
        include: impl Fn(&Property) -> bool,
    ) -> Vec<String> {
        let properties = self.properties().borrow();
        if let PropertyStorage::Small { entries } = &*properties {
            if self.0.index_property_count.get() == 0 {
                return entries
                    .iter()
                    .filter_map(|(key, property)| {
                        if is_internal_property_key(key) {
                            return None;
                        }
                        include(property).then(|| key.to_string())
                    })
                    .collect();
            }

            let mut indices = Vec::new();
            let mut strings = Vec::new();
            for (key, property) in entries {
                if is_internal_property_key(key) || !include(property) {
                    continue;
                }
                if let Some(index) = array_index_property_key(key) {
                    indices.push((index, key.to_string()));
                } else {
                    strings.push(key.to_string());
                }
            }
            indices.sort_by_key(|(index, _)| *index);
            return indices
                .into_iter()
                .map(|(_, key)| key)
                .chain(strings)
                .collect();
        }

        let order = properties
            .order()
            .expect("non-small property storage has a separate order");
        if self.0.index_property_count.get() == 0 {
            return order
                .iter()
                .filter_map(|key| {
                    if is_internal_property_key(key) {
                        return None;
                    }
                    let property = properties.get(key.as_ref())?;
                    include(&property).then(|| key.to_string())
                })
                .collect();
        }

        let mut indices = Vec::new();
        let mut strings = Vec::new();

        for key in order.iter() {
            if is_internal_property_key(key) {
                continue;
            }
            let Some(property) = properties.get(key.as_ref()) else {
                continue;
            };
            if !include(&property) {
                continue;
            }
            if let Some(index) = array_index_property_key(key) {
                indices.push((index, key.to_string()));
            } else {
                strings.push(key.to_string());
            }
        }

        indices.sort_by_key(|(index, _)| *index);
        indices
            .into_iter()
            .map(|(_, key)| key)
            .chain(strings)
            .collect()
    }
}
