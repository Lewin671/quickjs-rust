//! Structural edits and bounded sharing; no callback or text lookup occurs here.
use super::*;

impl Layout {
    fn copied(&self, dictionary: bool) -> Self {
        for _ in &self.entries {
            crate::diagnostics::count!(property_shape_field_copies);
        }
        Self {
            domain: self.domain.clone(),
            owner: self.owner.clone(),
            entries: self.entries.clone(),
            keys: self.keys.clone(),
            index: self.index.clone(),
            dictionary,
            edits: self.edits,
            transitions: RefCell::new(Vec::new()),
        }
    }
    fn append(&self, key: &KeyLease, attrs: Attributes, dictionary: bool) -> Rc<Self> {
        let transition = Transition {
            key: key.id(),
            attrs,
        };
        if !dictionary
            && let Some(shape) = self
                .transitions
                .borrow()
                .iter()
                .find(|(label, _)| *label == transition)
                .and_then(|(_, child)| child.upgrade())
        {
            crate::diagnostics::count!(property_transition_hits);
            return shape;
        }
        crate::diagnostics::count!(property_transition_misses);
        let mut next = self.copied(dictionary);
        let position = next.entries.len();
        let slot = u32::try_from(position).expect("property slot space exhausted");
        next.entries.push(ShapeEntry {
            key: key.id(),
            slot,
            order: slot,
            attrs,
            reserved: 0,
        });
        next.keys.push(key.clone());
        next.index.insert(key.id(), position);
        crate::diagnostics::count!(property_shape_nodes_created);
        let next = Rc::new(next);
        if !dictionary && let Some(owner) = self.owner.upgrade() {
            let mut edges = self.transitions.borrow_mut();
            edges.retain(|(_, child)| child.strong_count() > 0);
            if edges.len() == MAX_EDGES {
                edges.remove(0);
            }
            edges.push((transition, Rc::downgrade(&next)));
            let mut retained = owner.retained.borrow_mut();
            // Root plus retained nodes never exceeds MAX_CACHED_NODES.
            if retained.len() == MAX_CACHED_NODES - 1 {
                retained.pop_front();
            }
            retained.push_back(next.clone());
        }
        next
    }
}

impl Lookup<'_> {
    pub(crate) fn descriptor(&self) -> Option<Property> {
        self.position
            .map(|position| self.store.descriptor(position))
    }
    pub(crate) fn define(self, property: Property) {
        let attrs = Attributes::of(&property);
        match self.position {
            Some(position) => {
                if self.store.layout.entries[position].attrs != attrs {
                    self.store.edit_layout().entries[position].attrs = attrs;
                }
                self.store.install(position, property);
            }
            None => {
                let dictionary =
                    self.store.layout.dictionary || self.store.len() >= DICTIONARY_WIDTH;
                if dictionary {
                    let key = self.key.clone();
                    let layout = self.store.edit_layout();
                    let slot =
                        u32::try_from(layout.entries.len()).expect("property slot space exhausted");
                    layout.index.insert(key.id(), slot as usize);
                    layout.entries.push(ShapeEntry {
                        key: key.id(),
                        slot,
                        order: slot,
                        attrs,
                        reserved: 0,
                    });
                    layout.keys.push(key);
                } else {
                    self.store.layout = self.store.layout.append(self.key, attrs, false);
                    self.store.bump_epoch();
                }
                let position = self.store.values.len();
                self.store.values.push(Value::Undefined);
                self.store.install(position, property);
            }
        }
        self.store.verify();
    }
    pub(crate) fn remove(self) -> Option<Property> {
        let position = self.position?;
        let old = self.store.descriptor(position);
        let layout = self.store.edit_layout();
        layout.entries.remove(position);
        layout.keys.remove(position);
        // Compact on each delete: no order tombstones or unreclaimed slots.
        layout.index.remove(&self.key.id());
        for (position, entry) in layout.entries.iter_mut().enumerate().skip(position) {
            entry.slot = position as u32;
            entry.order = position as u32;
            *layout
                .index
                .get_mut(&entry.key)
                .expect("live entry indexed") = position;
        }
        self.store.values.remove(position);
        if let Some(slots) = self.store.accessors.as_mut() {
            let previous = std::mem::take(slots.as_mut());
            for (slot, pair) in previous {
                if (slot as usize) < position {
                    slots.insert(slot, pair);
                } else if (slot as usize) > position {
                    slots.insert(slot - 1, pair);
                }
            }
            if slots.is_empty() {
                self.store.accessors = None;
            }
        }
        self.store.verify();
        Some(old)
    }
}

impl NamedStore {
    fn bump_epoch(&mut self) {
        if let Some(next) = self.epoch.checked_add(1) {
            self.epoch = next;
        } else {
            // A fresh identity prevents an old dictionary token from becoming
            // valid again. Epochs are always paired with layout identity.
            self.layout = Rc::new(self.layout.copied(true));
            self.epoch = 0;
        }
    }
    fn edit_layout(&mut self) -> &mut Layout {
        self.bump_epoch();
        let edits = self.layout.edits.saturating_add(1);
        let dictionary = self.layout.dictionary
            || self.len() >= DICTIONARY_WIDTH
            || u64::from(edits) >= DICTIONARY_EDITS;
        if !self.layout.dictionary
            || Rc::strong_count(&self.layout) != 1
            || Rc::weak_count(&self.layout) != 0
        {
            self.layout = Rc::new(self.layout.copied(dictionary));
        }
        let layout = Rc::get_mut(&mut self.layout).expect("private edited layout");
        layout.edits = edits;
        layout
    }
    pub(crate) fn remove(&mut self, key: &KeyLease) -> Result<Option<Property>, ForeignKey> {
        Ok(self.lookup(key)?.remove())
    }
}
