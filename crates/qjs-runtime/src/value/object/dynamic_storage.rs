//! Cold dynamic payload; the enclosing storage's atom layout owns lookup and ordering.
use crate::Property;
use std::rc::Rc;
pub(super) struct DynamicPropertyStorage {
    pub(super) entries: Vec<(Rc<str>, Property)>,
}
impl DynamicPropertyStorage {
    pub(super) fn ordered(entries: Vec<(Rc<str>, Property)>) -> Self {
        Self { entries }
    }
}
