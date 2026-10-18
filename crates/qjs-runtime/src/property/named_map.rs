//! Array/function ordinary named portions. Their semantic hooks and ordering
//! remain receiver-owned; this adapter gives the installed descriptors the
//! same atom/slot/attribute authority as ordinary object payloads. A mutation
//! guard publishes descriptor edits before releasing the table borrow.

use super::{
    atom::AtomDomain,
    layout::{Attributes, Encoding, Field, PropertyLayout},
};
use crate::Property;
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
};

pub(crate) struct NamedPropertyMap {
    entries: Vec<(String, Property)>,
    layout: PropertyLayout,
}
impl Default for NamedPropertyMap {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            layout: PropertyLayout::new(AtomDomain::new(), Encoding::NamedMap),
        }
    }
}
impl NamedPropertyMap {
    fn publish(&mut self) {
        // Skip both allocation and publication on stable value-only writes.
        if self
            .entries
            .iter()
            .enumerate()
            .all(|(slot, (_, property))| {
                self.layout
                    .field(slot)
                    .is_some_and(|field| field.attributes == Attributes::of(property))
            })
        {
            return;
        }
        self.rebuild();
    }
    fn rebuild(&mut self) {
        let fields = self
            .entries
            .iter()
            .map(|(key, property)| Field {
                key: self.layout.domain().intern_key(key),
                attributes: Attributes::of(property),
            })
            .collect();
        self.layout.publish(
            Encoding::NamedMap,
            fields,
            (0..self.entries.len()).collect(),
        );
    }
    pub(crate) fn get(&self, key: &str) -> Option<&Property> {
        self.entries
            .get(self.layout.lookup(key)?.slot)
            .map(|(_, property)| property)
    }
    pub(crate) fn get_mut(&mut self, key: &str) -> Option<&mut Property> {
        self.entries
            .get_mut(self.layout.lookup(key)?.slot)
            .map(|(_, property)| property)
    }
    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.layout.lookup(key).is_some()
    }
    pub(crate) fn insert(&mut self, key: String, property: Property) -> Option<Property> {
        if let Some(entry) = self.layout.lookup(&key) {
            let old = std::mem::replace(&mut self.entries[entry.slot].1, property);
            self.publish();
            return Some(old);
        }
        let key = self.layout.domain().intern_key(&key).text().to_string();
        self.entries.push((key, property));
        self.rebuild();
        None
    }
    pub(crate) fn remove(&mut self, key: &str) -> Option<Property> {
        let slot = self.layout.lookup(key)?.slot;
        let removed = self.entries.remove(slot).1;
        self.rebuild();
        Some(removed)
    }
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub(crate) fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.iter().map(|(key, _)| key)
    }
    pub(crate) fn values(&self) -> impl Iterator<Item = &Property> {
        self.entries.iter().map(|(_, p)| p)
    }
    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut Property> {
        self.entries.iter_mut().map(|(_, p)| p)
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &Property)> {
        self.entries.iter().map(|(k, p)| (k, p))
    }
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut Property)> {
        self.entries.iter_mut().map(|(k, p)| (&*k, p))
    }
}
impl<'a> IntoIterator for &'a NamedPropertyMap {
    type Item = &'a (String, Property);
    type IntoIter = std::slice::Iter<'a, (String, Property)>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

#[derive(Default)]
pub(crate) struct NamedPropertyCell(RefCell<NamedPropertyMap>);
impl NamedPropertyCell {
    pub(crate) fn borrow(&self) -> Ref<'_, NamedPropertyMap> {
        self.0.borrow()
    }
    pub(crate) fn try_borrow(&self) -> Result<Ref<'_, NamedPropertyMap>, std::cell::BorrowError> {
        self.0.try_borrow()
    }
    pub(crate) fn borrow_mut(&self) -> PropertyMapEdit<'_> {
        PropertyMapEdit(self.0.borrow_mut())
    }
}
pub(crate) struct PropertyMapEdit<'a>(RefMut<'a, NamedPropertyMap>);
impl PropertyMapEdit<'_> {
    pub(crate) fn from_ref(value: RefMut<'_, NamedPropertyMap>) -> PropertyMapEdit<'_> {
        PropertyMapEdit(value)
    }
}
impl Deref for PropertyMapEdit<'_> {
    type Target = NamedPropertyMap;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for PropertyMapEdit<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for PropertyMapEdit<'_> {
    fn drop(&mut self) {
        self.0.publish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    #[test]
    fn descriptor_edits_publish_before_the_next_borrow_and_values_keep_the_token() {
        let cell = NamedPropertyCell::default();
        cell.borrow_mut()
            .insert("x".into(), Property::enumerable(Value::Number(1.0)));
        let before = cell.borrow().layout.token();
        cell.borrow_mut().get_mut("x").unwrap().value = Value::Number(2.0);
        assert_eq!(before, cell.borrow().layout.token());
        cell.borrow_mut().get_mut("x").unwrap().writable = false;
        assert!(
            !cell
                .borrow()
                .layout
                .lookup("x")
                .unwrap()
                .attributes
                .writable
        );
        assert_ne!(before, cell.borrow().layout.token());
        for (_, descriptor) in cell.borrow_mut().iter_mut() {
            descriptor.freeze_data();
        }
        let flags = cell.borrow().layout.lookup("x").unwrap().attributes;
        assert!(!flags.configurable && !flags.writable);
        cell.borrow_mut().remove("x");
        assert!(cell.borrow().layout.lookup("x").is_none());
    }
    #[test]
    fn computed_encodings_and_churn_keep_one_slot_and_bounded_live_keys() {
        let cell = NamedPropertyCell::default();
        let halves = crate::string::js_string_from_code_unit(0xd83d).to_string()
            + crate::string::js_string_from_code_unit(0xde00).as_ref();
        cell.borrow_mut()
            .insert("😀".into(), Property::enumerable(Value::Number(1.0)));
        cell.borrow_mut()
            .insert(halves.clone(), Property::enumerable(Value::Number(2.0)));
        assert_eq!(cell.borrow().len(), 1);
        assert_eq!(
            cell.borrow().get(&halves).unwrap().value,
            Value::Number(2.0)
        );
        for i in 0..10000 {
            let key = format!("temp{i}");
            cell.borrow_mut()
                .insert(key.clone(), Property::enumerable(Value::Null));
            cell.borrow_mut().remove(&key);
        }
        let table = cell.borrow();
        assert_eq!(table.len(), 1);
        assert!(table.layout.is_dictionary());
        assert!(table.layout.domain().table_len() <= 66);
    }
}
