//! Live data and cold accessor slots. Slot APIs require current layout validity.
use super::*;

impl NamedStore {
    pub(super) fn descriptor(&self, position: usize) -> Property {
        let entry = self.layout.entry(position);
        let attrs = entry.attrs;
        if attrs.has(Attributes::ACCESSOR) {
            let pair = &self.accessors.as_deref().expect("accessor table")[&entry.slot];
            Property::from_shared_accessors(
                pair.clone(),
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
        if let Some(pair) = property.shared_accessors() {
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
        Ok(self
            .position(key)?
            .filter(|&position| !self.layout.entry(position).attrs.has(Attributes::ACCESSOR)))
    }
    /// Caller proved descriptor kind with a live shape/dictionary guard.
    pub(crate) fn probe_slot(&self, key: &KeyLease) -> crate::value::named::Probe {
        use crate::value::named::Probe;
        let Some(position) = self.position(key).expect("own domain") else {
            return Probe::Missing;
        };
        let entry = self.layout.entry(position);
        if entry.attrs.has(Attributes::ACCESSOR) {
            Probe::Accessor { slot: entry.slot }
        } else {
            Probe::Data {
                slot: entry.slot,
                writable: entry.attrs.has(Attributes::WRITABLE),
            }
        }
    }
    pub(crate) fn overwrite_guarded_data(&mut self, slot: usize, value: Value) -> bool {
        let Some(target) = self.values.get_mut(slot) else {
            return false;
        };
        *target = value;
        self.verify_slot(slot);
        true
    }
    pub(crate) fn load_guarded_data(&self, slot: usize) -> Option<Value> {
        self.values.get(slot).cloned()
    }
    pub(crate) fn accessor_at(&self, slot: usize) -> Option<Property> {
        let entry = self.layout.entry_at(slot)?;
        entry
            .attrs
            .has(Attributes::ACCESSOR)
            .then(|| self.descriptor(slot))
    }
    #[cfg(test)]
    pub(crate) fn load_data(&self, slot: usize) -> Option<Value> {
        let entry = self.layout.entry_at(slot)?;
        (!entry.attrs.has(Attributes::ACCESSOR)).then(|| self.values[slot].clone())
    }
    /// None means accessor/missing; Some(false) is nonwritable own data.
    pub(crate) fn replace_data(&mut self, slot: usize, value: Value) -> Option<bool> {
        let attrs = self.layout.entry_at(slot)?.attrs;
        if attrs.has(Attributes::ACCESSOR) {
            return None;
        }
        if !attrs.has(Attributes::WRITABLE) {
            return Some(false);
        }
        self.values[slot] = value;
        self.verify_slot(slot);
        Some(true)
    }
    pub(crate) fn data_value_mut(&mut self, slot: usize) -> Option<&mut Value> {
        let attrs = self.layout.entry_at(slot)?.attrs;
        (!attrs.has(Attributes::ACCESSOR) && attrs.has(Attributes::WRITABLE))
            .then(|| &mut self.values[slot])
    }
    pub(crate) fn get_text(&self, key: &str) -> Option<Property> {
        self.get(&self.domain().find(key)?).expect("same domain")
    }
    pub(crate) fn descriptors(&self) -> impl Iterator<Item = Property> + '_ {
        (0..self.len()).map(|slot| self.descriptor(slot))
    }
    pub(crate) fn defer_children(&mut self, pending: &mut Vec<Value>) {
        if let Some(prototype) = self.prototype.take() {
            if let Some(prototype) = Rc::into_inner(prototype) {
                let mut value = prototype.into_value();
                crate::value::teardown::defer_if_last(&mut value, pending);
            }
        }
        for value in &mut self.values {
            crate::value::teardown::defer_if_last(value, pending);
        }
        if let Some(slots) = &mut self.accessors {
            for pair in slots.values_mut() {
                if let Some(pair) = Rc::get_mut(pair) {
                    pair.defer_children(pending);
                }
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn slot_matches(&self, slot: usize, key: &KeyLease) -> bool {
        self.layout.keys().get(slot) == Some(key)
    }
}
