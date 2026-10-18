//! Property-name atoms: an owned interning domain for string property keys.
//!
//! An [`AtomDomain`] maps the exact UTF-16 code units of a string property key
//! to one shared [`Atom`]. Realms that exchange objects share a domain; an
//! independent runtime root owns its own. Symbols and private names are
//! object identities and never enter a domain, so a string can never alias
//! one of them.
//!
//! Invariants that keep atom identity sound:
//!
//! - **Exact keys.** The runtime stores strings as UTF-8 with lone surrogates
//!   escaped to sentinel scalars, and a surrogate pair may be stored either
//!   whole or as two escaped halves. A domain interns the canonical form of
//!   the code units ([`canonical_property_text`]), so two keys share an atom
//!   exactly when their code units are equal. Nothing else is folded: `"01"`
//!   and `"1"` are distinct atoms, and only the canonical decimal form of an
//!   index carries [`Atom::array_index`].
//! - **One live atom per key.** Within one domain, two live atoms for the same
//!   code units are the same entry, so `Atom` equality is pointer identity.
//!   Atoms from different domains are never equal; code that holds atoms
//!   records the domain it linked against (see `bytecode/linked_code.rs`), so
//!   it never interprets a foreign domain's ids.
//! - **Ids are never reused.** Ids come from a per-domain monotonic counter.
//!   An atom that every owner dropped is reclaimed; interning its text again
//!   allocates a fresh id when a numbered operand requests it. Exhaustion is an
//!   error ([`AtomDomainExhausted`]), never a wrap. Storage owns [`AtomKey`]
//!   identities from the same entries without requesting finite relocation ids;
//!   this keeps infallible host storage APIs free of hidden limit panics. An
//!   owned/weak entry pins its allocation, so its identity cannot be recycled
//!   while any layout or operand can observe it.
//! - **Bounded retention.** The domain holds only weak references. Dead table
//!   entries are swept whenever the table doubles past its live size, so key
//!   churn costs memory proportional to the live atoms only.

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;
use std::rc::{Rc, Weak};

use crate::RuntimeError;
use crate::string::{string_code_units, string_from_code_units_canonical};
use crate::value::array_index_property_key;
use crate::value::name_hash::NameMap;

/// Smallest table size that triggers a sweep of dead entries.
const MIN_SWEEP_LEN: usize = 64;

/// Marks an atom whose key is not a canonical array index.
const NO_ARRAY_INDEX: u32 = u32::MAX;

/// An atom's id within its domain. Ids start at 1 and are never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct AtomId(NonZeroU32);

impl AtomId {
    pub(crate) fn get(self) -> u32 {
        self.0.get()
    }
}

/// The domain ran out of ids. Reported instead of wrapping, because a wrapped
/// id would alias a live atom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AtomDomainExhausted;

impl AtomDomainExhausted {
    /// The engine-limit error reported to the code that needed a new atom.
    /// Not a `From` impl: that would make every untyped `?` ambiguous.
    pub(crate) fn into_runtime_error(self) -> RuntimeError {
        RuntimeError {
            thrown: None,
            message: "RangeError: too many distinct property names".to_owned(),
        }
    }
}

struct AtomEntry {
    id: Cell<Option<AtomId>>,
    /// Canonical text: the table key for this entry while it is live.
    text: Rc<str>,
    /// The canonical array index this key denotes, or [`NO_ARRAY_INDEX`].
    array_index: u32,
    /// The owning domain. Weak, so atoms never keep a dead domain's table
    /// alive; compared only by allocation identity.
    domain: Weak<DomainState>,
}

/// An owned reference to one interned property key.
///
/// Cloning bumps a reference count. Equality is entry identity, which within
/// one domain is equality of code units.
#[derive(Clone)]
pub(crate) struct Atom(Rc<AtomEntry>);

impl Atom {
    #[allow(dead_code)] // Numeric relocation identity; storage owns the entry itself.
    pub(crate) fn id(&self) -> AtomId {
        self.0.id.get().expect("numbered atom handle")
    }

    /// The canonical stored text of this key.
    pub(crate) fn text(&self) -> &Rc<str> {
        &self.0.text
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0.text
    }

