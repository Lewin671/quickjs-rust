//! Ordinary named-property layout authority, independent of payload encoding.
//!
//! A layout owns its atom domain and maps exact key identities to live slots,
//! descriptor kind and attributes. Stable value writes do not edit it. Shapes
//! are immutable and shared by content through a weak, swept domain cache;
//! transition hints are weak and bounded. After repeated structural edits an
//! object keeps a private dictionary/version with amortized constant-time edits. Callers publish payload
//! and layout edits under the same storage borrow, before any JS re-entry.

use super::atom::{Atom, AtomDomain, AtomKey, AtomKeyIdentity};
use crate::Property;
use crate::value::name_hash::NameMap as HashMap;
use std::{
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

const NO_SLOT: usize = usize::MAX;

pub(crate) struct OrderedSlots<'a>(std::slice::Iter<'a, usize>);
impl Iterator for OrderedSlots<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        self.0.by_ref().copied().find(|&slot| slot != NO_SLOT)
    }
}

struct Dictionary {
    fields: Vec<Field>,
    // Tombstones make deletion and swap-slot remapping constant time. Compact
    // at 2 * live + 64, so fixed-live churn cannot retain an unbounded order.
    order: Vec<usize>,
    positions: Vec<usize>,
    index: HashMap<AtomKeyIdentity, Entry>,
    encoding: Encoding,
    version: u64,
    identity: Rc<()>,
}
fn order_positions(len: usize, order: &[usize]) -> Vec<usize> {
    let mut positions = vec![NO_SLOT; len];
    for (index, &slot) in order.iter().enumerate() {
        if slot != NO_SLOT {
            positions[slot] = index;
        }
    }
    positions
}
impl Dictionary {
    fn bump(&mut self) {
        if let Some(version) = self.version.checked_add(1) {
            self.version = version;
        } else {
            self.identity = Rc::new(());
            self.version = 0;
        }
    }
    fn compact_order(&mut self) {
        if self.order.len() > self.fields.len().saturating_mul(2).saturating_add(64) {
            self.order.retain(|&slot| slot != NO_SLOT);
            for (position, &slot) in self.order.iter().enumerate() {
                self.positions[slot] = position;
            }
        }
    }
}

fn remove_order_slot(order: &mut Vec<usize>, slot: usize, last: usize, swap: bool) {
    order.retain_mut(|entry| {
        if *entry == slot {
            return false;
        }
        if swap {
            if *entry == last {
                *entry = slot;
            }
        } else if *entry > slot {
            *entry -= 1;
        }
        true
    });
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
    pub(crate) fn order(&self) -> OrderedSlots<'_> {
        let order = match &self.state {
            State::Shape(shape) => &shape.order,
            State::Dictionary(dict) => &dict.order,
        };
        OrderedSlots(order.iter())
    }
    pub(crate) fn append(
        &mut self,
        text: &str,
        attributes: Attributes,
        encoding: Encoding,
        ordered: bool,
    ) {
        let field = Field {
            key: self.domain.intern_key(text),
            attributes,
        };
        match &mut self.state {
            State::Dictionary(dict) => {
                dict.bump();
                let slot = dict.fields.len();
                dict.index
                    .insert(field.key.identity(), Entry { slot, attributes });
                dict.fields.push(field);
                dict.positions
                    .push(if ordered { dict.order.len() } else { NO_SLOT });
                if ordered {
                    dict.order.push(slot);
                }
                dict.encoding = encoding;
            }
            State::Shape(shape) => {
                let mut fields = shape.fields.clone();
                let mut order = shape.order.clone();
                if ordered {
                    order.push(fields.len());
                }
                fields.push(field);
                self.publish(encoding, fields, order);
            }
        }
    }
    pub(crate) fn reconfigure(&mut self, slot: usize, attributes: Attributes) {
        match &mut self.state {
            State::Dictionary(dict) => {
                if dict.fields[slot].attributes == attributes {
                    return;
                }
                dict.bump();
                dict.fields[slot].attributes = attributes;
                dict.index
                    .get_mut(&dict.fields[slot].key.identity())
                    .expect("live layout key")
                    .attributes = attributes;
            }
            State::Shape(shape) => {
                if shape.fields[slot].attributes == attributes {
                    return;
                }
                let encoding = shape.encoding;
                let mut fields = shape.fields.clone();
                let order = shape.order.clone();
                fields[slot].attributes = attributes;
                self.publish(encoding, fields, order);
            }
        }
    }
    pub(crate) fn change_encoding(&mut self, encoding: Encoding) {
        match &mut self.state {
            State::Dictionary(dict) => {
                if dict.encoding != encoding {
                    dict.bump();
                    dict.encoding = encoding;
                }
            }
            State::Shape(shape) => {
                if shape.encoding == encoding {
                    return;
                }
                let fields = shape.fields.clone();
                let order = shape.order.clone();
                self.publish(encoding, fields, order);
            }
        }
    }
    pub(crate) fn remove(&mut self, slot: usize, swap: bool) {
        match &mut self.state {
            State::Dictionary(dict) => {
                dict.bump();
                let position = dict.positions[slot];
                if position != NO_SLOT {
                    dict.order[position] = NO_SLOT;
                }
                let removed = if swap {
                    dict.positions.swap_remove(slot);
                    dict.fields.swap_remove(slot)
                } else {
                    dict.positions.remove(slot);
                    dict.fields.remove(slot)
                };
                dict.index.remove(&removed.key.identity());
                if swap {
                    if let Some(moved) = dict.fields.get(slot) {
                        dict.index
                            .get_mut(&moved.key.identity())
                            .expect("moved layout key")
                            .slot = slot;
                        let position = dict.positions[slot];
                        if position != NO_SLOT {
                            dict.order[position] = slot;
                        }
                    }
                } else {
                    for (index, field) in dict.fields.iter().enumerate().skip(slot) {
                        dict.index
                            .get_mut(&field.key.identity())
                            .expect("shifted layout key")
                            .slot = index;
                        let position = dict.positions[index];
                        if position != NO_SLOT {
                            dict.order[position] = index;
                        }
                    }
                }
                dict.compact_order();
            }
            State::Shape(shape) => {
                let encoding = shape.encoding;
                let mut fields = shape.fields.clone();
                let mut order = shape.order.clone();
                let last = fields.len() - 1;
                if swap {
                    fields.swap_remove(slot);
                } else {
                    fields.remove(slot);
                }
                remove_order_slot(&mut order, slot, last, swap);
                self.publish(encoding, fields, order);
            }
        }
    }
    pub(crate) fn publish(&mut self, encoding: Encoding, fields: Vec<Field>, order: Vec<usize>) {
        match &mut self.state {
            State::Shape(old) if old.matches(encoding, &fields, &order) => return,
            State::Dictionary(dict) => {
                if dict.encoding == encoding && dict.fields == fields && dict.order == order {
                    return;
                }
                dict.bump();
                dict.index = make_index(&fields);
                dict.positions = order_positions(fields.len(), &order);
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
                positions: order_positions(fields.len(), &order),
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
