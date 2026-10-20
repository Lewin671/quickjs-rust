//! An object's own string keys in property-key order: array indices
//! ascending, then insertion order. The layout owns the order; internal keys
//! are kept out of it or filtered by name.

use std::rc::Rc;

use super::{ObjectRef, array_index_property_key, is_internal_property_key};
use crate::property::layout::Attributes;

impl ObjectRef {
    pub(crate) fn own_property_keys(&self) -> Vec<String> {
        self.ordered_property_names(Attributes::is_enumerable)
    }

    pub(crate) fn own_property_names(&self) -> Vec<String> {
        self.ordered_property_names(|_| true)
    }

    /// Own string keys in property-key order -- array indices ascending, then
    /// insertion order -- each with its enumerable flag, without cloning a
    /// descriptor. Internal keys are omitted. `for-in` walks every layer of
    /// the prototype chain with this.
    pub(crate) fn own_string_keys_with_enumerability(&self) -> Vec<(Rc<str>, bool)> {
        let properties = self.properties().borrow();
        let has_indices = self.0.index_property_count.get() > 0;
        let mut indices: Vec<(u32, Rc<str>, bool)> = Vec::new();
        let mut strings: Vec<(Rc<str>, bool)> = Vec::with_capacity(properties.len());
        for (_, field) in properties.ordered_fields() {
            let key = field.text();
            if is_internal_property_key(key) {
                continue;
            }
            let enumerable = field.attributes.is_enumerable();
            if has_indices && let Some(index) = array_index_property_key(key) {
                indices.push((index, Rc::clone(key), enumerable));
            } else {
                strings.push((Rc::clone(key), enumerable));
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
        include: impl Fn(Attributes) -> bool,
    ) -> Vec<String> {
        let properties = self.properties().borrow();
        let included = properties.ordered_fields().filter(|(_, field)| {
            !is_internal_property_key(field.text()) && include(field.attributes)
        });
        if self.0.index_property_count.get() == 0 {
            return included
                .map(|(_, field)| field.text().to_string())
                .collect();
        }
        let mut indices = Vec::new();
        let mut strings = Vec::new();
        for (_, field) in included {
            let key = field.text();
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
