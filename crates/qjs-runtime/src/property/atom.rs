//! Runtime property-key identity and weak interning.
//!
//! Strings use exact canonical UTF-16 equality; symbols use object identity in
//! one ID namespace. IDs are recycled only after their last lease is gone. Layouts and linked code own
//! leases. Content indices are weak and swept together; the ID directory is
//! test-only. Neither pins historical keys. A lease records its domain weakly;
//! layouts separately retain the owning domain.

use std::borrow::Cow;
use std::cell::RefCell;
#[cfg(test)]
use std::collections::HashMap;
use std::fmt;
use std::num::NonZeroU32;
use std::rc::{Rc, Weak};

use crate::ObjectRef;
use crate::string::{string_code_units, string_from_code_units_canonical};
use crate::value::array_index_property_key;
use crate::value::name_hash::NameMap;

/// Smallest table size that triggers a sweep of dead entries.
const MIN_SWEEP_LEN: usize = 64;

/// Marks an atom whose key is not a canonical array index.
const NO_ARRAY_INDEX: u32 = u32::MAX;

/// An ID borrowed from a live lease. Recycling cannot outlive that lease.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct KeyId(NonZeroU32);

/// String operands use the same namespace as all other property keys.
pub(crate) type AtomId = KeyId;

impl KeyId {
    pub(crate) fn get(self) -> u32 {
        self.0.get()
    }
}

/// The domain ran out of ids. Reported instead of wrapping, because a wrapped
/// id would alias a live atom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AtomDomainExhausted;

pub(crate) struct KeyRecord {
    id: AtomId,
    /// String units or a strong symbol identity, owned only by live leases.
    kind: KeyKind,
    /// The canonical array index this key denotes, or [`NO_ARRAY_INDEX`].
    array_index: u32,
    /// The owning domain. Weak, so atoms never keep a dead domain's table
    /// alive; compared only by allocation identity.
    domain: Weak<DomainState>,
}

impl Drop for KeyRecord {
    fn drop(&mut self) {
        if let Some(domain) = self.domain.upgrade() {
            domain.free_ids.borrow_mut().push(self.id);
        }
    }
}

enum KeyKind {
    String(Rc<str>),

    Symbol(ObjectRef),
}

/// An owned reference to one interned property key.
///
/// Cloning bumps a reference count. Equality is entry identity, which within
/// one domain is equality of code units.
#[derive(Clone)]
pub(crate) struct KeyLease(Rc<KeyRecord>);

/// Named bytecode operands are string key leases.
pub(crate) type Atom = KeyLease;

impl KeyLease {
    pub(crate) fn id(&self) -> AtomId {
        self.0.id
    }

    /// The canonical stored text of this key.
    pub(crate) fn text(&self) -> &Rc<str> {
        match &self.0.kind {
            KeyKind::String(text) => text,
            KeyKind::Symbol(_) => panic!("symbol lease used as a named string operand"),
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        self.text()
    }

    /// The array index this key denotes when it is the canonical decimal form
    /// of an integer below 2^32 - 1. `"01"` and `"4294967295"` are not.
    pub(crate) fn array_index(&self) -> Option<u32> {
        (self.0.array_index != NO_ARRAY_INDEX).then_some(self.0.array_index)
    }

    pub(crate) fn string_text(&self) -> Option<&str> {
        match &self.0.kind {
            KeyKind::String(text) => Some(text),
            KeyKind::Symbol(_) => None,
        }
    }

    pub(crate) fn symbol(&self) -> Option<&ObjectRef> {
        match &self.0.kind {
            KeyKind::Symbol(symbol) => Some(symbol),
            KeyKind::String(_) => None,
        }
    }

    /// Whether `domain` interned this atom.
    pub(crate) fn belongs_to(&self, domain: &AtomDomain) -> bool {
        std::ptr::eq(self.0.domain.as_ptr(), Rc::as_ptr(&domain.0))
    }
}

impl PartialEq for KeyLease {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for KeyLease {}

impl fmt::Debug for KeyLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Atom")
            .field(&self.0.id.get())
            .field(&self.string_text())
            .finish()
    }
}

