//! Predictions must preserve live payloads and exactly the completed prefix.
use super::*;
fn accessor(value: f64) -> Property {
    Property::accessor(Some(Value::Number(value)), None, false, true)
}
fn symbol(domain: &AtomDomain) -> crate::ObjectRef {
    let symbol = crate::ObjectRef::new(HashMap::new(), domain);
    symbol.mark_symbol_primitive();
    symbol
}
#[test]
fn descriptor_sequences_keep_live_accessors_and_rebuild_mismatching_prefixes() {
    let owner = PropertyOwner::new(AtomDomain::new());
    let domain = owner.domain();
    let first = symbol(domain);
    let second = symbol(domain);
    let mut initial = DataSequence::new(None, domain, None);
    initial.insert_property("a", accessor(1.0));
    initial.insert_symbol(&first, Property::non_enumerable(Value::Number(2.0)));
    initial.insert("2", Value::Number(3.0));
    let (_, prediction, _) = initial.finish();
    let prediction = prediction.unwrap();
    for mismatch in 0..3 {
        let mut sequence = DataSequence::new(Some(prediction.clone()), domain, None);
        sequence.insert_property("a", accessor(10.0));
        let chosen = if mismatch == 0 { &second } else { &first };
        let property = Property::data(Value::Number(20.0), mismatch == 1, true, true);
        sequence.insert_symbol(chosen, property);
        if mismatch != 2 {
            sequence.insert("2", Value::Number(30.0));
        }
        let (store, indices) = sequence.finish_store();
        assert_eq!(
            store.get_text("a").unwrap().getter(),
            Some(&Value::Number(10.0))
        );
        let key = domain.intern_symbol(chosen);
        let descriptor = store.get(&key).unwrap().unwrap();
        assert_eq!(descriptor.value, Value::Number(20.0));
        assert_eq!(descriptor.enumerable, mismatch == 1);
        assert_eq!(store.len(), if mismatch == 2 { 2 } else { 3 });
        assert_eq!(indices, usize::from(mismatch != 2));
        store.verify();
    }
    let mut exact = DataSequence::new(Some(prediction.clone()), domain, None);
    exact.insert_property("a", accessor(100.0));
    exact.insert_symbol(&first, Property::non_enumerable(Value::Number(200.0)));
    exact.insert("2", Value::Number(300.0));
    let (store, _) = exact.finish_store();
    assert!(Rc::ptr_eq(store.layout(), &prediction));
    assert_eq!(
        store.get_text("a").unwrap().getter(),
        Some(&Value::Number(100.0))
    );
    assert_eq!(store.get_text("2").unwrap().value, Value::Number(300.0));
}
#[test]
fn descriptor_predictions_reject_foreign_domains_and_prototype_edges() {
    let owner = PropertyOwner::new(AtomDomain::new());
    let mut initial = DataSequence::new(None, owner.domain(), None);
    initial.insert("x", Value::Null);
    let (_, prediction, _) = initial.finish();
    let foreign = PropertyOwner::new(AtomDomain::new());
    let mut sequence = DataSequence::new(prediction.clone(), foreign.domain(), None);
    sequence.insert("x", Value::Number(1.0));
    let (store, _) = sequence.finish_store();
    assert!(store.domain().ptr_eq(foreign.domain()));
    let prototype = crate::ObjectRef::new(HashMap::new(), owner.domain());
    let mut sequence = DataSequence::new(
        prediction,
        owner.domain(),
        Some(crate::Prototype::Object(prototype.clone())),
    );
    sequence.insert("x", Value::Number(2.0));
    let (store, _) = sequence.finish_store();
    assert!(store.prototype_is_object(&prototype));
}

#[test]
fn reusable_predictions_are_canonical_even_for_a_new_prototype_family() {
    let domain = crate::property::atom::AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let parent = crate::ObjectRef::new(std::collections::HashMap::new(), &domain);
    let mut builder = DataSequence::new(
        None,
        &domain,
        Some(crate::Prototype::Object(parent.clone())),
    );
    builder.insert("x", Value::Number(1.0));
    let (first, prediction, _) = builder.finish();
    let prediction = prediction.unwrap();
    assert!(!prediction.transient);
    assert!(Rc::ptr_eq(&first.layout, &prediction));
    let mut next = DataSequence::new(
        Some(prediction.clone()),
        &domain,
        Some(crate::Prototype::Object(parent)),
    );
    next.insert("x", Value::Number(2.0));
    let (second, _, _) = next.finish();
    assert!(Rc::ptr_eq(&second.layout, &prediction));
    assert_eq!(second.get_text("x").unwrap().value, Value::Number(2.0));
}

#[test]
fn string_wrapper_templates_publish_before_instances_are_minted() {
    let domain = crate::property::atom::AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let parent = crate::ObjectRef::new(std::collections::HashMap::new(), &domain);
    let template = StringWrapperLayout::new(&domain, parent);
    let a = template.instantiate(1.0);
    let b = template.instantiate(2.0);
    assert!(!a.layout.transient && !b.layout.transient);
    assert!(Rc::ptr_eq(&a.layout, &b.layout));
    assert_eq!(a.get_text("length").unwrap().value, Value::Number(1.0));
    assert_eq!(b.get_text("length").unwrap().value, Value::Number(2.0));
}
