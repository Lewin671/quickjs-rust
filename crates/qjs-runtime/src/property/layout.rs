//! Ordinary named-property layouts: the lookup authority of every store.
//!
//! A store pairs one [`Layout`] pointer with a payload of value slots. The
//! layout maps each key to its slot and attributes, fixes the enumeration
//! order and names the payload encoding; the payload holds no keys. The rules
//! are in `docs/design/property-layouts.md`; the invariants that keep them
//! correct are:
//!
//! - **Published shapes never change.** An edit moves the store to another
//!   node, so a node pointer is a valid cache token for as long as it lives.
//!   Only a private [`Dictionary`] is edited in place, and only through the
//!   store that uniquely owns it, before any observer runs.
//! - **Transitions are owned edges.** A parent owns its children strongly and
//!   a child holds no parent, so there is no cycle. A shape holds at most
//!   [`APPEND_EDGE_LIMIT`] append edges, one re-encoding edge per encoding and
//!   [`EDIT_EDGE_LIMIT`] edit edges; the oldest is evicted when full. Edges
//!   are recorded only while the domain's live edge-created shapes are under
//!   [`LIVE_SHAPE_BUDGET`].
//! - **Slots follow order in shapes.** A shape's slot order is its insertion
//!   order, so a delete shifts later slots down. A dictionary moves the last
//!   slot into a deleted one and keeps order separately, with tombstones.
//! - **Keys are compared, not re-interned.** An append that follows an edge
//!   compares the key's text with the edge's atom (pointer first) and interns
//!   nothing; only a miss interns. Shapes of up to [`LINEAR_LOOKUP_LIMIT`]
//!   fields probe linearly and hash nothing; larger nodes build an index after
//!   repeated read or write probes, including misses.

use std::cell::RefCell;
use std::fmt;
use std::rc::{Rc, Weak};

use super::atom::{Atom, AtomDomain};
use crate::Property;
use crate::value::name_hash::NameMap;

/// Shapes up to this many fields find a key by comparing text linearly.
pub(crate) const LINEAR_LOOKUP_LIMIT: usize = 12;
/// An append past this many fields switches the store to a dictionary.
pub(crate) const SHAPE_FIELD_LIMIT: usize = 128;
/// Non-append edits after which a store switches to a dictionary.
pub(crate) const DICTIONARY_AFTER_EDITS: u8 = 32;
/// Append edges a shape keeps; the oldest is evicted past this.
pub(crate) const APPEND_EDGE_LIMIT: usize = 16;
/// Delete, reconfigure and bulk-edit edges a shape keeps.
pub(crate) const EDIT_EDGE_LIMIT: usize = 8;
/// Live shapes created by edge misses, per domain, above which a miss
/// records no edge.
pub(crate) const LIVE_SHAPE_BUDGET: usize = 4096;

const NO_SLOT: u32 = u32::MAX;
/// Linear probes of a large shape before it builds its text index.
const INDEX_AFTER_PROBES: u8 = 4;

/// How a store's payload interprets its slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Encoding {
    /// Descriptor slots of a store built by ordinary adds, up to twelve keys.
    Small = 0,
    /// Descriptor slots of a grown or structurally edited store.
    Dynamic = 1,
    /// Descriptor slots of an object literal.
    Shaped = 2,
    /// Two enumerable, writable, configurable data values of a literal.
    Pair = 3,
}

const ENCODINGS: usize = 4;

/// A field's attributes: the descriptor kind, its three flags, and whether
/// the key takes part in enumeration order.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Attributes(u8);

impl Attributes {
    const ENUMERABLE: u8 = 1;
    const WRITABLE: u8 = 2;
    const CONFIGURABLE: u8 = 4;
    const ACCESSOR: u8 = 8;
    const UNORDERED: u8 = 16;

    /// An ordinary enumerable, writable, configurable data property.
    pub(crate) const DEFAULT: Self = Self(Self::ENUMERABLE | Self::WRITABLE | Self::CONFIGURABLE);

    pub(crate) fn of(property: &Property) -> Self {
        let mut bits = 0;
        if property.enumerable {
            bits |= Self::ENUMERABLE;
        }
        if property.is_accessor() {
            bits |= Self::ACCESSOR;
        } else if property.writable {
            bits |= Self::WRITABLE;
        }
        if property.configurable {
            bits |= Self::CONFIGURABLE;
        }
        Self(bits)
    }

    /// The same attributes for a key outside enumeration order.
    pub(crate) fn unordered(self) -> Self {
        Self(self.0 | Self::UNORDERED)
    }

    /// `self` with the descriptor part of `property`, keeping the order bit.
    pub(crate) fn with_descriptor_of(self, property: &Property) -> Self {
        Self(Self::of(property).0 | (self.0 & Self::UNORDERED))
    }

    pub(crate) fn is_ordered(self) -> bool {
        self.0 & Self::UNORDERED == 0
    }

    pub(crate) fn is_accessor(self) -> bool {
        self.0 & Self::ACCESSOR != 0
    }

