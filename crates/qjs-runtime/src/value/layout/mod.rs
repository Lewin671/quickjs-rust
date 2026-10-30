//! Ordinary named-property authority: shared immutable shapes or a private
//! dictionary, a single ID index, and keyless owning value slots.
//!
//! Owners retain bounded shape caches outside the atom domain. Layouts retain
//! the domain and only a weak cache owner, avoiding an owner/layout RC cycle.
//! Every structural edit publishes layout and slots under one mutable borrow;
//! data writes preserve identity and epoch. No borrow may cross a JS callback.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::{Rc, Weak};

use crate::property::atom::{AtomDomain, KeyId, KeyLease};
use crate::{Property, Value};

mod edit;
mod slots;
#[cfg(test)]
mod tests;

const MAX_EDGES: usize = 16;
const MAX_CACHED_NODES: usize = 4096;
const DICTIONARY_WIDTH: usize = 128;
const DICTIONARY_EDITS: u64 = 32;

/// Descriptor kind and flags live on the layout, never in data slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Attributes(u16);
impl Attributes {
    const ENUMERABLE: u16 = 1;
    const WRITABLE: u16 = 2;
    const CONFIGURABLE: u16 = 4;
    const ACCESSOR: u16 = 8;

    fn of(p: &Property) -> Self {
        Self(
            u16::from(p.enumerable)
                | (u16::from(p.writable) << 1)
                | (u16::from(p.configurable) << 2)
                | (u16::from(p.is_accessor()) << 3),
        )
    }
    fn has(self, flag: u16) -> bool {
        self.0 & flag != 0
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ShapeEntry {
    key: KeyId,
    slot: u32,
    order: u32,
    attrs: Attributes,
    reserved: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Transition {
    key: KeyId,
    attrs: Attributes,
}

/// The index is the only key lookup authority. Entries and leases run in
/// insertion order; dictionaries edit them privately without transitions.
pub(crate) struct Layout {
    domain: AtomDomain,
    owner: Weak<OwnerState>,
    entries: Vec<ShapeEntry>,
    keys: Vec<KeyLease>,
    index: HashMap<KeyId, usize>,
    dictionary: bool,
    edits: u8,
    transitions: RefCell<Vec<(Transition, Weak<Layout>)>>,
}

pub(crate) struct OwnerState {
    root: Rc<Layout>,
    retained: RefCell<VecDeque<Rc<Layout>>>,
}

/// Runtime/realm-owned roots. Cloning shares both the domain and shape cache.
/// A host must supply an owner explicitly, including for null-prototype objects.
#[derive(Clone)]
pub(crate) struct PropertyOwner(Rc<OwnerState>);
impl PropertyOwner {
    pub(crate) fn new(domain: AtomDomain) -> Self {
        if let Some(owner) = domain.shape_owner().borrow().upgrade() {
            return Self(owner);
        }
        let attachment = domain.clone();
        let owner = Rc::new_cyclic(|owner| OwnerState {
            root: Rc::new(Layout {
                domain,
                owner: owner.clone(),
                entries: Vec::new(),
                keys: Vec::new(),
                index: HashMap::new(),
                dictionary: false,
                edits: 0,
                transitions: RefCell::new(Vec::new()),
            }),
            retained: RefCell::new(VecDeque::new()),
        });
        *attachment.shape_owner().borrow_mut() = Rc::downgrade(&owner);
        Self(owner)
    }
    pub(crate) fn domain(&self) -> &AtomDomain {
        &self.0.root.domain
    }
    pub(crate) fn empty_store(&self) -> NamedStore {
        NamedStore {
            layout: self.0.root.clone(),
            values: Vec::new(),
            accessors: None,
            epoch: 0,
        }
    }
    #[cfg(test)]
    fn retained_nodes(&self) -> usize {
        1 + self.0.retained.borrow().len()
    }
}

struct AccessorPair {
    get: Option<Value>,
    set: Option<Value>,
}
type AccessorSlots = HashMap<u32, AccessorPair>;

pub(crate) struct NamedStore {
    layout: Rc<Layout>,
    values: Vec<Value>,
    accessors: Option<Box<AccessorSlots>>,
    epoch: u64,
}

/// A lookup borrows the store exclusively. Its miss proof therefore cannot
/// cross another edit or any callback capable of editing this store.
pub(crate) struct Lookup<'a> {
    store: &'a mut NamedStore,
    key: &'a KeyLease,
    position: Option<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ForeignKey;

impl NamedStore {
    pub(crate) fn domain(&self) -> &AtomDomain {
        &self.layout.domain
    }
    fn position(&self, key: &KeyLease) -> Result<Option<usize>, ForeignKey> {
        if !key.belongs_to(self.domain()) {
            return Err(ForeignKey);
        }
        crate::diagnostics::count!(property_id_probes);
        Ok(self.layout.index.get(&key.id()).copied())
    }
    pub(crate) fn lookup<'a>(&'a mut self, key: &'a KeyLease) -> Result<Lookup<'a>, ForeignKey> {
        let position = self.position(key)?;
        Ok(Lookup {
            store: self,
            key,
            position,
        })
    }
    pub(crate) fn get(&self, key: &KeyLease) -> Result<Option<Property>, ForeignKey> {
        Ok(self
            .position(key)?
            .map(|position| self.descriptor(position)))
    }
    pub(crate) fn define(&mut self, key: &KeyLease, property: Property) -> Result<(), ForeignKey> {
        self.lookup(key)?.define(property);
        Ok(())
    }
    pub(crate) fn keys(&self) -> Vec<KeyLease> {
        let mut indices = Vec::new();
        let mut strings = Vec::new();
        let mut symbols = Vec::new();
        for key in &self.layout.keys {
            if let Some(index) = key.array_index() {
                indices.push((index, key.clone()));
            } else if key.symbol().is_some() {
                symbols.push(key.clone());
            } else {
                strings.push(key.clone());
            }
        }
        indices.sort_unstable_by_key(|(index, _)| *index);
        indices
            .into_iter()
            .map(|(_, key)| key)
            .chain(strings)
            .chain(symbols)
            .collect()
    }
    pub(crate) fn layout(&self) -> &Rc<Layout> {
        &self.layout
    }
    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }
    pub(crate) fn len(&self) -> usize {
        self.layout.entries.len()
    }

    /// Structural verification has no callbacks and never repeats key coercion.
    fn verify(&self) {
        #[cfg(debug_assertions)]
        {
            let layout = &self.layout;
            assert_eq!(layout.entries.len(), layout.keys.len());
            assert_eq!(layout.entries.len(), layout.index.len());
            assert_eq!(layout.entries.len(), self.values.len());
            let mut accessor_count = 0;
            for (position, (entry, key)) in layout.entries.iter().zip(&layout.keys).enumerate() {
                assert_eq!(entry.key, key.id());
                assert!(key.belongs_to(&layout.domain));
                assert_eq!(entry.slot as usize, position);
                assert_eq!(entry.order as usize, position);
                assert_eq!(entry.reserved, 0);
                assert_eq!(layout.index.get(&entry.key), Some(&position));
                let accessor = entry.attrs.has(Attributes::ACCESSOR);
                assert_eq!(
                    self.accessors
                        .as_deref()
                        .is_some_and(|slots| slots.contains_key(&entry.slot)),
                    accessor
                );
                if accessor {
                    accessor_count += 1;
                    assert_eq!(self.values[position], Value::Undefined);
                }
                assert_eq!(Attributes::of(&self.descriptor(position)), entry.attrs);
            }
            assert_eq!(
                self.accessors.as_deref().map_or(0, HashMap::len),
                accessor_count
            );
            assert!(layout.transitions.borrow().len() <= MAX_EDGES);
            if layout.dictionary {
                assert!(layout.transitions.borrow().is_empty());
            }
        }
    }
}
