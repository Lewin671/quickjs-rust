//! Structural edits and bounded sharing; no callback or text lookup occurs here.
use super::*;

impl Layout {
    pub(super) fn copied(&self, dictionary: bool) -> Self {
        self.budget.add(size_of::<Self>());
        Self {
            budget: self.budget.clone(),
            domain: self.domain.clone(),
            owner: self.owner.clone(),
            data: Rc::new(RefCell::new(self.data.borrow().prefix(self.len))),
            len: self.len,
            parent: None,
            incoming: None,
            dictionary,
            edits: self.edits,
            next_order: self.next_order,
            transitions: RefCell::new(transitions::Transitions::default()),
        }
    }

    fn allocate_order(&mut self) -> u32 {
        if self.next_order == u32::MAX {
            let mut positions: Vec<_> = (0..self.len).collect();
            positions.sort_unstable_by_key(|&position| self.private_data().entries[position].order);
            for (order, position) in positions.into_iter().enumerate() {
                self.private_data().entries[position].order = order as u32;
            }
            self.next_order = self.len as u32;
        }
        let order = self.next_order;
        self.next_order = order
            .checked_add(1)
            .expect("property order space exhausted");
        order
    }
    fn append(self: &Rc<Self>, key: &KeyLease, attrs: Attributes, proof: IdProbe) -> Rc<Self> {
        let label = Transition::Append {
            key: key.id(),
            attrs,
        };
        if let Some(shape) = self.transitions.borrow().get(label) {
            crate::diagnostics::count!(property_transition_hits);
            return shape;
        }
        crate::diagnostics::count!(property_transition_misses);
        let tail = self.len > 0 && self.data.borrow().entries.len() == self.len;
        let data = if tail {
            self.data.clone()
        } else {
            Rc::new(RefCell::new(self.data.borrow().prefix(self.len)))
        };
        {
            let mut data = data.borrow_mut();
            let proof = if tail {
                proof
            } else {
                data.index.probe_uncounted(key.id())
            };
            data.append(key, attrs, proof);
        }
        self.budget.add(size_of::<Self>());
        let next = Rc::new(Self {
            budget: self.budget.clone(),
            domain: self.domain.clone(),
            owner: self.owner.clone(),
            data,
            len: self.len + 1,
            parent: Some(self.clone()),
            incoming: Some(label),
            dictionary: false,
            edits: self.edits,
            next_order: self.len as u32 + 1,
            transitions: RefCell::new(transitions::Transitions::default()),
        });
        crate::diagnostics::count!(property_shape_nodes_created);
        self.cache(label, &next);
        next
    }
    fn reconfigure(self: &Rc<Self>, position: usize, attrs: Attributes) -> Rc<Self> {
        let label = Transition::Reconfigure {
            position: position as u32,
            attrs,
        };
        if let Some(next) = self.transitions.borrow().get(label) {
            crate::diagnostics::count!(property_transition_hits);
            return next;
        }
        let edits = self.edits.saturating_add(1);
        let mut next = self.copied(u64::from(edits) >= DICTIONARY_EDITS);
        next.edits = edits;
        if !next.dictionary {
            next.parent = Some(self.clone());
            next.incoming = Some(label);
        }
        next.private_data().entries[position].attrs = attrs;
        crate::diagnostics::count!(property_transition_misses);
        crate::diagnostics::count!(property_shape_nodes_created);
        let next = Rc::new(next);
        if !next.dictionary {
            self.cache(label, &next);
        }
        next
    }
}

