use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};

use crate::value::{Property, Value};

use super::{ObjectData, ObjectRef, PropertyStorage, Prototype, SymbolBrand, is_array_index_key};

/// Incrementally builds a fresh ordinary object from source-ordered data
/// properties. Duplicate keys update their existing slot, so storage grows
/// with the number of unique keys rather than the number of source members.
pub(crate) struct OrderedDataPropertyBuilder {
    properties: PropertyStorage,
    index_property_count: usize,
}

impl OrderedDataPropertyBuilder {
    pub(crate) fn new() -> Self {
        Self {
            properties: PropertyStorage::Small {
                entries: Vec::new(),
            },
            index_property_count: 0,
        }
    }

    /// Applies one CreateDataProperty-style update. The generic storage insert
    /// path overwrites duplicates in place and promotes only when a ninth
    /// unique key arrives. Array-index classification happens only after the
    /// insert reports a new key, keeping duplicate updates allocation-free.
    pub(crate) fn insert(&mut self, key: Rc<str>, value: Value) {
        let classify_key = Rc::clone(&key);
        let inserted = self
            .properties
            .insert(key, Property::enumerable(value))
            .is_none();
        if inserted && is_array_index_key(&classify_key) {
            self.index_property_count += 1;
        }
    }

    pub(crate) fn finish(self, prototype: Option<ObjectRef>) -> ObjectRef {
        ObjectRef(Rc::new(ObjectData {
            properties: RefCell::new(self.properties),
            property_revision: Cell::new(0),
            index_property_count: Cell::new(self.index_property_count),
            extensible: Cell::new(true),
            prototype: RefCell::new(prototype.map(Prototype::Object)),
            raw_json: Cell::new(false),
            array_prototype_exotic: Cell::new(false),
            typed_array_exotic: Cell::new(false),
            symbol_brand: Cell::new(SymbolBrand::None),
            immutable_prototype_exotic: Cell::new(false),
            module_namespace_exotic: Cell::new(false),
            cold: OnceCell::new(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, rc::Rc};

    use super::{ObjectRef, OrderedDataPropertyBuilder, PropertyStorage};
    use crate::Value;

    #[test]
    fn duplicate_heavy_builder_stays_small_and_bounded_by_unique_keys() {
        let mut builder = OrderedDataPropertyBuilder::new();
        for value in 0..256 {
            builder.insert(
                Rc::from(format!("field{}", value % 8)),
                Value::Number(value as f64),
            );
        }

        assert!(matches!(
            &builder.properties,
            PropertyStorage::Small { entries }
                if entries.len() == 8 && entries.capacity() <= PropertyStorage::SMALL_LIMIT
        ));
        assert_eq!(builder.index_property_count, 0);

        let object = builder.finish(None);
        assert_eq!(
            object.own_property_names(),
            (0..8)
                .map(|index| format!("field{index}"))
                .collect::<Vec<_>>()
        );
        for index in 0..8 {
            assert_eq!(
                object.get(&format!("field{index}")),
                Some(Value::Number((248 + index) as f64))
            );
        }
    }

    #[test]
    fn duplicate_heavy_dynamic_builder_retains_only_unique_properties() {
        let mut builder = OrderedDataPropertyBuilder::new();
        for value in 0..300 {
            builder.insert(
                Rc::from(format!("field{}", value % 10)),
                Value::Number(value as f64),
            );
        }

        assert!(matches!(
            &builder.properties,
            PropertyStorage::Dynamic(dynamic)
                if dynamic.properties.len() == 10
                    && dynamic.order.len() == 10
                    && dynamic.properties.capacity() <= PropertyStorage::SMALL_LIMIT * 4
                    && dynamic.order.capacity() <= PropertyStorage::SMALL_LIMIT * 4
        ));
        assert_eq!(builder.index_property_count, 0);

        let object = builder.finish(None);
        assert_eq!(
            object.own_property_names(),
            (0..10)
                .map(|index| format!("field{index}"))
                .collect::<Vec<_>>()
        );
        for index in 0..10 {
            assert_eq!(
                object.get(&format!("field{index}")),
                Some(Value::Number((290 + index) as f64))
            );
        }
    }

    #[test]
    fn duplicate_heavy_numeric_keys_release_replaced_values_during_build() {
        let mut builder = OrderedDataPropertyBuilder::new();
        let mut previous = None;

        for _ in 0..256 {
            let value = ObjectRef::new(HashMap::new());
            let current = value.downgrade();
            builder.insert(Rc::from("123"), Value::Object(value));
            if let Some(replaced) = previous.replace(current) {
                assert!(replaced.upgrade().is_none());
            }
        }

        assert!(matches!(
            &builder.properties,
            PropertyStorage::Small { entries }
                if entries.len() == 1 && entries.capacity() <= PropertyStorage::SMALL_LIMIT
        ));
        assert_eq!(builder.index_property_count, 1);
        let retained = previous.expect("last property value must have a weak handle");
        assert!(retained.upgrade().is_some());
        drop(builder);
        assert!(retained.upgrade().is_none());
    }
}
