//! for-in key lists guarded by receiver layout and the first prototype's
//! transitive validity cell. Values never participate in this proof.
use crate::value::layout::{LayoutVersion, StructuralDependency};
use crate::{ArrayRef, ObjectRef, Prototype, Value};
use std::{cell::RefCell, fmt, rc::Rc};
#[derive(Clone, Default)]
pub(super) struct EnumerateKeysCache(Rc<RefCell<Option<Entry>>>);
struct Entry {
    receiver: LayoutVersion,
    validity: Option<StructuralDependency>,
    keys: ArrayRef,
}
impl fmt::Debug for EnumerateKeysCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EnumerateKeysCache(..)")
    }
}
impl EnumerateKeysCache {
    pub(super) fn get(&self, target: &Value) -> Option<ArrayRef> {
        let Value::Object(o) = target else {
            return None;
        };
        if !cacheable(o) {
            return None;
        }
        let state = self.0.borrow();
        let entry = state.as_ref()?;
        (o.layout_matches(&entry.receiver)
            && entry
                .validity
                .as_ref()
                .is_none_or(StructuralDependency::is_valid))
        .then(|| entry.keys.clone())
    }
    pub(super) fn record(&self, target: &Value, keys: ArrayRef) {
        *self.0.borrow_mut() = capture(target, keys)
    }
}
fn capture(target: &Value, keys: ArrayRef) -> Option<Entry> {
    let Value::Object(o) = target else {
        return None;
    };
    if !cacheable(o) || !o.with_named(|s| s.domain().ic_policy().enabled) {
        return None;
    }
    let mut current = o.prototype_slot();
    let validity = if current.is_some() {
        Some(o.with_named(|s| s.chain_dependency())?)
    } else {
        None
    };
    for _ in 0..64 {
        match current {
            None => {
                return Some(Entry {
                    receiver: o.layout_revision(),
                    validity,
                    keys,
                });
            }
            Some(Prototype::Object(ref node)) if cacheable(node) => current = node.prototype_slot(),
            _ => return None,
        }
    }
    None
}
fn cacheable(o: &ObjectRef) -> bool {
    o.named_kind() == Some(0)
}