impl Lookup<'_> {
    pub(crate) fn data_slot(&self) -> Option<usize> {
        self.position
            .filter(|&p| !self.store.layout.entry(p).attrs.has(Attributes::ACCESSOR))
    }
    pub(crate) fn descriptor(&self) -> Option<Property> {
        self.position
            .map(|position| self.store.descriptor(position))
    }
    pub(crate) fn define(self, property: Property) {
        let attrs = Attributes::of(&property);
        match self.position {
            Some(position) => {
                if self.store.layout.entry(position).attrs != attrs {
                    if self.store.layout.dictionary {
                        self.store.edit_layout().private_data().entries[position].attrs = attrs;
                    } else {
                        self.store.layout = self.store.layout.reconfigure(position, attrs);
                        self.store.bump_epoch();
                    }
                    self.store.install(position, property);
                    self.store.verify();
                    return;
                }
                self.store.install(position, property);
                self.store.verify_slot(position);
                return;
            }
            None => {
                let dictionary =
                    self.store.layout.dictionary || self.store.len() >= DICTIONARY_WIDTH;
                if dictionary {
                    let key = self.key.clone();
                    let same_index =
                        self.store.layout.data.borrow().entries.len() == self.store.len();
                    let layout = self.store.edit_layout();
                    layout.dictionary = true;
                    let order = layout.allocate_order();
                    {
                        let data = layout.private_data();
                        let slot = data.entries.len();
                        let proof = if same_index {
                            self.proof
                        } else {
                            data.index.probe_uncounted(key.id())
                        };
                        data.append(&key, attrs, proof);
                        data.entries[slot].order = order;
                    }
                    layout.len += 1;
                } else {
                    self.store.layout = self.store.layout.append(self.key, attrs, self.proof);
                    self.store.bump_epoch();
                }
                let position = self.store.values.len();
                self.store.values.push(Value::Undefined);
                self.store.install(position, property);
            }
        }
        self.store.verify();
    }
    #[allow(dead_code)] // Shared semantic API; ObjectRef consumes Lookup directly.
    pub(crate) fn remove(self) -> Option<Property> {
        let position = self.position?;
        let old = self.store.descriptor(position);
        if !self.store.layout.dictionary
            && matches!(self.store.layout.incoming, Some(Transition::Append { .. }))
            && position + 1 == self.store.len()
            && self.store.epoch != u64::MAX
            && let Some(parent) = self.store.layout.parent.clone()
        {
            self.store.layout = parent;
            self.store.bump_epoch();
            self.store.values.pop();
            if let Some(slots) = &mut self.store.accessors {
                slots.remove(&(position as u32));
            }
            if self
                .store
                .accessors
                .as_ref()
                .is_some_and(|slots| slots.is_empty())
            {
                self.store.accessors = None;
            }
            self.store.verify();
            return Some(old);
        }
        let same_index = self.store.layout.data.borrow().entries.len() == self.store.len();
        let layout = self.store.edit_layout();
        let before_bytes = layout.private_data().bytes();
        let last = layout.len - 1;
        let proof = if same_index {
            self.proof
        } else {
            layout.private_data().index.probe_uncounted(self.key.id())
        };
        layout.private_data().index.remove_at(proof);
        if layout.dictionary {
            layout.private_data().entries.swap_remove(position);
            layout.private_data().keys.swap_remove(position);
            if position < last {
                let data = layout.private_data();
                let moved = &mut data.entries[position];
                moved.slot = position as u32;
                data.index.set_position(moved.key, position);
            }
            layout.private_data().reaccount(before_bytes);
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
            layout.private_data().entries.remove(position);
            layout.private_data().keys.remove(position);
            let data = layout.private_data();
            for (position, entry) in data.entries.iter_mut().enumerate().skip(position) {
                entry.slot = position as u32;
                entry.order = position as u32;
                data.index.set_position(entry.key, position);
            }
            layout.next_order = layout.len as u32 - 1;
            layout.private_data().reaccount(before_bytes);
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
        self.store.layout_mut_len_after_remove();
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
            .private_data()
            .edit(|data| data.index.compact_if_due());
        self.store.verify();
        Some(old)
    }
}

impl NamedStore {
    fn layout_mut_len_after_remove(&mut self) {
        let layout = Rc::get_mut(&mut self.layout).expect("private edited layout");
        layout.len -= 1;
    }
    pub(super) fn bump_epoch(&mut self) {
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
            || self.len() > DICTIONARY_WIDTH
            || u64::from(edits) >= DICTIONARY_EDITS;
        if !self.layout.dictionary {
            self.layout = Rc::new(self.layout.copied(dictionary));
        }
        let layout = Rc::make_mut(&mut self.layout);
        layout.edits = edits;
        layout
    }
    #[allow(dead_code)] // Shared semantic API; ObjectRef consumes Lookup directly.
    pub(crate) fn remove(&mut self, key: &KeyLease) -> Result<Option<Property>, ForeignKey> {
        Ok(self.lookup(key)?.remove())
    }
}

impl Drop for Layout {
    fn drop(&mut self) {
        if let (Some(parent), Some(label)) = (&self.parent, self.incoming)
            && let Ok(mut edges) = parent.transitions.try_borrow_mut()
        {
            edges.remove(label, self as *const Layout);
        }
        self.budget.sub(size_of::<Self>());
    }
}
