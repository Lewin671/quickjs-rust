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
    ops::Deref,
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
        self.layout
            .publish(Encoding::NamedMap, fields, self.layout.order().collect());
    }
    pub(crate) fn get(&self, key: &str) -> Option<&Property> {
        self.entries
            .get(self.layout.lookup(key)?.slot)
            .map(|(_, property)| property)
    }
    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.layout.lookup(key).is_some()
    }
    fn insert(&mut self, key: String, property: Property) -> Option<Property> {
        let attributes = Attributes::of(&property);
        if let Some(entry) = self.layout.lookup(&key) {
            let old = std::mem::replace(&mut self.entries[entry.slot].1, property);
            self.layout.reconfigure(entry.slot, attributes);
            return Some(old);
        }
        let key = super::atom::canonical_property_text(&key).into_owned();
        self.entries.push((key.clone(), property));
        self.layout
            .append(&key, attributes, Encoding::NamedMap, true);
        None
    }
    fn remove(&mut self, key: &str) -> Option<Property> {
        let slot = self.layout.lookup(key)?.slot;
        let removed = self.entries.swap_remove(slot).1;
        self.layout.remove(slot, true);
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
        self.layout.order().map(|slot| &self.entries[slot].0)
    }
    pub(crate) fn values(&self) -> impl Iterator<Item = &Property> {
        self.entries.iter().map(|(_, p)| p)
    }
    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut Property> {
        self.entries.iter_mut().map(|(_, p)| p)
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &Property)> {
        self.layout.order().map(|slot| {
            let (k, p) = &self.entries[slot];
            (k, p)
        })
    }
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut Property)> {
        self.entries.iter_mut().map(|(k, p)| (&*k, p))
    }
}
pub(crate) struct NamedPropertyIter<'a> {
    entries: &'a [(String, Property)],
    order: super::layout::OrderedSlots<'a>,
}
impl<'a> Iterator for NamedPropertyIter<'a> {
    type Item = &'a (String, Property);
    fn next(&mut self) -> Option<Self::Item> {
        self.order.next().map(|slot| &self.entries[slot])
    }
}
impl<'a> IntoIterator for &'a NamedPropertyMap {
    type Item = &'a (String, Property);
    type IntoIter = NamedPropertyIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        NamedPropertyIter {
            entries: &self.entries,
            order: self.layout.order(),
        }
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
        PropertyMapEdit::from_ref(self.0.borrow_mut())
    }
}
#[derive(Default)]
enum Dirty {
    #[default]
    None,
    Slot(usize),
    Many(Box<crate::value::name_hash::NameSet<usize>>),
    All,
}
pub(crate) struct PropertyMapEdit<'a> {
    map: RefMut<'a, NamedPropertyMap>,
    dirty: Dirty,
}
impl PropertyMapEdit<'_> {
    pub(crate) fn from_ref(map: RefMut<'_, NamedPropertyMap>) -> PropertyMapEdit<'_> {
        PropertyMapEdit {
            map,
            dirty: Dirty::None,
        }
    }
    fn publish_dirty(&mut self) {
        match std::mem::take(&mut self.dirty) {
            Dirty::None => {}
            Dirty::Slot(slot) => {
                let attributes = Attributes::of(&self.map.entries[slot].1);
                self.map.layout.reconfigure(slot, attributes);
            }
            Dirty::Many(slots) => {
                for slot in *slots {
                    let attributes = Attributes::of(&self.map.entries[slot].1);
                    self.map.layout.reconfigure(slot, attributes);
                }
            }
            Dirty::All => self.map.publish(),
        }
    }
    pub(crate) fn get_mut(&mut self, key: &str) -> Option<&mut Property> {
        let slot = self.map.layout.lookup(key)?.slot;
        match &mut self.dirty {
            Dirty::None => self.dirty = Dirty::Slot(slot),
            Dirty::Slot(previous) if *previous == slot => {}
            Dirty::Slot(previous) => {
                let mut slots = Box::new(crate::value::name_hash::NameSet::default());
                slots.insert(*previous);
                slots.insert(slot);
                self.dirty = Dirty::Many(slots);
            }
            Dirty::Many(slots) => {
                slots.insert(slot);
            }
            Dirty::All => {}
        }
        self.map.entries.get_mut(slot).map(|(_, property)| property)
    }
    pub(crate) fn insert(&mut self, key: String, property: Property) -> Option<Property> {
        self.publish_dirty();
        self.map.insert(key, property)
    }
    pub(crate) fn remove(&mut self, key: &str) -> Option<Property> {
        self.publish_dirty();
        self.map.remove(key)
    }
    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut Property> {
        self.dirty = Dirty::All;
        self.map.values_mut()
    }
    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut Property)> {
        self.dirty = Dirty::All;
        self.map.iter_mut()
    }
}
impl Deref for PropertyMapEdit<'_> {
    type Target = NamedPropertyMap;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}
impl Drop for PropertyMapEdit<'_> {
    fn drop(&mut self) {
        self.publish_dirty();
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

#[cfg(test)]
mod mutation_batch_tests {
    use super::*;
    use crate::Value;
    #[test]
    fn pending_descriptor_edits_survive_slot_moves_and_bulk_freeze() {
        let cell = NamedPropertyCell::default();
        for i in 0..70 {
            cell.borrow_mut()
                .insert(format!("k{i}"), Property::enumerable(Value::Null));
        }
        {
            let mut edit = cell.borrow_mut();
            edit.get_mut("k1").unwrap().enumerable = false;
            edit.remove("k0");
            edit.get_mut("k2").unwrap().writable = false;
            edit.insert("after".into(), Property::enumerable(Value::Null));
            edit.get_mut("k10").unwrap().enumerable = false;
            edit.get_mut("k20").unwrap().writable = false;
        }
        let names = cell.borrow().keys().cloned().collect::<Vec<_>>();
        assert_eq!(
            names,
            (1..70)
                .map(|i| format!("k{i}"))
                .chain(["after".into()])
                .collect::<Vec<_>>()
        );
        assert!(
            !cell
                .borrow()
                .layout
                .lookup("k1")
                .unwrap()
                .attributes
                .enumerable
        );
        assert!(
            !cell
                .borrow()
                .layout
                .lookup("k2")
                .unwrap()
                .attributes
                .writable
        );
        assert!(
            !cell
                .borrow()
                .layout
                .lookup("k10")
                .unwrap()
                .attributes
                .enumerable
        );
        assert!(
            !cell
                .borrow()
                .layout
                .lookup("k20")
                .unwrap()
                .attributes
                .writable
        );
        for property in cell.borrow_mut().values_mut() {
            property.freeze_data();
        }
        assert_eq!(cell.borrow().keys().cloned().collect::<Vec<_>>(), names);
        for (name, property) in &*cell.borrow() {
            assert_eq!(
                cell.borrow().layout.lookup(name).unwrap().attributes,
                Attributes::of(property)
            );
        }
    }
}
