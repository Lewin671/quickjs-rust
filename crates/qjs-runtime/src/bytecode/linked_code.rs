//! Per-domain linking of a body's named property operands to atoms.
//!
//! A named property opcode (`GetPropNamed`, `SetPropNamed`) carries a
//! [`PropertyNameId`] into its body's [`PropertyNamePool`]: the symbolic
//! names, deduplicated by exact text and assigned once when the `Bytecode` is
//! built. The pool holds names, never atom ids, so the same compiled code
//! (a cached eval blueprint, a `Function` body, a module) can run under any
//! atom domain.
//!
//! The first named operation a body runs under a domain links it: every pool
//! name is interned once and the resulting atoms are cached on the body.
//! Every later dispatch under that domain indexes the cached link; it never
//! interns, hashes or compares a name.
//!
//! The invariant that keeps foreign ids out: an atom is only read from a link
//! whose recorded domain is the domain the caller asked for. The primary link
//! belongs to the first domain and never changes, so a dispatch borrows its
//! atom without a reference-count update. A body run under another domain,
//! or after its primary domain died, uses a short bounded list of foreign
//! links that re-intern by content; those dispatches clone the atom out so no
//! borrow of the list survives the operation. A link records its domain
//! weakly, which pins the domain's allocation, so a later domain at a reused
//! address can never match a dead domain's link.

use std::cell::{OnceCell, RefCell};
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

use crate::RuntimeError;
use crate::property::atom::{Atom, AtomDomain, WeakAtomDomain};
use crate::value::name_hash::NameMap;

use super::ir::{Bytecode, Op};

/// Foreign-domain links kept per body beyond the primary one.
const MAX_FOREIGN_LINKS: usize = 4;

/// The index of a named property operand in its body's name pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PropertyNameId(u32);

impl PropertyNameId {
    /// The value the compiler emits; `Bytecode` construction assigns the
    /// real index before the code can run.
    pub(super) const UNASSIGNED: Self = Self(u32::MAX);

    fn index(self) -> usize {
        self.0 as usize
    }
}

/// The distinct symbolic names of one body's named property operands.
#[derive(Clone, Debug, Default)]
pub(super) struct PropertyNamePool(Rc<[Rc<str>]>);

impl PropertyNamePool {
    /// Assigns every named property operand in `code` its pool index and
    /// returns the pool. Equal texts share one entry.
    pub(super) fn assign(code: &mut [Op]) -> Self {
        let mut ids: NameMap<Rc<str>, PropertyNameId> = NameMap::default();
        let mut names: Vec<Rc<str>> = Vec::new();
        for op in code {
            let (Op::GetPropNamed { key, name, .. } | Op::SetPropNamed { key, name, .. }) = op
            else {
                continue;
            };
            *name = match ids.get(key.as_ref()) {
                Some(id) => *id,
                None => {
                    let id = PropertyNameId(
                        u32::try_from(names.len()).expect("a name pool is smaller than its code"),
                    );
                    names.push(Rc::clone(key));
                    ids.insert(Rc::clone(key), id);
                    id
                }
            };
        }
        Self(names.into())
    }

    pub(super) fn names(&self) -> &[Rc<str>] {
        &self.0
    }

    fn link(&self, domain: &AtomDomain) -> Result<LinkedCode, RuntimeError> {
        crate::diagnostics::count!(named_operand_links);
        let atoms = self
            .0
            .iter()
            .map(|name| domain.intern(name))
            .collect::<Box<[Atom]>>();
        Ok(LinkedCode {
            domain: domain.downgrade(),
            atoms,
        })
    }
}

/// One body's pool linked to one domain: `atoms[i]` is pool name `i`.
struct LinkedCode {
    domain: WeakAtomDomain,
    atoms: Box<[Atom]>,
}

/// The cached links of one body.
#[derive(Default)]
pub(super) struct AtomLinks {
    primary: OnceCell<LinkedCode>,
    foreign: RefCell<Vec<LinkedCode>>,
}

impl Clone for AtomLinks {
    /// Links are a cache of the immutable pool; a clone relinks on demand.
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl fmt::Debug for AtomLinks {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AtomLinks")
            .field("primary", &self.primary.get().is_some())
            .field("foreign", &self.foreign.borrow().len())
            .finish()
    }
}

/// A named operand's atom: borrowed from the primary link, or owned when it
/// came from a foreign link.
pub(super) enum LinkedName<'a> {
    Primary(&'a Atom),
    Foreign(Atom),
}

impl Deref for LinkedName<'_> {
    type Target = Atom;

    #[inline]
    fn deref(&self) -> &Atom {
        match self {
            Self::Primary(atom) => atom,
            Self::Foreign(atom) => atom,
        }
    }
}

impl Bytecode {
    /// The atom for named operand `name` in `domain`, linking this body to
    /// `domain` on first use.
    #[inline]
    pub(super) fn linked_property_name(
        &self,
        domain: &AtomDomain,
        name: PropertyNameId,
    ) -> Result<LinkedName<'_>, RuntimeError> {
        if let Some(link) = self.atom_links.primary.get()
            && link.domain.is(domain)
            && let Some(atom) = link.atoms.get(name.index())
        {
            debug_assert!(atom.belongs_to(domain));
            return Ok(LinkedName::Primary(atom));
        }
        self.link_property_name(domain, name)
    }

    #[cold]
    #[inline(never)]
    fn link_property_name(
        &self,
        domain: &AtomDomain,
        name: PropertyNameId,
    ) -> Result<LinkedName<'_>, RuntimeError> {
        let index = name.index();
        if index >= self.property_names.names().len() {
            return Err(RuntimeError {
                thrown: None,
                message: "named property operand is not in its body's name pool".to_owned(),
            });
        }
        if self.atom_links.primary.get().is_none() {
            // Interning runs no JavaScript, so nothing can fill the cell
            // between this check and the set.
            let link = self.property_names.link(domain)?;
            let link = self.atom_links.primary.get_or_init(|| link);
            if link.domain.is(domain) {
                return Ok(LinkedName::Primary(&link.atoms[index]));
            }
        }
        let mut foreign = self.atom_links.foreign.borrow_mut();
        if let Some(link) = foreign.iter().find(|link| link.domain.is(domain)) {
            return Ok(LinkedName::Foreign(link.atoms[index].clone()));
        }
        foreign.retain(|link| link.domain.is_live());
        if foreign.len() >= MAX_FOREIGN_LINKS {
            foreign.remove(0);
        }
        let link = self.property_names.link(domain)?;
        let atom = link.atoms[index].clone();
        foreign.push(link);
        Ok(LinkedName::Foreign(atom))
    }

    /// The symbolic names of this body's named property operands.
    #[cfg(test)]
    pub(super) fn property_name_pool(&self) -> &[Rc<str>] {
        self.property_names.names()
    }

    /// Domains this body currently holds links for (primary and foreign).
    #[cfg(test)]
    pub(super) fn linked_domain_count(&self) -> usize {
        usize::from(self.atom_links.primary.get().is_some())
            + self.atom_links.foreign.borrow().len()
    }
}

#[cfg(test)]
mod tests;
