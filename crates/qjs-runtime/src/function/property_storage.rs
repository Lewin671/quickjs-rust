//! Lazy function named storage. The table stays unallocated until installed
//! properties are observed; every mutable borrow publishes descriptor metadata
//! before it releases the storage. No borrow may survive a JavaScript callback.
use std::cell::{Ref, RefCell, RefMut};

#[derive(Default)]
/// Installed descriptors use the shared ordinary atom/layout contract. Default
/// name/length/prototype semantic hooks remain in `value.rs`.
pub(crate) struct LazyFunctionProperties(
    RefCell<Option<Box<crate::property::named_map::NamedPropertyMap>>>,
);

impl LazyFunctionProperties {
    pub(crate) fn borrow(&self) -> Ref<'_, crate::property::named_map::NamedPropertyMap> {
        self.ensure_allocated();
        Ref::map(self.0.borrow(), |properties| {
            properties
                .as_deref()
                .expect("function properties initialized before borrowing")
        })
    }

    pub(crate) fn borrow_mut(&self) -> crate::property::named_map::PropertyMapEdit<'_> {
        self.ensure_allocated();
        crate::property::named_map::PropertyMapEdit::from_ref(RefMut::map(
            self.0.borrow_mut(),
            |properties| {
                properties
                    .as_deref_mut()
                    .expect("function properties initialized before borrowing")
            },
        ))
    }

    fn ensure_allocated(&self) {
        if self.0.borrow().is_none() {
            *self.0.borrow_mut() = Some(Box::default());
        }
    }

    #[cfg(test)]
    pub(super) fn is_allocated(&self) -> bool {
        self.0.borrow().is_some()
    }
}