    pub(crate) fn is_writable(self) -> bool {
        self.0 & Self::WRITABLE != 0
    }

    pub(crate) fn is_enumerable(self) -> bool {
        self.0 & Self::ENUMERABLE != 0
    }

    pub(crate) fn is_configurable(self) -> bool {
        self.0 & Self::CONFIGURABLE != 0
    }

    fn bits(self) -> u8 {
        self.0
    }
}

impl fmt::Debug for Attributes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Attributes({:#07b})", self.0)
    }
}

/// One key of a layout and its attributes. The atom's text is kept inline,
/// so a probe compares a field's key without following the atom.
#[derive(Clone)]
pub(crate) struct Field {
    text: Rc<str>,
    pub(crate) atom: Atom,
    pub(crate) attributes: Attributes,
}

impl Field {
    pub(crate) fn new(atom: Atom, attributes: Attributes) -> Self {
        Self {
            text: Rc::clone(atom.text()),
            atom,
            attributes,
        }
    }

    #[inline(always)]
    pub(crate) fn text(&self) -> &Rc<str> {
        &self.text
    }
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        self.atom == other.atom && self.attributes == other.attributes
    }
}

impl Eq for Field {}

/// Whether a field's key is `key`: length, pointer, first byte, then bounded
/// head/tail words for short keys or a general byte comparison for long keys.
/// Distinct equal-length keys usually differ in their first byte.
#[inline(always)]
fn text_matches(text: &str, key: &str) -> bool {
    let (text, key) = (text.as_bytes(), key.as_bytes());
    if text.len() != key.len() {
        return false;
    }
    if std::ptr::eq(text.as_ptr(), key.as_ptr()) {
        return true;
    }
    if let (Some(left), Some(right)) = (text.first(), key.first())
        && left != right
    {
        return false;
    }
    let len = text.len();
    // Fixed-width slice comparisons become safe word loads rather than a
    // general memcmp call. Overlapping head/tail words cover every byte.
    match len {
        0 | 1 => true,
        2..=3 => text[len - 2..] == key[len - 2..],
        4..=7 => text[..4] == key[..4] && text[len - 4..] == key[len - 4..],
        8..=15 => text[..8] == key[..8] && text[len - 8..] == key[len - 8..],
        _ => text == key,
    }
}

/// A bulk attribute edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BulkEdit {
    Seal,
    Freeze,
}

impl BulkEdit {
    fn apply(self, attributes: Attributes) -> Attributes {
        let mut bits = attributes.bits() & !Attributes::CONFIGURABLE;
        if self == Self::Freeze && !attributes.is_accessor() {
            bits &= !Attributes::WRITABLE;
        }
        Attributes(bits)
    }
}

#[derive(Clone, PartialEq, Eq)]
enum EditLabel {
    Remove(u32),
    Reconfigure(u32, Attributes),
    Bulk(BulkEdit),
}

/// An append edge: the appended field -- its text inline, so a hit compares
/// the key without following the atom -- and the child it leads to.
struct AppendEdge {
    field: Field,
    child: Layout,
}

struct EditEdge {
    label: EditLabel,
    child: Layout,
}

#[derive(Default)]
struct Edges {
    appends: Vec<AppendEdge>,
    recodes: [Option<Layout>; ENCODINGS],
    edits: Vec<EditEdge>,
}

/// A published, immutable layout.
pub(crate) struct Shape {
    domain: AtomDomain,
    encoding: Encoding,
    /// Counted against the domain's live-shape budget.
    counted: bool,
    fields: Box<[Field]>,
    /// Text index for shapes past the linear-lookup limit, built at the
    /// shape's repeated probes: shapes an object only passes through while it
    /// grows are never probed and never pay for one.
    index: std::cell::OnceCell<Box<NameMap<Rc<str>, u32>>>,
    /// Probes made by linear scan before the index is built.
    unindexed_probes: std::cell::Cell<u8>,
    /// Content hash of the encoding and fields.
    hash: u64,
    edges: RefCell<Edges>,
}

impl Drop for Shape {
    fn drop(&mut self) {
        if self.counted {
            self.domain.release_live_shape();
        }
    }
}

/// A private layout edited in place by the store that owns it.
pub(crate) struct Dictionary {
    domain: AtomDomain,
    encoding: Encoding,
    /// Fields by slot; a delete moves the last slot into the hole.
    fields: Vec<Field>,
    /// Slots in enumeration order; [`NO_SLOT`] marks a deleted entry.
    order: Vec<u32>,
    /// Each slot's position in `order`.
    positions: Vec<u32>,
    index: NameMap<Rc<str>, u32>,
    version: u64,
    identity: Rc<()>,
}

impl Dictionary {
    /// An independent copy with a fresh identity.
    fn copy(&self) -> Self {
        crate::diagnostics::count!(property_dictionary_fields_copied, self.fields.len());
        Self {
            domain: self.domain.clone(),
            encoding: self.encoding,
            fields: self.fields.clone(),
            order: self.order.clone(),
            positions: self.positions.clone(),
            index: self.index.clone(),
            version: 0,
            identity: Rc::new(()),
        }
    }

