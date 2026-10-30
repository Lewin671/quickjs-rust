//! Realm-owned construction metadata may retain only intrinsic symbol keys.
//! Actual values/accessor closures and receiver/prototype edges remain live.
use super::*;
impl RealmState {
    pub(crate) fn retain_arguments_layout(&self, object: &ObjectRef) {
        object.with_named(|store| {
            let Some(layout) = store.shared_layout() else { return; };
            let mut layouts = self.arguments_layouts.borrow_mut();
            if layouts.iter().any(|old| Rc::ptr_eq(old, layout)) {
                crate::diagnostics::count!(property_arguments_template_hits);
                return;
            }
            crate::diagnostics::count!(property_arguments_template_misses);
            if !store.domain().ptr_eq(self.property_owner.domain()) { return; }
            let Some(prototype) = self.object_prototype.get() else { return; };
            if !matches!(store.prototype(), Some(crate::Prototype::Object(actual)) if actual.ptr_eq(prototype)) { return; }
            let Some(symbols) = self.well_known_symbols.get() else { return; };
            if store.keys().iter().any(|key| key.symbol().is_some_and(|symbol|
                !symbols.iter().any(|intrinsic| intrinsic.ptr_eq(symbol)))) { return; }
            if layouts.len() == 32 { layouts.remove(0); }
            layouts.push(layout.clone());
            crate::diagnostics::count!(property_arguments_templates_inserted);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::layout::PropertyOwner;
    fn symbol(domain: &AtomDomain) -> ObjectRef {
        let object = ObjectRef::new(HashMap::new(), domain);
        object.mark_symbol_primitive();
        object
    }
    #[test]
    fn arguments_cache_is_bounded_and_rejects_foreign_symbols_and_private_layouts() {
        let realm = super::super::new_realm(HashMap::new());
        let domain = realm.property_owner.domain();
        let prototype = ObjectRef::new(HashMap::new(), domain);
        realm.object_prototype.set(prototype.clone()).unwrap();
        let iterator = symbol(domain);
        realm
            .well_known_symbols
            .set(vec![iterator.clone()])
            .unwrap();
        let make = |count| {
            let object = ObjectRef::with_prototype(HashMap::new(), Some(prototype.clone()), domain);
            object.define_property(
                "length".into(),
                crate::Property::non_enumerable(Value::Number(count as f64)),
            );
            for i in 0..count {
                object.define_property(i.to_string(), crate::Property::enumerable(Value::Null));
            }
            object.define_symbol_property(
                iterator.clone(),
                crate::Property::non_enumerable(Value::Undefined),
            );
            object
        };
        for count in 0..33 {
            realm.retain_arguments_layout(&make(count));
        }
        assert_eq!(realm.arguments_layouts.borrow().len(), 32);
        let object = make(1);
        object.define_symbol_property(symbol(domain), crate::Property::enumerable(Value::Null));
        realm.retain_arguments_layout(&object);
        assert_eq!(realm.arguments_layouts.borrow().len(), 32);
        assert!(
            !realm
                .arguments_layouts
                .borrow()
                .iter()
                .any(|layout| object.with_named(|s| Rc::ptr_eq(s.layout(), layout)))
        );
        let wide = make(130);
        realm.retain_arguments_layout(&wide);
        assert!(
            !realm
                .arguments_layouts
                .borrow()
                .iter()
                .any(|layout| wide.with_named(|s| Rc::ptr_eq(s.layout(), layout)))
        );
        let foreign = AtomDomain::new();
        let _foreign_owner = PropertyOwner::new(foreign.clone());
        let object = ObjectRef::new(HashMap::new(), &foreign);
        realm.retain_arguments_layout(&object);
        assert_eq!(realm.arguments_layouts.borrow().len(), 32);
    }
}
