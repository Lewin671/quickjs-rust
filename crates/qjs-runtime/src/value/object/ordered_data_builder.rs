use std::rc::Rc;

use crate::property::atom::AtomDomain;
use crate::value::{Property, Value};

use super::{ObjectRef, PropertyStorage, Prototype, is_array_index_key};

/// Incrementally builds a fresh ordinary object from source-ordered data
/// properties. Duplicate keys update their existing slot, so storage grows
/// with the number of unique keys rather than the number of source members.
/// The builder receives its atom domain before its first insert.
pub(crate) struct OrderedDataPropertyBuilder {
    properties: PropertyStorage,
    index_property_count: usize,
}

impl OrderedDataPropertyBuilder {
    pub(crate) fn new(domain: &AtomDomain) -> Self {
        Self::with_capacity(domain, 0)
    }

    /// A builder whose storage has room for `capacity` properties.
    pub(crate) fn with_capacity(domain: &AtomDomain, capacity: usize) -> Self {
        Self {
            properties: PropertyStorage::empty(domain, capacity),
            index_property_count: 0,
        }
    }

    /// Defines `key` with an arbitrary descriptor, as `insert` does for an
    /// enumerable data property.
    pub(crate) fn insert_property(&mut self, key: Rc<str>, property: Property) {
        if self.properties.insert_shared(&key, property).is_none() && is_array_index_key(&key) {
            self.index_property_count += 1;
        }
    }

    /// Applies one CreateDataProperty-style update. Duplicates overwrite in
    /// place; array-index classification happens only for a new key.
    pub(crate) fn insert(&mut self, key: Rc<str>, value: Value) {
        self.insert_property(key, Property::enumerable(value));
    }

    pub(crate) fn finish(self, prototype: Option<ObjectRef>) -> ObjectRef {
        ObjectRef::from_storage(
            self.properties,
            self.index_property_count,
            prototype.map(Prototype::Object),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, rc::Rc};

    use super::{ObjectRef, OrderedDataPropertyBuilder};
    use crate::Value;
    use crate::property::atom::AtomDomain;
    use crate::property::layout::{Encoding, LINEAR_LOOKUP_LIMIT};

    #[test]
    fn duplicate_heavy_builder_stays_small_and_bounded_by_unique_keys() {
        let mut builder = OrderedDataPropertyBuilder::new(&AtomDomain::new());
        for value in 0..256 {
            builder.insert(
                Rc::from(format!("field{}", value % 8)),
                Value::Number(value as f64),
            );
        }

        assert_eq!(builder.properties.len(), 8);
        assert_eq!(builder.properties.layout.encoding(), Encoding::Small);
        assert!(builder.properties.capacity() <= LINEAR_LOOKUP_LIMIT);
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
        let mut builder = OrderedDataPropertyBuilder::new(&AtomDomain::new());
        let unique_count = LINEAR_LOOKUP_LIMIT + 2;
        let value_count = unique_count * 30;
        for value in 0..value_count {
            builder.insert(
                Rc::from(format!("field{}", value % unique_count)),
                Value::Number(value as f64),
            );
        }

        assert_eq!(builder.properties.len(), unique_count);
        assert_eq!(builder.properties.layout.encoding(), Encoding::Dynamic);
        assert!(builder.properties.capacity() <= LINEAR_LOOKUP_LIMIT * 4);
        assert_eq!(builder.index_property_count, 0);

        let object = builder.finish(None);
        assert_eq!(
            object.own_property_names(),
            (0..unique_count)
                .map(|index| format!("field{index}"))
                .collect::<Vec<_>>()
        );
        for index in 0..unique_count {
            assert_eq!(
                object.get(&format!("field{index}")),
                Some(Value::Number((value_count - unique_count + index) as f64))
            );
        }
    }

    #[test]
    fn duplicate_heavy_numeric_keys_release_replaced_values_during_build() {
        let mut builder = OrderedDataPropertyBuilder::new(&AtomDomain::new());
        let mut previous = None;

        for _ in 0..256 {
            let value = ObjectRef::new(HashMap::new());
            let current = value.downgrade();
            builder.insert(Rc::from("123"), Value::Object(value));
            if let Some(replaced) = previous.replace(current) {
                assert!(replaced.upgrade().is_none());
            }
        }

        assert_eq!(builder.properties.len(), 1);
        assert!(builder.properties.capacity() <= LINEAR_LOOKUP_LIMIT);
        assert_eq!(builder.index_property_count, 1);
        let retained = previous.expect("last property value must have a weak handle");
        assert!(retained.upgrade().is_some());
        drop(builder);
        assert!(retained.upgrade().is_none());
    }
}
