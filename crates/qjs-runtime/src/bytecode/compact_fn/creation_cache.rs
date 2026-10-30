//! A named write site's proof that creating its property is a plain append.
//!
//! `this.left = left` in a constructor meets a receiver without the property,
//! so [[Set]] has to walk the prototype chain for a setter or a read-only
//! property of that name before it may create an own data property. Every
//! instance built by the constructor walks the same chain, so the site
//! remembers the chain it walked -- each prototype by identity and layout
//! revision -- and later receivers with that chain skip the walk. Adding,
//! removing or reconfiguring a property on any of those prototypes changes
//! its layout revision, and replacing a prototype changes the identity the
//! next receiver is checked against, so a stale proof only misses.

use std::cell::RefCell;

use crate::value::ObjectWeakRef;
use crate::{ObjectRef, Prototype};

/// Prototype chains longer than this are not remembered.
const MAX_CHAIN: usize = 3;

#[derive(Debug, Default)]
pub(in crate::bytecode) struct CreationCache(RefCell<Option<CreationProof>>);

/// A copied program starts without a proof.
impl Clone for CreationCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[derive(Debug)]
struct CreationProof {
    /// The chain from the receiver's prototype to the end, none of whose
    /// objects had an own property of the site's name.
    chain: Vec<(ObjectWeakRef, crate::value::layout::LayoutVersion)>,
}

impl CreationCache {
    pub(in crate::bytecode) fn permits(&self, object: &ObjectRef) -> bool {
        receiver_is_plain(object)
            && self
                .0
                .borrow()
                .as_ref()
                .is_some_and(|proof| chain_matches(object, &proof.chain))
    }

    /// Remembers `object`'s chain after the general path created `key` on
    /// it, when no object on the chain has an own property of that name.
    pub(in crate::bytecode) fn record(&self, object: &ObjectRef, key: &str) {
        let mut chain = Vec::new();
        let mut current = object.prototype_slot();
        loop {
            match current {
                None => break,
                Some(Prototype::Object(prototype)) => {
                    if chain.len() == MAX_CHAIN
                        || !ordinary(&prototype)
                        || prototype.own_property(key).is_some()
                    {
                        return;
                    }
                    current = prototype.prototype_slot();
                    chain.push((prototype.downgrade(), prototype.layout_revision()));
                }
                Some(_) => return,
            }
        }
        *self.0.borrow_mut() = Some(CreationProof { chain });
    }
}

fn ordinary(object: &ObjectRef) -> bool {
    !crate::symbol::is_symbol_primitive(object)
        && !crate::typed_array::is_typed_array_object(object)
        && !object.is_module_namespace_exotic()
}

fn receiver_is_plain(object: &ObjectRef) -> bool {
    ordinary(object) && object.is_extensible()
}

/// Whether `object`'s prototype chain is exactly `chain`, with every layout
/// unchanged since it was remembered.
fn chain_matches(
    object: &ObjectRef,
    chain: &[(ObjectWeakRef, crate::value::layout::LayoutVersion)],
) -> bool {
    let Some(((first, _), _)) = chain.split_first() else {
        return object.prototype_slot().is_none();
    };
    if !object.prototype_is_weak(first) {
        return false;
    }
    for (index, (link, revision)) in chain.iter().enumerate() {
        let Some(prototype) = link.upgrade() else {
            return false;
        };
        if prototype.layout_revision() != *revision {
            return false;
        }
        let continues = match chain.get(index + 1) {
            Some((next, _)) => prototype.prototype_is_weak(next),
            None => prototype.prototype_slot().is_none(),
        };
        if !continues {
            return false;
        }
    }
    true
}