    /// The array index this key denotes when it is the canonical decimal form
    /// of an integer below 2^32 - 1. `"01"` and `"4294967295"` are not.
    #[allow(dead_code)] // Classified by the layout authority (T056 M1.2); tested here.
    pub(crate) fn array_index(&self) -> Option<u32> {
        (self.0.array_index != NO_ARRAY_INDEX).then_some(self.0.array_index)
    }

    /// Whether `domain` interned this atom.
    pub(crate) fn belongs_to(&self, domain: &AtomDomain) -> bool {
        std::ptr::eq(self.0.domain.as_ptr(), Rc::as_ptr(&domain.0))
    }
}

/// An exact key identity held by ordinary layouts. Its allocation cannot be
/// recycled while a layout or weak observer owns it. Numbered operands and
/// storage keys share the same entry; only numbered operands consume `AtomId`s.
#[derive(Clone)]
pub(crate) struct AtomKey(Rc<AtomEntry>);

/// Borrowed allocation identity. Only index together with owned `AtomKey`
/// fields (or a live `Atom`); those owners prevent allocation reuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct AtomKeyIdentity(usize);

impl AtomKey {
    pub(crate) fn identity(&self) -> AtomKeyIdentity {
        AtomKeyIdentity(Rc::as_ptr(&self.0) as usize)
    }
    pub(crate) fn text(&self) -> &Rc<str> {
        &self.0.text
    }
}
impl PartialEq for AtomKey {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for AtomKey {}
impl Hash for AtomKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.0).hash(state);
    }
}
impl fmt::Debug for AtomKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.text.fmt(f)
    }
}

impl Atom {
    pub(crate) fn key_identity(&self) -> AtomKeyIdentity {
        AtomKeyIdentity(Rc::as_ptr(&self.0) as usize)
    }
    #[cfg(test)]
    pub(crate) fn key(&self) -> AtomKey {
        AtomKey(self.0.clone())
    }
}

impl PartialEq for Atom {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Atom {}

impl fmt::Debug for Atom {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Atom")
            .field(&self.id().get())
            .field(&crate::string::string_to_utf8_lossy(&self.0.text))
            .finish()
    }
}

struct AtomTable {
    entries: NameMap<Rc<str>, Weak<AtomEntry>>,
    /// The next id to hand out; `None` once `u32::MAX` has been issued.
    next_id: Option<NonZeroU32>,
    sweep_at: usize,
    created: u64,
}

impl AtomTable {
    fn allocate_id(&mut self) -> Result<AtomId, AtomDomainExhausted> {
        let id = self.next_id.ok_or(AtomDomainExhausted)?;
        self.next_id = id.checked_add(1);
        Ok(AtomId(id))
    }

    fn sweep_if_due(&mut self) {
        if self.entries.len() < self.sweep_at {
            return;
        }
        self.entries.retain(|_, entry| entry.strong_count() > 0);
        self.sweep_at = (self.entries.len() * 2).max(MIN_SWEEP_LEN);
    }
}

struct DomainState {
    table: RefCell<AtomTable>,
    shapes: RefCell<super::layout::ShapeCache>,
}

/// An owned atom interning domain. Cloning shares the domain.
#[derive(Clone)]
pub(crate) struct AtomDomain(Rc<DomainState>);

impl AtomDomain {
    pub(crate) fn new() -> Self {
        Self::starting_at(NonZeroU32::MIN)
    }

    fn starting_at(first_id: NonZeroU32) -> Self {
        Self(Rc::new(DomainState {
            table: RefCell::new(AtomTable {
                entries: NameMap::default(),
                next_id: Some(first_id),
                sweep_at: MIN_SWEEP_LEN,
                created: 0,
            }),
            shapes: RefCell::default(),
        }))
    }

    /// Interns the key `text` (stored runtime form), returning the domain's
    /// live atom for its code units or a new one.
    pub(crate) fn intern(&self, text: &str) -> Result<Atom, AtomDomainExhausted> {
        let key = self.intern_key(text);
        if key.0.id.get().is_none() {
            let id = self.0.table.borrow_mut().allocate_id()?;
            key.0.id.set(Some(id));
        }
        Ok(Atom(key.0))
    }

