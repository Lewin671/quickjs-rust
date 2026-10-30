//! Host text boundaries and bulk construction; the layout remains the only
//! ID index. Prelinked callers use the lease APIs directly.
use super::*;
use crate::value::{OwnDataPropertyRead, OwnDataPropertyWrite};

impl NamedStore {
    pub(crate) fn empty(domain: &AtomDomain) -> Self {
        PropertyOwner::new(domain.clone()).empty_store()
    }
    pub(crate) fn reserve(&mut self, capacity: usize) {
        self.values.reserve(capacity);
    }
    pub(crate) fn from_data_layout(layout: Rc<Layout>, values: Vec<Value>) -> Self {
        let layout = if layout.dictionary {
            Rc::new(layout.copied(true))
        } else {
            layout
        };
        let store = Self {
            layout,
            values,
            accessors: None,
            epoch: 0,
        };
        store.verify();
        store
    }
    pub(crate) fn insert_text(&mut self, key: &str, property: Property) -> bool {
        let key = self
            .domain()
            .find(key)
            .unwrap_or_else(|| self.domain().intern(key));
        let lookup = self.lookup(&key).expect("own domain");
        let added = lookup.position.is_none();
        lookup.define(property);
        added
    }
    pub(crate) fn read_data_slot(&self, key: &KeyLease) -> (OwnDataPropertyRead, Option<usize>) {
        let Some(position) = self.position(key).expect("own domain") else {
            return (OwnDataPropertyRead::Missing, None);
        };
        if self.layout.entries[position]
            .attrs
            .has(Attributes::ACCESSOR)
        {
            (OwnDataPropertyRead::NeedsSlowPath, None)
        } else {
            (
                OwnDataPropertyRead::Data(self.values[position].clone()),
                Some(position),
            )
        }
    }
    pub(crate) fn own_data_read(&self, key: &str) -> OwnDataPropertyRead {
        let Some(key) = self.domain().find(key) else {
            return OwnDataPropertyRead::Missing;
        };
        self.read_data_slot(&key).0
    }
    pub(crate) fn write_existing_data(
        &mut self,
        key: &str,
        value: &Value,
        guard: impl FnOnce(&Value) -> bool,
    ) -> OwnDataPropertyWrite {
        let Some(key) = self.domain().find(key) else {
            return OwnDataPropertyWrite::NeedsSlowPath;
        };
        let Some(slot) = self.data_slot(&key).expect("own domain") else {
            return OwnDataPropertyWrite::NeedsSlowPath;
        };
        if !guard(&self.values[slot]) {
            return OwnDataPropertyWrite::NeedsSlowPath;
        }
        match self.replace_data(slot, value.clone()) {
            Some(true) => OwnDataPropertyWrite::Written,
            Some(false) => OwnDataPropertyWrite::ReadOnly,
            None => OwnDataPropertyWrite::NeedsSlowPath,
        }
    }
    pub(crate) fn transform_descriptors(&mut self, transform: impl Fn(&mut Property)) {
        let keys = self.layout.keys.clone();
        for (position, key) in keys.iter().enumerate() {
            let mut descriptor = self.descriptor(position);
            transform(&mut descriptor);
            self.define(key, descriptor).expect("own domain");
        }
    }
}

impl std::fmt::Debug for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layout")
            .field("entries", &self.entries.len())
            .field("dictionary", &self.dictionary)
            .finish()
    }
}
