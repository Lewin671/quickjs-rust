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
            next_order: self.next_order,
            transitions: RefCell::new(Vec::new()),
        }
    }
    fn allocate_order(&mut self) -> u32 {
        if self.next_order == u32::MAX {
            let mut positions: Vec<_> = (0..self.entries.len()).collect();
            positions.sort_unstable_by_key(|&position| self.entries[position].order);
            for (order, position) in positions.into_iter().enumerate() {
                self.entries[position].order = order as u32;
            }
            self.next_order = self.entries.len() as u32;
        }
        let order = self.next_order;
        self.next_order = order
            .checked_add(1)
            .expect("property order space exhausted");
        order
    }
    fn append(
        &self,
        key: &KeyLease,
        attrs: Attributes,
        dictionary: bool,
        proof: IdProbe,
    ) -> Rc<Self> {
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
        next.index.insert_at(proof, position);
        next.next_order = slot + 1;
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
            } else {
                crate::diagnostics::count!(property_cached_shape_nodes);
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
                    let order = layout.allocate_order();
                    layout.index.insert_at(self.proof, slot as usize);
                    layout.entries.push(ShapeEntry {
                        key: key.id(),
                        slot,
                        order,
                        attrs,
                        reserved: 0,
                    });
                    layout.keys.push(key);
                } else {
                    self.store.layout =
                        self.store.layout.append(self.key, attrs, false, self.proof);
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
        let last = layout.entries.len() - 1;
        layout.index.remove_at(self.proof);
        if layout.dictionary {
            layout.entries.swap_remove(position);
            layout.keys.swap_remove(position);
            if position < last {
                let moved = &mut layout.entries[position];
                moved.slot = position as u32;
                layout.index.set_position(moved.key, position);
            }
            self.store.values.swap_remove(position);
            if let Some(slots) = self.store.accessors.as_mut() {
                slots.remove(&(position as u32));
                if position < last
                    && let Some(pair) = slots.remove(&(last as u32))
                {
                    slots.insert(position as u32, pair);
                }
            }
        } else {
            layout.entries.remove(position);
            layout.keys.remove(position);
            for (position, entry) in layout.entries.iter_mut().enumerate().skip(position) {
                entry.slot = position as u32;
                entry.order = position as u32;
                layout.index.set_position(entry.key, position);
            }
            layout.next_order = layout.entries.len() as u32;
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
            }
        }
        if self
            .store
            .accessors
            .as_ref()
            .is_some_and(|slots| slots.is_empty())
        {
            self.store.accessors = None;
        }
        Rc::get_mut(&mut self.store.layout)
            .expect("private edited layout")
            .index
            .compact_if_due();
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