struct AtomTable {
    entries: NameMap<Rc<str>, Weak<KeyRecord>>,
    symbols: NameMap<usize, Weak<KeyRecord>>,
    #[cfg(test)]
    directory: HashMap<KeyId, Weak<KeyRecord>>,
    /// The next id to hand out; `None` once `u32::MAX` has been issued.
    next_id: Option<NonZeroU32>,
    sweep_at: usize,
    created: u64,
    limit: u32,
}

impl AtomTable {
    fn allocate_id(&mut self) -> Result<AtomId, AtomDomainExhausted> {
        let id = self
            .next_id
            .filter(|id| id.get() <= self.limit)
            .ok_or(AtomDomainExhausted)?;
        self.next_id = id.checked_add(1);
        Ok(KeyId(id))
    }

    fn sweep_if_due(&mut self) {
        if self.entries.len() + self.symbols.len() < self.sweep_at {
            return;
        }
        self.entries.retain(|_, entry| entry.strong_count() > 0);
        self.symbols.retain(|_, entry| entry.strong_count() > 0);
        #[cfg(test)]
        self.directory.retain(|_, entry| entry.strong_count() > 0);
        self.sweep_at = ((self.entries.len() + self.symbols.len()) * 2).max(MIN_SWEEP_LEN);
    }
}

struct DomainState {
    table: RefCell<AtomTable>,
    free_ids: RefCell<Vec<KeyId>>,
    // Only an attachment, never a strong root: layout owners retain the domain.
    shape_owner: RefCell<Weak<crate::value::layout::OwnerState>>,
    empty_native_context: RefCell<Weak<crate::function::NativeContextData>>,
}

/// An owned atom interning domain. Cloning shares the domain.
#[derive(Clone)]
pub(crate) struct AtomDomain(Rc<DomainState>);

impl AtomDomain {
    pub(crate) fn new() -> Self {
        Self::with_limit(u32::MAX)
    }

    #[cfg(test)]
    pub(crate) fn limited_for_test(limit: u32) -> Self {
        Self::with_limit(limit)
    }

    fn with_limit(limit: u32) -> Self {
        Self(Rc::new(DomainState {
            free_ids: RefCell::new(Vec::new()),
            shape_owner: RefCell::new(Weak::new()),
            empty_native_context: RefCell::new(Weak::new()),
            table: RefCell::new(AtomTable {
                entries: NameMap::default(),
                symbols: NameMap::default(),
                #[cfg(test)]
                directory: HashMap::new(),
                next_id: Some(NonZeroU32::MIN),
                sweep_at: MIN_SWEEP_LEN,
                created: 0,
                limit,
            }),
        }))
    }

    /// Interns the key `text` (stored runtime form), returning the domain's
    /// live atom for its code units or a new one.
    pub(crate) fn intern(&self, text: &str) -> Atom {
        self.try_intern(text)
            .unwrap_or_else(|_| key_allocation_failure())
    }

    pub(crate) fn try_intern(&self, text: &str) -> Result<Atom, AtomDomainExhausted> {
        crate::diagnostics::count!(property_key_interns);
        crate::diagnostics::count!(property_text_probes);
        let canonical = canonical_property_text(text);
        let mut table = self.0.table.borrow_mut();
        if let Some(entry) = table
            .entries
            .get(canonical.as_ref())
            .and_then(Weak::upgrade)
        {
            return Ok(KeyLease(entry));
        }
        let id = match self.0.free_ids.borrow_mut().pop() {
            Some(id) => id,
            None => table.allocate_id()?,
        };
        let text: Rc<str> = Rc::from(canonical.as_ref());
        let array_index = array_index_property_key(&text)
            .and_then(|index| u32::try_from(index).ok())
            .unwrap_or(NO_ARRAY_INDEX);
        let entry = Rc::new(KeyRecord {
            id,
            kind: KeyKind::String(Rc::clone(&text)),
            array_index,
            domain: Rc::downgrade(&self.0),
        });
        // A dead entry for the same text keeps its old key allocation on a
        // plain insert; replace the pair so the key shares the new text.
        table.entries.remove(text.as_ref());
        table.sweep_if_due();
        table.entries.insert(text, Rc::downgrade(&entry));
        #[cfg(test)]
        table.directory.insert(id, Rc::downgrade(&entry));
        table.created += 1;
        crate::diagnostics::count!(property_atoms_created);
        Ok(KeyLease(entry))
    }

