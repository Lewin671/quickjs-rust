//! Cold dynamic payload; the enclosing storage's atom layout owns lookup.
use crate::Property;
use std::rc::Rc;
pub(super) struct DynamicPropertyStorage {
    pub(super) entries: Vec<(Rc<str>, Property)>,
    pub(super) order: Vec<Rc<str>>,
}
impl DynamicPropertyStorage {
    pub(super) fn ordered(entries: Vec<(Rc<str>, Property)>) -> Self {
        let order = entries.iter().map(|(key, _)| key.clone()).collect();
        Self { entries, order }
    }
}
