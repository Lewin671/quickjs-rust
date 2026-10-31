//! Structural edits and bounded sharing; no callback or text lookup occurs here.
use super::*;

impl Layout {
    pub(super) fn copied(&self, dictionary: bool) -> Self {
        self.budget.add(size_of::<Self>());
        let mut next = Self {
            budget: self.budget.clone(),
            domain: self.domain.clone(),
            owner: self.owner.clone(),
            data: Rc::new(RefCell::new(self.data.borrow().prefix(self.len))),
            len: self.len,
            links: OnceCell::new(),
            dictionary,
            epoch: std::cell::Cell::new(if dictionary {
                self.epoch.get().max(1)
            } else {
                0
            }),
            deferred_seed: false,
            transient: self.transient,
            edits: self.edits,
            prototype: self.prototype.clone(),
            flavor: self.flavor,
            capacity_hint: std::cell::Cell::new(0),
            append_misses: std::cell::Cell::new(0),
        };
        if let Some(root) = self.known_root() {
            next.set_root(root.clone());
        }
        next
    }

    fn allocate_order(&mut self) -> u32 {
        if self.private_data().next_order == u32::MAX {
            let mut positions: Vec<_> = (0..self.len).collect();
            positions.sort_unstable_by_key(|&position| self.private_data().entries[position].order);
            for (order, position) in positions.into_iter().enumerate() {
                self.private_data().entries[position].order = order as u32;
            }
            self.private_data().next_order = self.len as u32;
        }
        let order = self.private_data().next_order;
        self.private_data().next_order = order
            .checked_add(1)
            .expect("property order space exhausted");
        order
    }
    pub(super) fn append(
        self: &Rc<Self>,
        key: &KeyLease,
        attrs: Attributes,
        proof: IdProbe,
    ) -> Rc<Self> {
        let label = Transition::Append {
            key: key.id(),
            attrs,
        };
        if let Some(shape) = self.transition(label) {
            crate::diagnostics::count!(property_transition_hits);
            return shape;
        }
        crate::diagnostics::count!(property_transition_misses);
        self.append_misses
            .set(self.append_misses.get().saturating_add(1));
        let data = self.append_data(key, attrs, proof);
        if !self.transient {
            let root = Self::root_shape(self);
            root.capacity_hint
                .set(root.capacity_hint.get().max((self.len + 1).min(16) as u8));
        }
        self.budget.add(size_of::<Self>());
        let mut next = Self {
            budget: self.budget.clone(),
            domain: self.domain.clone(),
            owner: self.owner.clone(),
            data,
            len: self.len + 1,
            links: OnceCell::new(),
            dictionary: false,
            epoch: std::cell::Cell::new(0),
            deferred_seed: false,
            transient: self.transient,
            edits: self.edits,
            prototype: self.prototype.clone(),
            flavor: self.flavor,
            capacity_hint: std::cell::Cell::new(0),
            append_misses: std::cell::Cell::new(0),
        };
        if !self.transient {
            next.set_parent(self.clone(), label);
            next.set_root(Self::root_shape(self).clone());
        }
        let next = Rc::new(next);
        crate::diagnostics::count!(property_shape_nodes_created);
        if !self.transient {
            self.cache(label, &next);
        }
        next
    }
    fn reconfigure(self: &Rc<Self>, position: usize, attrs: Attributes) -> Rc<Self> {
        let label = Transition::Reconfigure {
            position: position as u32,
            attrs,
        };
        if let Some(next) = self.transition(label) {
            crate::diagnostics::count!(property_transition_hits);
            return next;
        }
        let edits = self.edits.saturating_add(1);
        let mut next = self.copied(u64::from(edits) >= DICTIONARY_EDITS);
        next.edits = edits;
        if !next.dictionary {
            next.set_parent(self.clone(), label);
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
    pub(crate) fn prototype(&self) -> Option<crate::Prototype> {
        self.store.prototype()
    }

    pub(crate) fn descriptor(&self) -> Option<Property> {
        self.position
            .map(|position| self.store.descriptor(position))
    }
    /// A present ordinary data property needs no descriptor reconstruction.
    pub(crate) fn write_existing(
        self,
        value: &Value,
    ) -> Result<(crate::value::OwnDataPropertyWrite, usize), Self> {
        use crate::value::OwnDataPropertyWrite;
        let Some(position) = self.position else {
            return Err(self);
        };
        let attrs = self.store.layout.entry(position).attrs;
        let result = if attrs.has(Attributes::ACCESSOR) {
            OwnDataPropertyWrite::NeedsSlowPath
        } else if !attrs.has(Attributes::WRITABLE) {
            OwnDataPropertyWrite::ReadOnly
        } else {
            self.store.overwrite_guarded_data(position, value.clone());
            self.store.verify_slot(position);
            OwnDataPropertyWrite::Written
        };
        Ok((result, position))
    }
    pub(crate) fn define(self, property: Property) {
        self.define_impl(property, false);
    }
    pub(crate) fn define_dynamic(self, property: Property) {
        self.define_impl(property, true);
    }
    fn define_impl(self, property: Property, dynamic: bool) {
        let attrs = Attributes::of(&property);
        match self.position {
            Some(position) => {
                if self.store.layout.entry(position).attrs != attrs {
                    if self.store.layout.dictionary {
                        self.store.edit_layout(false).private_data().entries[position].attrs =
                            attrs;
                    } else {
                        self.store.invalidate_dependencies();
                        self.store.layout = self.store.layout.reconfigure(position, attrs);
                        self.store.bump_epoch();
                    }
                    self.store.install(position, property, true);
                    self.store.verify();
                    return;
                }
                self.store.install(position, property, true);
                self.store.verify_slot(position);
                return;
            }
            None => {
                // A missing key is the only lookup outcome that can need a
                // deferred root. Existing-key writes keep this branch cold.
                self.store.materialize_write_layout();
                let dictionary = self.store.layout.dictionary
                    || self.store.len() >= DICTIONARY_WIDTH
                    || (dynamic
                        && self.store.layout.append_misses.get() > 8
                        && !self.store.layout.has_transition(Transition::Append {
                            key: self.key.id(),
                            attrs,
                        }));
                if dictionary {
                    let key = self.key.clone();
                    let same_index =
                        self.store.layout.data.borrow().entries.len() == self.store.len();
                    let layout = self.store.edit_layout(true);
                    layout.dictionary = true;
                    let order = layout.allocate_order();
                    {
                        let data = layout.private_data();
                        let slot = data.entries.len();
                        let proof = if same_index {
                            self.proof
                        } else {
                            data.probe(key.id(), false)
                        };
                        data.append(&key, attrs, proof);
                        data.entries[slot].order = order;
                    }
                    layout.len += 1;
                } else {
                    self.store.invalidate_dependencies();
                    self.store.layout = self.store.layout.append(self.key, attrs, self.proof);
                    self.store.bump_epoch();
                }
                let position = self.store.values.len();
                self.store.values.push(Value::Undefined);
                self.store.install(position, property, false);
            }
        }
        self.store.verify();
    }
    #[allow(dead_code)] // Shared semantic API; ObjectRef consumes Lookup directly.
    pub(crate) fn remove(self) -> Option<Property> {
        let position = self.position?;
        let old = self.store.descriptor(position);
        if !self.store.layout.dictionary
            && matches!(
                self.store.layout.incoming(),
                Some(Transition::Append { .. })
            )
            && position + 1 == self.store.len()
            && self.store.epoch() != u64::MAX
            && let Some(parent) = self.store.layout.parent().cloned()
        {
            self.store.invalidate_dependencies();
            self.store.layout = parent;
            self.store.bump_epoch();
            self.store.values.pop();
            if let Some(slots) = self.store.cold.as_mut() {
                slots.remove(&(position as u32));
            }
            if self
                .store
                .cold
                .as_ref()
                .is_some_and(|slots| slots.is_empty())
            {
                self.store.cold.clear();
            }
            self.store.verify();
            return Some(old);
        }
        let same_index = self.store.layout.data.borrow().entries.len() == self.store.len();
        let layout = self.store.edit_layout(false);
        let before_bytes = layout.private_data().bytes();
        let last = layout.len - 1;
        let proof = if same_index {
            self.proof
        } else {
            layout.private_data().probe(self.key.id(), false)
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
            if let Some(slots) = self.store.cold.as_mut() {
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
            layout.private_data().next_order = layout.len as u32 - 1;
            layout.private_data().reaccount(before_bytes);
            self.store.values.remove(position);
            if let Some(slots) = self.store.cold.as_mut() {
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
            .cold
            .as_ref()
            .is_some_and(|slots| slots.is_empty())
        {
            self.store.cold.clear();
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
        self.cold.clear_guard();
        if !self.layout.dictionary {
            return;
        }
        let epoch = match self.epoch().checked_add(1) {
            Some(next) if next != 0 => next,
            _ => {
                self.layout = Rc::new(self.layout.copied(true));
                1
            }
        };
        self.set_epoch(epoch);
    }
    fn edit_layout(&mut self, force_dictionary: bool) -> &mut Layout {
        self.invalidate_dependencies();
        self.bump_epoch();
        let edits = self.layout.edits.saturating_add(1);
        let dictionary = self.layout.dictionary
            || force_dictionary
            || self.len() > DICTIONARY_WIDTH
            || u64::from(edits) >= DICTIONARY_EDITS;
        if !self.layout.dictionary {
            self.layout = Rc::new(self.layout.copied(dictionary));
        }
        if dictionary && self.epoch() == 0 {
            self.set_epoch(1);
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
        if self.known_root().is_none()
            && let Some(owner) = self.owner.upgrade()
            && let Ok(mut roots) = owner.roots.try_borrow_mut()
        {
            let (kind, address) = self
                .prototype
                .as_ref()
                .map(WeakPrototype::identity)
                .unwrap_or((0, 0));
            let key = (kind, address, self.flavor);
            if roots
                .get(&key)
                .is_some_and(|w| std::ptr::eq(w.as_ptr(), self))
            {
                roots.remove(&key);
            }
            if roots.len().saturating_mul(4) < roots.capacity() {
                roots.shrink_to_fit();
            }
        }
        if let (Some(parent), Some(label)) = (self.parent(), self.incoming())
            && let Some(links) = parent.links.get()
            && let Ok(mut edges) = links.transitions.try_borrow_mut()
        {
            edges.remove(label, self as *const Layout);
        }
        if self.links.get().is_some() {
            self.budget.sub(size_of::<LayoutLinks>());
        }
        self.budget.sub(size_of::<Self>());
    }
}
