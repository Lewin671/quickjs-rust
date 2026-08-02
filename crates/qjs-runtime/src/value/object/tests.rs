//! Unit tests for the object representation.
//!
//! Split out of `object.rs`, which holds the representation itself: the
//! property storage kinds, the slot and shape accessors, and the ordinary
//! object protocol. The tests are about the same subject but are not part of
//! reading it.

use std::{collections::HashMap, mem, rc::Rc};

use super::{ObjectData, ObjectLiteralShape, ObjectRef, OwnDataPropertyWrite, PropertyStorage};
use crate::{Property, Value};

#[test]
fn array_index_keys_accept_only_canonical_decimal_spellings() {
    use super::array_index_property_key as parse;
    assert_eq!(parse("0"), Some(0));
    assert_eq!(parse("7"), Some(7));
    assert_eq!(parse("4294967294"), Some(u32::MAX - 1));
    // `u32::MAX` is the length bound, never an index.
    assert_eq!(parse("4294967295"), None);
    assert_eq!(parse("4294967296"), None);
    // Non-canonical spellings name ordinary string properties instead.
    assert_eq!(parse(""), None);
    assert_eq!(parse("00"), None);
    assert_eq!(parse("01"), None);
    assert_eq!(parse("+1"), None);
    assert_eq!(parse("-1"), None);
    assert_eq!(parse(" 1"), None);
    assert_eq!(parse("1 "), None);
    assert_eq!(parse("1.0"), None);
    assert_eq!(parse("1e2"), None);
    assert_eq!(parse("0x1"), None);
    assert_eq!(parse("99999999999"), None);
}

#[test]
fn cloned_object_is_a_pointer_sized_shared_handle() {
    let object = ObjectRef::new(HashMap::new());
    let cloned = object.clone();

    assert!(Rc::ptr_eq(&object.0, &cloned.0));
    assert!(object.ptr_eq(&cloned));
    assert_eq!(
        mem::size_of::<ObjectRef>(),
        mem::size_of::<Rc<ObjectData>>()
    );
}

#[test]
fn ordinary_object_keeps_cold_state_unallocated() {
    let object = ObjectRef::new(HashMap::from([
        ("a".to_owned(), Value::Number(1.0)),
        ("b".to_owned(), Value::Number(2.0)),
    ]));

    assert_eq!(object.get("a"), Some(Value::Number(1.0)));
    assert!(object.own_property_symbols().is_empty());
    assert!(object.to_string_tag().is_none());
    assert!(object.0.cold.get().is_none());
    // Boxing the cold `Dynamic` property-storage payload (HashMap + Vec)
    // keeps this at 104 bytes instead of the 136 it cost when that
    // payload sized the whole `PropertyStorage` enum for every object.
    assert!(mem::size_of::<ObjectData>() <= 112);
}

