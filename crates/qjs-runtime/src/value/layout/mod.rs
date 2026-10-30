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
mod host;
mod index;
mod storage;
mod transitions;
use index::{IdIndex, IdProbe};
mod lazy;
mod prototype;
mod slots;
mod value_slots;
mod version;
pub(crate) use lazy::NamedStorage;
pub(crate) use prototype::{PrototypeState, WeakPrototype};
use value_slots::ValueSlots;
pub(crate) use version::{LayoutVersion, StructuralDependency};
#[cfg(test)]
mod sharing_tests;
#[cfg(test)]
mod tests;

#[cfg(target_pointer_width = "64")]
const _: () = {
    assert!(std::mem::size_of::<KeyId>() == 4);
    assert!(std::mem::size_of::<KeyLease>() == 8);
    assert!(std::mem::size_of::<ShapeEntry>() == 16);
    assert!(std::mem::size_of::<NamedStore>() == 64);
    assert!(std::mem::size_of::<RefCell<NamedStore>>() == 72);
    assert!(std::mem::size_of::<Value>() == 16);
};

const MAX_CACHED_BYTES: usize = 1024 * 1024;
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
enum Transition {
    Append { key: KeyId, attrs: Attributes },
    Reconfigure { position: u32, attrs: Attributes },
    Integrity { frozen: bool },
}

/// The index is the only key lookup authority. Shapes keep insertion-ordered
/// entries; dictionaries retain explicit order numbers when slots move, and
/// edit privately without transitions.
pub(crate) struct Layout {
    budget: storage::Budget,
    domain: AtomDomain,
    owner: Weak<OwnerState>,
    data: Rc<RefCell<storage::LayoutData>>,
    len: usize,
    parent: Option<Rc<Layout>>,
    incoming: Option<Transition>,
    dictionary: bool,
    edits: u8,
    next_order: u32,
    transitions: RefCell<transitions::Transitions>,
    root: Option<Rc<Layout>>,
    prototype: Option<WeakPrototype>,
    prototype_value: RefCell<Weak<crate::Prototype>>,
    parent_state: Weak<PrototypeState>,
    mode: Option<Rc<PrototypeState>>,
    flavor: u16,
}

pub(crate) struct OwnerState {
    root: Rc<Layout>,
    roots: RefCell<crate::value::name_hash::NameMap<(u8, usize, u16), Weak<Layout>>>,
    retained: RefCell<transitions::Retention>,
}

#[cfg(feature = "perf-counters")]
impl Drop for OwnerState {
    fn drop(&mut self) {
        let released = 1 + self.retained.get_mut().nodes.len() as u64;
        crate::diagnostics::update(|counters| {
            // A diagnostic reset may occur while owners are live.
            counters.property_cached_shape_nodes = counters
                .property_cached_shape_nodes
                .saturating_sub(released);
        });
    }
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
        let budget = storage::Budget::default();
        budget.add(size_of::<Layout>());
        let owner = Rc::new_cyclic(|owner| OwnerState {
            root: Rc::new(Layout {
                budget: budget.clone(),
                domain,
                owner: owner.clone(),
                data: Rc::new(RefCell::new(storage::LayoutData::new(budget.clone()))),
                len: 0,
                parent: None,
                incoming: None,
                dictionary: false,
                edits: 0,
                next_order: 0,
                transitions: RefCell::new(transitions::Transitions::default()),
                root: None,
                prototype: None,
                prototype_value: RefCell::new(Weak::new()),
                parent_state: Weak::new(),
                mode: None,
                flavor: 0,
            }),
            roots: RefCell::default(),
            retained: RefCell::new(transitions::Retention::default()),
        });
        *attachment.shape_owner().borrow_mut() = Rc::downgrade(&owner);
        crate::diagnostics::count!(property_cached_shape_nodes);
        Self(owner)
    }
    pub(crate) fn domain(&self) -> &AtomDomain {
        &self.0.root.domain
    }
    pub(crate) fn empty_store(&self) -> NamedStore {
        NamedStore {
            layout: self.0.root.clone(),
            values: ValueSlots::Empty,
            accessors: None,
            epoch: 0,
            prototype: None,
        }
    }
    #[cfg(test)]
    fn retained_nodes(&self) -> usize {
        1 + self.0.retained.borrow().nodes.len()
    }
}

type AccessorSlots = HashMap<u32, Rc<super::property::AccessorState>>;

pub(crate) struct NamedStore {
    layout: Rc<Layout>,
    values: ValueSlots,
    accessors: Option<Box<AccessorSlots>>,
    epoch: u64,
    prototype: Option<Rc<crate::Prototype>>,
}

