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
pub(super) struct EnumerateKeysCache(Rc<RefCell<EnumerateKeysCacheState>>);

#[derive(Default)]
struct EnumerateKeysCacheState {
    entry: Option<EnumerateKeysCacheEntry>,
    /// The prototype chain -- the links after a target, to the end -- of the
    /// last ordinary target, at these layout revisions, with the keys that
    /// chain contributes to a `for-in` (its own enumeration, shadowing among
    /// its layers already applied). A target inheriting exactly this chain
    /// enumerates its own enumerable keys, then those of these it does not
    /// have as own properties. `walk(k, v)` recursing over many objects of
    /// one shape misses the per-target entry every time.
    /// Boxed so the cache stays one small allocation: growing it moved
    /// every later heap allocation of a script, and recursive_call_tree's
    /// helper program with them (+10% cycles, 2026-09-26).
    prototypes: Option<Box<PrototypeKeys>>,
}

struct PrototypeKeys {
    chain: Vec<OrdinaryChainLink>,
    inherited: Vec<Rc<str>>,
}

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
        let state = self.0.borrow();
        let entry = state.entry.as_ref()?;
        ordinary_chain_matches(target, &entry.chain).then(|| entry.keys.clone())
    }

    /// The keys of an ordinary `target` whose prototype chain is the
    /// remembered one: its own enumerable string keys in order, then the
    /// chain's keys that it does not shadow with an own property.
    pub(super) fn get_by_prototypes(&self, target: &Value) -> Option<ArrayRef> {
        let Value::Object(object) = target else {
            return None;
        };
        if !is_cacheable_ordinary_object(object) {
            return None;
        }
        let state = self.0.borrow();
        let prototypes = state.prototypes.as_ref()?;
        match object.prototype_slot() {
            Some(Prototype::Object(prototype)) => {
                if !ordinary_chain_matches(&Value::Object(prototype), &prototypes.chain) {
                    return None;
                }
            }
            None if prototypes.chain.is_empty() => {}
            _ => return None,
        }
        let own = object.own_string_keys_with_enumerability();
        let key_value = |key: &Rc<str>| Value::String(crate::JsString::from(&**key));
        let mut keys: Vec<Value> = own
            .iter()
            .filter(|(_, enumerable)| *enumerable)
            .map(|(key, _)| key_value(key))
            .collect();
        for key in &prototypes.inherited {
            if !own.iter().any(|(own_key, _)| own_key == key) {
                keys.push(key_value(key));
            }
        }
        Some(ArrayRef::new(keys))
    }

    /// Records a just-enumerated key array when `target` has an entirely
    /// ordinary object chain. Unsupported values intentionally clear a stale
    /// entry: a future ordinary target must rebuild before it can be cached.
    /// Also remembers the chain after `target` and the keys it contributes
    /// (see `get_by_prototypes`), unless that chain is already remembered.
    pub(super) fn record(&self, target: &Value, keys: ArrayRef) {
        let chain = capture_ordinary_chain(target);
        let mut state = self.0.borrow_mut();
        if let Some(chain) = &chain {
            let prototype_chain = &chain[1..];
            let current = state.prototypes.as_ref().is_some_and(|known| {
                known.chain.len() == prototype_chain.len()
                    && known
                        .chain
                        .iter()
                        .zip(prototype_chain)
                        .all(|(known, link)| {
                            known.layout_revision == link.layout_revision
                                && link
                                    .object
                                    .upgrade()
                                    .is_some_and(|object| known.object.ptr_eq(&object))
                        })
            });
            if !current {
                state.prototypes = prototype_keys(prototype_chain).map(Box::new);
            }
        }
        state.entry = chain.map(|chain| EnumerateKeysCacheEntry { chain, keys });
    }
}

/// The keys a `for-in` over an object with `chain` as its prototypes visits
/// from that chain: each layer's enumerable own string keys, less any name an
/// earlier layer has (enumerable or not).
fn prototype_keys(chain: &[OrdinaryChainLink]) -> Option<PrototypeKeys> {
    let mut seen: Vec<Rc<str>> = Vec::new();
    let mut inherited = Vec::new();
    for link in chain {
        let object = link.object.upgrade()?;
        for (key, enumerable) in object.own_string_keys_with_enumerability() {
            if seen.contains(&key) {
                continue;
            }
            if enumerable {
                inherited.push(key.clone());
            }
            seen.push(key);
        }
    }
    Some(PrototypeKeys {
        chain: chain
            .iter()
            .map(|link| OrdinaryChainLink {
                object: link.object.clone(),
                layout_revision: link.layout_revision,
            })
            .collect(),
        inherited,
    })
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
