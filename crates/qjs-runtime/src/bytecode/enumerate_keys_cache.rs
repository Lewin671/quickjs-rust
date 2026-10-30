//! Per-site for-in key lists guarded by receiver structure and weak prototype
//! dependencies. Value-only writes preserve the list; every structural edit
//! on its prototype chain invalidates the shared dependency before publication.
use crate::value::layout::{LayoutVersion, StructuralDependency};
use crate::value::named::WeakEdge;
use crate::{ArrayRef, ObjectRef, Prototype, Value};
use std::{cell::RefCell, fmt, rc::Rc};

#[derive(Clone)]
pub(super) struct EnumerateKeysCache(Rc<RefCell<Option<Entry>>>, bool);
impl Default for EnumerateKeysCache {
    fn default() -> Self {
        Self(
            Rc::new(RefCell::new(None)),
            std::env::var_os("QJS_DISABLE_IC").is_none(),
        )
    }
}
struct Entry {
    receiver: LayoutVersion,
    first: Option<WeakEdge>,
    validity: StructuralDependency,
    keys: ArrayRef,
}
impl fmt::Debug for EnumerateKeysCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EnumerateKeysCache(..)")
    }
}
impl EnumerateKeysCache {
    pub(super) fn get(&self, target: &Value) -> Option<ArrayRef> {
        if !self.1 {
            return None;
        }
        let Value::Object(object) = target else {
            return None;
        };
        if !cacheable(object) {
            return None;
        }
        let state = self.0.borrow();
        let entry = state.as_ref()?;
        if !entry.validity.is_valid() || !object.with_named(|s| s.matches_version(&entry.receiver))
        {
            return None;
        }
        match (&entry.first, object.prototype_slot()) {
            (None, None) => Some(entry.keys.clone()),
            (Some(edge), Some(prototype)) if edge.matches(&prototype) => Some(entry.keys.clone()),
            _ => None,
        }
    }
    pub(super) fn record(&self, target: &Value, keys: ArrayRef) {
        if !self.1 {
            return;
        }
        *self.0.borrow_mut() = capture(target, keys);
    }
}
fn capture(target: &Value, keys: ArrayRef) -> Option<Entry> {
    let Value::Object(object) = target else {
        return None;
    };
    if !cacheable(object) {
        return None;
    }
    let first = object.prototype_slot();
    let edge = match &first {
        Some(p) => Some(WeakEdge::new(p)?),
        None => None,
    };
    let validity = StructuralDependency::new();
    let mut current = first;
    for _ in 0..16 {
        match current {
            None => {
                return Some(Entry {
                    receiver: object.layout_revision(),
                    first: edge,
                    validity,
                    keys,
                });
            }
            Some(Prototype::Object(ref node)) if cacheable(node) => {
                node.with_named(|s| s.subscribe(&validity));
                current = node.prototype_slot();
            }
            _ => return None,
        }
    }
    None
}
fn cacheable(object: &ObjectRef) -> bool {
    object.named_kind() == Some(0)
}