    /// Interns an owned key identity without allocating a bytecode relocation id.
    /// Storage APIs are infallible: they hash this pinned allocation identity,
    /// never a finite integer. A numbered `intern` of the same key still checks
    /// id exhaustion before producing an `Atom`.
    pub(crate) fn intern_key(&self, text: &str) -> AtomKey {
        let canonical = canonical_property_text(text);
        let mut table = self.0.table.borrow_mut();
        if let Some(entry) = table
            .entries
            .get(canonical.as_ref())
            .and_then(Weak::upgrade)
        {
            return AtomKey(entry);
        }
        let text: Rc<str> = Rc::from(canonical.as_ref());
        let array_index = array_index_property_key(&text)
            .and_then(|index| u32::try_from(index).ok())
            .unwrap_or(NO_ARRAY_INDEX);
        let entry = Rc::new(AtomEntry {
            id: Cell::new(None),
            text: text.clone(),
            array_index,
            domain: Rc::downgrade(&self.0),
        });
        table.entries.remove(text.as_ref());
        table.sweep_if_due();
        table.entries.insert(text, Rc::downgrade(&entry));
        table.created += 1;
        crate::diagnostics::count!(property_atoms_created);
        AtomKey(entry)
    }

    /// Looks up a live key without allocating either an identity or an id.
    #[cfg(test)]
    pub(crate) fn key(&self, text: &str) -> Option<AtomKey> {
        let canonical = canonical_property_text(text);
        self.0
            .table
            .borrow()
            .entries
            .get(canonical.as_ref())
            .and_then(Weak::upgrade)
            .map(AtomKey)
    }

    pub(crate) fn key_identity(&self, text: &str) -> Option<AtomKeyIdentity> {
        let canonical = canonical_property_text(text);
        let table = self.0.table.borrow();
        let entry = table.entries.get(canonical.as_ref())?;
        (entry.strong_count() > 0).then_some(AtomKeyIdentity(entry.as_ptr() as usize))
    }

    pub(crate) fn shapes(&self) -> &RefCell<super::layout::ShapeCache> {
        &self.0.shapes
    }

    /// A non-owning handle that identifies this domain while it lives.
    pub(crate) fn downgrade(&self) -> WeakAtomDomain {
        WeakAtomDomain(Rc::downgrade(&self.0))
    }

    #[cfg(test)]
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Atoms this domain has created, including reclaimed ones.
    pub(crate) fn created_count(&self) -> u64 {
        self.0.table.borrow().created
    }

    /// Atoms some owner still holds.
    #[cfg(test)]
    pub(crate) fn live_count(&self) -> usize {
        self.0
            .table
            .borrow()
            .entries
            .values()
            .filter(|entry| entry.strong_count() > 0)
            .count()
    }

    /// Table entries, live or awaiting a sweep.
    #[cfg(test)]
    pub(crate) fn table_len(&self) -> usize {
        self.0.table.borrow().entries.len()
    }
}

impl Default for AtomDomain {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for AtomDomain {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AtomDomain")
            .field("created", &self.created_count())
            .finish_non_exhaustive()
    }
}

/// Identifies a domain without keeping its table alive.
///
/// The weak reference pins the domain's allocation, so its address cannot be
/// reused by a later domain while this handle exists: comparing addresses is
/// an identity test, not a guess.
#[derive(Clone)]
pub(crate) struct WeakAtomDomain(Weak<DomainState>);

impl WeakAtomDomain {
    #[inline]
    pub(crate) fn is(&self, domain: &AtomDomain) -> bool {
        std::ptr::eq(self.0.as_ptr(), Rc::as_ptr(&domain.0))
    }

    pub(crate) fn is_live(&self) -> bool {
        self.0.strong_count() > 0
    }
}

/// The canonical stored form of a string key's code units.
///
/// Only text with a four-byte scalar can hold a surrogate sentinel, and only
/// a sentinel can make two encodings of one code-unit sequence differ, so
/// everything else is already canonical and is borrowed.
pub(crate) fn canonical_property_text(text: &str) -> Cow<'_, str> {
    if text.bytes().all(|byte| byte < 0xF0) {
        return Cow::Borrowed(text);
    }
    let canonical = string_from_code_units_canonical(&string_code_units(text));
    if canonical == text {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(canonical)
    }
}

#[cfg(test)]
mod tests;
