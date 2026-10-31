//! A creation miss proof uses prototype-root identity and one transitive
//! prototype validity generation. No per-site subscriptions or chain hit walk.
use crate::value::layout::{LayoutVersion, StructuralDependency};
use crate::{ObjectRef, Prototype};
use std::cell::RefCell;
#[derive(Debug, Default)]
pub(in crate::bytecode) struct CreationCache {
    proof: RefCell<Option<CreationProof>>,
    #[cfg(test)]
    disabled: bool,
}
impl Clone for CreationCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}
#[derive(Debug)]
struct CreationProof {
    root: LayoutVersion,
    validity: Option<StructuralDependency>,
}
impl CreationCache {
    pub(in crate::bytecode) fn permits(&self, o: &ObjectRef) -> bool {
        #[cfg(test)]
        if self.disabled {
            return false;
        }
        if !ordinary(o) || !o.is_extensible() {
            return false;
        }
        self.proof.borrow().as_ref().is_some_and(|p| {
            o.with_named(|s| s.root_matches(&p.root))
                && p.validity
                    .as_ref()
                    .is_none_or(StructuralDependency::is_valid)
        })
    }
    pub(in crate::bytecode) fn record(&self, o: &ObjectRef, name: &str) {
        #[cfg(test)]
        if self.disabled {
            return;
        }
        if !o.with_named(|s| s.domain().ic_policy().enabled) {
            return;
        }
        let mut current = o.prototype_slot();
        let validity = if current.is_some() {
            match o.with_named(|s| s.chain_dependency()) {
                Some(v) => Some(v),
                None => return,
            }
        } else {
            None
        };
        for _ in 0..64 {
            match current {
                None => {
                    *self.proof.borrow_mut() = Some(CreationProof {
                        root: o.with_named(|s| s.root_version()),
                        validity,
                    });
                    return;
                }
                Some(Prototype::Object(ref node))
                    if ordinary(node) && node.own_property(name).is_none() =>
                {
                    current = node.prototype_slot()
                }
                _ => return,
            }
        }
    }
}
fn ordinary(o: &ObjectRef) -> bool {
    o.named_kind() == Some(0)
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    #[test]
    fn creation_proof_ignores_ordinary_transitions_and_checks_prototype_mutation() {
        let domain = crate::property::atom::AtomDomain::new();
        let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
        let parent = ObjectRef::new(HashMap::new(), &domain);
        let receiver = ObjectRef::with_prototype(HashMap::new(), Some(parent.clone()), &domain);
        let cache = CreationCache::default();
        cache.record(&receiver, "x");
        assert!(cache.permits(&receiver));
        for _ in 0..1000 {
            let other = ObjectRef::with_prototype(HashMap::new(), Some(parent.clone()), &domain);
            other.set("x".into(), crate::Value::Null);
            assert!(cache.permits(&receiver));
        }
        parent.set("x".into(), crate::Value::Null);
        assert!(!cache.permits(&receiver));
        let off = CreationCache {
            disabled: true,
            ..CreationCache::default()
        };
        off.record(&receiver, "z");
        assert!(!off.permits(&receiver));
    }
}
