//! Per-site cache for ordinary `for-in` key lists.
//!
//! `for-in` observes mutations made after its initial key-list collection by
//! re-checking every key before the loop body. A later execution of the same
//! bytecode site may reuse that hidden list only when the complete ordinary
//! object chain has retained its identities and enumerable-key layouts.

use std::{cell::RefCell, fmt, rc::Rc};

use crate::{ArrayRef, ObjectRef, Prototype, Value, value::ObjectWeakRef};

/// Bound chain validation work and decline rather than retaining an
/// unexpectedly deep prototype path in every compiled `for-in` site.
const MAX_ORDINARY_CHAIN_DEPTH: usize = 16;

#[derive(Clone, Default)]
pub(super) struct EnumerateKeysCache(Rc<RefCell<Option<EnumerateKeysCacheEntry>>>);

impl fmt::Debug for EnumerateKeysCache {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EnumerateKeysCache(..)")
    }
}

struct EnumerateKeysCacheEntry {
    chain: Vec<OrdinaryChainLink>,
    keys: ArrayRef,
}

struct OrdinaryChainLink {
    object: ObjectWeakRef,
    layout_revision: u64,
}

impl EnumerateKeysCache {
    /// Returns the hidden key array when every object the initial enumeration
    /// depended on still has the same identity, own-key layout, and ordinary
    /// object prototype link.
    pub(super) fn get(&self, target: &Value) -> Option<ArrayRef> {
        let entry = self.0.borrow();
        let entry = entry.as_ref()?;
        ordinary_chain_matches(target, &entry.chain).then(|| entry.keys.clone())
    }

    /// Records a just-enumerated key array when `target` has an entirely
    /// ordinary object chain. Unsupported values intentionally clear a stale
    /// entry: a future ordinary target must rebuild before it can be cached.
    pub(super) fn record(&self, target: &Value, keys: ArrayRef) {
        let chain = capture_ordinary_chain(target);
        *self.0.borrow_mut() = chain.map(|chain| EnumerateKeysCacheEntry { chain, keys });
    }
}

fn capture_ordinary_chain(target: &Value) -> Option<Vec<OrdinaryChainLink>> {
    let Value::Object(object) = target else {
        return None;
    };
    let mut object = object.clone();
    let mut chain = Vec::new();
    loop {
        if !is_cacheable_ordinary_object(&object) || chain.len() == MAX_ORDINARY_CHAIN_DEPTH {
            return None;
        }
        chain.push(OrdinaryChainLink {
            object: object.downgrade(),
            layout_revision: object.layout_revision(),
        });
        match object.prototype_slot() {
            None => return Some(chain),
            Some(Prototype::Object(prototype)) => object = prototype,
            Some(Prototype::Array(_))
            | Some(Prototype::Function(_))
            | Some(Prototype::Proxy(_)) => {
                return None;
            }
        }
    }
}

fn ordinary_chain_matches(target: &Value, expected: &[OrdinaryChainLink]) -> bool {
    let Value::Object(object) = target else {
        return false;
    };
    let mut object = object.clone();
    for (index, link) in expected.iter().enumerate() {
        if !is_cacheable_ordinary_object(&object)
            || !link.object.ptr_eq(&object)
            || link.layout_revision != object.layout_revision()
        {
            return false;
        }
        match object.prototype_slot() {
            None => return index + 1 == expected.len(),
            Some(Prototype::Object(prototype)) => object = prototype,
            Some(Prototype::Array(_))
            | Some(Prototype::Function(_))
            | Some(Prototype::Proxy(_)) => {
                return false;
            }
        }
    }
    false
}

fn is_cacheable_ordinary_object(object: &ObjectRef) -> bool {
    !crate::typed_array::is_typed_array_object(object)
        && !object.is_module_namespace_exotic()
        && !object.is_array_prototype_exotic()
        && !crate::symbol::is_symbol_primitive(object)
}
