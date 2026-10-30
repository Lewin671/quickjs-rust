//! A named store site's negative prototype proof. Creation still uses the
//! shared own-property algorithm; this memo only skips a previously established
//! prototype miss. Weak structural subscriptions guard the whole chain, and the
//! current receiver's first edge is checked explicitly on every hit.
use crate::value::{ObjectWeakRef, layout::StructuralDependency};
use crate::{ObjectRef, Prototype};
use std::cell::RefCell;

#[derive(Debug)]
pub(in crate::bytecode) struct CreationCache {
    proof: RefCell<Option<CreationProof>>,
    enabled: bool,
}
impl Default for CreationCache {
    fn default() -> Self {
        Self {
            proof: RefCell::new(None),
            enabled: std::env::var_os("QJS_DISABLE_IC").is_none(),
        }
    }
}
impl Clone for CreationCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}
#[derive(Debug)]
struct CreationProof {
    first: Option<ObjectWeakRef>,
    validity: StructuralDependency,
}
impl CreationCache {
    pub(in crate::bytecode) fn permits(&self, object: &ObjectRef) -> bool {
        if !self.enabled || !ordinary(object) || !object.is_extensible() {
            return false;
        }
        let proof = self.proof.borrow();
        let Some(proof) = proof.as_ref() else {
            return false;
        };
        proof.validity.is_valid()
            && match &proof.first {
                Some(first) => object.prototype_is_weak(first),
                None => object.prototype_slot().is_none(),
            }
    }
    pub(in crate::bytecode) fn record(&self, object: &ObjectRef, key: &str) {
        if !self.enabled {
            return;
        }
        let mut current = object.prototype_slot();
        let first = match &current {
            Some(Prototype::Object(o)) => Some(o.downgrade()),
            None => None,
            _ => return,
        };
        let validity = StructuralDependency::new();
        for _ in 0..16 {
            match current {
                None => {
                    *self.proof.borrow_mut() = Some(CreationProof { first, validity });
                    return;
                }
                Some(Prototype::Object(ref node))
                    if ordinary(node) && node.own_property(key).is_none() =>
                {
                    node.with_named(|store| store.subscribe(&validity));
                    current = node.prototype_slot();
                }
                _ => return,
            }
        }
    }
}
fn ordinary(object: &ObjectRef) -> bool {
    !crate::symbol::is_symbol_primitive(object)
        && !crate::typed_array::is_typed_array_object(object)
        && !object.is_module_namespace_exotic()
        && !object.is_array_prototype_exotic()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    #[test]
    fn creation_proof_observes_all_structural_edges_and_ignores_value_writes() {
        let domain = crate::property::atom::AtomDomain::new();
        let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
        let holder = ObjectRef::new(HashMap::new(), &domain);
        holder.set("other".into(), crate::Value::Null);
        let middle = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
        let receiver = ObjectRef::with_prototype(HashMap::new(), Some(middle.clone()), &domain);
        let cache = CreationCache {
            enabled: true,
            ..CreationCache::default()
        };
        cache.record(&receiver, "x");
        assert!(cache.permits(&receiver));
        holder.set("other".into(), crate::Value::Boolean(true));
        assert!(cache.permits(&receiver));
        holder.set("x".into(), crate::Value::Null);
        assert!(!cache.permits(&receiver));
        holder.delete_own_property("x");
        cache.record(&receiver, "x");
        assert!(cache.permits(&receiver));
        middle.set_prototype(None).unwrap();
        assert!(!cache.permits(&receiver));
        cache.record(&receiver, "x");
        assert!(cache.permits(&receiver));
        receiver.set_prototype(Some(holder)).unwrap();
        assert!(!cache.permits(&receiver));
        let disabled = CreationCache {
            enabled: false,
            ..CreationCache::default()
        };
        disabled.record(&receiver, "x");
        assert!(!disabled.permits(&receiver));
    }
}
