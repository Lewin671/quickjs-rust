//! Batch publication preserves descriptor, order, cache and ownership semantics.
use crate::property::atom::AtomDomain;
use crate::property::layout::SHAPE_FIELD_LIMIT;
use crate::{ObjectRef, Property, Value};
use std::{collections::HashMap, rc::Rc};

#[test]
fn duplicate_batch_members_keep_first_position_and_last_descriptor() {
    let object = ObjectRef::new(HashMap::new());
    object.define_non_enumerable("before".into(), Value::Number(0.0));
    let slot = object.shared_data_slot("before").unwrap();
    let revision = object.layout_revision();
    object.define_properties_ordered(vec![
        (
            Rc::from("before"),
            Property::accessor(None, None, false, true),
        ),
        (Rc::from("x"), Property::enumerable(Value::Number(1.0))),
        (Rc::from("y"), Property::non_enumerable(Value::Number(2.0))),
        (
            Rc::from("x"),
            Property::data(Value::Number(3.0), false, false, false),
        ),
    ]);
    assert_eq!(object.own_property_names(), ["before", "x", "y"]);
    let x = object.own_property("x").unwrap();
    assert_eq!(x.value, Value::Number(3.0));
    assert!(!x.enumerable && !x.writable && !x.configurable);
    assert_ne!(object.layout_revision(), revision);
    assert_eq!(object.shared_data_slot_value(&slot), None);
    assert!(object.own_property("before").unwrap().is_accessor());
}

#[test]
fn numeric_batch_uses_existing_index_classification_and_order() {
    let object = ObjectRef::new(HashMap::new());
    object.define_properties_ordered(vec![
        (Rc::from("10"), Property::enumerable(Value::Number(10.0))),
        (Rc::from("text"), Property::enumerable(Value::Undefined)),
        (Rc::from("2"), Property::enumerable(Value::Number(2.0))),
    ]);
    assert_eq!(object.own_property_names(), ["2", "10", "text"]);
    assert_eq!(object.0.index_property_count.get(), 2);
}

#[test]
fn batches_share_only_immutable_layouts_in_the_same_domain() {
    let domain = AtomDomain::new();
    let entries = || vec![(Rc::from("a"), Property::enumerable(Value::Number(1.0)))];
    let a = ObjectRef::new_in(&domain, HashMap::new());
    let b = ObjectRef::new_in(&domain, HashMap::new());
    let foreign = ObjectRef::new_in(&AtomDomain::new(), HashMap::new());
    for object in [&a, &b, &foreign] {
        object.define_properties_ordered(entries());
    }
    assert!(Rc::ptr_eq(
        &a.0.properties.borrow().layout,
        &b.0.properties.borrow().layout
    ));
    assert!(!Rc::ptr_eq(
        &a.0.properties.borrow().layout,
        &foreign.0.properties.borrow().layout
    ));
    a.set("a".into(), Value::Number(9.0));
    assert_eq!(b.get("a"), Some(Value::Number(1.0)));
    let weak = a.downgrade();
    drop(a);
    assert!(weak.upgrade().is_none());
}

#[test]
fn large_complete_batch_obeys_dictionary_boundary_and_subsequent_edits() {
    let object = ObjectRef::new(HashMap::new());
    object.define_properties_ordered(
        (0..SHAPE_FIELD_LIMIT + 2)
            .map(|i| {
                (
                    Rc::from(format!("key{i}")),
                    Property::enumerable(Value::Number(i as f64)),
                )
            })
            .collect(),
    );
    assert!(object.0.properties.borrow().layout.is_dictionary());
    assert!(object.delete_own_property("key2"));
    object.set("tail".into(), Value::Number(999.0));
    assert_eq!(object.get("key3"), Some(Value::Number(3.0)));
    assert_eq!(object.own_property_names().last().unwrap(), "tail");
}

#[cfg(feature = "perf-counters")]
#[test]
fn complete_batch_copies_each_final_field_once_instead_of_prefixes() {
    let object = ObjectRef::new(HashMap::new());
    crate::reset_counters();
    object.define_properties_ordered(
        (0..64)
            .map(|i| {
                (
                    Rc::from(format!("key{i}")),
                    Property::enumerable(Value::Undefined),
                )
            })
            .collect(),
    );
    let counters = crate::counters().unwrap();
    assert_eq!(counters.property_shapes_created, 1);
    assert_eq!(counters.property_shape_fields_copied, 64);
}
