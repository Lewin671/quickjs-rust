//! Ordered construction through the common named-store transitions.
use super::{ObjectRef, Prototype};
use crate::property::atom::AtomDomain;
use crate::value::layout::NamedStore;
use crate::{Property, Value};
use std::rc::Rc;

pub(crate) struct OrderedDataPropertyBuilder {
    properties: NamedStore,
    index_property_count: usize,
}
impl OrderedDataPropertyBuilder {
    pub(crate) fn finish_sequence(
        sequence: crate::value::layout::DataSequence,
    ) -> (ObjectRef, Option<Rc<crate::value::layout::Layout>>) {
        let (properties, prediction, indices) = sequence.finish();
        (ObjectRef::from_ready_named(properties, indices), prediction)
    }
    pub(crate) fn new(domain: &AtomDomain) -> Self {
        Self {
            properties: NamedStore::empty(domain),
            index_property_count: 0,
        }
    }
    pub(crate) fn with_capacity(capacity: usize, domain: &AtomDomain) -> Self {
        let mut b = Self::new(domain);
        b.properties.reserve(capacity);
        b
    }
    pub(crate) fn with_prototype(
        capacity: usize,
        domain: &AtomDomain,
        prototype: Option<Prototype>,
    ) -> Self {
        let mut properties = NamedStore::empty_with_prototype(domain, prototype, 0);
        properties.reserve(capacity);
        Self {
            properties,
            index_property_count: 0,
        }
    }
    pub(crate) fn insert_property(&mut self, key: Rc<str>, property: Property) {
        if self.properties.insert_text(&key, property) && super::is_array_index_key(&key) {
            self.index_property_count += 1;
        }
    }
    pub(crate) fn insert(&mut self, key: Rc<str>, value: Value) {
        self.insert_property(key, Property::enumerable(value));
    }
    pub(crate) fn finish(self, prototype: Option<ObjectRef>) -> ObjectRef {
        self.finish_slot(prototype.map(Prototype::Object))
    }
    pub(super) fn finish_slot(self, prototype: Option<Prototype>) -> ObjectRef {
        ObjectRef::from_named(self.properties, prototype, self.index_property_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property::atom::AtomDomain;
    use crate::value::layout::PropertyOwner;
    use std::collections::HashMap;
    #[test]
    fn duplicate_heavy_input_keeps_first_order_and_last_values() {
        let owner = PropertyOwner::new(AtomDomain::new());
        for width in [8, 140] {
            let mut builder = OrderedDataPropertyBuilder::new(owner.domain());
            for n in 0..width * 30 {
                builder.insert(
                    Rc::from(format!("field{}", n % width)),
                    Value::Number(n as f64),
                );
            }
            assert_eq!(builder.properties.len(), width);
            let object = builder.finish(None);
            assert_eq!(
                object.own_property_names(),
                (0..width).map(|n| format!("field{n}")).collect::<Vec<_>>()
            );
            for n in 0..width {
                assert_eq!(
                    object.get(&format!("field{n}")),
                    Some(Value::Number((width * 29 + n) as f64))
                );
            }
        }
    }
    #[test]
    fn overwritten_values_are_released_during_construction() {
        let owner = PropertyOwner::new(AtomDomain::new());
        let mut builder = OrderedDataPropertyBuilder::new(owner.domain());
        let mut previous: Option<super::super::ObjectWeakRef> = None;
        for _ in 0..256 {
            let child = ObjectRef::new(HashMap::new(), owner.domain());
            let weak = child.downgrade();
            builder.insert(Rc::from("123"), Value::Object(child));
            if let Some(old) = previous.replace(weak) {
                assert!(old.upgrade().is_none());
            }
        }
        assert_eq!(builder.index_property_count, 1);
        let weak = previous.unwrap();
        drop(builder);
        assert!(weak.upgrade().is_none());
    }
}
