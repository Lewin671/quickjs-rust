//! Observer-free host installation: publish one final named layout, preserving
//! existing positions and last-descriptor-wins semantics. Function defaults and
//! array index hooks remain the receiver's responsibility outside this table.

use std::rc::Rc;

use super::NamedTable;
use crate::Property;
use crate::property::atom::{AtomDomain, canonical_property_text};
use crate::property::layout::{
    Attributes, Encoding, LINEAR_LOOKUP_LIMIT, layout_for_descriptor_batch,
};
use crate::value::name_hash::NameMap;

impl NamedTable {
    /// Extends the installed named part with a complete, evaluated host batch.
    /// The first install fixes the domain; later batches use that same domain.
    pub(crate) fn insert_batch_in(
        &mut self,
        domain: &AtomDomain,
        entries: Vec<(Rc<str>, Property)>,
    ) {
        if entries.is_empty() {
            return;
        }
        let domain = self.domain().unwrap_or(domain).clone();
        let mut merged: Vec<_> = self
            .layout
            .as_ref()
            .into_iter()
            .flat_map(|layout| {
                layout.ordered_slots().map(|slot| {
                    (
                        Rc::clone(layout.fields()[slot].text()),
                        self.slots[slot].clone(),
                    )
                })
            })
            .collect();
        let mut index: NameMap<Rc<str>, usize> = merged
            .iter()
            .enumerate()
            .map(|(slot, (key, _))| (Rc::clone(key), slot))
            .collect();
        for (key, property) in entries {
            let canonical = canonical_property_text(&key);
            let key = if canonical.as_ref() == &*key {
                key
            } else {
                Rc::from(canonical.as_ref())
            };
            if let Some(&slot) = index.get(&key) {
                merged[slot].1 = property;
            } else {
                index.insert(Rc::clone(&key), merged.len());
                merged.push((key, property));
            }
        }
        let encoding = if merged.len() <= LINEAR_LOOKUP_LIMIT {
            Encoding::Small
        } else {
            Encoding::Dynamic
        };
        self.layout = Some(layout_for_descriptor_batch(
            &domain,
            encoding,
            merged
                .iter()
                .map(|(key, property)| (key.as_ref(), Attributes::of(property))),
        ));
        self.slots = merged.into_iter().map(|(_, property)| property).collect();
        self.edits = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn batch_preserves_positions_descriptors_domain_and_following_edits() {
        let domain = AtomDomain::new();
        let foreign = AtomDomain::new();
        let mut table = NamedTable::default();
        table.insert_in(&domain, "before", Property::enumerable(Value::Number(1.0)));
        let entries = || {
            vec![
                (Rc::from("tail"), Property::enumerable(Value::Number(2.0))),
                (
                    Rc::from("before"),
                    Property::accessor(None, None, false, true),
                ),
                (
                    Rc::from("tail"),
                    Property::data(Value::Number(3.0), false, false, false),
                ),
            ]
        };
        let mut sequential = NamedTable::default();
        sequential.insert_in(&domain, "before", Property::enumerable(Value::Number(1.0)));
        for (key, value) in entries() {
            sequential.insert_in(&domain, &key, value);
        }
        table.insert_batch_in(&foreign, entries());
        assert_eq!(table.keys().collect::<Vec<_>>(), ["before", "tail"]);
        assert!(table.get("before").unwrap().is_accessor());
        let tail = table.get("tail").unwrap();
        assert_eq!(tail.value, Value::Number(3.0));
        assert!(!tail.writable && !tail.enumerable && !tail.configurable);
        assert!(table.keys().eq(sequential.keys()));
        assert!(
            table.layout.as_ref().unwrap().fields()[1]
                .atom
                .belongs_to(&domain)
        );
        table.remove("before");
        table.insert_in(&domain, "after", Property::enumerable(Value::Undefined));
        assert_eq!(table.keys().collect::<Vec<_>>(), ["tail", "after"]);
    }

    #[test]
    fn complete_large_batch_becomes_a_private_dictionary() {
        let domain = AtomDomain::new();
        let mut table = NamedTable::default();
        table.insert_batch_in(
            &domain,
            (0..130)
                .map(|i| {
                    (
                        Rc::from(format!("field{i}")),
                        Property::enumerable(Value::Number(i as f64)),
                    )
                })
                .collect(),
        );
        assert!(table.layout.as_ref().unwrap().is_dictionary());
        table.remove("field3");
        table.insert_in(&domain, "tail", Property::enumerable(Value::Undefined));
        assert_eq!(table.get("field129").unwrap().value, Value::Number(129.0));
        assert_eq!(table.keys().last(), Some("tail"));
    }

    #[cfg(feature = "perf-counters")]
    #[test]
    fn batch_avoids_materializing_prefix_layouts() {
        let domain = AtomDomain::new();
        let mut table = NamedTable::default();
        crate::reset_counters();
        table.insert_batch_in(
            &domain,
            (0..64)
                .map(|i| {
                    (
                        Rc::from(format!("field{i}")),
                        Property::enumerable(Value::Undefined),
                    )
                })
                .collect(),
        );
        let counts = crate::counters().unwrap();
        assert_eq!(counts.property_shapes_created, 1);
        assert_eq!(counts.property_shape_fields_copied, 64);
    }
}
