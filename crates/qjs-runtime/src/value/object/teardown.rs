//! An object's side of `value::teardown`: its data property values are the
//! children a chain runs through.

use std::rc::Rc;

use super::{ObjectData, ObjectRef, PropertyStorage};
use crate::value::{Value, teardown};

impl ObjectRef {
    pub(in crate::value) fn is_last_reference(&self) -> bool {
        Rc::strong_count(&self.0) == 1
    }

    /// Drops this reference; when it was the last, the object's own
    /// children go onto `pending` first.
    pub(in crate::value) fn release_into(self, pending: &mut Vec<Value>) {
        if let Some(mut data) = Rc::into_inner(self.0) {
            defer_children(data.properties.get_mut(), pending);
        }
    }
}

fn defer_children(properties: &mut PropertyStorage, pending: &mut Vec<Value>) {
    match properties {
        PropertyStorage::Small { entries } => {
            for (_, property) in entries {
                teardown::defer_if_last(&mut property.value, pending);
            }
        }
        PropertyStorage::Dynamic(dynamic) => {
            for (_, property) in &mut dynamic.entries {
                teardown::defer_if_last(&mut property.value, pending);
            }
        }
        PropertyStorage::Shaped { properties, .. } => {
            for property in properties {
                teardown::defer_if_last(&mut property.value, pending);
            }
        }
        PropertyStorage::ShapedPair { values, .. } => {
            for value in values {
                teardown::defer_if_last(value, pending);
            }
        }
    }
}

impl Drop for ObjectData {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        defer_children(self.properties.get_mut(), &mut pending);
        if !pending.is_empty() {
            teardown::release(pending);
        }
    }
}
