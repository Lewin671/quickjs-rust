//! Ordinary named-property layout authority, independent of payload encoding.
//!
//! A layout owns its atom domain and maps exact key identities to live slots,
//! descriptor kind and attributes. Stable value writes do not edit it. Shapes
//! are immutable and shared by content through a weak, swept domain cache;
//! transition hints are weak and bounded. After repeated structural edits an
//! object keeps a private dictionary/version instead. Callers publish payload
//! and layout edits under the same storage borrow, before any JS re-entry.

use super::atom::{Atom, AtomDomain, AtomKey, AtomKeyIdentity};
use crate::Property;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    rc::{Rc, Weak},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Encoding {
    Small,
    Dynamic,
    Shaped,
    Pair,
    NamedMap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Attributes {
    pub(crate) accessor: bool,
    pub(crate) enumerable: bool,
    pub(crate) writable: bool,
    pub(crate) configurable: bool,
}
impl Attributes {
    pub(crate) fn of(property: &Property) -> Self {
        Self {
            accessor: property.is_accessor(),
            enumerable: property.enumerable,
            writable: property.writable,
            configurable: property.configurable,
        }
    }
    pub(crate) const DEFAULT: Self = Self {
        accessor: false,
        enumerable: true,
        writable: true,
        configurable: true,
    };
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Field {
    pub(crate) key: AtomKey,
    pub(crate) attributes: Attributes,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) slot: usize,
    pub(crate) attributes: Attributes,
}

struct Shape {
    encoding: Encoding,
    fields: Vec<Field>,
    order: Vec<usize>,
    index: HashMap<AtomKeyIdentity, Entry>,
    // Hints never own successors, and even a long-lived root retains at most
    // 16 weak allocations. The domain cache shares nonadjacent histories too.
    transitions: std::cell::RefCell<Vec<Weak<Shape>>>,
}
impl Shape {
    fn matches(&self, encoding: Encoding, fields: &[Field], order: &[usize]) -> bool {
        self.encoding == encoding && self.fields == fields && self.order == order
    }
}

/// The cache keys are hashes, not owned property names: dead shapes cannot
/// retain churned atoms. Hash collisions always compare the complete layout.
#[derive(Default)]
pub(crate) struct ShapeCache {
    entries: HashMap<u64, Vec<Weak<Shape>>>,
    count: usize,
    sweep_at: usize,
}
impl ShapeCache {
    fn intern(&mut self, encoding: Encoding, fields: Vec<Field>, order: Vec<usize>) -> Rc<Shape> {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        (encoding, &fields, &order).hash(&mut hash);
        let hash = hash.finish();
        if let Some(shape) = self
            .entries
            .get(&hash)
            .into_iter()
            .flatten()
            .filter_map(Weak::upgrade)
            .find(|shape| shape.matches(encoding, &fields, &order))
        {
            return shape;
        }
        if self.count >= self.sweep_at.max(64) {
            self.count = 0;
            self.entries.retain(|_, bucket| {
                bucket.retain(|shape| shape.strong_count() > 0);
                self.count += bucket.len();
                !bucket.is_empty()
            });
            self.sweep_at = (self.count * 2).max(64);
        }
        let index = make_index(&fields);
        let shape = Rc::new(Shape {
            encoding,
            fields,
            order,
            index,
            transitions: Default::default(),
        });
        self.entries
            .entry(hash)
            .or_default()
            .push(Rc::downgrade(&shape));
        self.count += 1;
        shape
    }
}
fn make_index(fields: &[Field]) -> HashMap<AtomKeyIdentity, Entry> {
    fields
        .iter()
        .enumerate()
        .map(|(slot, field)| {
            (
                field.key.identity(),
                Entry {
                    slot,
                    attributes: field.attributes,
                },
            )
        })
        .collect()
}

struct Dictionary {
    fields: Vec<Field>,
    order: Vec<usize>,
    index: HashMap<AtomKeyIdentity, Entry>,
    encoding: Encoding,
    version: u64,
    // Owned identity: a token can never alias a dictionary at a recycled address.
    identity: Rc<()>,
}
enum State {
    Shape(Rc<Shape>),
    Dictionary(Box<Dictionary>),
}

pub(crate) struct PropertyLayout {
    domain: AtomDomain,
    state: State,
    edits: u8,
}
impl PropertyLayout {
    const DICTIONARY_AFTER: u8 = 32;
    pub(crate) fn new(domain: AtomDomain, encoding: Encoding) -> Self {
        let shape = domain
            .shapes()
            .borrow_mut()
            .intern(encoding, Vec::new(), Vec::new());
        Self {
            domain,
            state: State::Shape(shape),
            edits: 0,
        }
    }
    pub(crate) fn domain(&self) -> &AtomDomain {
        &self.domain
    }
    pub(crate) fn field(&self, slot: usize) -> Option<&Field> {
        match &self.state {
            State::Shape(shape) => shape.fields.get(slot),
            State::Dictionary(dict) => dict.fields.get(slot),
        }
    }
    fn index(&self) -> &HashMap<AtomKeyIdentity, Entry> {
        match &self.state {
            State::Shape(shape) => &shape.index,
            State::Dictionary(dict) => &dict.index,
        }
    }
    pub(crate) fn lookup(&self, key: &str) -> Option<Entry> {
        self.index().get(&self.domain.key_identity(key)?).copied()
    }
    #[allow(dead_code)] // Staged atom contract; VM routing is M1.3.
    pub(crate) fn lookup_atom(&self, atom: &Atom) -> Option<Entry> {
        if atom.belongs_to(&self.domain) {
            self.index().get(&atom.key_identity()).copied()
        } else {
            self.lookup(atom.as_str())
        }
    }
    pub(crate) fn publish(&mut self, encoding: Encoding, fields: Vec<Field>, order: Vec<usize>) {
        match &mut self.state {
            State::Shape(old) if old.matches(encoding, &fields, &order) => return,
            State::Dictionary(dict) => {
                if dict.encoding == encoding && dict.fields == fields && dict.order == order {
                    return;
                }
                // No version wraps into an old token: start a fresh identity.
                if let Some(version) = dict.version.checked_add(1) {
                    dict.version = version;
                } else {
                    dict.identity = Rc::new(());
                    dict.version = 0;
                }
                dict.index = make_index(&fields);
                dict.fields = fields;
                dict.order = order;
                dict.encoding = encoding;
                return;
            }
            _ => {}
        }
        self.edits += 1;
        if self.edits >= Self::DICTIONARY_AFTER {
            self.state = State::Dictionary(Box::new(Dictionary {
                index: make_index(&fields),
                fields,
                order,
                encoding,
                version: 0,
                identity: Rc::new(()),
            }));
            return;
        }
        let State::Shape(old) = &self.state else {
            unreachable!()
        };
        let hint = old
            .transitions
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .find(|shape| shape.matches(encoding, &fields, &order));
        let shape = hint.unwrap_or_else(|| {
            self.domain
                .shapes()
                .borrow_mut()
                .intern(encoding, fields, order)
        });
        {
            let mut hints = old.transitions.borrow_mut();
            hints.retain(|hint| hint.strong_count() > 0);
            if hints.len() == 16 {
                hints.remove(0);
            }
            hints.push(Rc::downgrade(&shape));
        }
        self.state = State::Shape(shape);
    }
    #[cfg(test)]
    pub(crate) fn token(&self) -> (usize, u64) {
        match &self.state {
            State::Shape(shape) => (Rc::as_ptr(shape) as usize, 0),
            State::Dictionary(dict) => (Rc::as_ptr(&dict.identity) as usize, dict.version),
        }
    }
    #[cfg(test)]
    pub(crate) fn is_dictionary(&self) -> bool {
        matches!(self.state, State::Dictionary(_))
    }
}

#[cfg(test)]
mod tests;