    fn bump(&mut self) {
        match self.version.checked_add(1) {
            Some(version) => self.version = version,
            None => {
                self.identity = Rc::new(());
                self.version = 0;
            }
        }
    }

    /// Drops tombstones once they outnumber twice the live keys plus a
    /// constant, so fixed-size churn keeps `order` bounded.
    fn compact_order(&mut self) {
        if self.order.len() > self.fields.len().saturating_mul(2).saturating_add(64) {
            self.order.retain(|&slot| slot != NO_SLOT);
            for (position, &slot) in self.order.iter().enumerate() {
                self.positions[slot as usize] = position as u32;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn order_len(&self) -> usize {
        self.order.len()
    }

    #[cfg(test)]
    pub(crate) fn version(&self) -> u64 {
        self.version
    }

    /// Asserts that `order`, `positions` and `index` describe exactly the
    /// live fields: each live slot sits once in `order` at its recorded
    /// position, every other entry is a tombstone, and the index maps each
    /// live key to its slot and nothing else. Returns the tombstone count.
    #[cfg(test)]
    pub(crate) fn assert_consistent(&self) -> usize {
        assert_eq!(self.positions.len(), self.fields.len());
        assert_eq!(self.index.len(), self.fields.len());
        for (slot, field) in self.fields.iter().enumerate() {
            let position = self.positions[slot] as usize;
            assert_eq!(self.order[position], slot as u32, "slot {slot}");
            assert_eq!(self.index.get(field.text().as_ref()), Some(&(slot as u32)));
        }
        let live = self.order.iter().filter(|&&slot| slot != NO_SLOT).count();
        assert_eq!(live, self.fields.len(), "order holds each live slot once");
        self.order.len() - live
    }

    /// Identity of this dictionary for a cache that holds it weakly.
    pub(crate) fn identity(&self) -> &Rc<()> {
        &self.identity
    }
}

/// A layout node: a shared shape or a private dictionary.
pub(crate) enum LayoutNode {
    Shape(Shape),
    Dictionary(Dictionary),
}

impl fmt::Debug for LayoutNode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct(if self.is_dictionary() {
                "Dictionary"
            } else {
                "Shape"
            })
            .field("encoding", &self.encoding())
            .field("fields", &self.len())
            .finish()
    }
}

/// One store's layout pointer.
pub(crate) type Layout = Rc<LayoutNode>;

/// The empty root of each encoding for one domain. Realms of the domain hold
/// it; the domain refers to it only weakly, so shapes, which hold their
/// domain, form no cycle through it.
pub(crate) struct DomainRoots {
    roots: [Layout; ENCODINGS],
}

impl DomainRoots {
    pub(crate) fn new(domain: &AtomDomain) -> Self {
        crate::diagnostics::count!(property_root_sets_created);
        let root = |encoding| {
            Rc::new(LayoutNode::Shape(Shape::new(
                domain.clone(),
                encoding,
                Box::new([]),
                seed(encoding),
                false,
            )))
        };
        Self {
            roots: [
                root(Encoding::Small),
                root(Encoding::Dynamic),
                root(Encoding::Shaped),
                root(Encoding::Pair),
            ],
        }
    }

    pub(crate) fn root(&self, encoding: Encoding) -> &Layout {
        &self.roots[encoding as usize]
    }
}

const HASH_SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

fn mix(hash: u64, word: u64) -> u64 {
    (hash.rotate_left(5) ^ word).wrapping_mul(HASH_SEED)
}

fn seed(encoding: Encoding) -> u64 {
    mix(0, encoding as u64 + 1)
}

fn fold_field(hash: u64, field: &Field) -> u64 {
    mix(
        hash,
        u64::from(field.atom.id().get()) | (u64::from(field.attributes.bits()) << 32),
    )
}

fn content_hash(encoding: Encoding, fields: &[Field]) -> u64 {
    fields.iter().fold(seed(encoding), fold_field)
}

fn build_index(fields: &[Field]) -> Box<NameMap<Rc<str>, u32>> {
    Box::new(
        fields
            .iter()
            .enumerate()
            .map(|(slot, field)| (Rc::clone(field.text()), slot as u32))
            .collect(),
    )
}

impl Shape {
    fn new(
        domain: AtomDomain,
        encoding: Encoding,
        fields: Box<[Field]>,
        hash: u64,
        counted: bool,
    ) -> Self {
        crate::diagnostics::count!(property_shapes_created);
        crate::diagnostics::count!(property_shape_fields_copied, fields.len());
        if counted {
            domain.acquire_live_shape();
        }
        Self {
            domain,
            encoding,
            counted,
            fields,
            index: std::cell::OnceCell::new(),
            unindexed_probes: std::cell::Cell::new(0),
            hash,
            edges: RefCell::default(),
        }
    }

