//! Empty receiver allocation from a metadata-only constructor root link.
use super::*;
impl ObjectRef {
    pub(crate) fn with_constructor_layout(
        cache: &mut Option<Rc<super::super::layout::Layout>>,
        domain: &crate::property::atom::AtomDomain,
        prototype: Option<Prototype>,
    ) -> Self {
        Self::from_ready_named(NamedStore::empty_cached(cache, domain, prototype), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roots_relink_on_domain_or_prototype_change_without_publishing_future_fields() {
        let a = crate::property::atom::AtomDomain::new();
        let b = crate::property::atom::AtomDomain::new();
        let owner_a = super::super::super::layout::PropertyOwner::new(a.clone());
        let _owner_b = super::super::super::layout::PropertyOwner::new(b.clone());
        let parent = ObjectRef::new(HashMap::new(), &a);
        let mut cache = None;
        let first = ObjectRef::with_constructor_layout(
            &mut cache,
            &a,
            Some(Prototype::Object(parent.clone())),
        );
        for key in ["a", "b", "c", "d", "e"] {
            first.set(key.into(), Value::Null);
        }
        let root = cache.as_ref().unwrap().clone();
        let next = ObjectRef::with_constructor_layout(
            &mut cache,
            &a,
            Some(Prototype::Object(parent.clone())),
        );
        assert!(Rc::ptr_eq(&root, cache.as_ref().unwrap()));
        assert!(next.own_property_names().is_empty());
        assert_eq!(
            next.prototype_slot().unwrap().identity(),
            Prototype::Object(parent.clone()).identity()
        );
        let weak = parent.downgrade();
        drop(first);
        drop(next);
        drop(parent);
        drop(owner_a);
        assert!(
            weak.upgrade().is_none(),
            "root metadata must not pin its prototype"
        );
        let foreign = ObjectRef::with_constructor_layout(&mut cache, &b, None);
        assert!(foreign.domain().ptr_eq(&b));
        assert!(!Rc::ptr_eq(&root, cache.as_ref().unwrap()));
    }
}
