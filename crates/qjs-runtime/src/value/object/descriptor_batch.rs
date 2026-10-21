//! Complete host descriptor batches publish one final immutable layout.
//!
//! Callers materialize descriptors before publication and do not expose an
//! intermediate state. Ordinary incremental edits still use transition edges.
//! Duplicate keys retain their first slot and take the last descriptor; special
//! index/global identity installs keep the existing sequential path.

use std::rc::Rc;

use super::{ObjectRef, PropertyStorage, is_array_index_key};
use crate::property::atom::canonical_property_text;
use crate::{Function, NativeFunction, Property, Value};

impl ObjectRef {
    /// Defines a complete, already-evaluated batch of ordinary descriptors.
    /// No JS getter, setter or callback runs while the store is borrowed.
    pub(crate) fn define_properties_ordered(&self, entries: Vec<(Rc<str>, Property)>) {
        if entries.is_empty() {
            return;
        }
        if entries
            .iter()
            .any(|(key, _)| &**key == "globalThis" || is_array_index_key(key))
            || self
                .0
                .properties
                .borrow()
                .layout
                .fields()
                .iter()
                .any(|field| !field.attributes.is_ordered())
        {
            for (key, property) in entries {
                self.define_property(key.to_string(), property);
            }
            return;
        }
        let mut properties = self.0.properties.borrow_mut();
        let domain = properties.domain().clone();
        let mut merged: Vec<_> = properties
            .layout
            .ordered_slots()
            .map(|slot| {
                (
                    Rc::clone(properties.field(slot).unwrap().text()),
                    properties.slot(slot).unwrap().to_property(),
                )
            })
            .collect();
        let mut index: crate::value::name_hash::NameMap<Rc<str>, usize> = merged
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
        *properties = PropertyStorage::from_descriptor_batch(&domain, merged);
        self.bump_property_revision();
    }

    /// Installs a source-ordered table of nonconstructor native methods.
    /// Finish the table before resolving aliases to its function identities.
    pub(crate) fn define_native_methods(&self, methods: &[(&str, usize, NativeFunction)]) {
        let domain = self.store_domain();
        self.define_properties_ordered(
            methods
                .iter()
                .map(|(name, length, native)| {
                    (
                        Rc::from(*name),
                        Property::non_enumerable(Value::Function(Function::new_native(
                            &domain,
                            Some(name),
                            *length,
                            *native,
                            false,
                        ))),
                    )
                })
                .collect(),
        );
    }
}

#[cfg(test)]
mod tests;
