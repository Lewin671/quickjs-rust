//! Installation of observer-free constant public class-field segments.
//!
//! Admission completes before mutation. Only ordinary storage with
//! configurable existing descriptors is admitted; every other receiver keeps
//! the sequential DefineOwnProperty path. No JavaScript runs under the borrow.

use std::rc::Rc;

use crate::{Property, Value};

use super::ObjectRef;

impl ObjectRef {
    /// Defines a constant segment in order with one check and reservation.
    /// The caller proves that values are primitive constants, keys are resolved
    /// strings, and no initializer/observer occurs between these definitions.
    /// Returns false before mutation if ordinary default definitions might fail.
    pub(crate) fn install_constant_class_fields<'a>(
        &self,
        fields: impl ExactSizeIterator<Item = (&'a Rc<str>, &'a Value)>,
    ) -> bool {
        if self.0.flag(super::flags::STRING_INDICES)
            || !self.is_extensible()
            || crate::typed_array::is_typed_array_object(self)
            || self.is_module_namespace_exotic()
            || self.is_array_prototype_exotic()
            || self.is_symbol_object()
        {
            return false;
        }
        if self
            .0
            .properties
            .borrow()
            .descriptors()
            .any(|p| !p.configurable)
        {
            return false;
        }
        for (key, value) in fields {
            self.define_shared_property(key.clone(), Property::enumerable(value.clone()));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, rc::Rc};

    use crate::{ObjectRef, Property, Value};

    #[test]
    fn declines_without_mutation_for_descriptor_boundaries() {
        let domain = crate::property::atom::AtomDomain::new();
        let a: Rc<str> = "a".into();
        let b: Rc<str> = "b".into();
        let value = Value::Number(2.0);
        let fields = [(&a, &value), (&b, &value)];
        let object = ObjectRef::new(HashMap::new(), &domain);
        let mut fixed = Property::enumerable(Value::Number(1.0));
        fixed.configurable = false;
        object.define_property("b".into(), fixed);
        assert!(!object.install_constant_class_fields(fields.into_iter()));
        assert!(!object.has_own_property("a"));
        assert_eq!(object.get("b"), Some(Value::Number(1.0)));
    }

    #[test]
    fn duplicates_keep_slots_and_invalidate_cached_layouts() {
        let domain = crate::property::atom::AtomDomain::new();
        let object = ObjectRef::new(HashMap::new(), &domain);
        let key: Rc<str> = "0".into();
        let first = Value::Number(1.0);
        let last = Value::Number(2.0);
        let layout = object.layout_revision();
        assert!(object.install_constant_class_fields([(&key, &first), (&key, &last)].into_iter()));
        assert_eq!(object.get("0"), Some(last));
        assert_eq!(object.0.index_property_count.get(), 1);
        assert_ne!(object.layout_revision(), layout);
    }
}
