//! Realm-owned metadata for an intrinsic String wrapper's fixed own length.
//! The template owns no values; its prototype is already a realm intrinsic.
use super::*;
pub(crate) struct StringWrapperLayout {
    layout: Rc<Layout>,
    prototype: Option<Rc<crate::Prototype>>,
}
impl StringWrapperLayout {
    pub(crate) fn new(domain: &AtomDomain, prototype: crate::ObjectRef) -> Self {
        let store = NamedStore::string_wrapper(domain, Some(prototype), 0.0);
        Self {
            layout: store.layout,
            prototype: store.prototype,
        }
    }
    pub(crate) fn instantiate(&self, length: f64) -> NamedStore {
        let store = NamedStore {
            layout: self.layout.clone(),
            prototype: self.prototype.clone(),
            values: ValueSlots::One(Value::Number(length)),
            accessors: None,
            epoch: 0,
        };
        store.verify();
        store
    }
}
impl NamedStore {
    /// Unobserved String construction installs the final exotic flavor first.
    pub(crate) fn string_wrapper(
        domain: &AtomDomain,
        prototype: Option<crate::ObjectRef>,
        length: f64,
    ) -> Self {
        let mut store =
            Self::empty_with_prototype(domain, prototype.map(crate::Prototype::Object), 3);
        store.insert_text(
            "length",
            Property::data(Value::Number(length), false, false, false),
        );
        store
    }
}