#[test]
fn ordinary_small_object_promotes_only_after_the_compact_limit() {
    let object = ObjectRef::new(HashMap::new());

    for index in 0..PropertyStorage::SMALL_LIMIT {
        object.set(format!("field{index}"), Value::Number(index as f64));
    }
    assert!(matches!(
        &*object.0.properties.borrow(),
        PropertyStorage::Small { entries } if entries.len() == PropertyStorage::SMALL_LIMIT
    ));

    let dynamic_index = PropertyStorage::SMALL_LIMIT;
    object.set(
        format!("field{dynamic_index}"),
        Value::Number(dynamic_index as f64),
    );
    assert!(matches!(
        &*object.0.properties.borrow(),
        PropertyStorage::Dynamic(dynamic)
            if dynamic.properties.len() == PropertyStorage::SMALL_LIMIT + 1
                && dynamic.order.len() == PropertyStorage::SMALL_LIMIT + 1
    ));
    assert_eq!(
        object.own_property_names(),
        (0..=PropertyStorage::SMALL_LIMIT)
            .map(|index| format!("field{index}"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn ordinary_object_retains_shared_static_property_key() {
    let object = ObjectRef::new(HashMap::new());
    let key: Rc<str> = Rc::from("field");

    object.set_shared_key(Rc::clone(&key), Value::Number(1.0));

    assert!(matches!(
        &*object.0.properties.borrow(),
        PropertyStorage::Small { entries }
            if entries.len() == 1 && Rc::ptr_eq(&entries[0].0, &key)
    ));
    assert_eq!(object.get("field"), Some(Value::Number(1.0)));
}

#[test]
fn small_object_removal_preserves_property_order() {
    let object = ObjectRef::new(HashMap::new());
    object.set("first".to_owned(), Value::Number(1.0));
    object.set("second".to_owned(), Value::Number(2.0));
    object.set("third".to_owned(), Value::Number(3.0));

    assert!(object.delete_own_property("second"));
    object.set("second".to_owned(), Value::Number(4.0));

    assert_eq!(object.own_property_names(), ["first", "third", "second"]);
    assert!(matches!(
        &*object.0.properties.borrow(),
        PropertyStorage::Small { entries } if entries.len() == 3
    ));
}

#[test]
fn small_object_enumerates_indices_before_strings() {
    let object = ObjectRef::new(HashMap::new());
    object.set("10".to_owned(), Value::Number(10.0));
    object.set("label".to_owned(), Value::Number(0.0));
    object.set("2".to_owned(), Value::Number(2.0));

    assert_eq!(object.own_property_names(), ["2", "10", "label"]);
    assert!(matches!(
        &*object.0.properties.borrow(),
        PropertyStorage::Small { entries } if entries.len() == 3
    ));
}

#[test]
fn existing_own_data_write_updates_or_rejects_without_slow_path() {
    let object = ObjectRef::new(HashMap::from([("writable".to_owned(), Value::Number(1.0))]));
    object.define_property(
        "readonly".to_owned(),
        Property::data(Value::Number(2.0), true, false, true),
    );

    assert!(matches!(
        object.write_existing_own_data_property("writable", &Value::Number(3.0)),
        OwnDataPropertyWrite::Written
    ));
    assert_eq!(object.get("writable"), Some(Value::Number(3.0)));
    assert!(matches!(
        object.write_existing_own_data_property("readonly", &Value::Number(4.0)),
        OwnDataPropertyWrite::ReadOnly
    ));
    assert_eq!(object.get("readonly"), Some(Value::Number(2.0)));
    assert!(matches!(
        object.write_existing_own_data_property("missing", &Value::Number(5.0)),
        OwnDataPropertyWrite::NeedsSlowPath
    ));
}

#[test]
fn literal_pair_keeps_inline_values_until_descriptor_mutation() {
    let shape = ObjectLiteralShape::new(vec![Rc::from("a"), Rc::from("b")]);
    let object =
        ObjectRef::with_literal_pair(shape, [Value::Number(1.0), Value::Number(2.0)], None);

    assert!(matches!(
        &*object.0.properties.borrow(),
        PropertyStorage::ShapedPair { .. }
    ));
    assert!(matches!(
        object.write_existing_own_data_property("a", &Value::Number(3.0)),
        OwnDataPropertyWrite::Written
    ));
    assert_eq!(object.get("a"), Some(Value::Number(3.0)));
    assert!(matches!(
        &*object.0.properties.borrow(),
        PropertyStorage::ShapedPair { .. }
    ));

    object.define_property(
        "a".to_owned(),
        Property::data(Value::Number(4.0), false, false, true),
    );
    assert!(matches!(
        &*object.0.properties.borrow(),
        PropertyStorage::Dynamic(_)
    ));
    let descriptor = object.own_property("a").expect("defined property");
    assert_eq!(descriptor.value, Value::Number(4.0));
    assert!(!descriptor.enumerable);
    assert!(!descriptor.writable);
}

#[test]
fn module_namespace_own_data_write_stays_on_slow_path() {
    let object = ObjectRef::new(HashMap::from([("exported".to_owned(), Value::Number(1.0))]));
    object.mark_module_namespace_exotic();

    assert!(matches!(
        object.write_existing_own_data_property("exported", &Value::Number(2.0)),
        OwnDataPropertyWrite::NeedsSlowPath
    ));
    assert_eq!(object.get("exported"), Some(Value::Number(1.0)));
}
