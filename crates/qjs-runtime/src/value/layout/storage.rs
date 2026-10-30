//! Append-only descriptor arrays shared by prefix shapes. A tail append updates
//! the shared index once; prefix lookup rejects positions outside its own length.
//! Branches copy only their prefix. Private edits always detach the array first.
use super::*;
use std::cell::{Cell, Ref};

#[derive(Clone, Default)]
pub(super) struct Budget(Rc<Cell<usize>>);
impl Budget {
    pub(super) fn add(&self, bytes: usize) {
        self.0.set(self.0.get() + bytes);
    }
    pub(super) fn sub(&self, bytes: usize) {
        self.0.set(self.0.get() - bytes);
    }
    pub(super) fn bytes(&self) -> usize {
        self.0.get()
    }
}

pub(super) struct LayoutData {
    budget: Budget,
    key_bytes: usize,
    pub(super) entries: Vec<ShapeEntry>,
    pub(super) keys: Vec<KeyLease>,
    pub(super) index: IdIndex,
}
impl LayoutData {
    pub(super) fn new(budget: Budget) -> Self {
        let data = Self {
            budget,
            key_bytes: 0,
            entries: Vec::new(),
            keys: Vec::new(),
            index: IdIndex::new(),
        };
        data.budget.add(data.bytes());
        data
    }
    pub(super) fn prefix(&self, len: usize) -> Self {
        for _ in 0..len {
            crate::diagnostics::count!(property_shape_field_copies);
        }
        let index = if len == self.entries.len() {
            self.index.clone()
        } else {
            let mut index = IdIndex::new();
            for (position, entry) in self.entries[..len].iter().enumerate() {
                index.insert_at(index.probe_uncounted(entry.key), position);
            }
            index
        };
        let data = Self {
            budget: self.budget.clone(),
            key_bytes: self.keys[..len].iter().map(Self::key_bytes).sum(),
            entries: self.entries[..len].to_vec(),
            keys: self.keys[..len].to_vec(),
            index,
        };
        data.budget.add(data.bytes());
        data
    }
    pub(super) fn bytes(&self) -> usize {
        size_of::<Self>()
            + self.entries.capacity() * size_of::<ShapeEntry>()
            + self.keys.capacity() * size_of::<KeyLease>()
            + self.index.bytes()
            + self.key_bytes
    }
    fn key_bytes(key: &KeyLease) -> usize {
        key.string_text().map_or(0, str::len) + 64
    }
    pub(super) fn append(&mut self, key: &KeyLease, attrs: Attributes, proof: IdProbe) {
        let before = self.bytes();
        let slot = self.entries.len() as u32;
        self.entries.push(ShapeEntry {
            key: key.id(),
            slot,
            order: slot,
            attrs,
            reserved: 0,
        });
        self.keys.push(key.clone());
        self.key_bytes += Self::key_bytes(key);
        self.index.insert_at(proof, slot as usize);
        self.budget.add(self.bytes() - before);
    }
    pub(super) fn reaccount(&mut self, before: usize) {
        self.key_bytes = self.keys.iter().map(Self::key_bytes).sum();
        self.budget.sub(before);
        self.budget.add(self.bytes());
    }
    pub(super) fn edit(&mut self, edit: impl FnOnce(&mut Self)) {
        let before = self.bytes();
        edit(self);
        self.key_bytes = self.keys.iter().map(Self::key_bytes).sum();
        self.budget.sub(before);
        self.budget.add(self.bytes());
    }
}
impl Drop for LayoutData {
    fn drop(&mut self) {
        self.budget.sub(self.bytes());
    }
}
impl Layout {
    pub(super) fn entries(&self) -> Ref<'_, [ShapeEntry]> {
        Ref::map(self.data.borrow(), |data| &data.entries[..self.len])
    }
    pub(super) fn keys(&self) -> Ref<'_, [KeyLease]> {
        Ref::map(self.data.borrow(), |data| &data.keys[..self.len])
    }
    pub(super) fn entry(&self, position: usize) -> ShapeEntry {
        self.entries()[position]
    }
    pub(super) fn entry_at(&self, position: usize) -> Option<ShapeEntry> {
        self.entries().get(position).copied()
    }
    pub(super) fn probe(&self, key: KeyId) -> IdProbe {
        let mut proof = self.data.borrow().index.probe(key);
        proof.position = proof.position.filter(|&position| position < self.len);
        proof
    }
    pub(super) fn verify_position(&self, key: KeyId) -> Option<usize> {
        self.data
            .borrow()
            .index
            .verify_position(key)
            .filter(|&position| position < self.len)
    }
    pub(super) fn private_data(&mut self) -> &mut LayoutData {
        Rc::get_mut(&mut self.data)
            .expect("detached edit data")
            .get_mut()
    }
}