    #[inline(always)]
    fn lookup(&self, key: &str) -> Option<usize> {
        if self.fields.len() > LINEAR_LOOKUP_LIMIT {
            if let Some(index) = self.index.get() {
                crate::diagnostics::count!(property_lookup_hashes);
                return index.get(key).map(|&slot| slot as usize);
            }
            return self.first_lookups(key);
        }
        // This bounded probe is the hot path for small stores. Keep its
        // scan in this body: Iterator::position can become a separately
        // instantiated call when unrelated code changes its inline budget.
        let mut slot = 0;
        while slot < self.fields.len() {
            if text_matches(self.fields[slot].text(), key) {
                return Some(slot);
            }
            slot += 1;
        }
        None
    }

    /// A large shape's first probes scan its fields; the index is built once
    /// the shape has been probed a few times. A shape an object only passes
    /// through while it grows is probed once, by that add, and never indexed.
    #[inline(never)]
    fn first_lookups(&self, key: &str) -> Option<usize> {
        let probes = self.unindexed_probes.get();
        if probes < INDEX_AFTER_PROBES {
            self.unindexed_probes.set(probes + 1);
            return self
                .fields
                .iter()
                .position(|field| text_matches(field.text(), key));
        }
        crate::diagnostics::count!(property_lookup_hashes);
        let index = self.index.get_or_init(|| build_index(&self.fields));
        index.get(key).map(|&slot| slot as usize)
    }

    /// Whether this shape has exactly `encoding` and `fields`.
    fn has_content(&self, encoding: Encoding, fields: &[Field]) -> bool {
        crate::diagnostics::count!(property_field_vector_compares);
        self.encoding == encoding && *self.fields == *fields
    }

    /// The child an existing append edge leads to, without interning.
    #[inline(always)]
    fn append_edge(&self, key: &str, attributes: Attributes) -> Option<Layout> {
        let edges = self.edges.borrow();
        let edge = edges.appends.iter().rev().find(|edge| {
            text_matches(edge.field.text(), key) && edge.field.attributes == attributes
        })?;
        crate::diagnostics::count!(property_edge_hits);
        Some(Rc::clone(&edge.child))
    }

    /// The child for appending `key`, through an edge when one exists.
    #[inline(never)]
    fn append_child(this: &Layout, key: &str, attributes: Attributes) -> Layout {
        let LayoutNode::Shape(shape) = &**this else {
            unreachable!("append_child is called on shapes")
        };
        if let Some(child) = shape.append_edge(key, attributes) {
            return child;
        }
        let atom = shape.domain.intern_key(key);
        Self::append_atom(this, atom, attributes)
    }

    #[inline(never)]
    fn append_shared_child(this: &Layout, key: &Rc<str>, attributes: Attributes) -> Layout {
        let LayoutNode::Shape(shape) = &**this else {
            unreachable!("append_shared_child is called on shapes")
        };
        if let Some(child) = shape.append_edge(key, attributes) {
            return child;
        }
        Self::append_atom(this, shape.domain.intern_shared_key(key), attributes)
    }

    fn append_atom(this: &Layout, atom: Atom, attributes: Attributes) -> Layout {
        let LayoutNode::Shape(shape) = &**this else {
            unreachable!("append_atom is called on shapes")
        };
        // A key in another stored form can intern to an existing edge's atom.
        if let Some(edge) = shape
            .edges
            .borrow()
            .appends
            .iter()
            .find(|edge| edge.field.attributes == attributes && edge.field.atom == atom)
        {
            crate::diagnostics::count!(property_edge_hits);
            return Rc::clone(&edge.child);
        }
        crate::diagnostics::count!(property_edge_misses);
        let field = Field::new(atom, attributes);
        let hash = fold_field(shape.hash, &field);
        let mut fields = Vec::with_capacity(shape.fields.len() + 1);
        fields.extend_from_slice(&shape.fields);
        fields.push(field.clone());
        let record = shape.domain.live_shapes() < LIVE_SHAPE_BUDGET;
        let child = Rc::new(LayoutNode::Shape(Shape::new(
            shape.domain.clone(),
            shape.encoding,
            fields.into_boxed_slice(),
            hash,
            true,
        )));
        if record {
            let mut edges = shape.edges.borrow_mut();
            if edges.appends.len() >= APPEND_EDGE_LIMIT {
                edges.appends.remove(0);
            }
            edges.appends.push(AppendEdge {
                field,
                child: Rc::clone(&child),
            });
        }
        child
    }

    /// The same fields under `encoding`, through a re-encoding edge.
    fn recode_child(this: &Layout, encoding: Encoding) -> Layout {
        let LayoutNode::Shape(shape) = &**this else {
            unreachable!("recode_child is called on shapes")
        };
        if let Some(child) = &shape.edges.borrow().recodes[encoding as usize] {
            crate::diagnostics::count!(property_edge_hits);
            return Rc::clone(child);
        }
        crate::diagnostics::count!(property_edge_misses);
        let fields: Box<[Field]> = shape.fields.clone();
        let record = shape.domain.live_shapes() < LIVE_SHAPE_BUDGET;
        let hash = content_hash(encoding, &fields);
        let child = Rc::new(LayoutNode::Shape(Shape::new(
            shape.domain.clone(),
            encoding,
            fields,
            hash,
            true,
        )));
        if record {
            shape.edges.borrow_mut().recodes[encoding as usize] = Some(Rc::clone(&child));
        }
        child
    }

