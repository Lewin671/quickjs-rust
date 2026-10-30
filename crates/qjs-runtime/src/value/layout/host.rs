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
        Self::from_slots(layout, ValueSlots::from_vec(values))
    }
    /// Unique two-key input is already in slot order and needs no Vec allocation.
    pub(crate) fn from_data_pair(layout: Rc<Layout>, values: [Value; 2]) -> Self {
        Self::from_slots(layout, ValueSlots::pair(values))
    }
    fn from_slots(layout: Rc<Layout>, values: ValueSlots) -> Self {
        let layout = if layout.dictionary {
            Rc::new(layout.copied(true))
        } else {
            layout
        };
        let epoch = if layout.dictionary { 1 } else { 0 };
        let cached = layout
            .root
            .as_ref()
            .unwrap_or(&layout)
            .prototype_value
            .borrow()
            .upgrade();
        let prototype = cached.or_else(|| {
            Self::retain_prototype(
                &layout,
                layout.prototype.as_ref().and_then(WeakPrototype::upgrade),
            )
        });
        let store = Self {
            layout,
            values,
            accessors: None,
            epoch,
            prototype,
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
        assert!(key.belongs_to(self.domain()), "own domain");
        let Some(entry) = self.layout.entry_for_key(key.id()) else {
            return (OwnDataPropertyRead::Missing, None);
        };
        let position = entry.slot as usize;
        if entry.attrs.has(Attributes::ACCESSOR) {
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
        self.write_existing_key(&key, value, guard)
    }
    pub(crate) fn write_existing_key(
        &mut self,
        key: &KeyLease,
        value: &Value,
        guard: impl FnOnce(&Value) -> bool,
    ) -> OwnDataPropertyWrite {
        assert!(key.belongs_to(self.domain()), "own domain");
        let Some(entry) = self.layout.entry_for_key(key.id()) else {
            return OwnDataPropertyWrite::NeedsSlowPath;
        };
        if entry.attrs.has(Attributes::ACCESSOR) {
            return OwnDataPropertyWrite::NeedsSlowPath;
        }
        let slot = entry.slot as usize;
        if !guard(&self.values[slot]) {
            return OwnDataPropertyWrite::NeedsSlowPath;
        }
        if !entry.attrs.has(Attributes::WRITABLE) {
            return OwnDataPropertyWrite::ReadOnly;
        }
        self.values[slot] = value.clone();
        self.verify_slot(slot);
        OwnDataPropertyWrite::Written
    }
    pub(crate) fn transform_descriptors(&mut self, frozen: bool) {
        let source = self.layout.clone();
        let label = Transition::Integrity { frozen };
        if let Some(next) = source.transitions.borrow().get(label) {
            crate::diagnostics::count!(property_transition_hits);
            self.invalidate_dependencies();
            self.layout = next;
            self.bump_epoch();
            return;
        }
        let change = |attrs: Attributes| {
            let mut next = Attributes(attrs.0 & !Attributes::CONFIGURABLE);
            if frozen && !attrs.has(Attributes::ACCESSOR) {
                next.0 &= !Attributes::WRITABLE;
            }
            next
        };
        if source
            .entries()
            .iter()
            .all(|entry| change(entry.attrs) == entry.attrs)
        {
            return;
        }
        let mut next = source.copied(source.dictionary);
        for entry in &mut next.private_data().entries {
            entry.attrs = change(entry.attrs);
        }
        if !source.dictionary {
            next.parent = Some(source.clone());
            next.incoming = Some(label);
        }
        crate::diagnostics::count!(property_transition_misses);
        crate::diagnostics::count!(property_shape_nodes_created);
        let next = Rc::new(next);
        if !source.dictionary {
            source.cache(label, &next);
        }
        self.invalidate_dependencies();
        self.layout = next;
        self.bump_epoch();
        self.verify();
    }
}

impl std::fmt::Debug for Layout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Layout")
            .field("entries", &self.len)
            .field("dictionary", &self.dictionary)
            .finish()
    }
}

impl NamedStore {
    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub(crate) fn contains_text(&self, key: &str) -> bool {
        self.domain()
            .find(key)
            .is_some_and(|k| self.position(&k).expect("own domain").is_some())
    }
    pub(crate) fn remove_text(&mut self, key: &str) -> Option<Property> {
        let key = self.domain().find(key)?;
        self.remove(&key).expect("own domain")
    }
    pub(crate) fn text_keys(&self) -> Vec<String> {
        self.keys()
            .into_iter()
            .filter(|key| key.symbol().is_none())
            .map(|key| key.as_str().to_owned())
            .collect()
    }
}
