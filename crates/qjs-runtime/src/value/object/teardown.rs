//! An object's side of `value::teardown`: its data property values are the
//! children a chain runs through.

use std::rc::Rc;

use super::{ObjectData, ObjectRef};
use crate::value::{Value, teardown};

impl ObjectRef {
    pub(in crate::value) fn is_last_reference(&self) -> bool {
        Rc::strong_count(&self.0) == 1
    }

    /// Drops this reference; when it was the last, the object's own
    /// children go onto `pending` first.
    pub(in crate::value) fn release_into(self, pending: &mut Vec<Value>) {
        if let Some(mut data) = Rc::into_inner(self.0) {
            data.properties.get_mut().defer_children(pending);
        }
    }
}

impl Drop for ObjectData {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        self.properties.get_mut().defer_children(&mut pending);
        if !pending.is_empty() {
            teardown::release(pending);
        }
    }
}