    /// The shape an edit produces: an edit edge, then the content cache,
    /// then a new shape.
    fn edit_child(
        this: &Layout,
        label: EditLabel,
        encoding: Encoding,
        build: impl FnOnce(&[Field]) -> Vec<Field>,
    ) -> Layout {
        let LayoutNode::Shape(shape) = &**this else {
            unreachable!("edit_child is called on shapes")
        };
        if let Some(edge) = shape
            .edges
            .borrow()
            .edits
            .iter()
            .find(|edge| edge.label == label)
        {
            crate::diagnostics::count!(property_edge_hits);
            return Rc::clone(&edge.child);
        }
        crate::diagnostics::count!(property_edge_misses);
        let fields = build(&shape.fields);
        let hash = content_hash(encoding, &fields);
        let record = shape.domain.live_shapes() < LIVE_SHAPE_BUDGET;
        // A cached shape may be an ancestor of this one, so following it is
        // never recorded as an edge: an edge to an ancestor would be a cycle.
        if let Some(cached) = shape.domain.cached_shape(hash, |candidate| {
            matches!(&**candidate, LayoutNode::Shape(candidate) if candidate.has_content(encoding, &fields))
        }) {
            return cached;
        }
        let child = Rc::new(LayoutNode::Shape(Shape::new(
            shape.domain.clone(),
            encoding,
            fields.into_boxed_slice(),
            hash,
            true,
        )));
        shape.domain.cache_shape(hash, &child);
        if record {
            let mut edges = shape.edges.borrow_mut();
            if edges.edits.len() >= EDIT_EDGE_LIMIT {
                edges.edits.remove(0);
            }
            edges.edits.push(EditEdge {
                label,
                child: Rc::clone(&child),
            });
        }
        child
    }

    #[cfg(test)]
    pub(crate) fn append_edge_keys(&self) -> Vec<String> {
        self.edges
            .borrow()
            .appends
            .iter()
            .map(|edge| edge.field.atom.as_str().to_owned())
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn edge_count(&self) -> usize {
        let edges = self.edges.borrow();
        edges.appends.len() + edges.edits.len() + edges.recodes.iter().flatten().count()
    }
}

/// What a removal did to the slots, which the payload must mirror.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SlotRemoval {
    /// Later slots moved down by one.
    Shift,
    /// The last slot moved into the removed one.
    SwapLast,
}

impl LayoutNode {
    pub(crate) fn domain(&self) -> &AtomDomain {
        match self {
            Self::Shape(shape) => &shape.domain,
            Self::Dictionary(dictionary) => &dictionary.domain,
        }
    }

    #[inline(always)]
    pub(crate) fn encoding(&self) -> Encoding {
        match self {
            Self::Shape(shape) => shape.encoding,
            Self::Dictionary(dictionary) => dictionary.encoding,
        }
    }

    #[inline(always)]
    pub(crate) fn fields(&self) -> &[Field] {
        match self {
            Self::Shape(shape) => &shape.fields,
            Self::Dictionary(dictionary) => &dictionary.fields,
        }
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.fields().len()
    }

    #[inline(always)]
    pub(crate) fn field(&self, slot: usize) -> Option<&Field> {
        self.fields().get(slot)
    }

    /// The field at `slot` of a `Small` shape: one variant test, for the
    /// typed loop tier's inlined numeric field access.
    #[inline(always)]
    pub(crate) fn small_field(&self, slot: usize) -> Option<&Field> {
        match self {
            Self::Shape(shape) if shape.encoding == Encoding::Small => shape.fields.get(slot),
            _ => None,
        }
    }

    pub(crate) fn is_dictionary(&self) -> bool {
        matches!(self, Self::Dictionary(_))
    }

    /// The slot holding `key`.
    #[inline]
    pub(crate) fn lookup(&self, key: &str) -> Option<usize> {
        match self {
            Self::Shape(shape) => shape.lookup(key),
            Self::Dictionary(dictionary) => {
                crate::diagnostics::count!(property_lookup_hashes);
                dictionary.index.get(key).map(|&slot| slot as usize)
            }
        }
    }

    /// Reads and writes share the same bounded probe budget, including
    /// misses: repeated absent-key probes of a shared shape also amortize.
    #[inline]
    pub(crate) fn lookup_for_write(&self, key: &str) -> Option<usize> {
        self.lookup(key)
    }

    /// Slots in enumeration order, including unordered keys' slots for a
    /// shape only where they fall; callers filter with
    /// [`Attributes::is_ordered`].
    pub(crate) fn ordered_slots(&self) -> OrderedSlots<'_> {
        match self {
            Self::Shape(shape) => OrderedSlots::Shape(0..shape.fields.len()),
            Self::Dictionary(dictionary) => OrderedSlots::Dictionary(dictionary.order.iter()),
        }
    }

    #[cfg(test)]
    pub(crate) fn as_shape(&self) -> Option<&Shape> {
        match self {
            Self::Shape(shape) => Some(shape),
            Self::Dictionary(_) => None,
        }
    }