    /// Looks up a live lease without allocating a new ID. Read-only property
    /// probes must remain possible after namespace exhaustion.
    pub(crate) fn find(&self, text: &str) -> Option<KeyLease> {
        crate::diagnostics::count!(property_text_probes);
        let canonical = canonical_property_text(text);
        self.0
            .table
            .borrow()
            .entries
            .get(canonical.as_ref())
            .and_then(Weak::upgrade)
            .map(KeyLease)
    }

    pub(crate) fn find_symbol(&self, symbol: &ObjectRef) -> Option<KeyLease> {
        crate::diagnostics::count!(property_symbol_probes);
        self.0
            .table
            .borrow()
            .symbols
            .get(&symbol.address())
            .and_then(Weak::upgrade)
            .map(KeyLease)
    }

    /// Symbols are interned by identity, never by description. A live lease
    /// pins its symbol, so its address cannot be reused while indexed.
    pub(crate) fn intern_symbol(&self, symbol: &ObjectRef) -> KeyLease {
        self.try_intern_symbol(symbol)
            .unwrap_or_else(|_| key_allocation_failure())
    }

    fn try_intern_symbol(&self, symbol: &ObjectRef) -> Result<KeyLease, AtomDomainExhausted> {
        crate::diagnostics::count!(property_key_interns);
        crate::diagnostics::count!(property_symbol_probes);
        let mut table = self.0.table.borrow_mut();
        if let Some(entry) = table.symbols.get(&symbol.address()).and_then(Weak::upgrade) {
            return Ok(KeyLease(entry));
        }
        let id = match self.0.free_ids.borrow_mut().pop() {
            Some(id) => id,
            None => table.allocate_id()?,
        };
        let entry = Rc::new(KeyRecord {
            id,
            kind: KeyKind::Symbol(symbol.clone()),
            array_index: NO_ARRAY_INDEX,
            domain: Rc::downgrade(&self.0),
        });
        table.sweep_if_due();
        table
            .symbols
            .insert(symbol.address(), Rc::downgrade(&entry));
        #[cfg(test)]
        table.directory.insert(id, Rc::downgrade(&entry));
        table.created += 1;
        crate::diagnostics::count!(property_atoms_created);
        Ok(KeyLease(entry))
    }

    /// A borrowed numeric ID alone never keeps its record alive.
    #[cfg(test)]
    pub(crate) fn lease(&self, id: KeyId) -> Option<KeyLease> {
        self.0
            .table
            .borrow()
            .directory
            .get(&id)
            .and_then(Weak::upgrade)
            .map(KeyLease)
    }

    pub(crate) fn empty_native_context(
        &self,
    ) -> &RefCell<Weak<crate::function::NativeContextData>> {
        &self.0.empty_native_context
    }

    pub(crate) fn shape_owner(&self) -> &RefCell<Weak<crate::value::layout::OwnerState>> {
        &self.0.shape_owner
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
            .directory
            .values()
            .filter(|entry| entry.strong_count() > 0)
            .count()
    }

    /// Table entries, live or awaiting a sweep.
    #[cfg(test)]
    pub(crate) fn table_len(&self) -> usize {
        self.0.table.borrow().directory.len()
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

/// Same non-unwinding resource failure path as Rust allocation failure.
fn key_allocation_failure() -> ! {
    std::alloc::handle_alloc_error(std::alloc::Layout::new::<KeyRecord>())
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