/// A lookup borrows the store exclusively. Its miss proof therefore cannot
/// cross another edit or any callback capable of editing this store.
pub(crate) struct Lookup<'a> {
    store: &'a mut NamedStore,
    key: &'a KeyLease,
    position: Option<usize>,
    proof: IdProbe,
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
        Ok(self.layout.probe(key.id()).position)
    }
    pub(crate) fn lookup<'a>(&'a mut self, key: &'a KeyLease) -> Result<Lookup<'a>, ForeignKey> {
        if !key.belongs_to(self.domain()) {
            return Err(ForeignKey);
        }
        let proof = self.layout.probe(key.id());
        Ok(Lookup {
            store: self,
            key,
            position: proof.position,
            proof,
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
    pub(crate) fn key_enumerability(&self) -> Vec<(KeyLease, bool)> {
        let keys = self.layout.keys();
        let fields = self.layout.entries();
        let mut entries: Vec<_> = keys.iter().zip(fields.iter()).collect();
        entries.sort_unstable_by_key(|(key, entry)| match key.array_index() {
            Some(index) => (0, index),
            None if key.symbol().is_none() => (1, entry.order),
            None => (2, entry.order),
        });
        entries
            .into_iter()
            .map(|(key, entry)| (key.clone(), entry.attrs.has(Attributes::ENUMERABLE)))
            .collect()
    }
    fn verify_slot(&self, position: usize) {
        #[cfg(debug_assertions)]
        {
            assert_eq!(self.layout.len, self.values.len());
            assert_eq!(self.layout.len, self.layout.keys().len());
            assert!(self.layout.data.borrow().entries.len() >= self.layout.len);
            let entry = self.layout.entry(position);
            assert_eq!(entry.slot as usize, position);
            assert_eq!(entry.key, self.layout.keys()[position].id());
            assert_eq!(self.layout.verify_position(entry.key), Some(position));
            assert_eq!(
                self.accessors
                    .as_deref()
                    .is_some_and(|slots| slots.contains_key(&entry.slot)),
                entry.attrs.has(Attributes::ACCESSOR)
            );
            if entry.attrs.has(Attributes::ACCESSOR) {
                assert_eq!(self.values[position], Value::Undefined);
            }
        }
        #[cfg(not(debug_assertions))]
        let _ = position;
    }
    pub(crate) fn keys(&self) -> Vec<KeyLease> {
        let mut indices = Vec::new();
        let mut strings = Vec::new();
        let mut symbols = Vec::new();
        for (key, entry) in self.layout.keys().iter().zip(self.layout.entries().iter()) {
            if let Some(index) = key.array_index() {
                indices.push((index, key.clone()));
            } else if key.symbol().is_some() {
                symbols.push((entry.order, key.clone()));
            } else {
                strings.push((entry.order, key.clone()));
            }
        }
        indices.sort_unstable_by_key(|(index, _)| *index);
        if self.layout.dictionary {
            strings.sort_unstable_by_key(|(order, _)| *order);
            symbols.sort_unstable_by_key(|(order, _)| *order);
        }
        indices
            .into_iter()
            .chain(strings)
            .chain(symbols)
            .map(|(_, key)| key)
            .collect()
    }
    pub(crate) fn layout(&self) -> &Rc<Layout> {
        &self.layout
    }
    #[cfg(test)]
    pub(crate) fn test_set_epoch(&mut self, epoch: u64) {
        self.epoch = epoch;
    }
    #[cfg(test)]
    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }
    #[cfg(test)]
    pub(crate) fn is_dictionary(&self) -> bool {
        self.layout.dictionary
    }
    pub(crate) fn len(&self) -> usize {
        self.layout.len
    }

    /// Structural verification has no callbacks and never repeats key coercion.
    fn verify(&self) {
        #[cfg(debug_assertions)]
        {
            let layout = &self.layout;
            assert_eq!(layout.len, layout.keys().len());
            assert!(layout.data.borrow().entries.len() >= layout.len);
            assert_eq!(layout.len, self.values.len());
            let mut accessor_count = 0;
            let mut orders = std::collections::HashSet::new();
            for (position, (entry, key)) in layout
                .entries()
                .iter()
                .zip(layout.keys().iter())
                .enumerate()
            {
                assert_eq!(entry.key, key.id());
                assert!(key.belongs_to(&layout.domain));
                assert_eq!(entry.slot as usize, position);
                assert!(entry.order < layout.next_order);
                assert!(orders.insert(entry.order));
                assert_eq!(entry.reserved, 0);
                assert_eq!(layout.verify_position(entry.key), Some(position));
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
            if layout.dictionary {
                assert!(layout.transitions.borrow().is_empty());
            }
        }
    }
}