    pub(crate) fn as_dictionary(&self) -> Option<&Dictionary> {
        match self {
            Self::Dictionary(dictionary) => Some(dictionary),
            Self::Shape(_) => None,
        }
    }
}

/// Slot iterator in enumeration order.
pub(crate) enum OrderedSlots<'a> {
    Shape(std::ops::Range<usize>),
    Dictionary(std::slice::Iter<'a, u32>),
}

impl Iterator for OrderedSlots<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        match self {
            Self::Shape(range) => range.next(),
            Self::Dictionary(order) => order
                .by_ref()
                .copied()
                .find(|&slot| slot != NO_SLOT)
                .map(|slot| slot as usize),
        }
    }
}

/// The structural edits of one store's layout. Each takes the store's
/// layout pointer and edit counter; the caller mirrors the slot change in
/// its payload under the same borrow.
pub(crate) struct LayoutEdit;

impl LayoutEdit {
    /// The store's dictionary, made unique first if a reader shares it.
    fn unique_dictionary(layout: &mut Layout) -> Option<&mut Dictionary> {
        let LayoutNode::Dictionary(dictionary) = &**layout else {
            return None;
        };
        if Rc::strong_count(layout) != 1 || Rc::weak_count(layout) != 0 {
            let copy = dictionary.copy();
            *layout = Rc::new(LayoutNode::Dictionary(copy));
        }
        match Rc::get_mut(layout) {
            Some(LayoutNode::Dictionary(dictionary)) => Some(dictionary),
            _ => unreachable!("a dictionary layout was just made unique"),
        }
    }

    /// Appends `key` as a new last slot. `key` must be absent. Returns the
    /// slot. A `Small` shape at its limit, or a literal encoding, first
    /// moves to `Dynamic`; the caller converts its payload when the encoding
    /// it reads before and after differ.
    #[inline]
    pub(crate) fn append(layout: &mut Layout, key: &str, attributes: Attributes) -> usize {
        let slot = layout.len();
        if let LayoutNode::Shape(shape) = &**layout
            && match shape.encoding {
                Encoding::Small => slot < LINEAR_LOOKUP_LIMIT,
                Encoding::Dynamic => slot < SHAPE_FIELD_LIMIT,
                Encoding::Shaped | Encoding::Pair => false,
            }
        {
            *layout = match shape.append_edge(key, attributes) {
                Some(child) => child,
                None => Shape::append_child(layout, key, attributes),
            };
            return slot;
        }
        Self::append_slow(layout, key, attributes)
    }

    #[inline(never)]
    fn append_slow(layout: &mut Layout, key: &str, attributes: Attributes) -> usize {
        let slot = layout.len();
        if let Some(dictionary) = Self::unique_dictionary(layout) {
            let atom = dictionary.domain.intern_key(key);
            dictionary.index.insert(Rc::clone(atom.text()), slot as u32);
            dictionary.positions.push(dictionary.order.len() as u32);
            dictionary.order.push(slot as u32);
            dictionary.fields.push(Field::new(atom, attributes));
            dictionary.bump();
            return slot;
        }
        let encoding = layout.encoding();
        let grows_past_small = encoding == Encoding::Small && slot >= LINEAR_LOOKUP_LIMIT;
        if grows_past_small || matches!(encoding, Encoding::Shaped | Encoding::Pair) {
            *layout = Shape::recode_child(layout, Encoding::Dynamic);
        }
        if slot >= SHAPE_FIELD_LIMIT {
            Self::to_dictionary(layout);
            return Self::append_slow(layout, key, attributes);
        }
        *layout = Shape::append_child(layout, key, attributes);
        slot
    }

    /// Appends a proven-absent shared key, retaining its canonical text on a miss.
    #[inline]
    pub(crate) fn append_shared(
        layout: &mut Layout,
        key: &Rc<str>,
        attributes: Attributes,
    ) -> usize {
        let slot = layout.len();
        if let LayoutNode::Shape(shape) = &**layout
            && match shape.encoding {
                Encoding::Small => slot < LINEAR_LOOKUP_LIMIT,
                Encoding::Dynamic => slot < SHAPE_FIELD_LIMIT,
                Encoding::Shaped | Encoding::Pair => false,
            }
        {
            *layout = match shape.append_edge(key, attributes) {
                Some(child) => child,
                None => Shape::append_shared_child(layout, key, attributes),
            };
            return slot;
        }
        Self::append_shared_slow(layout, key, attributes)
    }

