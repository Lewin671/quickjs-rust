//! Live data and cold accessor slots. Slot APIs require current layout validity.
use super::*;

impl NamedStore {
    pub(super) fn descriptor(&self, position: usize) -> Property {
        let entry = self.layout.entries[position];
        let attrs = entry.attrs;
        if attrs.has(Attributes::ACCESSOR) {
            let pair = &self.accessors.as_deref().expect("accessor table")[&entry.slot];
            Property::accessor(
                pair.get.clone(),
                pair.set.clone(),
                attrs.has(Attributes::ENUMERABLE),
                attrs.has(Attributes::CONFIGURABLE),
            )
        } else {
            Property::data(
                self.values[position].clone(),
                attrs.has(Attributes::ENUMERABLE),
                attrs.has(Attributes::WRITABLE),
                attrs.has(Attributes::CONFIGURABLE),
            )
        }
    }
    pub(super) fn install(&mut self, position: usize, property: Property) {
        if property.is_accessor() {
            let pair = AccessorPair {
                get: property.getter().cloned(),
                set: property.setter().cloned(),
            };
            self.values[position] = Value::Undefined;
            self.accessors
                .get_or_insert_with(|| Box::new(HashMap::new()))
                .insert(position as u32, pair);
        } else {
            self.values[position] = property.value;
            if let Some(slots) = self.accessors.as_mut() {
                slots.remove(&(position as u32));
                if slots.is_empty() {
                    self.accessors = None;
                }
            }
        }
    }
    pub(crate) fn data_slot(&self, key: &KeyLease) -> Result<Option<usize>, ForeignKey> {
        Ok(self.position(key)?.filter(|&position| {
            !self.layout.entries[position]
                .attrs
                .has(Attributes::ACCESSOR)
        }))
    }
    pub(crate) fn load_data(&self, slot: usize) -> Option<Value> {
        let entry = self.layout.entries.get(slot)?;
        (!entry.attrs.has(Attributes::ACCESSOR)).then(|| self.values[slot].clone())
    }
    /// None means accessor/missing; Some(false) is nonwritable own data.
    pub(crate) fn replace_data(&mut self, slot: usize, value: Value) -> Option<bool> {
        let attrs = self.layout.entries.get(slot)?.attrs;
        if attrs.has(Attributes::ACCESSOR) {
            return None;
        }
        if !attrs.has(Attributes::WRITABLE) {
            return Some(false);
        }
        self.values[slot] = value;
        self.verify();
        Some(true)
    }
    pub(crate) fn slot_key(&self, slot: usize) -> Option<&KeyLease> {
        self.layout.keys.get(slot)
    }
}