    #[inline(never)]
    fn append_shared_slow(layout: &mut Layout, key: &Rc<str>, attributes: Attributes) -> usize {
        let slot = layout.len();
        if let Some(dictionary) = Self::unique_dictionary(layout) {
            let atom = dictionary.domain.intern_shared_key(key);
            dictionary.index.insert(Rc::clone(atom.text()), slot as u32);
            dictionary.positions.push(dictionary.order.len() as u32);
            dictionary.order.push(slot as u32);
            dictionary.fields.push(Field::new(atom, attributes));
            dictionary.bump();
            return slot;
        }
        let encoding = layout.encoding();
        let grows_past_small = encoding == Encoding::Small && slot >= LINEAR_LOOKUP_LIMIT;
        if grows_past_small || matches!(encoding, Encoding::Shaped | Encoding::Pair) {
            *layout = Shape::recode_child(layout, Encoding::Dynamic);
        }
        if slot >= SHAPE_FIELD_LIMIT {
            Self::to_dictionary(layout);
            return Self::append_shared_slow(layout, key, attributes);
        }
        *layout = Shape::append_shared_child(layout, key, attributes);
        slot
    }

    /// Follows an existing append edge for `key` from a `Small` or `Dynamic`
    /// shape. An edge's key is absent from its parent, so a hit proves `key`
    /// absent without probing the fields. Returns the new slot.
    #[inline(always)]
    pub(crate) fn append_by_edge(
        layout: &mut Layout,
        key: &str,
        attributes: Attributes,
    ) -> Option<usize> {
        let LayoutNode::Shape(shape) = &**layout else {
            return None;
        };
        if !matches!(shape.encoding, Encoding::Small | Encoding::Dynamic) {
            return None;
        }
        let slot = shape.fields.len();
        let child = shape.append_edge(key, attributes)?;
        *layout = child;
        Some(slot)
    }

    /// Re-encodes the same fields, for a payload changing representation.
    pub(crate) fn recode(layout: &mut Layout, encoding: Encoding) {
        match Self::unique_dictionary(layout) {
            Some(dictionary) => {
                dictionary.encoding = encoding;
                dictionary.bump();
            }
            None => *layout = Shape::recode_child(layout, encoding),
        }
    }

    /// Replaces the attributes of `slot`. Equal attributes change nothing.
    pub(crate) fn reconfigure(
        layout: &mut Layout,
        edits: &mut u8,
        slot: usize,
        attributes: Attributes,
    ) {
        if layout.field(slot).map(|field| field.attributes) == Some(attributes) {
            return;
        }
        if let Some(dictionary) = Self::dictionary_for_edit(layout, edits) {
            dictionary.fields[slot].attributes = attributes;
            dictionary.bump();
            return;
        }
        let encoding = Self::edited_encoding(layout.encoding());
        *layout = Shape::edit_child(
            layout,
            EditLabel::Reconfigure(slot as u32, attributes),
            encoding,
            |fields| {
                let mut fields = fields.to_vec();
                fields[slot].attributes = attributes;
                fields
            },
        );
    }

    /// Applies `edit` to every field's attributes in one transition.
    pub(crate) fn bulk(layout: &mut Layout, edits: &mut u8, edit: BulkEdit) {
        if layout
            .fields()
            .iter()
            .all(|field| edit.apply(field.attributes) == field.attributes)
        {
            return;
        }
        if let Some(dictionary) = Self::dictionary_for_edit(layout, edits) {
            for field in &mut dictionary.fields {
                field.attributes = edit.apply(field.attributes);
            }
            dictionary.bump();
            return;
        }
        let encoding = Self::edited_encoding(layout.encoding());
        *layout = Shape::edit_child(layout, EditLabel::Bulk(edit), encoding, |fields| {
            fields
                .iter()
                .map(|field| Field {
                    attributes: edit.apply(field.attributes),
                    ..field.clone()
                })
                .collect()
        });
    }

    /// Removes `slot`.
    pub(crate) fn remove(layout: &mut Layout, edits: &mut u8, slot: usize) -> SlotRemoval {
        if let Some(dictionary) = Self::dictionary_for_edit(layout, edits) {
            let removed = dictionary.fields.swap_remove(slot);
            dictionary.index.remove(removed.text().as_ref());
            let position = dictionary.positions.swap_remove(slot) as usize;
            dictionary.order[position] = NO_SLOT;
            if slot < dictionary.fields.len() {
                // The former last slot now lives at `slot`.
                let moved = &dictionary.fields[slot];
                dictionary
                    .index
                    .insert(Rc::clone(moved.text()), slot as u32);
                let moved_position = dictionary.positions[slot] as usize;
                dictionary.order[moved_position] = slot as u32;
            }
            dictionary.compact_order();
            dictionary.bump();
            return SlotRemoval::SwapLast;
        }
        // A removal keeps `Small` and `Dynamic` stores in their encoding and
        // moves a literal to `Dynamic`.
        let encoding = match layout.encoding() {
            Encoding::Small => Encoding::Small,
            Encoding::Dynamic | Encoding::Shaped | Encoding::Pair => Encoding::Dynamic,
        };
        *layout = Shape::edit_child(layout, EditLabel::Remove(slot as u32), encoding, |fields| {
            let mut fields = fields.to_vec();
            fields.remove(slot);
            fields
        });
        SlotRemoval::Shift
    }

    /// A `Pair` becomes `Dynamic` when its attributes change; the other
    /// encodings keep descriptor slots and stay.
    fn edited_encoding(encoding: Encoding) -> Encoding {
        match encoding {
            Encoding::Pair => Encoding::Dynamic,
            other => other,
        }
    }

    /// Counts one non-append edit and returns the dictionary to edit in place
    /// when the store has, or now switches to, one.
    fn dictionary_for_edit<'a>(
        layout: &'a mut Layout,
        edits: &mut u8,
    ) -> Option<&'a mut Dictionary> {
        if !layout.is_dictionary() {
            *edits = edits.saturating_add(1);
            if *edits < DICTIONARY_AFTER_EDITS {
                return None;
            }
            Self::to_dictionary(layout);
        }
        Self::unique_dictionary(layout)
    }

    /// Moves a shape-backed store to a private dictionary with the same
    /// fields and slots.
    pub(crate) fn to_dictionary(layout: &mut Layout) {
        if layout.is_dictionary() {
            return;
        }
        crate::diagnostics::count!(property_dictionaries_created);
        let fields = layout.fields().to_vec();
        let index = fields
            .iter()
            .enumerate()
            .map(|(slot, field)| (Rc::clone(field.text()), slot as u32))
            .collect();
        let slots = (0..fields.len() as u32).collect::<Vec<_>>();
        let encoding = match layout.encoding() {
            Encoding::Pair | Encoding::Shaped | Encoding::Small => Encoding::Dynamic,
            Encoding::Dynamic => Encoding::Dynamic,
        };
        *layout = Rc::new(LayoutNode::Dictionary(Dictionary {
            domain: layout.domain().clone(),
            encoding,
            fields,
            positions: slots.clone(),
            order: slots,
            index,
            version: 0,
            identity: Rc::new(()),
        }));
    }
}

/// Weak content cache of edit-created shapes, per domain. Entries never keep
/// a shape alive; dead ones are swept when the table doubles.
#[derive(Default)]
pub(crate) struct ContentCache {
    entries: NameMap<u64, Vec<Weak<LayoutNode>>>,
    sweep_at: usize,
}

impl ContentCache {
    pub(crate) fn find(&self, hash: u64, matches: impl Fn(&Layout) -> bool) -> Option<Layout> {
        self.entries
            .get(&hash)?
            .iter()
            .filter_map(Weak::upgrade)
            .find(|candidate| matches(candidate))
    }

    pub(crate) fn insert(&mut self, hash: u64, shape: &Layout) {
        if self.entries.len() >= self.sweep_at.max(64) {
            self.entries.retain(|_, bucket| {
                bucket.retain(|shape| shape.strong_count() > 0);
                !bucket.is_empty()
            });
            self.sweep_at = self.entries.len() * 2;
        }
        let bucket = self.entries.entry(hash).or_default();
        // Repeated publications can have one hash while every prior shape
        // dies. Table-size sweeps alone never visit that single-key bucket.
        bucket.retain(|shape| shape.strong_count() > 0);
        bucket.push(Rc::downgrade(shape));
    }
}

/// Publishes a complete distinct-key descriptor batch, without constructing
/// unobservable prefix shapes. The weak content cache retains no extra shape;
/// subsequent incremental edits still use this node's bounded owned edges.
pub(crate) fn layout_for_descriptor_batch<'a>(
    domain: &AtomDomain,
    encoding: Encoding,
    keys: impl IntoIterator<Item = (&'a str, Attributes)>,
) -> Layout {
    let fields: Vec<_> = keys
        .into_iter()
        .map(|(key, attributes)| Field::new(domain.intern_key(key), attributes))
        .collect();
    if fields.is_empty() {
        return domain.root(encoding);
    }
    let hash = content_hash(encoding, &fields);
    if let Some(cached) = domain.cached_shape(hash, |candidate| {
        matches!(&**candidate, LayoutNode::Shape(shape) if shape.has_content(encoding, &fields))
    }) { return cached; }
    let shape = Rc::new(LayoutNode::Shape(Shape::new(
        domain.clone(),
        encoding,
        fields.into_boxed_slice(),
        hash,
        false,
    )));
    domain.cache_shape(hash, &shape);
    // Large complete batches obey the same dictionary boundary as appends.
    if shape.len() > SHAPE_FIELD_LIMIT {
        let mut layout = shape;
        LayoutEdit::to_dictionary(&mut layout);
        return layout;
    }
    shape
}

/// Builds the layout of a store whose keys arrive in order, appending each
/// through edges from the domain root of `encoding`, which the payload
/// already uses. Used for host maps, builders and literal sites.
pub(crate) fn layout_for_keys<'a>(
    domain: &AtomDomain,
    encoding: Encoding,
    keys: impl IntoIterator<Item = (&'a str, Attributes)>,
) -> Layout {
    let mut layout = domain.root(encoding);
    for (key, attributes) in keys {
        if layout.is_dictionary() || layout.len() >= SHAPE_FIELD_LIMIT {
            LayoutEdit::to_dictionary(&mut layout);
            LayoutEdit::append(&mut layout, key, attributes);
        } else {
            layout = Shape::append_child(&layout, key, attributes);
        }
    }
    layout
}

#[cfg(test)]
mod tests;
